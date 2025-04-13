mod model;

use anyhow::anyhow;
use datatest_stable::harness;
use rusmart_lang_test::run;
use rusmart_smt_derive::model;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::{env, fs};
static ENV_UPDATE_BASELINE: &str = "UPBL";

/// Checks the consistency between the `mod.rs` file and the directory contents.
///
/// This function reads the `mod.rs` file and collects all module names declared in it.
/// Then it reads the directory containing the `mod.rs` file and collects all submodule names,
/// either as subdirectories containing a `mod.rs` file or as `.rs` files.
/// It then cross-checks to ensure that every module declared in `mod.rs` has a corresponding file/directory,
/// and that every file/directory has a corresponding module declaration in `mod.rs`.
///
/// # Arguments
///
/// * `path` - A reference to the `mod.rs` file to check.
///
/// # Returns
///
/// * `datatest_stable::Result<()>` - Returns `Ok(())` if the check passes, or an error otherwise.
fn check_mod(path: &Path) -> datatest_stable::Result<()> {
    // Create a set to store module names declared in mod.rs
    let mut modules = BTreeSet::new();

    // Read the content of the mod.rs file
    let content = fs::read_to_string(path)?;

    // Iterate over each line in the mod.rs file
    for line in content.lines() {
        let line = line.trim(); // Trim whitespace

        if line.is_empty() {
            continue; // Skip empty lines
        }

        // Check if the line starts with "mod " and ends with ";"
        match line.strip_prefix("mod ").and_then(|e| e.strip_suffix(";")) {
            None => {
                // If not, return an error indicating an invalid line
                return Err(anyhow!("invalid line: {}", line).into());
            }
            Some(base) => {
                // If yes, extract the module name and add it to the set
                modules.insert(base.to_string());
            }
        }
    }

    // Create a set to store file/directory names in the same directory as mod.rs
    let mut files = BTreeSet::new();

    // Get the directory containing the mod.rs file
    let path_dir = path.parent().expect("mod directory");

    // Read the directory entries
    for entry in fs::read_dir(path_dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .expect("ascii filename only");

        // Check if the entry is a directory
        if entry.file_type()?.is_dir() {
            // Add the directory name to the set
            files.insert(name);
        } else {
            // For files, check if they are Rust source files (ending with .rs)
            match name.strip_suffix(".rs") {
                None | Some("mod") => continue, // Skip files that are not .rs files or are mod.rs
                Some(base) => {
                    // Add the base name (without .rs extension) to the set
                    files.insert(base.to_string());
                }
            }
        }
    }

    // Cross-check that every module declared in mod.rs has a corresponding file/directory
    for name in &modules {
        if !files.contains(name) {
            return Err(anyhow!("mod {} without backing file", name).into());
        }
    }

    // Cross-check that every file/directory has a corresponding module declaration in mod.rs
    for name in &files {
        if !modules.contains(name) {
            return Err(anyhow!("file {} without backing mod", name).into());
        }
    }

    // All checks passed
    Ok(())
}

/// This function is designed to be used with the `datatest_stable` harness for file-driven tests.
/// It handles test files and compares their output to expected results (if present).
///
/// # Arguments
///
/// * `path` - A reference to the Rust test file to run.
///
/// # Returns
///
/// * `datatest_stable::Result<()>` - Returns `Ok(())` if the test passes, or an error otherwise.
fn test_model(path: &Path) -> datatest_stable::Result<()> {
    // Handle mod.rs files differently by checking module consistency
    let base = path
        .file_name() // gets the final part of the path (for example, in path/to/mod.rs, it would return mod.rs)
        .expect("filename")
        .to_str()
        .expect("ascii-based filename");

    if base == "mod.rs" {
        return check_mod(path);
    }

    let mut expected = BTreeMap::new();
    // get the absolute path of the file and the directory path
    let absolute_path = fs::canonicalize(path).expect("Failed to get absolute path");
    let directory_path = absolute_path.clone().with_extension("");

    // prg.rs will be a file and prg/ will be a directory
    if directory_path.is_dir() && absolute_path.is_file() {
        if let Ok(entries) = fs::read_dir(directory_path.clone()) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    if let Some(file_name) = file_path.file_name().and_then(|f| f.to_str()) {
                        if let Ok(content) = fs::read_to_string(&file_path) {
                            // the expected results when running invoke_backend on the rs file for each model
                            expected.insert(file_name.to_string(), content);
                        }
                    }
                }
            }
        }
    }

    // `UPBL=1 cargo test --test integration` will update the baseline files.
    let update = match env::var_os(ENV_UPDATE_BASELINE) {
        None => false,
        Some(e) => e.into_string().map_or(false, |s| s == "1"),
    };

    // invoke the backend on the rusmart file and get the results (sat, unsat, unknown)
    let actual = run(absolute_path.clone());
    if update {
        for (act_name, act_response) in actual.iter() {
            // create directory of path if it does not exist
            if !directory_path.exists() {
                fs::create_dir_all(&directory_path).expect("Failed to create directory");
            }
            // add act_name to path
            let path = directory_path.join(act_name);
            // write the content of act_response to the file
            fs::write(&path, act_response).expect("Failed to write to file");
        }
    } else {
        if actual.len() != expected.len() {
            return Err(anyhow!("number of files mismatch").into());
        }
        for (act, exp) in actual.iter().zip(expected.iter()) {
            let (act_name, act_response) = act;
            let (exp_name, exp_response) = exp;
            if act_name != exp_name {
                return Err(anyhow!("file name mismatch: {} vs {}", act_name, exp_name).into());
            }
            if act_response != exp_response {
                return Err(
                    anyhow!("output mismatch: {} vs {}", act_response, exp_response).into(),
                );
            }
        }
    }
    // All checks passed
    Ok(())
}

// This macro sets up the datatest harness, which runs `test_model` on all `.rs` files in "tests/model" directory.
// This is done recursively, meaning that all subdirectories are also included.
datatest_stable::harness! {
    { test = test_model, root = "tests/model", pattern = r"^.*\.rs$" },
}

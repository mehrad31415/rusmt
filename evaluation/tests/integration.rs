use anyhow::anyhow;
use datatest_stable::harness;
use rusmart_smt_derive::{model, solve};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::{env, fs};

mod parser;
mod solver;
mod x;

static ENV_UPDATE_BASELINE: &str = "UPBL";

/// Checks the consistency between the `mod.rs` file and the directory contents.
/// This function reads the `mod.rs` file and collects all module names declared in it.
/// Then it reads the directory containing the `mod.rs` file and collects all submodule names,
/// either as subdirectories containing a `mod.rs` file or as `.rs` files.
/// It then cross-checks to ensure that every module declared in `mod.rs` has a corresponding file/directory,
/// and that every file/directory has a corresponding module declaration in `mod.rs`.
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
            let corresponding_rs_file = path_dir.join(format!("{}.rs", name));
            if corresponding_rs_file.is_file() {
                continue; // Skip this directory.
            }
            // For directories, check if they contain a mod.rs file
            if !entry.path().join("mod.rs").is_file() {
                return Err(
                    anyhow!("directory {} without mod.rs -- expected a sub-module", name).into(),
                );
            }
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

/// parser test runner for all test cases.
/// It handles test files and compares their output to expected results (if present).
fn test_parser(path: &Path) -> datatest_stable::Result<()> {
    // Handle mod.rs files differently by checking module consistency
    let base = path
        .file_name() // gets the final part of the path (for example, in path/to/mod.rs, it would return mod.rs)
        .expect("filename")
        .to_str()
        .expect("ascii-based filename");
    if base == "mod.rs" {
        return check_mod(path);
    }

    // Load existing expected output from a corresponding .exp file
    let path_exp = path.with_extension("exp");
    let expected = if path_exp.exists() {
        Some(fs::read_to_string(&path_exp)?)
    } else {
        None
    };

    // Convention: only files ending with `_ok.rs` are expected to pass successfully
    let ok_hint = base.ends_with("_ok.rs");

    // Check whether we need to update the baseline (.exp files) (e.g., when the output has changed)
    // The ENV_UPDATE_BASELINE is "UPBL". The match env::var_os(...) checks whether the environment variable UPBL has been set to 1.
    // `UPBL=1 cargo test` => executing this line in the terminal will render `update` as true.
    // `UPBL=2 cargo test` => update = false
    // `cargo test`        => update = false
    let update = match env::var_os(ENV_UPDATE_BASELINE) {
        None => false,
        Some(e) => e.into_string().map_or(false, |s| s == "1"), // if the e cannot be converted to a string, it returns the default value of false. Otherwise, it checks if the string is equal to "1".
    };

    // Run the model function on the test file
    match (model(path), expected) {
        // Test passed, and no expected error output file exists
        (Ok(_), None) => {
            if !ok_hint {
                // If the file is not supposed to pass (does not end with `_ok.rs`), report an error
                return Err(
                    anyhow!("file {:?} with successful test has no `_ok` suffix", path).into(),
                );
            }
        }
        // Test passed, but there is an expected error output file (test was expected to fail)
        (Ok(_), Some(exp)) => {
            if !update {
                // If not updating the baseline, report an error with the expected failure message
                return Err(anyhow!(
                    "test file {:?} passed while expecting failure\n{}",
                    path,
                    exp
                )
                .into());
            }
            if !ok_hint {
                // If the file is not supposed to pass, report an error
                return Err(
                    anyhow!("file {:?} with successful test has no `_ok` suffix", path).into(),
                );
            }
            // Since updating the baseline, remove the expected error output file
            fs::remove_file(path_exp)?;
        }
        // Test failed, and no expected error output file exists (unexpected failure)
        (Err(err), None) => {
            if !update {
                // If not updating the baseline, report an error with the failure message
                return Err(anyhow!(
                    "test file {:?} failed while expecting success\n{}",
                    path,
                    err
                )
                .into());
            }
            if ok_hint {
                // If the file is supposed to pass, report an error
                return Err(anyhow!("file {:?} with failed test has `_ok` suffix", path).into());
            }
            // Since updating the baseline, write the failure message to the expected output file
            fs::write(path_exp, err.to_string())?;
        }
        // Test failed, and there is an expected error output file (test was expected to fail)
        (Err(err), Some(exp)) => {
            let msg = err.to_string();
            if exp != msg {
                if !update {
                    // If not updating the baseline, report a mismatch between expected and actual outputs
                    return Err(anyhow!(
                        "failure mismatch\n==== expect ===={}\n==== actual ===={}",
                        exp,
                        msg
                    )
                    .into());
                }
                // Since updating the baseline, update the expected output file with the new message
                fs::write(path_exp, msg)?;
            }
            if ok_hint {
                // If the file is supposed to pass, report an error
                return Err(anyhow!("file {:?} with failed test has `_ok` suffix", path).into());
            }
        }
    };

    // All checks passed
    Ok(())
}

fn test_solver(path: &Path) -> datatest_stable::Result<()> {
    // Handle mod.rs files differently by checking module consistency
    let base = path
        .file_name()
        .expect("filename")
        .to_str()
        .expect("ascii-based filename");

    if base == "mod.rs" {
        return check_mod(path);
    }

    let mut expected = BTreeMap::new();

    let absolute_path = fs::canonicalize(path).expect("Failed to get absolute path");
    let directory_path = absolute_path.clone().with_extension("");
    let expected_dir = directory_path.join("expected");

    // prgX.rs will be a file and prgX/expected will be a directory
    if expected_dir.is_dir() && absolute_path.is_file() {
        if let Ok(entries) = fs::read_dir(expected_dir.clone()) {
            for entry in entries.into_iter() {
                let file_path = entry.unwrap().path();
                if file_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        expected.insert(
                            Path::new(file_path.file_name().expect("file should have a name"))
                                .file_stem()
                                .expect("file should have a stem")
                                .to_string_lossy()
                                .into_owned(),
                            content,
                        );
                    }
                }
            }
        }
    } else {
        panic!(
            "expected a directory with name {} and a file with name {}",
            expected_dir.display(),
            absolute_path.display()
        );
    }

    // `UPBL=1 cargo test --test integration` will update the baseline files.
    let update = match env::var_os(ENV_UPDATE_BASELINE) {
        None => false,
        Some(e) => e.into_string().map_or(false, |s| s == "1"),
    };

    // invoke the backend on the rusmart file and get the results (sat, unsat, unknown)
    solve(&model(absolute_path.clone())?, directory_path.clone())?;
    let actual = collect_responses(&directory_path)?;
    if update {
        for (act_name, act_response) in actual.iter() {
            // create directory of path if it does not exist
            if !expected_dir.exists() {
                fs::create_dir_all(&expected_dir).expect("Failed to create directory");
            }
            // add act_name to path
            let path = expected_dir.join(act_name);
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

/// Collects responses from the solver directories.
fn collect_responses(root: &Path) -> datatest_stable::Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("non-utf8 dir name"))?;
            if dir_name == "expected" {
                continue; // Skip the expected directory
            }
            // analyze the solver directories
            if !entry.path().is_dir() {
                continue; // Skip if not a directory
            }
            for inner_entry in fs::read_dir(entry.path())? {
                let inner_entry = inner_entry?;
                if inner_entry.file_type()?.is_dir() {
                    // If the inner entry is a directory and has a response.exp file, read it
                    let resp_path = inner_entry.path().join("response.exp");
                    if resp_path.exists() {
                        let contents = fs::read_to_string(&resp_path)?;
                        map.insert(
                            format!("{}", inner_entry.file_name().to_string_lossy()),
                            contents,
                        );
                    }
                }
            }
        }
    }
    Ok(map)
}

// This macro sets up the datatest harness, which runs `test_parser` on all `.rs` files in "tests/parser" directory (same for solver).
// This is done recursively, meaning that all subdirectories are also included.
harness! {
    // { test = test_parser, root = "tests/s", pattern = r"^.*\.rs$" },
    { test = test_solver, root = "tests/x", pattern = r"^.*\.rs$" },
}

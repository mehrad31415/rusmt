//! Test harness for Rust to SMT translation

mod translation;

use anyhow::anyhow;
use datatest_stable::harness;
use rusmart_smt_derive::derive;
use std::collections::BTreeSet;
use std::path::Path;
use std::{env, fs};

static ENV_UPDATE_BASELINE: &str = "UPBL";

/// Checks the consistency between the `mod.rs` file and the directory contents.
fn check_mod(path: &Path) -> datatest_stable::Result<()> {
    let mut modules = BTreeSet::new();
    let content = fs::read_to_string(path)?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line.strip_prefix("mod ").and_then(|e| e.strip_suffix(";")) {
            None => {
                return Err(anyhow!("invalid line: {}", line).into());
            }
            Some(base) => {
                modules.insert(base.to_string());
            }
        }
    }

    let mut files = BTreeSet::new();
    let path_dir = path.parent().expect("mod directory");

    for entry in fs::read_dir(path_dir)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .expect("ascii filename only");

        if entry.file_type()?.is_dir() {
            if !entry.path().join("mod.rs").is_file() {
                return Err(
                    anyhow!("directory {} without mod.rs -- expected a sub-module", name).into(),
                );
            }
            files.insert(name);
        } else {
            match name.strip_suffix(".rs") {
                None | Some("mod") => continue,
                Some(base) => {
                    files.insert(base.to_string());
                }
            }
        }
    }

    for name in &modules {
        if !files.contains(name) {
            return Err(anyhow!("mod {} without backing file", name).into());
        }
    }

    for name in &files {
        if !modules.contains(name) {
            return Err(anyhow!("file {} without backing mod", name).into());
        }
    }

    Ok(())
}

/// Testing the translation of the program to SMT.
fn test_translation(path: &Path) -> datatest_stable::Result<()> {
    let base = path
        .file_name()
        .expect("filename")
        .to_str()
        .expect("ascii-based filename");

    if base == "mod.rs" {
        return check_mod(path);
    }

    // Convention: only files ending with `_ok.rs` are expected to pass successfully
    let ok_hint = base.ends_with("_ok.rs");

    // Check whether we need to update/regenerate outputs
    // Run with: UPBL=1 cargo test
    let update = match env::var_os(ENV_UPDATE_BASELINE) {
        None => false,
        Some(e) => e.into_string().map_or(false, |s| s == "1"),
    };

    // Output directory is adjacent to test file
    // Example: tests/translation/prog1_basic_types_ok.rs -> tests/translation/prog1_basic_types_ok/
    let output_dir = path.with_extension("");
    
    // Check if we should regenerate or fail on existing output
    if output_dir.exists() && !update {
        return Err(anyhow!(
            "Output already exists: {:?}\nTo regenerate, run: UPBL=1 cargo test",
            output_dir
        ).into());
    }

    // Call derive() - it will:
    // 1. Parse Rust -> IR (may fail here for syntax/semantic errors)
    // 2. Generate SMT code (may fail here for unsupported features)
    // 3. Write to output_dir/<solver_name>/main.smt2
    // 4. Invoke Z3 and write response to output_dir/<solver_name>/response.exp
    // Note: derive() handles deleting output_dir if it exists (line 96-100 in lib.rs)
    let derive_result = derive(path, &output_dir);

    // After derive() runs, check what actually got created
    // Look for any solver subdirectory (z3, z3_chc, etc.)
    let solver_dirs: Vec<_> = if output_dir.exists() {
        fs::read_dir(&output_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect()
    } else {
        vec![]
    };

    // Check the result
    match derive_result {
        Ok(_) => {
            // Derive succeeded
            if !ok_hint {
                return Err(anyhow!(
                    "Test passed but file doesn't have `_ok` suffix: {:?}",
                    path
                ).into());
            }

            // Verify at least one solver directory was created
            if solver_dirs.is_empty() {
                return Err(anyhow!(
                    "Derive succeeded but no solver output directories found in {:?}",
                    output_dir
                ).into());
            }

            // Check each solver directory has both SMT and response files
            for solver_dir in solver_dirs {
                let solver_path = solver_dir.path();
                let smt_files: Vec<_> = fs::read_dir(&solver_path)?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.starts_with("main."))
                            .unwrap_or(false)
                    })
                    .collect();

                let resp_path = solver_path.join("response.exp");

                if smt_files.is_empty() {
                    return Err(anyhow!(
                        "No SMT file generated in {:?}",
                        solver_path
                    ).into());
                }

                if !resp_path.exists() {
                    return Err(anyhow!(
                        "No response file generated in {:?}",
                        solver_path
                    ).into());
                }
            }
        }
        Err(err) => {
            // Derive failed
            if ok_hint {
                return Err(anyhow!(
                    "Test with `_ok` suffix failed: {:?}\nError: {}",
                    path,
                    err
                ).into());
            }
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = test_translation, root = "tests/translation", pattern = r"^.*\.rs$" },
}

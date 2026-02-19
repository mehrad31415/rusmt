//! Test harness for:
//! - IR-only stdlib availability (fast, no solver)
//! - End-to-end translation to SMT-LIB + Z3 execution (baseline-checked)

mod stdlib;
mod translation;

use anyhow::anyhow;
use datatest_stable::harness;
use rusmart_smt_derive::{derive, model};
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
            let dir = entry.path();
            if dir.join("mod.rs").is_file() {
                // A proper Rust submodule directory.
                files.insert(name);
            } else if dir.join("z3_chc").is_dir() {
                // Translation baseline output directory:
                // tests/<suite>/<case>/{z3_chc/main.smt2,z3_chc/response.exp}
                continue;
            } else {
                return Err(
                    anyhow!("directory {} without mod.rs -- expected a sub-module", name).into(),
                );
            }
        } else {
            match name.strip_suffix(".rs") {
                None  => return Err(anyhow!("invalid file: {}", name).into()),
                Some("mod") => continue,
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

/// Testing the stdlib methods available in the DSL.
fn test_stdlib(path: &Path) -> datatest_stable::Result<()> {
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

    // We only require IR model generation here.
    // This ensures that the stdlib methods are available in the DSL.
    let result = model(path);

    // Check the result
    match result {
        Ok(_ir) => {
            // Derive succeeded
            if !ok_hint {
                return Err(
                    anyhow!("Test passed but file doesn't have `_ok` suffix: {:?}", path).into(),
                );
            }
        }
        Err(err) => {
            // Derive failed
            if ok_hint {
                return Err(
                    anyhow!("Test with `_ok` suffix failed: {:?}\nError: {}", path, err).into(),
                );
            }
        }
    }

    Ok(())
}

/// End-to-end translation test:
/// - Parse Rust -> IR
/// - Generate SMT-LIB
/// - Run Z3
/// - Compare `main.smt2` and `response.exp` against checked-in baselines
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

    // Check whether we need to update/regenerate outputs.
    // Run with: UPBL=1 cargo test -p rusmart-programs --test integration
    let update = match env::var_os(ENV_UPDATE_BASELINE) {
        None => false,
        Some(e) => e.into_string().map_or(false, |s| s == "1"),
    };

    // Expected baseline directory is adjacent to test file.
    // Example: tests/translation/basic_types_ok.rs -> tests/translation/basic_types_ok/
    let expected_dir = path.with_extension("");
    let expected_solver_dir = expected_dir.join("z3_chc");
    let expected_resp = expected_solver_dir.join("response.exp");

    if update {
        // Regenerate baselines in-place.
        derive(path, &expected_dir)?;
        return Ok(());
    }

    // Require baselines to exist.
    if !expected_resp.is_file() {
        return Err(anyhow!(
            "Missing translation baselines for {:?}\nExpected files:\n- {:?}\n- {:?}\nTo generate, run: UPBL=1 cargo test -p rusmart-programs --test integration",
            path,
            expected_solver_dir.join("main.smt2"),
            expected_resp,
        )
        .into());
    }

    // Run derive() into a temporary output directory.
    let tmp = tempfile::tempdir()?;
    let out_dir = tmp.path().join(base.strip_suffix(".rs").unwrap_or(base));
    let derive_result = derive(path, &out_dir);

    match derive_result {
        Ok(_) => {
            if !ok_hint {
                return Err(
                    anyhow!("Test passed but file doesn't have `_ok` suffix: {:?}", path).into(),
                );
            }
        }
        Err(err) => {
            if ok_hint {
                return Err(
                    anyhow!("Test with `_ok` suffix failed: {:?}\nError: {}", path, err).into(),
                );
            } else {
                // Expected to fail and did fail.
                return Ok(());
            }
        }
    }

    // Compare outputs.
    let got_solver_dir = out_dir.join("z3_chc");
    let got_resp = got_solver_dir.join("response.exp");

    let exp_resp = fs::read_to_string(&expected_resp)?;
    let got_resp = fs::read_to_string(&got_resp)?;

    if exp_resp != got_resp {
        return Err(anyhow!(
            "Z3 response mismatch for {:?}\nExpected: {:?}\nGot: {:?}\n\nExpected response:\n{}\n\nGot response:\n{}",
            path,
            expected_resp,
            got_solver_dir.join("response.exp"),
            exp_resp,
            got_resp,
        )
        .into());
    }

    Ok(())
}

datatest_stable::harness! {
    { test = test_stdlib, root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/stdlib"), pattern = r"^.*\.rs$" },
    { test = test_translation, root = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/translation"), pattern = r"^.*\.rs$" },
}

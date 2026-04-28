//! This is the main entry point for the derive crate.
//! usage: cargo run -- <parser_name> <top_level_fn> [text|api|both] [k=<N>]
//!
//! `k=<N>` enables bounded-recursion unrolling in the text backend:
//!   k=0 (or omitted) -> recursive define-funs-rec.
//!   k=N (N≥1)        -> every recursive SCC is unrolled to depth N.

use rusmart_smt_derive::{model, solve, solve_z3_api};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = root_crate_dir
        .parent()
        .expect("Failed to find workspace root")
        .parent()
        .expect("Failed to find project root");

    // Path to lang/src which contains the parsers/interpreters
    let lang_src_dir = workspace_root.join("lang").join("src");
    if !lang_src_dir.exists() {
        return Err(format!("Lang source directory not found at {:?}", lang_src_dir).into());
    }
    // Path to lang/synthesis which contains the synthesis outputs
    let synthesis_base = lang_src_dir.join("synthesis");

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 {
        let parser_name = &args[1];
        let top_level_fn = &args[2];
        let parser_dir = lang_src_dir.join(parser_name);
        if !parser_dir.exists() {
            return Err(format!("Parser '{}' not found at {:?}", parser_name, parser_dir).into());
        }
        let output_dir = synthesis_base.join(parser_name);
        
        let mut backend: &str = "text";
        let mut unroll_depth: usize = 0;
        let mut k_set = false;
        let mut backend_set = false;
        for extra in args.iter().skip(3) {
            if let Some(n_str) = extra.strip_prefix("k=") {
                if k_set {
                    return Err(format!("k= specified more than once").into());
                }
                unroll_depth = n_str
                    .parse::<usize>()
                    .map_err(|_| format!("k= must be a non-negative integer, got '{}'", n_str))?;
                k_set = true;
            } else if matches!(extra.as_str(), "api" | "text" | "both") {
                if backend_set {
                    return Err(format!("backend specified more than once").into());
                }
                backend = extra.as_str();
                backend_set = true;
            } else {
                return Err(format!("unrecognized argument: '{}'", extra).into());
            }
        }

        if output_dir.exists() {
            fs::remove_dir_all(&output_dir)?;
        }
        fs::create_dir_all(&output_dir)?;

        let model = model(&parser_dir)?;

        match backend {
            "api" => {
                // Z3 API backend only
                solve_z3_api(
                    &model,
                    Some(top_level_fn.as_str()),
                    &output_dir,
                    unroll_depth,
                )?;
            }
            "text" => {
                // Run the text backend only
                solve(
                    &model,
                    Some(top_level_fn.as_str()),
                    &output_dir,
                    unroll_depth,
                )?;
            }
            "both" => {
                // Run both backends for comparison
                solve(
                    &model,
                    Some(top_level_fn.as_str()),
                    &output_dir,
                    unroll_depth,
                )?;
                solve_z3_api(
                    &model,
                    Some(top_level_fn.as_str()),
                    &output_dir,
                    unroll_depth,
                )?;
            }
            _ => {
                return Err(format!("Invalid backend: {}", backend).into());
            }
        }
    } else {
        return Err(
            "Usage: cargo run -- <parser_name> <top_level_fn> [text|api|both] [k=<N>]\nExample: cargo run -- toml parse_toml k=3".into()
        );
    }

    Ok(())
}

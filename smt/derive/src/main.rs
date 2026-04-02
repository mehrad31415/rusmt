//! This is the main entry point for the derive crate.
//! usage: cargo run -- <parser_name> <top_level_fn>
//! Example: cargo run -- toml parse_toml

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
        // Determine which backend to use
        let backend = args.get(3).map(|s| s.as_str()).unwrap_or("text");

        match backend {
            "api" => {
                // Z3 API backend only
                eprintln!("[z3_api] Creating output directory...");
                fs::create_dir_all(&output_dir)?;
                eprintln!("[z3_api] Building IR model from parser...");
                let models = model(&parser_dir)?;
                eprintln!("[z3_api] IR model built. {} error targets found.", models.error_targets.len());
                eprintln!("[z3_api] Starting Z3 API synthesis...");
                solve_z3_api(&models, Some(top_level_fn.as_str()), &output_dir)?;
                eprintln!("[z3_api] Synthesis complete.");
            }
            "both" => {
                // Run both backends for comparison
                if output_dir.exists() {
                    fs::remove_dir_all(&output_dir)?;
                }
                fs::create_dir_all(&output_dir)?;
                let models = model(&parser_dir)?;
                println!("Running text backend (z3_chc)...");
                solve(&models, Some(top_level_fn.as_str()), &output_dir)?;
                println!("Running Z3 API backend (z3_api)...");
                solve_z3_api(&models, Some(top_level_fn.as_str()), &output_dir)?;
                println!("Both backends complete. Results in {:?}", output_dir);
            }
            _ => {
                // Default: text backend only (original behavior)
                if output_dir.exists() {
                    fs::remove_dir_all(&output_dir)?;
                }
                fs::create_dir_all(&output_dir)?;
                let models = model(&parser_dir)?;
                solve(&models, Some(top_level_fn.as_str()), &output_dir)?;
            }
        }
    } else {
        return Err(
            "Usage: cargo run -- <parser_name> <top_level_fn> [text|api|both]\nExample: cargo run -- toml parse_toml api".into()
        );
    }

    Ok(())
}

//! This is the main entry point for the derive crate.
//! usage: cargo run -- <parser_name>
//! if no parser_name is provided, all parsers will be processed

use rusmart_smt_derive::derive;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    if !synthesis_base.exists() {
        // create the directory
        fs::create_dir_all(&synthesis_base)?;
    }

    // Check if we got a specific parser argument
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Test a specific parser
        let parser_name = &args[1];
        let parser_dir = lang_src_dir.join(parser_name);

        if !parser_dir.exists() {
            return Err(format!("Parser '{}' not found at {:?}", parser_name, parser_dir).into());
        }

        let output_dir = synthesis_base.join(parser_name);
        test_parser(&parser_dir, &output_dir)?;
    } else {
        // Test all parsers
        for entry in fs::read_dir(&lang_src_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only process directories (each parser is in its own directory)
            if path.is_dir() {
                let parser_name = path.file_name().unwrap().to_str().unwrap();
                let output_dir = synthesis_base.join(parser_name);
                test_parser(&path, &output_dir)?;
            }
        }
    }

    Ok(())
}

fn test_parser(parser_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Clean up this parser's previous outputs (not the entire synthesis directory)
    // E.g., deletes lang/synthesis/toml/ but not lang/synthesis/wasm/
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    // Generate SMT from the parser implementation
    derive(parser_dir, output_dir)?;
    Ok(())
}

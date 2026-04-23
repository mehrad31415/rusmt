//! # Executable Semantics of Programming Languages
//!
//! This is the main entry point for running the interpreters on concrete inputs.

use clap::{Parser, Subcommand};
use rusmart_lang::rego::{ParseResult as RegoParseResult, State as RegoState, parse_policy};
use rusmart_lang::toml::{
    ParseResult as TomlParseResult, State as TomlState, default_parser_context, parse_toml,
};
use rusmart_smt_stdlib::{Integer, Seq, String};
use std::fs;
use std::path::PathBuf;

/// Struct representing the command-line interface (CLI).
#[derive(Parser, Debug)]
#[command(name = "rusmart-lang", about, version, rename_all = "kebab-case")]
#[command(
    help_template = "Tool: {name}\nVersion: {version}{about-section}\n{usage-heading} {usage} \n {all-args} {tab}"
)]
struct Cli {
    /// Subcommand
    #[command(subcommand)]
    languages: Languages,
}

/// Enum representing available subcommands for the CLI
/// `cargo run -- toml <file_path>` or `cargo run toml <file_path>`
/// `cargo run -- rego <file_path>` or `cargo run rego <file_path>`
/// `cargo run -- help` (or `cargo run -- --help` or `cargo run help`) shows the help message for the CLI.
/// Note that this is different from `cargo run --help` which shows the help message for the `cargo run` command. Also `cargo help` shows the help message for the `cargo` command.
#[derive(Subcommand, Debug)]
enum Languages {
    // These descriptions are displayed in the help message when the user runs the CLI help command.
    /// Parse and execute a TOML file.
    Toml {
        /// Path to the TOML file to parse.
        #[arg(required = true)]
        file: PathBuf,
    },
    /// Interpret a Rego (subset) policy file.
    Rego {
        /// Path to the Rego policy file to interpret.
        #[arg(required = true)]
        file: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments into the `Cli` struct
    let cli = Cli::parse();

    match cli.languages {
        // `cargo run toml <FILE>`
        Languages::Toml { file: input_file } => {
            println!("[Rusmart] Parsing TOML file: {}", input_file.display());

            // Read the file content into a string.
            // this will trigger an error if the file does not contain valid UTF-8 content.
            let _content = fs::read_to_string(&input_file)
                .map_err(|e| format!("Failed to read file '{}': {}", input_file.display(), e))?;

            let mut char_seq = Seq::new();
            for ch in _content.chars() {
                let smt_char = String::from(ch.to_string());
                char_seq = char_seq.append(smt_char);
            }

            let initial_state = TomlState {
                stream: char_seq,
                cursor: Integer::from(0),
                context: default_parser_context(),
            };

            let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let file_name = format!("{}.txt", input_file.file_stem().unwrap().to_string_lossy());
            match parse_toml(initial_state) {
                TomlParseResult::Ok(toml_value, _) => {
                    let output_path = PathBuf::from(format!(
                        "{}/toml/output/{}",
                        root_crate_dir.display(),
                        file_name
                    ));
                    fs::create_dir_all(output_path.parent().unwrap())?;
                    fs::write(&output_path, format!("{:#?}", toml_value))?;
                    println!("[Rusmart] Successfully parsed TOML file!");
                }
                TomlParseResult::Err(_e) => {
                    println!("[Rusmart] Parse Error occurred!");
                }
                TomlParseResult::NoMatch => panic!("No match found while parsing TOML file."),
            }
        }
        Languages::Rego { file: input_file } => {
            println!("[Rusmart] Interpreting Rego file: {}", input_file.display());

            let _content = fs::read_to_string(&input_file)
                .map_err(|e| format!("Failed to read file '{}': {}", input_file.display(), e))?;

            let mut char_seq = Seq::new();
            for ch in _content.chars() {
                let smt_char = String::from(ch.to_string());
                char_seq = char_seq.append(smt_char);
            }

            let initial_state = RegoState {
                stream: char_seq,
                cursor: Integer::from(0),
            };

            let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let file_name = format!("{}.txt", input_file.file_stem().unwrap().to_string_lossy());
            match parse_policy(initial_state) {
                RegoParseResult::Ok(module, _) => {
                    let output_path = PathBuf::from(format!(
                        "{}/rego/output/{}",
                        root_crate_dir.display(),
                        file_name
                    ));
                    fs::create_dir_all(output_path.parent().unwrap())?;
                    fs::write(&output_path, format!("{:#?}", module))?;
                    println!("[Rusmart] Successfully interpreted Rego file!");
                }
                RegoParseResult::Err(_e) => {
                    println!("[Rusmart] Interpretation Error occurred!");
                }
                RegoParseResult::NoMatch => panic!("No match found while interpreting Rego file."),
            }
        }
    }

    Ok(())
}

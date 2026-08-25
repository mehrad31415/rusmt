//! # Executable Semantics of Programming Languages
//!
//! This is the main entry point for running the executable semantics of programming languages on concrete inputs.

use clap::{Parser, Subcommand, ValueEnum};
use rusmt_lang::imp::{EvalResult, eval_com, parser::format_store, parser::parse_imp_source};
use rusmt_lang::toml::{ParseResult as TomlParseResult, parse_toml};
use rusmt_smt_stdlib::{Seq, U32};
use std::fs;
use std::path::{Path, PathBuf};

/// Struct representing the command-line interface (CLI).
#[derive(Parser, Debug)]
#[command(name = "rusmt-lang", about, version, rename_all = "kebab-case")]
#[command(
    help_template = "Tool: {name}\nVersion: {version}{about-section}\n{usage-heading} {usage} \n {all-args} {tab}"
)]
struct Cli {
    /// Subcommand
    #[command(subcommand)]
    command: Command,
}

/// The object languages a semantics is written for.
#[derive(ValueEnum, Clone, Copy, Debug)]
enum Language {
    Toml,
    Imp,
}

/// The CLI's subcommands.
///
/// `cargo run -- toml <file_path>` or `cargo run toml <file_path>`.
/// `cargo run -- help` or `cargo run help` or `cargo run -- --help` shows this. Note that is different from `cargo run --help` which describes `cargo run` itself.
#[derive(Subcommand, Debug)]
enum Command {
    // These descriptions are displayed in the help message when the user runs the CLI help command.
    /// Parse and execute a TOML file, or every `.toml` file under a directory.
    Toml {
        /// A `.toml` file, or a directory searched recursively for `.toml` files.
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Parse and execute an IMP/WHILE program, or every `.imp` file under a directory.
    Imp {
        /// An `.imp` file, or a directory searched recursively for `.imp` files.
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Print one input's canonical observable behaviour, for differential
    /// comparison against an independent implementation.
    ///
    /// Writes exactly one line and nothing else, one of:
    ///
    ///   OK            the input parsed
    ///   ERR <marker>  it was rejected, naming the error that fired
    ///   NOMATCH       the parser did not match
    ///   TIMEOUT       the parse exceeded its budget
    #[command(verbatim_doc_comment)]
    Observe {
        /// Object language.
        #[arg(required = true, value_enum)]
        language: Language,
        /// The input file.
        #[arg(required = true)]
        path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments into the `Cli` struct
    let cli = Cli::parse();
    let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    match cli.command {
        // `cargo run toml <FILE_OR_DIR>`
        Command::Toml { path } => {
            for file in collect_files(&path, "toml")? {
                if let Err(e) = process_toml_file(&file, &root_crate_dir) {
                    eprintln!("[RuSmt] Skipping '{}': {}", file.display(), e);
                }
            }
        }
        // `cargo run imp <FILE_OR_DIR>`
        Command::Imp { path } => {
            for file in collect_files(&path, "imp")? {
                if let Err(e) = process_imp_file(&file, &root_crate_dir) {
                    eprintln!("[RuSmt] Skipping '{}': {}", file.display(), e);
                }
            }
        }
        // `cargo run observe <toml|imp> <FILE>`
        Command::Observe { language, path } => {
            let source = fs::read_to_string(&path)?;
            let line = match language {
                Language::Toml => {
                    rusmt_lang::certify::observe_toml(&source, rusmt_lang::certify::DEFAULT_BUDGET)
                }
                Language::Imp => rusmt_lang::certify::observe_imp(&source),
            };
            println!("{line}");
        }
    }

    Ok(())
}

/// Resolve an input path into the concrete list of files to process.
fn collect_files(path: &Path, ext: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    if path.is_dir() {
        collect_dir(path, ext, &mut files)?;
        files.sort();
        if files.is_empty() {
            eprintln!(
                "[RuSmt] No '.{}' files found under directory '{}'.",
                ext,
                path.display()
            );
        }
    } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
        files.push(path.to_path_buf());
    } else {
        return Err(format!("Expected a '.{}' file, but got '{}'.", ext, path.display()).into());
    }
    Ok(files)
}

/// Recursively walk `dir`, pushing every file with extension `ext` into `out`.
fn collect_dir(
    dir: &Path,
    ext: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_dir(&entry_path, ext, out)?;
        } else if entry_path.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(entry_path);
        }
    }
    Ok(())
}

/// Parse a single TOML file and write its parsed value to `toml/output/`.
fn process_toml_file(
    input_file: &Path,
    root_crate_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[RuSmt] Parsing TOML file: {}", input_file.display());

    // Read the file content into a string.
    // this will trigger an error if the file does not contain valid UTF-8 content.
    let content = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read file '{}': {}", input_file.display(), e))?;

    let mut codepoint_seq: Seq<U32> = Seq::new();
    for ch in content.chars() {
        codepoint_seq = codepoint_seq.append(U32::from(ch as u32));
    }

    let file_name = format!("{}.txt", input_file.file_stem().unwrap().to_string_lossy());
    match parse_toml(codepoint_seq) {
        TomlParseResult::Ok(toml_value, _) => {
            let output_path = root_crate_dir.join("toml/output").join(&file_name);
            fs::create_dir_all(output_path.parent().unwrap())?;
            fs::write(&output_path, format!("{:#?}", toml_value))?;
            println!("[RuSmt] Successfully parsed TOML file!");
        }
        TomlParseResult::Err(_e) => {
            println!("[RuSmt] Parse Error occurred!");
        }
        TomlParseResult::NoMatch => panic!("No match found while parsing TOML file."),
    }

    Ok(())
}

/// Execute a single IMP/WHILE program and write its final store to `imp/output/`.
fn process_imp_file(
    input_file: &Path,
    root_crate_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[RuSmt] Executing IMP file: {}", input_file.display());

    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read file '{}': {}", input_file.display(), e))?;

    let program = parse_imp_source(&source)
        .map_err(|e| format!("Failed to parse IMP file '{}': {}", input_file.display(), e))?;

    let final_store_text = match eval_com(program) {
        EvalResult::Ok(store) => format_store(store),
        EvalResult::Err(_path) => {
            println!("[RuSmt] Path-condition marker reached during execution.");
            return Ok(());
        }
    };

    let file_name = format!("{}.txt", input_file.file_stem().unwrap().to_string_lossy());
    let output_path = root_crate_dir.join("imp/output").join(&file_name);
    fs::create_dir_all(output_path.parent().unwrap())?;
    fs::write(&output_path, &final_store_text)?;
    println!("[RuSmt] Successfully executed IMP file!");

    Ok(())
}

#[cfg(test)]
mod codepoint_sanity {
    #[test]
    fn ascii_codepoints_match_hex_constants() {
        for (ch, hex) in [
            ('[', 0x5B),
            (']', 0x5D),
            (',', 0x2C),
            ('=', 0x3D),
            ('\n', 0x0A),
            ('\r', 0x0D),
            ('"', 0x22),
            ('\'', 0x27),
            ('{', 0x7B),
            ('}', 0x7D),
            ('.', 0x2E),
            ('#', 0x23),
            ('\t', 0x09),
            (' ', 0x20),
            ('0', 0x30),
            ('9', 0x39),
            ('A', 0x41),
            ('Z', 0x5A),
            ('a', 0x61),
            ('z', 0x7A),
        ] {
            assert_eq!(ch as u32, hex, "{:?}", ch);
        }
    }

    #[test]
    fn invalid_codepoints_are_none() {
        assert!(char::from_u32(0xD800).is_none()); // surrogate start
        assert!(char::from_u32(0xDFFF).is_none()); // surrogate end
        assert!(char::from_u32(0x110000).is_none()); // > 0x10FFFF
    }
}

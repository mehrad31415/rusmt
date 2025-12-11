use rusmart_smt_derive::derive;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Navigate to the TOML source directory
    let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = root_crate_dir
        .parent()
        .expect("Failed to find workspace root")
        .parent()
        .expect("Failed to find project root");
    let toml_src_dir = workspace_root.join("lang").join("src").join("toml");

    // Define where we want the Z3 output files to live
    let output_dir = root_crate_dir.join("z3_synthesis");

    // 2. Clean up previous runs
    if output_dir.exists() {
        println!("Cleaning previous output directory: {:?}", output_dir);
        fs::remove_dir_all(&output_dir)?;
    }

    // 3. Define the maximum error ID you want to solve for
    let max_errors = 10; // Change this to match the highest error ID in your parser

    println!("--- Starting Synthesis Pipeline ---");
    println!("Input Source: {:?}", toml_src_dir);
    println!("Output Dir:   {:?}", output_dir);
    println!("Target Errors: 1..={}", max_errors);
    println!("-----------------------------------");

    // 4. Run the Derive & Solve Pipeline
    // This will generate folders like: z3_synthesis/z3/error_1/
    match derive(&toml_src_dir, &output_dir, max_errors) {
        Ok(_) => println!("\n[Pipeline Complete] Solvers finished executing.\n"),
        Err(e) => {
            eprintln!("\n[Fatal Error] Pipeline failed: {:?}", e);
            std::process::exit(1);
        }
    }
    inspect_results(&output_dir, max_errors);

    Ok(())
}

fn inspect_results(output_root: &Path, max_errors: usize) {
    let z3_dir = output_root.join("z3");

    if !z3_dir.exists() {
        println!("No Z3 output directory found. Did the solver run?");
        return;
    }

    println!("--- Synthesis Results ---");

    for id in 1..=max_errors {
        let error_dir = z3_dir.join(format!("error_{}", id));
        let response_file = error_dir.join("response.exp");

        if response_file.exists() {
            // Read the output from the file written by `solve`
            let content = fs::read_to_string(&response_file).unwrap_or_default();
            let trimmed = content.trim();

            if trimmed.contains("unsat") {
                println!(
                    "Error #{:<3} : [UNREACHABLE] (Input cannot trigger this error)",
                    id
                );
            } else if trimmed.contains("sat") {
                println!("Error #{:<3} : [SUCCESS]     Found triggering input!", id);
                println!("---------------------------------------------------");
                println!("{}", extract_model(trimmed));
                println!("---------------------------------------------------\n");
            } else if trimmed.contains("unknown") {
                println!("Error #{:<3} : [UNKNOWN]     Solver gave up (timeout?)", id);
            } else {
                println!(
                    "Error #{:<3} : [ERROR]       Backend crashed or invalid output",
                    id
                );
            }
        } else {
            println!("Error #{:<3} : [SKIPPED]     No response file found.", id);
        }
    }
}

// Helper to strip "sat" and whitespace to make the output clean
fn extract_model(z3_out: &str) -> String {
    if let Some(idx) = z3_out.find("sat") {
        // "sat" is usually at the start, take everything after it
        let model_part = &z3_out[idx + 3..];
        return model_part.trim().to_string();
    }
    z3_out.to_string()
}

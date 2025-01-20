use log::{debug, info}; // Log messages at different levels (e.g., debug, info)
use std::fs; // File system operations
use std::path::Path; // Path manipulation
use syn::Result; // 'syn' is a parsing library for Rust code, 'Result' is an alias for std::result::Result

use rusmart_utils::config::initialize; // initialize all configs

use crate::backend::error::BackendError; // An error for backend generator (e.g., not supported)
use crate::backend::exec::invoke_backend; // Unified backend generation and execution service
use crate::backend::solvers; // Available list of backend solvers (z3 and cvc5)
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context; // Context manager for holding marked items

// #[allow(dead_code)]
mod analysis;
// #[allow(dead_code)]
mod backend;
// #[allow(dead_code)]
mod ir;
mod parser;

/// Runs the default pipeline after a `Context` is constructed, parsing the input and generating intermediate representations (IR).
///
/// # Arguments
///
/// * `ctxt` - The parsing context containing the input to be processed.
///
/// # Returns
///
/// * A `Result` containing a vector of `IRContext` instances if successful, or an error if parsing fails.
fn pipeline(ctxt: Context) -> Result<Vec<IRContext>> {
    // Chain parsing methods to process generics, types, function signatures, and function bodies.
    let parsed = ctxt
        .parse_generics()?
        .parse_types()?
        .parse_func_sigs()?
        .parse_func_body()?
        .finalize();

    let mut models = vec![];
    // Iterate over all refinements obtained from the parsed context.
    for item in parsed.refinements() {
        debug!("processing verification condition for {}", item);
        // Build the intermediate representation (IR) for each refinement item.
        let ir = IRBuilder::build(&parsed, item);
        models.push(ir);
    }
    Ok(models)
}

/// Internal entrypoint for front-end, processing the input file and generating intermediate representations.
///
/// # Type Parameters
///
/// * `P` - A type that can be converted into a `Path`, typically `&str` or `PathBuf`.
///
/// # Arguments
///
/// * `input` - The path to the input file to be processed.
///
/// # Returns
///
/// * A `Result` containing a vector of `IRContext` instances if successful, or an error if parsing fails.
pub fn model<P: AsRef<Path>>(input: P) -> Result<Vec<IRContext>> {
    // Create a new parsing context from the input file and run the pipeline.
    // the `new` function collects all the smt-marked items from the input file and stores them in the context (types, specs, impls, axioms)
    pipeline(Context::new(input)?)
}

/// Internal entrypoint for back-end, solving the models with available solvers and writing outputs to the specified directory.
///
/// # Type Parameters
///
/// * `P` - A type that can be converted into a `Path`, typically `&str` or `PathBuf`.
///
/// # Arguments
///
/// * `models` - A slice of `IRContext` instances representing the models to be solved.
/// * `output` - The path to the output directory where results will be stored.
///
/// # Panics
///
/// This function will panic if the output directory already exists or if directory creation fails.
pub fn solve<P: AsRef<Path>>(models: &[IRContext], output: P) {
    // Prepare the workspace by ensuring the output directory does not exist and then creating it.
    let output = output.as_ref();
    if output.exists() {
        panic!("output directory exists");
    }
    fs::create_dir_all(output).expect("output directory created"); // Use create_dir_all to create the entire path, including all necessary parent directories.

    // Initialize a counter for naming subdirectories or tracking progress.
    let mut count = 0;
    // Iterate over each model (IRContext).
    for ir in models {
        // For each model, iterate over all available solvers (z3 and cvc5).
        for solver in solvers() {
            count += 1;

            let name = solver.name();
            debug!("[{}] solving {} with {}", count, ir.desc, name);

            // Create a workspace directory for this specific solver run.
            let path_wks = output.join(count.to_string());
            fs::create_dir(&path_wks).expect("workspace freshly created"); // Use create_dir for a single directory when parent directories exist.

            // Invoke the backend solver with the IR, solver, and workspace path.
            match invoke_backend(ir, solver.as_ref(), &path_wks) {
                Ok(response) => {
                    // Log the successful response from the solver.
                    debug!(
                        "[{}] solving {} with {}: {}",
                        count, ir.desc, name, response
                    );
                }
                Err(BackendError::NotSupported) => {
                    // Log if the solver does not support this IR or operation.
                    info!(
                        "[{}] solving {} with {}: not supported",
                        count, ir.desc, name
                    );
                }
            }
        }
    }
}

/// Main entry point that combines front-end and back-end processing, from input to output.
///
/// # Type Parameters
///
/// * `P1` - A type that can be converted into a `Path` for the input.
/// * `P2` - A type that can be converted into a `Path` for the output.
///
/// # Arguments
///
/// * `input` - The path to the input file to be processed.
/// * `output` - The path to the output directory where results will be stored.
///
/// # Returns
///
/// A `Result` indicating success or failure during the processing.
///
/// # Errors
///
/// This function propagates errors from parsing and the pipeline processing.
pub fn derive<P1: AsRef<Path>, P2: AsRef<Path>>(input: P1, output: P2) -> Result<()> {
    // Initialize configurations (e.g., logging, environment variables).
    initialize();

    // Run the pipeline to parse the input and generate models (IRContexts).
    let models = pipeline(Context::new(input)?)?;
    debug!("derivation completed");
    // Solve the models using available solvers and write outputs.
    solve(&models, output);

    Ok(())
}

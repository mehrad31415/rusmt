use log::{debug, info}; // Log messages at different levels (e.g., debug, info)
use rusmart_utils::config::initialize; // initialize all configs
use std::fs; // File system operations
use std::path::{Path, PathBuf}; // Path manipulation
use syn::Result; // 'syn' is a parsing library for Rust code, 'Result' is an alias for std::result::Result
use walkdir::WalkDir;

use crate::backend::codegen::solvers; // Available list of backend solvers: for now z3
use crate::backend::error::BackendError; // An error for backend generator (e.g., not supported)
use crate::backend::exec::{create_smt_file, invoke_backend}; // Unified backend generation and execution service
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context; // Context manager for holding marked items

mod backend;
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
    // println!("parsed context: {:?}", parsed.types);

    let mut models = vec![];
    // Iterate over all refinements obtained from the parsed context.
    for item in parsed.refinements() {
        // println!("processing verification condition for {}", item);
        debug!("processing verification condition for {}", item);
        // Build the intermediate representation (IR) for each refinement item.
        let ir = IRBuilder::build(&parsed, item);
        // println!("IR: {:?}", ir.axiom_registry);
        models.push(ir);
    }
    // println!("models: {:?}", models);
    // println!(
    //     "len_refinements: {}",
    //     parsed.refinements().collect::<Vec<_>>().len()
    // );
    Ok(models)
}

/// Entrypoint for front-end, processing the input file and generating intermediate representations.
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

/// Entrypoint for back-end, solving the models with available solvers and writing outputs to the specified directory.
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
    // ensuring the output directory does not exist and then creating it.
    let output = output.as_ref();
    if output.exists() {
        panic!("output directory exists");
    }
    fs::create_dir(output).expect("output directory created");

    // Initialize a counter for naming subdirectories or tracking progress.
    let mut count = 0;
    // Iterate over each model (IRContext).
    // println!("model_len: {}", models.len());

    let mut paths: Vec<(&IRContext, Box<_>, PathBuf)> = Vec::new();

    // for each rusmart file, we can have a list of models (refinements)
    for ir in models {
        // For each model, iterate over all available solvers (for now, just Z3).
        for solver in solvers() {
            // println!("count: {}", count);
            count += 1;

            let name = solver.name();
            debug!("[{}] solving {} with {}", count, ir.desc, name);

            // Create a workspace directory for this specific solver run.
            let path_wks = output.join(count.to_string());
            fs::create_dir(&path_wks).expect("workspace freshly created"); // Use create_dir for a single directory when parent directories exist.

            // println!("workspace created: {}", path_wks.display());
            paths.push((ir, solver, path_wks.clone()));
        }
    }

    let mut path_sources = Vec::new();
    // first for each model, we create the smt2 file (this is so that if we have an error in the model, we can still see all the smt2 files)
    for (ir, solver, path_wks) in paths.iter() {
        // println!("path_wks: {:?}", path_wks);
        // println!("calling process from loop");
        let path_src = create_smt_file(ir, solver.as_ref(), &path_wks);
        path_sources.push((path_src, ir, solver));
    }

    // then for each model, we invoke the backend solver
    for (path_src, ir, solver) in path_sources.iter() {
        // Invoke the backend solver with the IR, solver, and workspace path.
        match invoke_backend(path_src) {
            Ok(response) => {
                // Log the successful response from the solver.
                debug!(
                    "[{}] solving {} with {}: {}",
                    count,
                    ir.desc,
                    solver.name(),
                    response
                );
            }
            Err(BackendError) => {
                // Log if the solver does not support this IR or operation.
                info!(
                    "[{}] solving {} with {}: not supported",
                    count,
                    ir.desc,
                    solver.name()
                );
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
/// * `input` - The path to the input directory or file to be processed.
/// * `output` - The path to the output directory where results will be stored.
///
/// # Returns
///
/// A `Result` indicating success or failure during the processing.
pub fn derive<P1: AsRef<Path>, P2: AsRef<Path>>(input: P1, output: P2) -> Result<()> {
    // Initialize configurations (e.g., logging, environment variables).
    initialize();

    // ensuring the output directory does not exist and then creating it.
    let output = output.as_ref();
    if output.exists() {
        panic!("output directory exists");
    }
    fs::create_dir_all(output).expect("output directory created"); // Use create_dir_all to create the entire path, including all necessary parent directories.

    let mut count = 0;
    // Run the pipeline to parse the input and generate models (IRContexts).
    for entry in WalkDir::new(input) {
        let entry = entry.expect("input file error"); // Handle errors
        let path = entry.path();

        // if path is mod.rs, or the path is not a file, or the path does not exist, or the path is not a .rs file, continue
        if !path.is_file()
            || !path.exists()
            || path.file_name().unwrap() == "mod.rs"
            || path.extension().is_none()
            || path.extension().unwrap() != "rs"
        {
            continue;
        }
        // else derive the model and solve it
        debug!("deriving model {:?}", path);
        let models = model(path)?;
        debug!("derivation completed for {:?}", path);

        // Solve the models using available solvers and write outputs.
        debug!("solving model {:?}", path);
        let output = output.join(format!("{}", count));
        solve(&models, output);
        debug!("solving completed for {:?}", path);
        count += 1;
    }

    Ok(())
}

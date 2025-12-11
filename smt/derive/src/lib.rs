//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::backend::error::BackendError;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use log::{debug, info};
use rusmart_utils::config::initialize;
use std::fs;
use std::path::Path;
use syn::Result;

// module tree
mod backend;
mod ir;
mod parser;

/// Derive the VCs and solve them
pub fn derive<P1: AsRef<Path> + Clone, P2: AsRef<Path>>(input: P1, output: P2) -> Result<()> {
    // Initialize configurations (e.g., logging, environment variables).
    initialize();

    let output = output.as_ref();
    if output.exists() {
        panic!("output directory exists: {}", output.display());
    }
    // Use create_dir_all to create the entire path, including all necessary parent directories.
    fs::create_dir_all(output).expect("output directory created");

    // Derive the model (IR) from the input source
    debug!("deriving models");
    let input_path = input.as_ref().to_path_buf();
    let models = model(input_path)?;
    debug!("derivation completed");

    // Solve the models using available solvers and write outputs.
    debug!("solving models");
    solve(&models, output)?;
    debug!("solving completed");

    // done
    Ok(())
}

/// Solve the models using available solvers and write outputs.
pub fn solve<P: AsRef<Path>>(models: &IRContext, output: P) -> Result<()> {
    let output = output.as_ref();
    let mut count = 0;

    // For each model, iterate over all available solvers (e.g., Z3, CVC5).
    for solver in solvers() {
        let name = solver.name();

        // Create a workspace directory for the solver if it does not exist.
        let path_solver = output.join(name.clone());
        if !path_solver.exists() {
            fs::create_dir(&path_solver).expect("solver directory created");
        }

        debug!("[{}] solving context with {}", count, name);

        // Create a workspace directory for this specific solver run.
        // <rusmart/studio/native/rego/z3_chc/0> directory
        let path_wks = path_solver.join(&count.to_string());
        if path_wks.exists() {
            panic!("count already exists: {}", path_wks.display());
        }
        // Use create_dir for a single directory when parent directories exist.
        fs::create_dir(&path_wks).expect("workspace freshly created");

        let res = solver.process(ir, &path_wks);
        match res {
            Ok((response, model)) => {
                // Log the successful response from the solver.
                debug!(
                    "[{}] solving context with {}: {} --- model: {:?}",
                    count,
                    solver.name(),
                    response,
                    model
                );
                // Write the response to a file in the workspace directory.
                let path = path_wks.join("response.exp");
                fs::write(path, response.to_string()).expect("failed to write response");
            }
            Err(BackendError) => {
                // Log if the solver does not support this IR or operation.
                info!(
                    "[{}] solving context with {}: not supported",
                    count,
                    solver.name()
                );
            }
        }
        count += 1;
    }
    Ok(())
}

/// Create a new parsing context from the input file and run the pipeline.
pub fn model<P: AsRef<Path>>(input: P) -> Result<IRContext> {
    // The `new` function collects all the smt-marked items from the input file
    // and stores them in the context (types, impls).
    pipeline(Context::new(input)?)
}

/// Create the intermediate representations (IR) from the parsing context.
fn pipeline(ctxt: Context) -> Result<IRContext> {
    // Chain parsing methods to process generics, types, function signatures, and function bodies.
    // This accumulates all necessary definitions into `ContextWithFunc`.
    let parsed = ctxt
        .parse_generics()?
        .parse_types()?
        .parse_func_sigs()?
        .parse_func_body()?;

    debug!("building intermediate representation");

    // Build the Intermediate Representation (IR) for the entire parsed context.
    // This now registers all types and all functions found in the context.
    let ir = IRBuilder::build(&parsed);
    Ok(ir)
}

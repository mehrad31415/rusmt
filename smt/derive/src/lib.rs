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
    fs::create_dir_all(output).expect("output directory created"); // Use create_dir_all to create the entire path, including all necessary parent directories.

    // derive the model and solve it
    debug!("deriving models");
    let input = input.as_ref().to_path_buf();
    let models = model(input.clone())?;
    debug!("derivation completed");

    // Solve the models using available solvers and write outputs.
    debug!("solving models");
    solve(&models, output)?; // TODO: Change this for the languages to do enumerative testing and equivalence testing.
    debug!("solving completed");

    // done
    Ok(())
}

/// Solve the models using available solvers and write outputs.
pub fn solve<P: AsRef<Path>>(models: &[IRContext], output: P) -> Result<()> {
    let output = output.as_ref();
    if !output.exists() {
        panic!("output directory does not exist: {:?}", output);
    }

    let mut count = 0;
    // for each rusmart file, we can have a list of models (refinements)
    for ir in models {
        // For each model, iterate over all available solvers (for now, just Z3).
        for solver in solvers() {
            let name = solver.name();

            // Create a workspace directory for the solver if it does not exist.
            let path_solver = output.join(name.clone());
            if !path_solver.exists() {
                fs::create_dir(&path_solver).expect("solver directory created");
            }

            debug!("[{}] solving {} with {}", count, ir.desc, name);

            // Create a workspace directory for this specific solver run.
            let path_wks = path_solver.join(&count.to_string());
            // <rusmart/studio/native/rego/z3_chc/0> directory
            if path_wks.exists() {
                panic!("count already exists: {}", path_wks.display());
            }
            fs::create_dir(&path_wks).expect("workspace freshly created"); // Use create_dir for a single directory when parent directories exist.

            let res = solver.process(ir, &path_wks);
            match res {
                Ok((response, model)) => {
                    // Log the successful response from the solver.
                    debug!(
                        "[{}] solving {} with {}: {} --- model: {:?}",
                        count,
                        ir.desc,
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
                        "[{}] solving {} with {}: not supported",
                        count,
                        ir.desc,
                        solver.name()
                    );
                }
            }
            count += 1;
        }
    }
    Ok(())
}

/// Create a new parsing context from the input file and run the pipeline.
pub fn model<P: AsRef<Path>>(input: P) -> Result<Vec<IRContext>> {
    // the `new` function collects all the smt-marked items from the input file and stores them in the context (types, specs, impls, axioms)
    pipeline(Context::new(input)?)
}

/// Create the intermediate representations (IR) from the parsing context.
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
        debug!("processing verification condition for {item}");
        // Build the intermediate representation (IR) for each refinement item.
        let ir = IRBuilder::build(&parsed, item);

        models.push(ir);
    }
    Ok(models)
}

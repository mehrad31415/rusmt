//! Pipeline for deriving and solving models from Rust code using SMT solvers.
#![warn(missing_docs)]

use crate::backend::codegen::solvers;
use crate::backend::error::BackendError;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use log::{debug, warn};
use std::fs;
use std::path::Path;
use syn::Result;

// module tree
pub mod backend;
mod ir;
mod parser;

/// Create a new parsing context from the input file and run the pipeline.
pub fn model<P: AsRef<Path>>(input: P) -> Result<IRContext> {
    // The `new` function collects all the smt-marked items from the input file
    // and stores them in the context.
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

    // Build the Intermediate Representation (IR) for the entire parsed context.
    // This now registers all types and all functions found in the context.
    let ir = IRBuilder::build(&parsed);
    Ok(ir)
}

/// Solve the models by synthesizing inputs for specific Error IDs.
pub fn solve<P: AsRef<Path>>(models: &IRContext, output: P) -> Result<()> {
    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./output/z3)
        let path_solver = output.as_ref().join(name.clone());
        // it can never exist beforehand
        fs::create_dir_all(&path_solver).expect("workspace freshly created");
        // 1. Generate SMTLIB2 source code from the IR using the backend's process method.
        let code = solver.process(models);

        // 2. Create path to `main.smt2`.
        let path_src = path_solver.join(format!("{}.{}", "main", solver.flavor()));
        // if the file already exists, panic
        if path_src.exists() {
            panic!("source file already exists");
        }
        // 3. Write the generated code to the file.
        match code {
            Ok(code) => {
                fs::write(&path_src, code)
                    .unwrap_or_else(|e| panic!("IO error on source file: {}", e));

                let resp_path = path_solver.join("response.exp");
                let response = solver.invoke_backend(&path_src);

                match response {
                    Ok(resp) => {
                        // Write the raw response to a file
                        fs::write(&resp_path, resp.to_string()).expect("failed to write response");
                    }
                    Err(BackendError) => {
                        warn!(
                            "[{}] backend invocation failed for {}",
                            name,
                            path_src.display()
                        );
                        // Still write the error to the response file so tests can detect it
                        fs::write(&resp_path, "ERROR: backend invocation failed\n")
                            .expect("failed to write error response");
                    }
                }
            }
            Err(e) => {
                panic!("error generating code: {}", e);
            }
        }
    }
    Ok(())
}

/// Derive the VCs and solve them
pub fn derive<P1: AsRef<Path> + Clone, P2: AsRef<Path>>(input: P1, output: P2) -> Result<()> {
    // Derive the model (IR) from the input source
    debug!("deriving models");
    let models = model(input)?;
    debug!("IR completed");

    let output = output.as_ref();
    // Remove existing output directory if it exists
    if output.exists() {
        fs::remove_dir_all(output).expect("failed to remove existing output directory");
    }
    // Use create_dir_all to create the entire path, including all necessary parent directories.
    fs::create_dir_all(output).expect("output directory created");

    // Solve the models using available solvers and write outputs.
    debug!("solving models");
    solve(&models, output)?;
    debug!("solving completed");

    // done
    Ok(())
}

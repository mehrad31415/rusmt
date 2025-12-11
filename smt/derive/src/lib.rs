//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::backend::error::BackendError;
use crate::backend::exec::{create_smt_file, invoke_backend};
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

/// Create a new parsing context from the input file and run the pipeline.
pub fn model<P: AsRef<Path>>(input: P) -> Result<IRContext> {
    // The `new` function collects all the smt-marked items from the input file
    // and stores them in the context.
    pipeline(Context::new(input)?)
}
/// Solve the models by synthesizing inputs for specific Error IDs.
pub fn solve<P: AsRef<Path>>(models: &IRContext, output: P, max_errors: usize) -> Result<()> {
    let output = output.as_ref();

    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./output/z3)
        let path_solver = output.join(name.clone());
        if !path_solver.exists() {
            fs::create_dir_all(&path_solver).expect("solver directory created");
        }

        for error_id in 1..=max_errors {
            debug!("[{}] synthesizing input for Error #{}", name, error_id);

            let path_wks = path_solver.join(format!("error_{}", error_id));
            if path_wks.exists() {
                // remove existing to ensure fresh run
                fs::remove_dir_all(&path_wks).expect("failed to clean workspace");
            }
            fs::create_dir_all(&path_wks).expect("workspace freshly created");

            // 1. Generate the base SMT file (Definitions only)
            let res = create_smt_file(models, solver.as_ref(), &path_wks);

            match res {
                Ok(path) => {
                    // 2. INJECT THE QUERY
                    if let Err(e) = append_error_query(&path, error_id) {
                        info!("failed to append query for error #{}: {}", error_id, e);
                        continue;
                    }

                    debug!(
                        "[{}] Error #{}: generated SMT file at {}",
                        name,
                        error_id,
                        path.display()
                    );

                    // 3. Invoke the Backend (Run Z3)
                    let response = invoke_backend(&path);
                    match response {
                        Ok(resp) => {
                            // Write the raw response to a file
                            let resp_path = path_wks.join("response.exp");
                            fs::write(&resp_path, resp.to_string())
                                .expect("failed to write response");
                        }
                        Err(BackendError) => {
                            info!("[{}] Error #{}: invocation failed", name, error_id);
                        }
                    }
                }
                Err(BackendError) => {
                    info!(
                        "[{}] solving context with {}: not supported",
                        error_id, name
                    );
                }
            }
        }
    }
    Ok(())
}

/// Helper function to append the assertion logic to the SMT file
fn append_error_query(file_path: &Path, error_id: usize) -> std::io::Result<()> {
    use std::io::Write;
    let query = format!(
        "\n; --- Synthesis Query for Error {id} ---\n(assert (= res (mk-ParseResult-Err (mk-Error {id}))))\n(check-sat)\n(get-model)\n",
        id = error_id
    );

    // Open file in append mode
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(file_path)?;

    write!(file, "{}", query)?;
    Ok(())
}

/// Derive the VCs and solve them
pub fn derive<P1: AsRef<Path> + Clone, P2: AsRef<Path>>(
    input: P1,
    output: P2,
    max_errors: usize,
) -> Result<()> {
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
    solve(&models, output, max_errors)?;
    debug!("solving completed");

    // done
    Ok(())
}

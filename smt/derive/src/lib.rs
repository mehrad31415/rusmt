//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use log::{debug, warn};
use std::fs;
use std::path::Path;
use syn::Result;

// module tree
mod backend;
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

        // Create a root directory for the solver (e.g., ./output/z3_chc)
        let path_solver = output.as_ref().join(name.clone());
        fs::create_dir_all(&path_solver).expect("workspace freshly created");

        // 1. Generate base SMT-LIB (types + functions + error helpers, no queries).
        let base_code = match solver.process(models) {
            Ok(code) => code,
            Err(e) => panic!("error generating SMT-LIB code: {:?}", e),
        };

        // 2. Write main.smt2 (base declarations, no check-sat).
        let path_src = path_solver.join(format!("main.{}", solver.flavor()));
        if path_src.exists() {
            panic!("source file already exists");
        }
        fs::write(&path_src, &base_code)
            .unwrap_or_else(|e| panic!("IO error on source file: {}", e));

        // 3. Run the base file through Z3 to produce response.exp for backward compatibility.
        //    (No check-sat in base file → Z3 returns empty → Response::Unknown.)
        let resp_path = path_solver.join("response.exp");
        match solver.invoke_backend(&path_src) {
            Ok(resp) => {
                fs::write(&resp_path, resp.to_string()).expect("failed to write response");
            }
            Err(x) => {
                warn!(
                    "[{}] base backend invocation failed for {}: {:?}",
                    name,
                    path_src.display(),
                    x
                );
                fs::write(&resp_path, format!("{}", x)).expect("failed to write error response");
            }
        }

        // 4. For each error ID, generate targeted queries and store models.
        let error_count = models.error_count;
        for error_id in 0..error_count {
            let path_error_dir = path_solver.join(format!("error_{}", error_id));
            fs::create_dir_all(&path_error_dir).expect("error directory created");

            let queries = solver.process_error_queries(&base_code, models, error_id);

            if queries.is_empty() {
                // No function directly triggers this error ID — write a note.
                fs::write(
                    path_error_dir.join("response.txt"),
                    format!("no function directly triggers error ID {}\n", error_id),
                )
                .expect("failed to write no-query response");
                continue;
            }

            for (fn_name, query_code) in queries {
                // Write the per-function query file.
                let query_path =
                    path_error_dir.join(format!("{}.{}", fn_name, solver.flavor()));
                fs::write(&query_path, &query_code).expect("failed to write query file");

                // Run Z3 on the query (respects the configured timeout).
                let resp_file =
                    path_error_dir.join(format!("{}.response.txt", fn_name));
                match solver.invoke_backend(&query_path) {
                    Ok(resp) => {
                        fs::write(&resp_file, resp.to_string())
                            .expect("failed to write query response");
                        debug!(
                            "[{}] error {} / fn {}: {}",
                            name, error_id, fn_name, resp
                        );
                    }
                    Err(x) => {
                        warn!(
                            "[{}] backend failed for error {} fn {}: {:?}",
                            name, error_id, fn_name, x
                        );
                        fs::write(&resp_file, format!("backend error: {:?}\n", x))
                            .expect("failed to write error response");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Derive the VCs and solve them
pub fn derive<P1: AsRef<Path> + Clone, P2: AsRef<Path>>(input: P1, output: P2) -> Result<()> {
    // Derive the model (IR) from the input source
    debug!("deriving model");
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

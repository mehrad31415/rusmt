//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
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
    let ir = IRBuilder::build(&parsed);
    Ok(ir)
}

/// Solve the models by synthesizing inputs for specific Error IDs.
pub fn solve<P: AsRef<Path>>(
    models: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
) -> Result<()> {
    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./output/z3_chc)
        let path_solver = output.as_ref().join(name);
        fs::create_dir_all(&path_solver).expect("workspace freshly created");

        // Generate base SMT-LIB (types + functions, no queries).
        let base_code = match solver.process(models) {
            Ok(code) => code,
            Err(e) => panic!("error generating SMT-LIB code: {:?}", e),
        };

        // Write main.smt2 (base declarations, no check-sat).
        let path_src = path_solver.join(format!("main.{}", solver.flavor()));
        fs::write(&path_src, &base_code)
            .unwrap_or_else(|e| panic!("IO error on source file: {}", e));

        // For each error target, generate one query against `top_level_fn` and run it.
        // Skip entirely if no top-level function was specified.
        let Some(top_level_fn) = top_level_fn else {
            continue;
        };
        for (target_idx, target_ids) in models.error_targets.iter().enumerate() {
            let target_label = format!("target_{}", target_idx);
            let path_error_dir = path_solver.join(&target_label);
            fs::create_dir_all(&path_error_dir).expect("error directory created");

            let query_code =
                solver.process_error_queries(&base_code, models, top_level_fn, target_ids);
            let query_path = path_error_dir.join(format!("main.{}", solver.flavor()));
            fs::write(&query_path, &query_code).expect("failed to write query file");

            let resp_file = path_error_dir.join("response.txt");
            match solver.invoke_backend(&query_path) {
                Ok(resp) => {
                    fs::write(&resp_file, resp.to_string())
                        .expect("failed to write query response");
                }
                Err(x) => {
                    fs::write(
                        &resp_file,
                        format!(
                            "[{}] backend failed for {} fn {}: {:?}",
                            name, target_label, top_level_fn, x
                        ),
                    )
                    .expect("failed to write error response");
                }
            }
        }
    }
    Ok(())
}

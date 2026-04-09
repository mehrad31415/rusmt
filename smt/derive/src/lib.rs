//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use proc_macro2::Span;
use std::fs;
use std::path::Path;
use std::time::Instant;
use syn::Result;

// module tree
mod backend;
mod ir;
mod parser;

/// Create the intermediate representations (IR) from the parsing context.
pub fn model<P: AsRef<Path>>(input: P) -> Result<IRContext> {
    // The `new` function collects all the smt-marked items from the input file
    // and stores them in the context.
    let context = Context::new(input)?;

    // Chain parsing methods to process generics, types, function signatures, and function bodies.
    // This accumulates all necessary definitions into `ContextWithFunc`.
    let parsed = context
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
    model: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
) -> Result<()> {
    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./lang/src/synthesis/<parser_name>/<solver_name>)
        let path_solver = output.as_ref().join(name);
        fs::create_dir_all(&path_solver).expect("workspace freshly created");

        // Generate base SMT-LIB (types + functions, no queries).
        let base_code = match solver.process(model) {
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
        for (target_idx, target_ids) in model.error_targets.iter().enumerate() {
            let target_label = format!("target_{}", target_idx);
            let path_error_dir = path_solver.join(&target_label);
            fs::create_dir_all(&path_error_dir).expect("error directory created");

            let query_code =
                solver.process_error_queries(&base_code, model, top_level_fn, target_ids);
            let query_path = path_error_dir.join(format!("main.{}", solver.flavor()));
            fs::write(&query_path, &query_code).expect("failed to write query file");

            let resp_file = path_error_dir.join("response.txt");
            let timing_file = path_error_dir.join("timing.txt");
            let start = Instant::now();
            match solver.invoke_backend(&query_path) {
                Ok(resp) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    fs::write(&resp_file, resp.to_string())
                        .expect("failed to write query response");
                    fs::write(&timing_file, format!("{}ms", elapsed_ms))
                        .expect("failed to write timing file");
                }
                Err(x) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    fs::write(
                        &resp_file,
                        format!(
                            "[{}] backend failed for {} fn {}: {:?}",
                            name, target_label, top_level_fn, x
                        ),
                    )
                    .expect("failed to write error response");
                    fs::write(&timing_file, format!("{}ms", elapsed_ms))
                        .expect("failed to write timing file");
                }
            }
        }
    }
    Ok(())
}

/// Solve using the Z3 API backend.
/// Results are written to `lang/src/synthesis/<parser_name>/z3_api/target_N/response.txt` incrementally.
pub fn solve_z3_api<P: AsRef<Path>>(
    model: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
) -> Result<()> {
    // let path_solver = output.as_ref().join("z3_api");
    // fs::create_dir_all(&path_solver).expect("workspace freshly created");

    // // Skip entirely if no top-level function was specified.
    // let Some(top_level_fn) = top_level_fn else {
    //     return Err(syn::Error::new(Span::call_site(), "No top-level function specified"));
    // };

    // // Write results incrementally as each target completes
    // let write_result = |result: &backend::z3_api::solver::SolveResult| {
    //     let target_label = format!("target_{}", result.target_idx);
    //     let path_error_dir = path_solver.join(&target_label);
    //     fs::create_dir_all(&path_error_dir).expect("error directory created");

    //     let resp_file = path_error_dir.join("response.txt");
    //     fs::write(&resp_file, result.response.to_string()).expect("failed to write query response");

    //     let timing_file = path_error_dir.join("timing.txt");
    //     fs::write(&timing_file, format!("{}ms", result.elapsed_ms))
    //         .expect("failed to write timing file");
    // };

    // backend::z3_api::solver::solve_with_api(model, top_level_fn, &write_result);

    Ok(())
}

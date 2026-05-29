//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use rusmt_lang::imp_render::render_response;
use std::fs;
use std::path::Path;
use std::time::Instant;
use syn::Result;

/// For the `imp` case study, a `sat` model is stored as the rendered `.imp`
/// program (the inverse of the Z3 encoding) rather than the raw solver text; any
/// other parser, or a non-`sat` response, is stored verbatim.
fn response_to_store(parser_dir: &Path, raw: String) -> String {
    let is_imp = parser_dir.file_name().and_then(|s| s.to_str()) == Some("imp");
    if is_imp {
        render_response(&raw).unwrap_or(raw)
    } else {
        raw
    }
}

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

/// Solve the models by synthesizing inputs for specific Path IDs.
///
/// `unroll_depth` controls bounded-recursion unrolling in the text backend
/// (see `CodeGen::process`); pass 0 or not specified to keep the existing recursive emission.
///
/// When the env var `RUSMT_SKIP_INVOKE=1` is set, the per-target Z3 invocation
/// is skipped — only the SMT files are generated. Useful when iterating on the
/// codegen and you want to inspect / spot-check a few targets manually.
pub fn solve<P: AsRef<Path>>(
    model: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
    unroll_depth: usize,
) -> Result<()> {
    let skip_invoke = std::env::var("RUSMT_SKIP_INVOKE").ok().as_deref() == Some("1");
    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./lang/src/synthesis/<parser_name>/<solver_name>)
        let path_solver = output.as_ref().join(name);
        fs::create_dir_all(&path_solver).expect("workspace freshly created");

        // Generate base SMT-LIB (types + functions, no queries).
        let base_code = match solver.process(model, unroll_depth) {
            Ok(code) => code,
            Err(e) => panic!("error generating SMT-LIB code: {:?}", e),
        };

        // Write main.smt2 (base declarations, no check-sat).
        let path_src = path_solver.join(format!("main.{}", solver.flavor()));
        fs::write(&path_src, &base_code)
            .unwrap_or_else(|e| panic!("IO error on source file: {}", e));

        // For each path-marker target, generate one query against `top_level_fn` and run it.
        // Skip entirely if no top-level function was specified.
        let Some(top_level_fn) = top_level_fn else {
            continue;
        };
        for (target_idx, target_ids) in model.path_targets.iter().enumerate() {
            let target_label = format!("target_{}", target_idx);
            let path_target_dir = path_solver.join(&target_label);
            fs::create_dir_all(&path_target_dir).expect("target directory created");

            let query_code =
                solver.process_path_queries(&base_code, model, top_level_fn, target_ids);
            let query_path = path_target_dir.join(format!("main.{}", solver.flavor()));
            fs::write(&query_path, &query_code).expect("failed to write query file");

            if skip_invoke {
                continue;
            }

            let resp_file = path_target_dir.join("response.txt");
            let timing_file = path_target_dir.join("timing.txt");
            let start = Instant::now();
            match solver.invoke_backend(&query_path) {
                Ok(resp) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    let body = response_to_store(output.as_ref(), resp.to_string());
                    fs::write(&resp_file, body).expect("failed to write query response");
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
/// Results are written to `lang/src/synthesis/<parser_name>/z3_api/target_N/response.txt` and `timing.txt`.
///
/// `unroll_depth` controls bounded-recursion unrolling.
pub fn solve_z3_api<P: AsRef<Path>>(
    model: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
    unroll_depth: usize,
) -> Result<()> {
    let path_solver = output.as_ref().join("z3_api");
    fs::create_dir_all(&path_solver).expect("workspace freshly created");

    let Some(top_level_fn) = top_level_fn else {
        return Ok(());
    };

    let write_result = |result: &backend::z3_api::SolveResult| {
        let target_label = format!("target_{}", result.target_idx);
        let path_target_dir = path_solver.join(&target_label);
        fs::create_dir_all(&path_target_dir).expect("target directory created");

        let resp_file = path_target_dir.join("response.txt");
        let body = response_to_store(output.as_ref(), result.response.to_string());
        fs::write(&resp_file, body).expect("failed to write query response");

        let timing_file = path_target_dir.join("timing.txt");
        fs::write(&timing_file, format!("{}ms", result.elapsed_ms))
            .expect("failed to write timing file");
    };

    backend::z3_api::solver::solve_with_api(model, top_level_fn, &write_result, unroll_depth);

    Ok(())
}

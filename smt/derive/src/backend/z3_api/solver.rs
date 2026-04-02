//! Solver pipeline: uses in-process Z3 via z3-sys to solve error targets.
//!
//! Strategy: load base definitions via Z3_eval_smtlib2_string (which handles
//! declare-datatypes, define-funs-rec, etc.), then load assertions via
//! Z3_parse_smtlib2_string and solve with Z3_solver_check (which respects timeout).

use crate::backend::codegen::CodeGen;
use crate::backend::response::{Response, BACKEND_TIMEOUT};
use crate::backend::z3::ctxt::CodeGenZ3;
pub use crate::backend::z3_api::SolveResult;
use crate::backend::z3_api::{mk_string_symbol, model_to_string, Z3Context};
use crate::ir::ctxt::IRContext;
use std::collections::BTreeSet;
use std::ffi::CStr;
use std::time::{Duration, Instant};

fn set_global_params() {
    z3::set_global_param("sat.random_seed", "42");
    z3::set_global_param("smt.random_seed", "42");
    z3::set_global_param("parallel.enable", "false");
    z3::set_global_param("sat.restart.max", "100000");
    z3::set_global_param("smt.arith.solver", "6");
    z3::set_global_param("smt.case_split", "3");
    z3::set_global_param("smt.phase_selection", "3");
    z3::set_global_param("smt.mbqi", "true");
    z3::set_global_param("smt.qi.eager_threshold", "10.0");
    z3::set_global_param("smt.qi.max_multi_patterns", "1000");
    z3::set_global_param("smt.ematching", "true");
    z3::set_global_param("smt.auto_config", "false");
}

/// Run the Z3 API solver on the given IR.
pub fn solve_with_api(
    ir: &IRContext,
    top_level_fn: Option<&str>,
    on_result: &dyn Fn(&SolveResult),
) {
    let Some(top_level_fn) = top_level_fn else {
        return;
    };

    eprintln!("[z3_api] Setting global Z3 params...");
    set_global_params();

    let text_backend = CodeGenZ3::new();
    let base_code = match text_backend.process(ir) {
        Ok(code) => code,
        Err(_) => {
            eprintln!("[z3_api] ERROR: Failed to generate base SMT-LIB2 code");
            return;
        }
    };
    eprintln!("[z3_api] Base SMT-LIB2 generated ({} bytes)", base_code.len());

    let api_timeout_ms = std::env::var("Z3_API_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(BACKEND_TIMEOUT.as_millis() as u64);

    eprintln!(
        "[z3_api] Processing {} error targets (timeout={}ms)...",
        ir.error_targets.len(),
        api_timeout_ms
    );

    for (target_idx, target_ids) in ir.error_targets.iter().enumerate() {
        eprintln!(
            "[z3_api] Target {}/{}: solving...",
            target_idx,
            ir.error_targets.len()
        );
        let start = Instant::now();

        let response = solve_single_target(
            &text_backend,
            &base_code,
            ir,
            top_level_fn,
            target_ids,
            api_timeout_ms,
        );

        let elapsed_ms = start.elapsed().as_millis();
        let status = match &response {
            Response::Sat(_) => "sat",
            Response::Unsat => "unsat",
            Response::Unknown => "unknown",
            Response::Timeout => "timeout",
        };
        eprintln!("[z3_api] Target {}: {} in {}ms", target_idx, status, elapsed_ms);

        on_result(&SolveResult {
            target_idx,
            response,
            elapsed_ms,
        });
    }
}

/// Solve a single error target.
///
/// Approach:
/// 1. Create a fresh Z3 context
/// 2. Load base definitions (types + functions) via Z3_eval_smtlib2_string
/// 3. Generate the error-specific query (declare-const + assert)
/// 4. Parse the assertions via Z3_parse_smtlib2_string
/// 5. Add assertions to a solver with timeout
/// 6. Call Z3_solver_check (which respects the timeout)
/// 7. Extract model if sat
fn solve_single_target(
    text_backend: &CodeGenZ3,
    base_code: &str,
    ir: &IRContext,
    top_level_fn: &str,
    target_ids: &BTreeSet<usize>,
    timeout_ms: u64,
) -> Response {
    let z3_ctx = Z3Context::new();
    let ctx = z3_ctx.ctx;

    unsafe {
        // Step 1: Load base definitions
        let base_c = std::ffi::CString::new(base_code).unwrap();
        z3_sys::Z3_eval_smtlib2_string(ctx, base_c.as_ptr());

        // Step 2: Generate the error-specific query
        // process_error_queries appends: declare-const, assert, check-sat, get-model
        let full_query = text_backend.process_error_queries(base_code, ir, top_level_fn, target_ids);

        // Extract ONLY the error-specific part (after the base code)
        let error_part = &full_query[base_code.len()..];

        // We need to separate declarations from assertions.
        // The error part looks like:
        //   (declare-const input_0 SomeSort)
        //   ...
        //   ; Find input to parse_toml that triggers error IDs {N}
        //   (assert ...)
        //   (check-sat)
        //   (get-model)
        //
        // Load declare-const via eval, then parse the assertion.
        let mut decl_part = String::new();
        let mut assert_part = String::new();

        for line in error_part.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("(declare-const") || trimmed.starts_with(";") {
                decl_part.push_str(line);
                decl_part.push('\n');
            } else if trimmed.starts_with("(assert") {
                assert_part.push_str(line);
                assert_part.push('\n');
            }
            // Skip (check-sat) and (get-model) - we'll use the solver API
        }

        // Load declarations
        if !decl_part.is_empty() {
            let decl_c = std::ffi::CString::new(decl_part).unwrap();
            z3_sys::Z3_eval_smtlib2_string(ctx, decl_c.as_ptr());
        }

        // Parse assertions
        let assert_c = std::ffi::CString::new(assert_part).unwrap();
        let assertions = z3_sys::Z3_parse_smtlib2_string(
            ctx,
            assert_c.as_ptr(),
            0, std::ptr::null(), std::ptr::null(),
            0, std::ptr::null(), std::ptr::null(),
        ).expect("Z3_parse_smtlib2_string");
        z3_sys::Z3_ast_vector_inc_ref(ctx, assertions);

        // Create solver with timeout
        let solver = z3_sys::Z3_mk_solver(ctx).expect("Z3_mk_solver");
        z3_sys::Z3_solver_inc_ref(ctx, solver);

        let params = z3_sys::Z3_mk_params(ctx).expect("Z3_mk_params");
        z3_sys::Z3_params_inc_ref(ctx, params);
        let timeout_sym = mk_string_symbol(ctx, "timeout");
        z3_sys::Z3_params_set_uint(ctx, params, timeout_sym, timeout_ms as u32);
        z3_sys::Z3_solver_set_params(ctx, solver, params);
        z3_sys::Z3_params_dec_ref(ctx, params);

        // Add all parsed assertions to solver
        let n_asserts = z3_sys::Z3_ast_vector_size(ctx, assertions);
        for i in 0..n_asserts {
            let ast = z3_sys::Z3_ast_vector_get(ctx, assertions, i).expect("Z3_ast_vector_get");
            z3_sys::Z3_solver_assert(ctx, solver, ast);
        }
        z3_sys::Z3_ast_vector_dec_ref(ctx, assertions);

        // Solve!
        let start = Instant::now();
        let check_result = z3_sys::Z3_solver_check(ctx, solver);

        let response = match check_result {
            z3_sys::Z3_L_TRUE => {
                let model = z3_sys::Z3_solver_get_model(ctx, solver);
                if model.is_none() {
                    Response::Sat("sat\n(model not available)".to_string())
                } else {
                    let model = model.expect("model");
                    z3_sys::Z3_model_inc_ref(ctx, model);
                    let model_str = model_to_string(ctx, model);
                    z3_sys::Z3_model_dec_ref(ctx, model);
                    Response::Sat(format!("sat\n{}", model_str))
                }
            }
            z3_sys::Z3_L_FALSE => Response::Unsat,
            _ => {
                if start.elapsed() >= Duration::from_millis(timeout_ms) {
                    Response::Timeout
                } else {
                    Response::Unknown
                }
            }
        };

        z3_sys::Z3_solver_dec_ref(ctx, solver);
        response
    }
}

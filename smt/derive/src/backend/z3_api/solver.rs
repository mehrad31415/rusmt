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
        let eval_result = z3_sys::Z3_eval_smtlib2_string(ctx, base_c.as_ptr());
        let eval_str = if eval_result.is_null() {
            eprintln!("[z3_api] Z3_eval_smtlib2_string returned NULL");
            String::new()
        } else {
            let s = CStr::from_ptr(eval_result).to_string_lossy().into_owned();
            if !s.is_empty() {
                // Print first 500 chars to see what Z3 responded
                let preview: String = s.chars().take(500).collect();
                eprintln!("[z3_api] Z3_eval output (first 500 chars): {}", preview);
            } else {
                eprintln!("[z3_api] Z3_eval output: (empty string)");
            }
            s
        };
        // Check for errors
        let err = z3_sys::Z3_get_error_code(ctx);
        if err != z3_sys::ErrorCode::Ok {
            let err_msg = z3_sys::Z3_get_error_msg(ctx, err);
            let msg = CStr::from_ptr(err_msg).to_string_lossy();
            eprintln!("[z3_api] ERROR after loading base definitions: {} (code {:?})", msg, err);
            return Response::Unknown;
        }
        // Check if eval output contains errors
        if eval_str.contains("(error") {
            eprintln!("[z3_api] Z3 reported errors in base definitions!");
            eprintln!("[z3_api] {}", eval_str);
            return Response::Unknown;
        }

        // Step 2: Generate the full query (base + declarations + assert + check-sat + get-model)
        let full_query = text_backend.process_error_queries(base_code, ir, top_level_fn, target_ids);

        // Extract ONLY the error-specific part (after the base code)
        let error_part = &full_query[base_code.len()..];

        // Feed the error-specific part (declare-const, assert, check-sat, get-model)
        // via Z3_eval_smtlib2_string — it runs in the same context where base defs are loaded
        let error_c = std::ffi::CString::new(error_part).unwrap();

        let start = Instant::now();
        let result_ptr = z3_sys::Z3_eval_smtlib2_string(ctx, error_c.as_ptr());

        let output = if result_ptr.is_null() {
            String::new()
        } else {
            CStr::from_ptr(result_ptr).to_string_lossy().into_owned()
        };

        // Parse the output — Z3_eval_smtlib2_string returns the combined output
        // of all commands (check-sat prints "sat"/"unsat", get-model prints the model)
        let verdict = output
            .lines()
            .map(|l| l.trim())
            .find(|&l| l == "sat" || l == "unsat" || l == "unknown");

        let response = match verdict {
            Some("sat") => Response::Sat(output),
            Some("unsat") => Response::Unsat,
            Some("unknown") => {
                if start.elapsed() >= Duration::from_millis(timeout_ms) {
                    Response::Timeout
                } else {
                    Response::Unknown
                }
            }
            _ => {
                // No verdict — could be timeout or error
                if start.elapsed() >= Duration::from_millis(timeout_ms) {
                    Response::Timeout
                } else {
                    eprintln!("[z3_api] No verdict in output: {}", &output[..output.len().min(200)]);
                    Response::Unknown
                }
            }
        };

        response
    }
}

//! Solver pipeline: uses in-process Z3 via z3-sys to solve error targets.
//!
//! All Z3 interaction is through the C API — no SMT-LIB2 text generation.

use crate::backend::response::{BACKEND_TIMEOUT, Response};
use crate::backend::z3::sort::resolve_type_name;
pub use crate::backend::z3_api::SolveResult;
use crate::backend::z3_api::context::Z3ApiContext;
use crate::backend::z3_api::{mk_string_symbol, model_to_string, Z3Context};
use crate::ir::ctxt::IRContext;
use crate::ir::index::UsrFunId;
use crate::ir::sort::{DataType, Sort, Variant};
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

/// Run the Z3 API solver on the given model.
pub fn solve_with_api(model: &IRContext, top_level_fn: &str, on_result: &dyn Fn(&SolveResult)) {
    set_global_params();

    let api_timeout_ms = std::env::var("Z3_API_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(BACKEND_TIMEOUT.as_millis() as u64);

    eprintln!(
        "[z3_api] Processing {} error targets (timeout={}ms)...",
        model.error_targets.len(),
        api_timeout_ms
    );

    // Find the top-level function ID.
    let top_fn_id = find_top_level_fn(model, top_level_fn);

    for (target_idx, target_ids) in model.error_targets.iter().enumerate() {
        eprintln!(
            "[z3_api] Target {}/{}: solving...",
            target_idx,
            model.error_targets.len()
        );
        let start = Instant::now();

        let response =
            solve_single_target(model, top_fn_id, target_ids, api_timeout_ms);

        let elapsed_ms = start.elapsed().as_millis();
        let status = match &response {
            Response::Sat(_) => "sat",
            Response::Unsat => "unsat",
            Response::Unknown => "unknown",
            Response::Timeout => "timeout",
        };
        eprintln!(
            "[z3_api] Target {}: {} in {}ms",
            target_idx, status, elapsed_ms
        );

        on_result(&SolveResult {
            target_idx,
            response,
            elapsed_ms,
        });
    }
}

/// Find the monomorphic top-level function by name.
fn find_top_level_fn(ir: &IRContext, name: &str) -> UsrFunId {
    let instantiations = ir
        .fn_registry
        .lookup
        .iter()
        .find(|(n, _)| n.as_ref() == name)
        .unwrap_or_else(|| {
            panic!("top-level function '{}' not found in function registry", name)
        })
        .1;
    assert_eq!(
        instantiations.len(), 1,
        "top-level function '{}' must be monomorphic, found {} instantiations",
        name, instantiations.len()
    );
    *instantiations.values().next().unwrap()
}

/// Solve a single error target using a fresh Z3 context.
fn solve_single_target(
    ir: &IRContext,
    top_fn_id: UsrFunId,
    target_ids: &BTreeSet<usize>,
    timeout_ms: u64,
) -> Response {
    // Create fresh Z3 context and build everything from IR.
    let z3_ctx = Z3Context::new();
    let ctx = z3_ctx.ctx;
    let mut api_ctx = Z3ApiContext::new(ctx, ir);

    unsafe {
        // Create solver.
        let solver = z3_sys::Z3_mk_solver(ctx).expect("mk_solver");
        z3_sys::Z3_solver_inc_ref(ctx, solver);

        // Set timeout via solver params.
        let params = z3_sys::Z3_mk_params(ctx).expect("mk_params");
        z3_sys::Z3_params_inc_ref(ctx, params);
        let timeout_sym = mk_string_symbol(ctx, "timeout");
        z3_sys::Z3_params_set_uint(ctx, params, timeout_sym, timeout_ms as u32);
        z3_sys::Z3_solver_set_params(ctx, solver, params);
        z3_sys::Z3_params_dec_ref(ctx, params);

        // Build and assert error reachability assertion.
        let assertion = build_error_assertion(&mut api_ctx, ir, top_fn_id, target_ids);
        z3_sys::Z3_solver_assert(ctx, solver, assertion);

        // Backup timeout: interrupt Z3 from a separate thread.
        // Z3_context is NonNull<_> (not Send). Smuggle as usize.
        let ctx_addr = ctx.as_ptr() as usize;
        let timeout_dur = Duration::from_millis(timeout_ms);
        let start = Instant::now();
        let interrupt_handle = std::thread::spawn(move || {
            std::thread::sleep(timeout_dur);
            unsafe {
                if let Some(ctx_nn) = std::ptr::NonNull::new(ctx_addr as *mut _) {
                    z3_sys::Z3_interrupt(ctx_nn);
                }
            }
        });

        // Solve.
        let check_result = z3_sys::Z3_solver_check(ctx, solver);

        let response = match check_result {
            z3_sys::Z3_L_TRUE => {
                match z3_sys::Z3_solver_get_model(ctx, solver) {
                    Some(model) => {
                        z3_sys::Z3_model_inc_ref(ctx, model);
                        let model_str = model_to_string(ctx, model);
                        z3_sys::Z3_model_dec_ref(ctx, model);
                        Response::Sat(format!("sat\n{}", model_str))
                    }
                    None => {
                        eprintln!("[z3_api] sat but model is null");
                        Response::Sat("sat\n(model unavailable)".to_string())
                    }
                }
            }
            z3_sys::Z3_L_FALSE => Response::Unsat,
            _ => {
                // Z3_L_UNDEF — could be timeout or unknown.
                if start.elapsed() >= timeout_dur {
                    Response::Timeout
                } else {
                    // Check reason string.
                    let reason_ptr = z3_sys::Z3_solver_get_reason_unknown(ctx, solver);
                    let reason = if reason_ptr.is_null() {
                        "unknown".to_string()
                    } else {
                        CStr::from_ptr(reason_ptr).to_string_lossy().into_owned()
                    };
                    if reason.contains("timeout") || reason.contains("canceled") || reason.contains("interrupted") {
                        Response::Timeout
                    } else {
                        eprintln!("[z3_api] unknown reason: {}", reason);
                        Response::Unknown
                    }
                }
            }
        };

        z3_sys::Z3_solver_dec_ref(ctx, solver);

        // Cancel the interrupt thread (it will fire harmlessly if already past).
        drop(interrupt_handle);

        response
    }
}

/// Build the error reachability assertion for the given target error IDs.
///
/// This is the API equivalent of `extract_error_assertion` in the text backend.
/// For a top-level function returning `ParseResult<T>` (an enum with an `Err(Error)` variant):
///   assert (and (is-Err (parse_toml input_0)) (select (accessor (parse_toml input_0)) error_id))
unsafe fn build_error_assertion(
    api_ctx: &mut Z3ApiContext,
    ir: &IRContext,
    top_fn_id: UsrFunId,
    target_ids: &BTreeSet<usize>,
) -> z3_sys::Z3_ast {
    let ctx = api_ctx.ctx;
    let sig = ir.fn_registry.retrieve_sig(top_fn_id);
    let func_decl = api_ctx.get_func_decl(top_fn_id);

    // Create input constants (one per parameter).
    let mut input_asts: Vec<z3_sys::Z3_ast> = Vec::new();
    for (i, (_, param_sort)) in sig.params.iter().enumerate() {
        let z3_sort = api_ctx.translate_sort(param_sort);
        let name = format!("input_{}", i);
        let sym = mk_string_symbol(ctx, &name);
        let c = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("mk_const");
        z3_sys::Z3_inc_ref(ctx, c);
        input_asts.push(c);
    }

    // Build function application: (top_fn input_0 input_1 ...)
    let call_result = z3_sys::Z3_mk_app(
        ctx, func_decl,
        input_asts.len() as u32, input_asts.as_ptr(),
    ).expect("mk_app");
    z3_sys::Z3_inc_ref(ctx, call_result);

    // Build membership assertions for each error_id in the target.
    let member_assertions: Vec<z3_sys::Z3_ast> = target_ids
        .iter()
        .map(|&error_id| build_single_error_assertion(api_ctx, ir, &sig.ret_ty, call_result, error_id))
        .collect();

    // Combine: single → use directly; multiple → AND them.
    let assertion = if member_assertions.len() == 1 {
        member_assertions[0]
    } else {
        z3_sys::Z3_mk_and(ctx, member_assertions.len() as u32, member_assertions.as_ptr()).expect("mk_and")
    };

    assertion
}

/// Build a single error membership assertion for one error_id.
///
/// Handles two cases:
/// - Sort::Error → (select call_result error_id)
/// - Sort::User (enum) → scan variants for Error fields, build (and (is-Variant call) (select (accessor call) error_id))
unsafe fn build_single_error_assertion(
    api_ctx: &mut Z3ApiContext,
    ir: &IRContext,
    ret_sort: &Sort,
    call_result: z3_sys::Z3_ast,
    error_id: usize,
) -> z3_sys::Z3_ast {
    let ctx = api_ctx.ctx;
    let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
    let idx_str = std::ffi::CString::new(error_id.to_string()).unwrap();
    let idx_ast = z3_sys::Z3_mk_numeral(ctx, idx_str.as_ptr(), int_sort).expect("mk_numeral");

    match ret_sort {
        Sort::Error => {
            // Direct: (select call_result error_id)
            z3_sys::Z3_mk_select(ctx, call_result, idx_ast).expect("mk_select")
        }
        Sort::User(sid) => {
            let dt = ir.ty_registry.retrieve(*sid);
            let type_name = resolve_type_name(ir, *sid);
            match dt {
                DataType::Enum(variants) => {
                    let mut variant_assertions: Vec<z3_sys::Z3_ast> = Vec::new();

                    for (vname, vdef) in variants {
                        match vdef {
                            Variant::Tuple(slots) => {
                                for (i, slot_sort) in slots.iter().enumerate() {
                                    if *slot_sort == Sort::Error {
                                        let tester = api_ctx.get_tester(*sid, vname);
                                        let accessor = api_ctx.get_accessor(*sid, vname, i);
                                        let is_variant = z3_sys::Z3_mk_app(ctx, tester, 1, [call_result].as_ptr()).expect("mk_app");
                                        let field_val = z3_sys::Z3_mk_app(ctx, accessor, 1, [call_result].as_ptr()).expect("mk_app");
                                        let selected = z3_sys::Z3_mk_select(ctx, field_val, idx_ast).expect("mk_select");
                                        let conj = z3_sys::Z3_mk_and(ctx, 2, [is_variant, selected].as_ptr()).expect("mk_and");
                                        variant_assertions.push(conj);
                                    }
                                }
                            }
                            Variant::Record(fields) => {
                                for (fi, (_, field_sort)) in fields.iter().enumerate() {
                                    if *field_sort == Sort::Error {
                                        let tester = api_ctx.get_tester(*sid, vname);
                                        let accessor = api_ctx.get_accessor(*sid, vname, fi);
                                        let is_variant = z3_sys::Z3_mk_app(ctx, tester, 1, [call_result].as_ptr()).expect("mk_app");
                                        let field_val = z3_sys::Z3_mk_app(ctx, accessor, 1, [call_result].as_ptr()).expect("mk_app");
                                        let selected = z3_sys::Z3_mk_select(ctx, field_val, idx_ast).expect("mk_select");
                                        let conj = z3_sys::Z3_mk_and(ctx, 2, [is_variant, selected].as_ptr()).expect("mk_and");
                                        variant_assertions.push(conj);
                                    }
                                }
                            }
                            Variant::Unit => {}
                        }
                    }

                    assert!(
                        !variant_assertions.is_empty(),
                        "return type '{}' has no Error fields in any variant",
                        type_name
                    );

                    if variant_assertions.len() == 1 {
                        variant_assertions[0]
                    } else {
                        z3_sys::Z3_mk_or(ctx, variant_assertions.len() as u32, variant_assertions.as_ptr()).expect("mk_or")
                    }
                }
                _ => panic!(
                    "return type is not an enum — cannot extract error assertion from {:?}",
                    ret_sort
                ),
            }
        }
        _ => panic!(
            "return sort {:?} does not contain Error — cannot generate error query",
            ret_sort
        ),
    }
}

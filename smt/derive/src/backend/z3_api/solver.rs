//! Solver pipeline: uses in-process Z3 via z3-sys to solve error targets.

use crate::backend::response::{BACKEND_TIMEOUT, Response};
use crate::backend::z3::sort::resolve_type_name;
pub use crate::backend::z3_api::SolveResult;
use crate::backend::z3_api::context::Z3ApiContext;
use crate::backend::z3_api::{Z3Context, mk_string_symbol, model_to_string};
use crate::ir::ctxt::IRContext;
use crate::ir::index::UsrFunId;
use crate::ir::sort::{DataType, Sort, Variant};
use std::collections::BTreeSet;
use std::ffi::CStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Solve every error target in the IR. Each target gets its own fresh Z3
/// context so failures don't leak between queries.
pub fn solve_with_api(ir: &IRContext, top_level_fn: &str, on_result: &dyn Fn(&SolveResult)) {
    let top_fid = find_top_level_fn(ir, top_level_fn);
    set_global_params();

    for (idx, target_ids) in ir.error_targets.iter().enumerate() {
        let (response, elapsed_ms) = solve_single_target(ir, top_fid, target_ids);

        on_result(&SolveResult {
            target_idx: idx,
            response,
            elapsed_ms,
        });
    }
}

fn set_global_params() {
    z3::set_global_param("sat.random_seed", "42");
    z3::set_global_param("smt.random_seed", "42");
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

fn find_top_level_fn(ir: &IRContext, name: &str) -> UsrFunId {
    let insts = ir
        .fn_registry
        .lookup()
        .iter()
        .find(|(n, _)| n.as_ref() == name)
        .unwrap_or_else(|| panic!("top-level function '{}' not found", name))
        .1;
    assert_eq!(
        insts.len(),
        1,
        "top-level function '{}' must be monomorphic, found {} instantiations",
        name,
        insts.len()
    );
    *insts.values().next().unwrap()
}

fn solve_single_target(
    ir: &IRContext,
    top_fid: UsrFunId,
    target_ids: &BTreeSet<usize>,
) -> (Response, u128) {
    let timeout_ms = BACKEND_TIMEOUT.as_millis() as u32;
    let z3_ctx = Z3Context::new(timeout_ms);
    let ctx = z3_ctx.ctx;
    let mut api = Z3ApiContext::new(ctx, ir);

    unsafe {
        let solver = z3_sys::Z3_mk_solver(ctx).expect("mk_solver");
        z3_sys::Z3_solver_inc_ref(ctx, solver);

        // Configure timeout.
        let params = z3_sys::Z3_mk_params(ctx).expect("mk_params");
        z3_sys::Z3_params_inc_ref(ctx, params);
        let sym = mk_string_symbol(ctx, "timeout");
        z3_sys::Z3_params_set_uint(ctx, params, sym, timeout_ms);
        z3_sys::Z3_solver_set_params(ctx, solver, params);
        z3_sys::Z3_params_dec_ref(ctx, params);

        // Assert choose! axioms collected during context construction.
        for axiom in &api.axioms {
            z3_sys::Z3_solver_assert(ctx, solver, *axiom);
        }

        // Assert the error-reachability predicate for this target.
        let assertion = build_error_assertion(&mut api, ir, top_fid, target_ids);
        z3_sys::Z3_solver_assert(ctx, solver, assertion);

        // Backup timeout: a watchdog thread polls every 100ms and calls
        // `Z3_interrupt` once the deadline passes. A shared flag tells the
        // watchdog to exit cleanly as soon as `Z3_solver_check` returns, so
        // we never call `Z3_interrupt` on a context that's about to be freed.
        let ctx_addr = ctx.as_ptr() as usize;
        let timeout_dur = Duration::from_millis(timeout_ms.into());
        let start = Instant::now();
        let done = Arc::new(AtomicBool::new(false));
        let done_watchdog = Arc::clone(&done);
        let watchdog = std::thread::spawn(move || {
            let mut interrupted = false;
            loop {
                if done_watchdog.load(Ordering::Acquire) {
                    return;
                }
                let elapsed = start.elapsed();
                if !interrupted && elapsed >= timeout_dur {
                    if let Some(ctx_nn) = std::ptr::NonNull::new(ctx_addr as *mut _) {
                        z3_sys::Z3_interrupt(ctx_nn);
                    }
                    interrupted = true;
                }
                // Hard fail-safe: if Z3 is stuck in preprocessing that ignores
                // `Z3_interrupt`, give up and terminate the process so the
                // caller at least sees the results produced so far.
                if interrupted && elapsed >= timeout_dur * 8 {
                    std::process::exit(2);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        let check = z3_sys::Z3_solver_check(ctx, solver);

        // Let the watchdog exit before the context is dropped.
        done.store(true, Ordering::Release);
        let _ = watchdog.join();
        let response = match check {
            z3_sys::Z3_L_TRUE => match z3_sys::Z3_solver_get_model(ctx, solver) {
                Some(model) => {
                    z3_sys::Z3_model_inc_ref(ctx, model);
                    let text = model_to_string(ctx, model);
                    z3_sys::Z3_model_dec_ref(ctx, model);
                    Response::Sat(format!("sat\n{}", text))
                }
                None => Response::Sat("sat\n(model unavailable)".to_string()),
            },
            z3_sys::Z3_L_FALSE => Response::Unsat,
            _ => {
                if start.elapsed() >= timeout_dur {
                    Response::Timeout
                } else {
                    let reason_ptr = z3_sys::Z3_solver_get_reason_unknown(ctx, solver);
                    let reason = if reason_ptr.is_null() {
                        "unknown".to_string()
                    } else {
                        CStr::from_ptr(reason_ptr).to_string_lossy().into_owned()
                    };
                    if reason.contains("timeout")
                        || reason.contains("canceled")
                        || reason.contains("interrupted")
                    {
                        Response::Timeout
                    } else {
                        Response::Unknown
                    }
                }
            }
        };

        z3_sys::Z3_solver_dec_ref(ctx, solver);
        (response, start.elapsed().as_millis())
    }
}

/// Build the error-reachability assertion for one target.
///   1. Create fresh input constants, one per parameter of the top-level fn.
///   2. Build the call `top_fn(input_0, …, input_N)`.
///   3. For each error_id in the target, build a membership assertion against
///      the returned value.
///   4. If multiple IDs, AND them together.
unsafe fn build_error_assertion(
    api: &mut Z3ApiContext,
    ir: &IRContext,
    top_fid: UsrFunId,
    target_ids: &BTreeSet<usize>,
) -> z3_sys::Z3_ast {
    unsafe {
        let ctx = api.ctx;
        let sig = ir.fn_registry.retrieve_sig(top_fid).clone();
        let fdecl = api.get_func_decl(top_fid);

        let mut inputs: Vec<z3_sys::Z3_ast> = Vec::new();
        for (i, (_, psort)) in sig.params.iter().enumerate() {
            let z3_sort = api.translate_sort(psort);
            let name = format!("input_{}", i);
            let sym = mk_string_symbol(ctx, &name);
            let c = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("mk_const");
            z3_sys::Z3_inc_ref(ctx, c);
            inputs.push(c);
        }

        let call =
            z3_sys::Z3_mk_app(ctx, fdecl, inputs.len() as u32, inputs.as_ptr()).expect("mk_app");
        z3_sys::Z3_inc_ref(ctx, call);

        let parts: Vec<z3_sys::Z3_ast> = target_ids
            .iter()
            .map(|&id| build_single_error_assertion(api, ir, &sig.ret_ty, call, id))
            .collect();

        if parts.len() == 1 {
            parts[0]
        } else {
            z3_sys::Z3_mk_and(ctx, parts.len() as u32, parts.as_ptr()).expect("mk_and")
        }
    }
}

unsafe fn build_single_error_assertion(
    api: &mut Z3ApiContext,
    ir: &IRContext,
    ret_sort: &Sort,
    call: z3_sys::Z3_ast,
    error_id: usize,
) -> z3_sys::Z3_ast {
    unsafe {
        let ctx = api.ctx;
        let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
        let idx = z3_sys::Z3_mk_int(ctx, error_id as i32, int_sort).expect("mk_int");

        match ret_sort {
            Sort::Error => z3_sys::Z3_mk_select(ctx, call, idx).expect("select"),
            Sort::User(sid) => {
                let dt = ir.ty_registry.retrieve(*sid);
                let tname = resolve_type_name(ir, *sid);
                let variants = match dt {
                    DataType::Enum(v) => v,
                    _ => panic!(
                        "return type is not an enum — cannot extract error assertion from {:?}",
                        ret_sort
                    ),
                };

                let mut alts: Vec<z3_sys::Z3_ast> = Vec::new();
                for (vname, vdef) in variants {
                    match vdef {
                        Variant::Tuple(slots) => {
                            for (i, slot) in slots.iter().enumerate() {
                                if *slot == Sort::Error {
                                    let tester = api.get_tester(*sid, vname);
                                    let acc = api.get_accessor(*sid, vname, i);
                                    let is_v = z3_sys::Z3_mk_app(ctx, tester, 1, [call].as_ptr())
                                        .expect("app");
                                    let field = z3_sys::Z3_mk_app(ctx, acc, 1, [call].as_ptr())
                                        .expect("app");
                                    let sel =
                                        z3_sys::Z3_mk_select(ctx, field, idx).expect("select");
                                    let conj = z3_sys::Z3_mk_and(ctx, 2, [is_v, sel].as_ptr())
                                        .expect("and");
                                    alts.push(conj);
                                }
                            }
                        }
                        Variant::Record(fields) => {
                            for (fi, (_, fsort)) in fields.iter().enumerate() {
                                if *fsort == Sort::Error {
                                    let tester = api.get_tester(*sid, vname);
                                    let acc = api.get_accessor(*sid, vname, fi);
                                    let is_v = z3_sys::Z3_mk_app(ctx, tester, 1, [call].as_ptr())
                                        .expect("app");
                                    let field = z3_sys::Z3_mk_app(ctx, acc, 1, [call].as_ptr())
                                        .expect("app");
                                    let sel =
                                        z3_sys::Z3_mk_select(ctx, field, idx).expect("select");
                                    let conj = z3_sys::Z3_mk_and(ctx, 2, [is_v, sel].as_ptr())
                                        .expect("and");
                                    alts.push(conj);
                                }
                            }
                        }
                        Variant::Unit => {}
                    }
                }
                assert!(
                    !alts.is_empty(),
                    "return type '{}' has no Error fields in any variant",
                    tname
                );
                if alts.len() == 1 {
                    alts[0]
                } else {
                    z3_sys::Z3_mk_or(ctx, alts.len() as u32, alts.as_ptr()).expect("or")
                }
            }
            _ => panic!(
                "return sort {:?} does not contain Error — cannot generate error query",
                ret_sort
            ),
        }
    }
}

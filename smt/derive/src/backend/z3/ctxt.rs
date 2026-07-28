//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::ContentBuilder;
use crate::backend::codegen::l;
use crate::backend::error::{BackendError, BackendResult};
use crate::backend::response::backend_timeout;
// use crate::backend::response::NUM_CPU_CORES;
use crate::backend::response::Response;
use crate::backend::z3::exp::format_expression;
use crate::backend::z3::fun::collect_function_call_edges;
use crate::backend::z3::fun::format_sort_for_fn;
use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::fun::{mk_function_str, mk_functions_rec_str, mk_functions_unrolled_str};
use crate::backend::z3::sort::format_sort;
use crate::backend::z3::sort::get_generic_param_count;
use crate::backend::z3::sort::{collect_type_edges, resolve_type_name, scc_from_edges};
use crate::backend::z3::sort::{
    mk_enum_str, mk_named_tuple_str, mk_record_str, mk_unnamed_tuple_str,
};
use crate::ir::ctxt::IRContext;
use crate::ir::exp::Expression;
use crate::ir::index::{UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort, Variant};
use command_group::CommandGroup;
use log::{debug, warn};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// A wrapper for Z3 backends that implements the `CodeGen` trait.
pub struct CodeGenZ3;

impl CodeGen for CodeGenZ3 {
    /// Constructs a new `CodeGenZ3` wrapper.
    fn new() -> Self {
        Self
    }

    /// Returns the name of the backend code generator, which is "z3_chc".
    fn name(&self) -> &'static str {
        "z3_chc"
    }

    /// Returns the file extension (or flavor) of the source code, which is "smt2" for Z3.
    fn flavor(&self) -> &'static str {
        "smt2"
    }

    /// Generates the backend source code from the provided `IRContext` and give the response.
    fn process(&self, ir: &IRContext, unroll_depth: usize) -> BackendResult<String> {
        // create a new content builder for writing the SMT-LIB code
        let mut x = ContentBuilder::new();
        // destructure the IRContext
        let IRContext {
            ty_registry,
            fn_registry,
            path_targets: _,
            marker_names: _,
        } = ir;

        l!(x, "(set-option :print-success false)");
        l!(x, "(set-option :produce-models true)");
        l!(x, "(set-option :produce-proofs false)");
        l!(x, "(set-option :produce-unsat-cores false)");
        // Reproducibility - fixed seed = deterministic search order = same runtime every time.
        l!(x, "(set-option :sat.random_seed 42)"); // It decides the order to assign the variables
        l!(x, "(set-option :smt.random_seed 42)"); // like which theory to check first, which equality to propagate first.
        // Parallelism - bus error
        // l!(x, "(set-option :parallel.enable true)");
        // l!(x, "(set-option :parallel.threads.max {})", *NUM_CPU_CORES);
        // l!(x, "(set-option :parallel.conquer.delay 6000)");
        l!(x); // add new line
        // SAT Solver Optimizations - restart is sequential (one thread retrying with better knowledge).
        l!(x, "(set-option :sat.restart.max 100000)");
        // SMT Solver Optimizations
        l!(x, "(set-option :smt.arith.solver 6)");
        l!(x, "(set-option :smt.case_split 3)");
        l!(x, "(set-option :smt.phase_selection 3)");
        // Quantifier Handling
        l!(x, "(set-option :smt.mbqi true)");
        l!(x, "(set-option :smt.qi.eager_threshold 10.0)");
        l!(x, "(set-option :smt.qi.max_multi_patterns 1000)");
        l!(x, "(set-option :smt.ematching true)");
        // Auto-configuration
        l!(x, "(set-option :smt.auto_config false)");
        l!(x); // add new line

        // Cloak<T> is erased at the IR layer; recursive ADTs are emitted as
        // direct mutually-recursive datatypes (see ir/sort.rs::resolve_type).
        l!(
            x,
            "(declare-datatypes ((RuSmtSet 1)) ((par (T) ((mk-rset (rset-data (Array T Bool)) (rset-card Int))))))"
        );
        l!(
            x,
            "(declare-datatypes ((RuSmtArray 2)) ((par (K V) ((mk-rarr (rarr-data (Array K V)) (rarr-pres (Array K Bool)) (rarr-card Int))))))"
        );
        l!(x);

        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            l!(x, "; Define user-defined types");

            // A generic type is declared once, as a `par` template shared by every
            // instantiation, and that template's body mentions only its own type
            // parameters plus the concrete types in its own definition. So the graph
            // that governs declaration order lives on *canonical* (template) sids, not
            // on the monomorphized ones: an instantiation-argument edge such as
            // `ParseResult<Value> -> Value` exists in the IR but never appears in the
            // emitted declaration, so it must not constrain the order.
            //
            // Building the graph on canonical sids up front means the SCCs *are* the
            // declaration groups, already in dependency order — there is nothing to
            // canonicalize-and-merge afterwards, which is what used to lose a group's
            // position and could emit a type ahead of one it references.

            // Pick the representative sid per type name: the `par` template for a
            // generic type, the type itself otherwise.
            let mut name_to_best_sid: HashMap<String, UsrSortId> = HashMap::new();
            for &sid in ty_registry.data_types().keys() {
                let type_name_z3 = resolve_type_name(ir, sid);
                let (ty_name, type_params) = ir.ty_registry.reverse_lookup(sid);

                let is_canonical = match ty_name {
                    None => true, // unnamed tuple — unique by construction
                    Some(name) => {
                        type_params.is_empty()
                            || type_params.iter().all(|sort| {
                                matches!(
                                    sort,
                                    Sort::Uninterpreted(smt_name)
                                        if smt_name.as_ref().starts_with(&format!("{name}_"))
                                )
                            })
                    }
                };

                if !is_canonical {
                    continue;
                }

                let prev = name_to_best_sid.insert(type_name_z3.clone(), sid);
                assert!(
                    prev.is_none(),
                    "duplicate canonical sid for type '{type_name_z3}'"
                );
            }

            let canonical_sid = |sid: UsrSortId| -> UsrSortId {
                let type_name = resolve_type_name(ir, sid);
                *name_to_best_sid.get(&type_name).unwrap_or_else(|| {
                    panic!("no template representative for type '{type_name}' (sid {sid:?})")
                })
            };
            let canonical_sids: BTreeSet<UsrSortId> = name_to_best_sid.values().copied().collect();

            // Keep only edges *out of* a canonical body — an instantiation has no
            // declaration of its own, so its field edges are not emission
            // dependencies — and retarget each edge at the referenced type's
            // representative. Self-loops are kept so a recursive type stays a vertex.
            let edges: Vec<(UsrSortId, UsrSortId)> = collect_type_edges(ty_registry.data_types())
                .into_iter()
                .filter(|(src, _)| canonical_sids.contains(src))
                .map(|(src, dst)| (src, canonical_sid(dst)))
                .collect();

            // Reverse so that types referenced by other types are declared first;
            // an SCC with more than one member is mutually recursive and needs a
            // single `declare-datatypes` block.
            let sccs: Vec<BTreeSet<UsrSortId>> = scc_from_edges(&edges).into_iter().rev().collect();

            // Canonical types touching no edge at all: nothing they reference and
            // nothing that references them is declared, so their position is free.
            let covered: BTreeSet<UsrSortId> = sccs.iter().flatten().copied().collect();
            let decl_groups: Vec<BTreeSet<UsrSortId>> = sccs
                .into_iter()
                .chain(
                    canonical_sids
                        .difference(&covered)
                        .map(|&sid| BTreeSet::from([sid])),
                )
                .collect();

            for scc in decl_groups.iter() {
                let mut decl_headers = Vec::new();
                let mut decl_bodies = Vec::new();

                for sid in scc {
                    let dt = ir.ty_registry.retrieve(*sid);
                    let type_name_str = resolve_type_name(ir, *sid);
                    let (generic_param_count, type_params) = get_generic_param_count(ir, *sid);

                    decl_headers.push(format!("({type_name_str} {generic_param_count})"));

                    // generate the body of the declare-datatype(s) command
                    let body_str = match dt {
                        DataType::Tuple(elems)
                            if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                        {
                            mk_unnamed_tuple_str(type_name_str, elems, ir, &type_params)
                        }
                        DataType::Tuple(elems) => {
                            mk_named_tuple_str(type_name_str, elems, ir, &type_params)
                        }
                        DataType::Record(fields) => {
                            mk_record_str(type_name_str, fields, ir, &type_params)
                        }
                        DataType::Enum(variants) => {
                            mk_enum_str(type_name_str, variants, ir, &type_params)
                        }
                    };
                    decl_bodies.push(body_str);
                }

                if scc.len() == 1 {
                    let sid = *scc.iter().next().unwrap();
                    let type_name_str = resolve_type_name(ir, sid);
                    l!(
                        x,
                        "(declare-datatype {}\n\t{}\n)",
                        type_name_str,
                        decl_bodies[0]
                    );
                } else {
                    // Mutually recursive types: use declare-datatypes
                    l!(
                        x,
                        "(declare-datatypes\n\t({})\n\t(\n\t\t{}\n\t)\n)",
                        decl_headers.join(" "),
                        decl_bodies.join("\n\t\t")
                    );
                }
            }

            l!(x); // Empty line after types
        }

        // Integer string parsing helpers (hex / octal / binary).
        l!(x, "; Integer string parsing helpers");
        l!(x, "(define-fun rusmt_hex_char_to_int ((s String)) Int");
        l!(
            x,
            "\t(ite (and (str.<= \"0\" s) (str.<= s \"9\")) (- (str.to_code s) 48)"
        );
        l!(
            x,
            "\t(ite (and (str.<= \"A\" s) (str.<= s \"F\")) (- (str.to_code s) 55)"
        );
        l!(
            x,
            "\t(ite (and (str.<= \"a\" s) (str.<= s \"f\")) (- (str.to_code s) 87)"
        );
        l!(x, "\t0))))");
        l!(x);
        l!(
            x,
            "(define-fun-rec rusmt_from_hex_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmt_from_hex_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(
            x,
            "\t\t\t(+ (* acc 16) (rusmt_hex_char_to_int (str.at s 0))))))"
        );
        l!(x);
        l!(
            x,
            "(define-fun rusmt_from_hex_str ((s String)) Int (rusmt_from_hex_str_impl s 0))"
        );
        l!(x);
        l!(x, "(define-fun rusmt_oct_char_to_int ((s String)) Int");
        l!(
            x,
            "\t(ite (and (str.<= \"0\" s) (str.<= s \"7\")) (- (str.to_code s) 48) 0))"
        );
        l!(x);
        l!(
            x,
            "(define-fun-rec rusmt_from_oct_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmt_from_oct_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(
            x,
            "\t\t\t(+ (* acc 8) (rusmt_oct_char_to_int (str.at s 0))))))"
        );
        l!(x);
        l!(
            x,
            "(define-fun rusmt_from_oct_str ((s String)) Int (rusmt_from_oct_str_impl s 0))"
        );
        l!(x);
        l!(
            x,
            "(define-fun-rec rusmt_from_bin_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmt_from_bin_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(x, "\t\t\t(+ (* acc 2) (ite (= (str.at s 0) \"1\") 1 0)))))");
        l!(x);
        l!(
            x,
            "(define-fun rusmt_from_bin_str ((s String)) Int (rusmt_from_bin_str_impl s 0))"
        );
        l!(x);

        // Function registry
        if !fn_registry.lookup().is_empty() {
            l!(x, "; Define user-defined functions");

            // Collect only monomorphized function IDs.
            // See book/src/dev/smt/generics.md for the full design rationale.
            let mono_fids: BTreeSet<UsrFunId> = fn_registry
                .lookup()
                .values()
                .flat_map(|insts| insts.iter())
                .filter(|(ty_args, _)| !ty_args.iter().any(|s| matches!(s, Sort::Uninterpreted(_))))
                .map(|(_, fn_id)| *fn_id)
                .collect();

            // Detect IterChoose functions (those whose root body is IterChoose).
            // These use Hilbert choice semantics which can't be expressed as a define-fun
            // body in standard SMT-LIB2. Instead we emit them as uninterpreted declare-fun
            // so callers can reference them with the right types.
            // Note: choose! must be the sole expression in a function body (enforced by parser).
            let mut choose_fids: BTreeSet<UsrFunId> = BTreeSet::new();
            for &fid in &mono_fids {
                let def = fn_registry.retrieve_def(fid);
                let root_exp = def.body.lookup_exp(&def.root_exp_id);
                if matches!(root_exp, Expression::IterChoose { .. }) {
                    choose_fids.insert(fid);
                }
            }

            // Emit choose functions as declare-fun + axiom BEFORE define-fun/define-funs-rec.
            //
            // choose!(v in collection => predicate(v)) translates to:
            //   (declare-fun fn_name ((param sorts)) ret_sort)
            //   (assert (forall ((p1 S1) ...)
            //       (let ((v (fn_name p1 ...)))
            //           predicate)))
            //
            // The declare-fun says the function exists. The axiom constrains it:
            // for all inputs, the return value must satisfy the predicate.
            if !choose_fids.is_empty() {
                l!(
                    x,
                    "; Uninterpreted functions (Hilbert choice / epsilon semantics)"
                );
                for &fid in &choose_fids {
                    let function_name = resolve_function_name(ir, fid);
                    let sig = fn_registry.retrieve_sig(fid);
                    let def = fn_registry.retrieve_def(fid);

                    // declare-fun
                    let param_list: Vec<String> = sig
                        .params
                        .iter()
                        .map(|(_, sort)| format_sort_for_fn(sort, ir))
                        .collect();
                    let ret_sort = format_sort_for_fn(&sig.ret_ty, ir);
                    l!(
                        x,
                        "(declare-fun {} ({}) {})",
                        function_name,
                        param_list.join(" "),
                        ret_sort
                    );

                    // assert
                    let root_exp = def.body.lookup_exp(&def.root_exp_id);
                    if let Expression::IterChoose { vars, body, rets } = root_exp {
                        // Build forall parameter declarations
                        let mut param_decls: Vec<String> = sig
                            .params
                            .iter()
                            .map(|(name, sort)| {
                                format!("({} {})", name, format_sort_for_fn(sort, ir))
                            })
                            .collect();

                        // Build the call expression: (fn_name p1 p2 ...)
                        let param_names_list: Vec<String> = sig
                            .params
                            .iter()
                            .map(|(name, _)| name.to_string())
                            .collect();

                        let call_expr = if param_names_list.is_empty() {
                            function_name.clone()
                        } else {
                            format!("({} {})", function_name, param_names_list.join(" "))
                        };

                        // Build let-bindings: each choose variable is bound to the function result.
                        let mut let_bindings = Vec::new();
                        if rets.len() == 1 {
                            let var = def.body.lookup_var(&rets[0]);
                            let_bindings.push(format!("({} {})", var.name, call_expr));
                        } else {
                            // Multi-return: result is a tuple, project each field
                            for (i, vid) in rets.iter().enumerate() {
                                let var = def.body.lookup_var(vid);
                                let tuple_sort = format_sort_for_fn(&sig.ret_ty, ir);
                                let accessor = format!("field_{}_{}_", tuple_sort, i + 1);
                                let_bindings
                                    .push(format!("({} ({} {}))", var.name, accessor, call_expr));
                            }
                        }
                        let body_str = format_expression(&def.body, *body, ir);

                        // Build membership + non-emptiness constraints from vars.
                        let mut membership_constraints = Vec::new();
                        let mut non_empty_preconditions: Vec<String> = Vec::new();
                        // Set for a single-index `Seq` choose: see `SEQ_MIN_IDX`.
                        let mut seq_minimality: Option<String> = None;
                        for (vid, coll_eid) in vars {
                            let var = def.body.lookup_var(vid);
                            let coll_exp = def.body.lookup_exp(coll_eid);
                            let coll_sort = match coll_exp {
                                // without losing expressiveness we can assume that the collection is a variable
                                Expression::Var(coll_vid) => {
                                    def.body.lookup_var(coll_vid).sort.clone()
                                }
                                _ => panic!("choose! collection must be a variable"),
                            };
                            let coll_str = format_expression(&def.body, *coll_eid, ir);

                            let (membership, non_empty) = match &coll_sort {
                                Sort::Array(key_sort, _) => {
                                    let key_str = format_sort(key_sort, ir);
                                    let m =
                                        format!("(select (rarr-pres {}) {})", coll_str, var.name);
                                    let ne = format!(
                                        "(exists ((__ne_k {key_str})) (select (rarr-pres {coll_str}) __ne_k))"
                                    );
                                    (m, ne)
                                }
                                Sort::Set(elem_sort) => {
                                    // Sets are RuSmtSet wrapping (Array T Bool), membership via rset-data
                                    let elem_str = format_sort(elem_sort, ir);
                                    let m =
                                        format!("(select (rset-data {}) {})", coll_str, var.name);
                                    let ne = format!(
                                        "(exists ((__ne_v {elem_str})) (select (rset-data {coll_str}) __ne_v))"
                                    );
                                    (m, ne)
                                }
                                Sort::Seq(_) => {
                                    let m = format!(
                                        "(and (>= {} 0) (< {} (seq.len {})))",
                                        var.name, var.name, coll_str
                                    );
                                    let ne = format!("(>= (seq.len {coll_str}) 1)");
                                    if vars.len() == 1 && rets.len() == 1 {
                                        seq_minimality = Some(seq_min_conjunct(
                                            &var.name.to_string(),
                                            &body_str,
                                        ));
                                    }
                                    (m, ne)
                                }
                                _ => panic!(
                                    "choose! collection must be Array, Set, or Seq, got {coll_sort:?}"
                                ),
                            };
                            membership_constraints.push(membership);
                            non_empty_preconditions.push(non_empty);
                        }

                        // Pin the tie-break. The extra binder joins the *existing*
                        // `forall` block rather than opening a nested one.
                        if let Some(min) = seq_minimality {
                            param_decls.push(format!("({SEQ_MIN_IDX} Int)"));
                            membership_constraints.push(min);
                        }

                        // Combine memberships + body into the constrained branch.
                        let full_condition = if membership_constraints.is_empty() {
                            body_str
                        } else {
                            membership_constraints.push(body_str);
                            format!("(and {})", membership_constraints.join(" "))
                        };

                        // Guard the whole conjunction with non-emptiness so the
                        // axiom is vacuously satisfied for empty collections.
                        let guarded = if non_empty_preconditions.is_empty() {
                            full_condition
                        } else {
                            let precond = if non_empty_preconditions.len() == 1 {
                                non_empty_preconditions.pop().unwrap()
                            } else {
                                format!("(and {})", non_empty_preconditions.join(" "))
                            };
                            format!("(=> {precond} {full_condition})")
                        };

                        l!(
                            x,
                            "(assert (forall ({}) (let ({}) {})))",
                            param_decls.join(" "),
                            let_bindings.join(" "),
                            guarded
                        );
                    }
                }
                l!(x);
            }

            // Remaining functions: all monomorphized, non-choose functions
            let all_ids: BTreeSet<_> = mono_fids.difference(&choose_fids).copied().collect();
            // collect function call edges from the source function to the called functions inside the body
            let edges = collect_function_call_edges(&all_ids, fn_registry);

            let recursive_sccs: Vec<BTreeSet<UsrFunId>> =
                scc_from_edges(&edges).into_iter().rev().collect();
            let covered: BTreeSet<_> = recursive_sccs
                .iter()
                .flat_map(|s| s.iter().copied())
                .collect();
            // Isolated functions are functions that are not referenced by any other function or do not reference any other function.
            let isolated: Vec<UsrFunId> = all_ids.difference(&covered).copied().collect();

            // Generate isolated functions using define-fun
            for fid in isolated {
                let function_name = resolve_function_name(ir, fid);
                let sig = ir.fn_registry.retrieve_sig(fid);
                let def = ir.fn_registry.retrieve_def(fid);

                let function_str = mk_function_str(function_name, sig, def, ir);
                l!(x, "{}", function_str);
            }

            for scc in recursive_sccs {
                // A singleton SCC without a self-loop is structurally
                // non-recursive — it has outgoing call or incoming call edges to other
                // (possibly recursive) functions but not both or self-loop.
                if scc.len() == 1 {
                    let fid = *scc.iter().next().unwrap();
                    let has_self_loop = edges.iter().any(|(a, b)| *a == fid && *b == fid);
                    if !has_self_loop {
                        let function_name = resolve_function_name(ir, fid);
                        let sig = ir.fn_registry.retrieve_sig(fid);
                        let def = ir.fn_registry.retrieve_def(fid);
                        let function_str = mk_function_str(function_name, sig, def, ir);
                        l!(x, "{}", function_str);
                        continue;
                    }
                }
                let functions_str = if unroll_depth >= 1 {
                    mk_functions_unrolled_str(&scc, unroll_depth, ir)
                } else {
                    mk_functions_rec_str(&scc, ir)
                };
                l!(x, "{}", functions_str);
            }

            l!(x); // Empty line after functions
        }

        // Don't add check-sat or get-model here - they will be added by error discovery
        l!(x, "; Base SMT-LIB definitions complete");
        l!(x, "; Add error-specific assertions below");
        l!(x);

        Ok(x.build())
    }

    /// Execute the backend solver on the generated SMTLIB2 file.
    fn invoke_backend(&self, path_src: &Path) -> BackendResult<Response> {
        // Z3 -t:N sets the timeout in milliseconds; on expiry Z3 outputs "unknown" and exits 0.
        let timeout = backend_timeout();
        let timeout_ms = timeout.as_millis();
        let mut cmd = Command::new("z3");
        cmd.arg("-smt2")
            .arg(format!("-t:{timeout_ms}"))
            .arg(path_src)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Command::new("z3") builds the command. group_spawn() launches Z3 as an OS process inside a process group. Z3's parallel solver can itself spawn worker sub-processes. A process group means child.kill() kills Z3 AND all its workers in one shot. If you used regular .spawn() + .kill(), you'd only kill the Z3 main process and leave orphaned workers running.
        let mut child = cmd.group_spawn().map_err(|e| {
            warn!("Failed to spawn z3 process: {e}");
            BackendError
        })?;

        let mut stdout = child.inner().stdout.take().ok_or_else(|| {
            warn!("Failed to capture stdout");
            BackendError
        })?;

        let mut stderr = child.inner().stderr.take().ok_or_else(|| {
            warn!("Failed to capture stderr");
            BackendError
        })?;

        let start = Instant::now();

        // Read stdout in a separate thread to avoid blocking
        let stdout_thread = thread::spawn(move || {
            let mut output = String::new();
            if let Err(e) = stdout.read_to_string(&mut output) {
                warn!("failed to read Z3 stdout: {e}");
            }
            output
        });

        // Read stderr in a separate thread
        let stderr_thread = thread::spawn(move || {
            let mut message = String::new();
            if let Err(e) = stderr.read_to_string(&mut message) {
                warn!("failed to read Z3 stderr: {e}");
            }
            message
        });

        // Monitor the execution
        let monitor_thread = thread::spawn(move || {
            loop {
                // check status
                if let Ok(Some(status)) = child.try_wait() {
                    return Some(status);
                }

                // check timeout
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return None;
                }

                // wait a bit longer
                thread::sleep(Duration::from_millis(200));
            }
        });

        // Wait for monitoring thread to finish
        let status = monitor_thread.join().map_err(|_| {
            warn!("Monitoring thread panicked");
            BackendError
        })?;

        // Wait for the stdout and stderr threads to finish
        let output = stdout_thread.join().map_err(|_| {
            warn!("Stdout thread panicked");
            BackendError
        })?;

        let stderr_output = stderr_thread.join().map_err(|_| {
            warn!("Stderr thread panicked");
            BackendError
        })?;

        if !stderr_output.is_empty() {
            debug!("Z3 stderr: {stderr_output}");
        }

        // Interpret the output
        let response = match status {
            None => {
                // Rust monitor killed Z3 (backup path: Z3 didn't respect its own -t:N flag).
                if !output.is_empty() {
                    warn!("Output received from timeout execution: {output}");
                }
                Response::Timeout
            }
            Some(exit_status) => {
                if !exit_status.success() {
                    // Z3 crashed during model output (e.g. segfault in get-model).
                    warn!("Z3 exited with status {exit_status} (model may be incomplete)");
                }

                let verdict = output
                    .lines()
                    .map(|l| l.trim())
                    .find(|&l| l == "sat" || l == "unsat" || l == "unknown");

                match verdict {
                    Some("unsat") => Response::Unsat,
                    Some("unknown") => Response::Unknown(extract_reason_unknown(&output)),
                    Some("sat") => Response::Sat(output),
                    Some(_) => unreachable!("unexpected verdict"),
                    None => {
                        warn!("Z3 produced no verdict. Output: {}", output.trim());
                        return Err(BackendError);
                    }
                }
            }
        };

        Ok(response)
    }

    fn process_path_queries(
        &self,
        base_code: &str,
        ir: &IRContext,
        top_level_fn: &str,
        target_ids: &BTreeSet<usize>,
    ) -> String {
        // Look up the top-level function directly by name.
        let instantiations = ir
            .fn_registry
            .lookup()
            .iter()
            .find(|(name, _)| name.as_ref() == top_level_fn)
            .unwrap_or_else(|| {
                panic!("top-level function '{top_level_fn}' not found in function registry")
            })
            .1;

        // Top-level function must be monomorphic (exactly one instantiation).
        assert_eq!(
            instantiations.len(),
            1,
            "top-level function '{}' must be monomorphic, found {} instantiations",
            top_level_fn,
            instantiations.len()
        );
        let fn_id = instantiations.values().next().unwrap();
        let sig = ir.fn_registry.retrieve_sig(*fn_id);

        // Declare one fresh SMT constant per parameter of the top-level function,
        // and — for any U32 or Seq<U32> parameter — emit a Unicode-scalar-value
        // bound on it. Without this, Z3 may pick BV32 values outside the valid
        // codepoint range (surrogates 0xD800-0xDFFF, or > 0x10FFFF), which the
        // generic seq solver tries to materialize via str.from_code and can
        // overflow internal vectors.
        let mut query = base_code.to_string();
        let mut param_vars: Vec<String> = Vec::new();
        for (i, (_, param_sort)) in sig.params.iter().enumerate() {
            let var_name = format!("input_{i}");
            let sort_str = format_sort_for_fn(param_sort, ir);
            query.push_str(&format!("\n(declare-const {var_name} {sort_str})"));
            if let Some(bound) = unicode_bound_for(&var_name, param_sort) {
                query.push_str(&format!("\n{bound}"));
            }
            for wf in collection_wellformed(&var_name, param_sort, ir) {
                query.push_str(&format!("\n{wf}"));
            }
            param_vars.push(var_name);
        }

        // Build the call expression: (top_level_fn input_0 input_1 ...) or just top_level_fn.
        let call_expr = if param_vars.is_empty() {
            top_level_fn.to_string()
        } else {
            format!("({} {})", top_level_fn, param_vars.join(" "))
        };

        // One assertion for the whole target: "every marker in it fired".
        //   single target {n}    → (= (bvand p M_n) M_n)
        //   merge target {a,b,c} → (= (bvand p M_abc) M_abc)
        // The target set goes in as a single mask rather than one assertion per
        // id: over the bit-set encoding a conjunction of per-id tests would also
        // be correct, but building the mask once keeps the emitted query small
        // and mirrors how the set is represented.
        let assertion = extract_path_assertion(&sig.ret_ty, &call_expr, target_ids, ir);

        // Append the assertion, check-sat, get-info, and get-model.
        query.push_str(&format!("(assert {assertion})\n"));
        query.push_str("(check-sat)\n");
        query.push_str("(get-info :reason-unknown)\n");
        query.push_str("(get-model)\n");

        query
    }
}

/// Binder for the competing index in the `choose!`-over-`Seq` tie-break. The
/// `__` prefix is this backend's reserved namespace for generated binders, so it
/// cannot collide with a user variable.
const SEQ_MIN_IDX: &str = "__min_idx";

/// The conjunct that pins `choose!(v in s => P(v))` to the index the concrete
/// semantics returns.
///
/// `Seq::iterator()` yields `0..len` in increasing order and `choose!` breaks at
/// the first hit, so the stdlib's answer is the *smallest* satisfying index.
/// Without this the axiom says only "some satisfying index", leaving Z3 free to
/// answer with a different one — the model then validates, replay picks another
/// index, and the candidate is rejected. Sound either way (the certifier never
/// accepts it), but it burns proposer rounds on a witness that was never wrong.
///
/// # Why this is not a nested quantifier
///
/// The obvious encoding puts `(forall ((w Int)) (=> (and (>= w 0) (< w v)) (not
/// P(w))))` *inside* the axiom's existing `forall` over the parameters. But `v`
/// is `(f p1 ..)` — a function of the outer binders only — and `w` occurs in no
/// other conjunct, so the inner `forall` can be lifted into the outer binder
/// list unchanged:
///
/// ```text
///   ∀p. (memb(v) ∧ P(v) ∧ ∀w. (0 ≤ w < v → ¬P(w)))
/// ≡ ∀p,w. (memb(v) ∧ P(v) ∧      (0 ≤ w < v → ¬P(w)))
/// ```
///
/// The conjuncts that don't mention `w` are simply replicated, which is
/// harmless. So this adds one binder to a block Z3 was already instantiating
/// instead of creating a second instantiation site with its own pattern set.
/// There is no quantifier alternation here at all — both are universal.
///
/// `P(w)` is obtained by *shadowing*: SMT-LIB `let` is lexically scoped, so
/// wrapping the already-rendered body in `(let ((v __min_idx)) ..)` rebinds the
/// loop variable for that subterm only. No expression rewriting needed, and the
/// enclosing `(< __min_idx v)` still sees the outer `v`.
fn seq_min_conjunct(var_name: &str, body_str: &str) -> String {
    format!(
        "(=> (and (>= {SEQ_MIN_IDX} 0) (< {SEQ_MIN_IDX} {var_name})) \
         (not (let (({var_name} {SEQ_MIN_IDX})) {body_str})))"
    )
}

/// Extract the reason string Z3 reported via `(get-info :reason-unknown)`.
pub(crate) fn extract_reason_unknown(output: &str) -> String {
    let marker = "(:reason-unknown";
    let after_marker = match output.find(marker) {
        Some(idx) => &output[idx + marker.len()..],
        None => return String::new(),
    };
    let bytes = after_marker.as_bytes();
    // Find the opening quote.
    let mut i = 0;
    while i < bytes.len() && bytes[i] != b'"' {
        i += 1;
    }
    if i >= bytes.len() {
        return String::new();
    }
    i += 1; // skip the opening quote
    let start = i;
    let mut out = String::new();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // SMT-LIB: `""` is an escaped quote within a string literal.
            if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                out.push('"');
                i += 2;
                continue;
            }
            // Otherwise this `"` closes the literal.
            return out;
        }
        // Push raw byte (UTF-8 safe because we're rebuilding from the source).
        out.push(bytes[i] as char);
        i += 1;
    }
    // Reached end without closing quote — return what we have.
    let _ = start;
    out
}

/// If `param_sort` is `U32` or `Seq<U32>`, return an SMT-LIB assertion
/// constraining the variable's codepoints to `[0x0, 0xD7FF] ∪ [0xE000, 0x2FFFF]`.
/// Returns `None` for sorts that don't reach a `U32` representing a codepoint.
///
/// Two separate exclusions, for two different reasons:
///
/// * **Surrogates `[0xD800, 0xDFFF]`** are not Unicode scalar values at all, so
///   no concrete `char` input can produce them (`char as u32` never does). Free
///   to exclude — nothing real is lost.
///
/// * **Everything above `0x2FFFF`** is excluded because that is the largest
///   character Z3's string theory can represent. Above it `str.from_code`
///   yields the *empty* string, while the stdlib's `String::from_code`
///   (`char::from_u32(..).unwrap()`) yields a one-character string — so a model
///   that picked, say, `0x30000` would have the solver and the executable
///   semantics disagreeing on `length()` at the very first step. `toml::mod`
///   feeds input codepoints straight into `String::from_code`, so this is
///   reachable, not hypothetical.
///
///   Unlike the surrogate case this *does* cost completeness: planes 3–16 are
///   perfectly good `char`s that we now refuse to propose as inputs. That is the
///   right trade — a witness Z3 mis-models is worse than a witness we never
///   offer — but it means "no input found" for a target does not rule out one
///   that needs a codepoint above `0x2FFFF`.
///
/// Soundness is otherwise unchanged: this only narrows the search space, so
/// every model satisfying the constraint still satisfies the unconstrained
/// problem.
///
/// Measured on Z3 4.15.4: `(str.to_code "\u{2FFFF}")` = 196607, and both
/// `str.to_code` and `str.from_code` fail (`-1` / `""`) from `0x30000` up.
fn unicode_bound_for(var_expr: &str, sort: &Sort) -> Option<String> {
    match sort {
        Sort::U32 => Some(format!(
            "(assert (or (bvule {var_expr} #x0000D7FF) (and (bvuge {var_expr} #x0000E000) (bvule {var_expr} #x0002FFFF))))"
        )),
        Sort::Seq(inner) if matches!(**inner, Sort::U32) => Some(format!(
            "(assert (forall ((__i Int)) (=> (and (>= __i 0) (< __i (seq.len {var_expr}))) (let ((__c (seq.nth {var_expr} __i))) (or (bvule __c #x0000D7FF) (and (bvuge __c #x0000E000) (bvule __c #x0002FFFF)))))))"
        )),
        _ => None,
    }
}

/// Well-formedness assertions for a *freshly declared* `RuSmtSet` / `RuSmtArray`
/// constant.
///
/// The count is an ordinary `Int` field of the record: the `declare-datatypes`
/// line ties it to nothing. Every value the transpiler *builds* keeps it in step
/// with membership, because `mk-rset` / `mk-rarr` are only ever applied by the
/// Set/Array intrinsics, which bump the count exactly when a presence bit flips.
/// A `declare-const` of that sort has no such history, so Z3 is free to answer
/// with an empty set whose `rset-card` is 7 — a value that is not the image of
/// any concrete `Set`, and that would make `is_empty` disagree with `contains`
/// in the very model we hand back as a witness.
///
/// The exact invariant `card = |{k : pres[k]}|` is not expressible (the key
/// domain is infinite). These two are, and they close the contradictions the
/// stdlib's own predicates can observe:
///
/// * `card >= 0` — `Set::length` / `Array::length` come from a `usize`;
/// * `card = 0  <=>  membership everywhere false` — keeps `is_empty`
///   (`(= card 0)`) consistent with `contains` / `contains_key` (`select`).
///
/// Scope: top-level parameters whose sort *is* a Set/Array. A collection nested
/// inside a `Seq` or a user datatype parameter is still unconstrained — that
/// needs a quantified assertion per nesting site, and no case study has one.
fn collection_wellformed(var_expr: &str, sort: &Sort, ir: &IRContext) -> Vec<String> {
    match sort {
        Sort::Set(elem) => {
            let es = format_sort_for_fn(elem, ir);
            vec![
                format!("(assert (>= (rset-card {var_expr}) 0))"),
                format!(
                    "(assert (= (= (rset-card {var_expr}) 0) (= (rset-data {var_expr}) ((as const (Array {es} Bool)) false))))"
                ),
            ]
        }
        Sort::Array(key, _) => {
            let ks = format_sort_for_fn(key, ir);
            vec![
                format!("(assert (>= (rarr-card {var_expr}) 0))"),
                format!(
                    "(assert (= (= (rarr-card {var_expr}) 0) (= (rarr-pres {var_expr}) ((as const (Array {ks} Bool)) false))))"
                ),
            ]
        }
        _ => Vec::new(),
    }
}

/// Build an SMT-LIB assertion that checks whether `path_id` is reachable from
/// the result of `call_expr`.
///
/// # Background
///
/// Concretely a `Path` is a *set* of marker ids, and in SMT it is a bit-set with
/// one bit per named marker (see `z3/path.rs`). Membership of a whole target
/// `T` is therefore one formula, `(= (bvand p M_T) M_T)` — "every bit of `T` is
/// set in `p`" — which covers a singleton `Path::named` target and a
/// multi-marker `Path::merge` target alike, and says nothing about the other
/// bits, so a run that fires additional markers still satisfies it.
///
/// # How it works
///
/// The top-level function (e.g. `parse_toml`) typically returns a user-defined
/// enum like `ParseResult<T>` rather than a bare `Path`. This function inspects
/// the return sort and generates the appropriate assertion:
///
/// - **`Sort::Path`** — the function returns `Path` directly.
///   Emits: `(= (fn input_0 ...) path_id)`.
///
/// - **`Sort::User` (enum)** — scans each variant for fields of type `Sort::Path`.
///   For each such field, emits a guarded assertion:
///   `(and (is-VariantName result) (= (accessor result) path_id))`.
///   If multiple variants carry Path fields, the assertions are OR'd together.
///
/// # Limitations
///
/// - **Enums only**: only `DataType::Enum` is handled. Structs, tuples, and
///   primitives will panic.
/// - **One level deep**: only checks direct fields of the enum variants. If Path
///   is nested inside a struct inside an enum variant, it will not be found.
/// - **No recursion**: does not follow nested user-defined types to find deeply
///   buried Path fields.
///
/// These limitations are acceptable because interpreter entry points return
/// flat enums like `ParseResult<T>` where Path is a direct field.
///
/// # Panics
///
/// Panics if the return sort does not contain a reachable `Sort::Path` field.
/// This indicates a bug: `path_targets` was populated but the top-level function
/// does not surface path markers in its return type.
fn extract_path_assertion(
    ret_sort: &Sort,
    call_expr: &str,
    path_ids: &BTreeSet<usize>,
    ir: &IRContext,
) -> String {
    // "Every marker in the target fired." One mask handles both target shapes:
    // a singleton `Path::named` target and a multi-marker `Path::merge` target.
    // Building it once, rather than per id, is what makes the merge case work —
    // conjoining per-id equalities over a single-value encoding was a
    // contradiction, unsat for every input regardless of the program.
    let member = |expr: &str| crate::backend::z3::path::contains_all(expr, path_ids, ir);
    match ret_sort {
        // Function returns Path directly.
        // Example: (= (bvand (parse_toml input_0) (_ bv4 8)) (_ bv4 8))
        Sort::Path => member(call_expr),
        // Function returns a user-defined enum — find the variant(s) that carry a Path field.
        // Example for ParseResult with Err(Path):
        //   (and (is-Err (parse_toml input_0))
        //        (= (bvand (field_ParseResult_Err_1_ (parse_toml input_0)) M) M))
        Sort::User(sid) => {
            let dt = ir.ty_registry.retrieve(*sid);
            let type_name = resolve_type_name(ir, *sid);
            match dt {
                DataType::Enum(variants) => {
                    let mut assertions: Vec<String> = Vec::new();
                    for (vname, vdef) in variants {
                        match vdef {
                            Variant::Tuple(slots) => {
                                for (i, slot_sort) in slots.iter().enumerate() {
                                    if *slot_sort == Sort::Path {
                                        let tester = format!("is-{type_name}_{vname}");
                                        let accessor =
                                            format!("field_{}_{}_{}_", type_name, vname, i + 1);
                                        let field = format!("({accessor} {call_expr})");
                                        assertions.push(format!(
                                            "(and ({tester} {call_expr}) {})",
                                            member(&field)
                                        ));
                                    }
                                }
                            }
                            Variant::Record(fields) => {
                                for (field_key, field_sort) in fields {
                                    if *field_sort == Sort::Path {
                                        let tester = format!("is-{type_name}_{vname}");
                                        let accessor =
                                            format!("record_{type_name}_{vname}_{field_key}_");
                                        let field = format!("({accessor} {call_expr})");
                                        assertions.push(format!(
                                            "(and ({tester} {call_expr}) {})",
                                            member(&field)
                                        ));
                                    }
                                }
                            }
                            Variant::Unit => {} // Unit variants carry no data.
                        }
                    }
                    assert!(
                        !assertions.is_empty(),
                        "return type '{type_name}' has no Path fields in any variant"
                    );
                    if assertions.len() == 1 {
                        assertions.remove(0)
                    } else {
                        format!("(or {})", assertions.join(" "))
                    }
                }
                _ => panic!(
                    "return type is not an enum — cannot extract path assertion from {ret_sort:?}"
                ),
            }
        }
        _ => panic!("return sort {ret_sort:?} does not contain Path — cannot generate path query"),
    }
}

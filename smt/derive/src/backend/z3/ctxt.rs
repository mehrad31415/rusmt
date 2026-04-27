//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::ContentBuilder;
use crate::backend::codegen::l;
use crate::backend::error::{BackendError, BackendResult};
use crate::backend::response::BACKEND_TIMEOUT;
// use crate::backend::response::NUM_CPU_CORES;
use crate::backend::response::Response;
use crate::backend::z3::exp::format_expression;
use crate::backend::z3::fun::collect_function_call_edges;
use crate::backend::z3::fun::format_sort_for_fn;
use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::fun::{mk_function_str, mk_functions_rec_str};
use crate::backend::z3::intrinsics::array_null_value;
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
use std::collections::HashMap;
use std::collections::{BTreeSet, HashSet};
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
    fn process(&self, ir: &IRContext) -> BackendResult<String> {
        // create a new content builder for writing the SMT-LIB code
        let mut x = ContentBuilder::new();
        // destructure the IRContext
        let IRContext {
            ty_registry,
            fn_registry,
            error_count: _,
            error_targets: _,
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

        // Declare the Cloak datatype — used for recursive type indirection.
        // Cloak<T> wraps a value of type T, with a Cloak-null base case that provides
        // the well-foundedness guarantee Z3 requires for recursive datatypes.
        l!(
            x,
            "(declare-datatype Cloak (par (T) ((mk-Cloak (uncloak T)) (Cloak-null))))"
        );
        l!(
            x,
            "(declare-datatypes ((RusmartSet 1)) ((par (T) ((mk-rset (rset-data (Array T Bool)) (rset-card Int))))))"
        );
        l!(
            x,
            "(declare-datatypes ((RusmartArray 2)) ((par (K V) ((mk-rarr (rarr-data (Array K V)) (rarr-card Int))))))"
        );
        l!(x);

        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            l!(x, "; Define user-defined types");
            let edges = collect_type_edges(ty_registry.data_types());
            // Recursive/mutually recursive types — must use declare-datatypes
            // reverse the order so that types that are referenced by other types are declared first
            let recursive_sccs: Vec<BTreeSet<UsrSortId>> =
                scc_from_edges(&edges).into_iter().rev().collect();

            let all_ids: BTreeSet<_> = ty_registry.data_types().keys().copied().collect();
            let covered: BTreeSet<_> = recursive_sccs
                .iter()
                .flat_map(|s| s.iter().copied())
                .collect();
            // Isolated types are types that are not referenced by any other type or do not reference any other type.
            let isolated: Vec<UsrSortId> = all_ids.difference(&covered).copied().collect();

            // Deduplicate SCCs by type names to avoid declaring the same type multiple times.
            // This happens with generic types that have multiple instantiations in the IR
            // (e.g., ParseResult<String> and ParseResult<Value> both map to one ParseResult in Z3).
            // We group by type name and choose the instance whose type parameters are
            // uninterpreted sorts (the "generic" version) as the representative.
            let mut name_to_best_sid: HashMap<String, UsrSortId> = HashMap::new();

            // find the best representative for each type name
            for &sid in recursive_sccs.iter().flatten().chain(isolated.iter()) {
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
                                        if smt_name.as_ref().starts_with(&format!("{}_", name))
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
                    "duplicate canonical sid for type '{}'",
                    type_name_z3
                );
            }

            // Second pass: build deduplicated SCCs using the best representatives.
            // Each SCC's monomorphized sids are replaced with their generic template sid.
            // SCCs that share any sid after canonicalization are merged together.
            //
            // Example: if SCC-A = {Config, ParseResult<String>} and SCC-B = {ParseResult<T>},
            // after canonicalization both contain sid for ParseResult<T>. They must be merged
            // into one declare-datatypes block: {Config, ParseResult<T>}.
            let mut deduplicated_sccs: Vec<BTreeSet<UsrSortId>> = Vec::new();

            for scc in recursive_sccs.iter() {
                // Map each sid to its best representative's sid
                let canonical_scc: BTreeSet<UsrSortId> = scc
                    .iter()
                    .filter_map(|&sid| {
                        let type_name = resolve_type_name(ir, sid);
                        Some(*name_to_best_sid.get(&type_name).unwrap_or_else(|| {
                            panic!(
                                "no template representative for type '{}' (sid {:?})",
                                type_name, sid
                            )
                        }))
                    })
                    .collect();

                // Find existing SCCs that share any sid with this canonical SCC and merge
                let mut merged = canonical_scc;
                let mut i = 0;
                while i < deduplicated_sccs.len() {
                    if deduplicated_sccs[i].intersection(&merged).next().is_some() {
                        // Overlap found — absorb the existing SCC into merged
                        let existing = deduplicated_sccs.remove(i);
                        merged.extend(existing);
                    } else {
                        i += 1;
                    }
                }
                deduplicated_sccs.push(merged);
            }

            // Also add isolated types that weren't covered by any recursive SCC
            for &sid in &isolated {
                let type_name = resolve_type_name(ir, sid);
                if let Some(&best_sid) = name_to_best_sid.get(&type_name) {
                    // Check if this type is already in a deduplicated SCC
                    let already_covered =
                        deduplicated_sccs.iter().any(|scc| scc.contains(&best_sid));
                    if !already_covered {
                        deduplicated_sccs.push(BTreeSet::from([best_sid]));
                    }
                }
            }

            for scc in deduplicated_sccs.iter() {
                let mut decl_headers = Vec::new();
                let mut decl_bodies = Vec::new();

                for sid in scc {
                    let dt = ir.ty_registry.retrieve(*sid);
                    let type_name_str = resolve_type_name(ir, *sid);
                    let (generic_param_count, type_params) = get_generic_param_count(ir, *sid);

                    decl_headers.push(format!("({} {})", type_name_str, generic_param_count));

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

        // Null sentinel constants for array default values.
        {
            let mut declared_null_names: HashSet<Sort> = HashSet::new();
            l!(x, "; Null sentinel constants for array default values");

            // User-defined sorts
            for sid in ty_registry.data_types().keys() {
                let (_, type_args) = ty_registry.reverse_lookup(*sid);
                if type_args
                    .iter()
                    .any(|s| matches!(s, Sort::Uninterpreted(_)))
                {
                    continue;
                }

                let v_sort = Sort::User(*sid);
                if declared_null_names.insert(v_sort.clone()) {
                    let const_name = array_null_value(&v_sort, ir);
                    let sort_str = format_sort(&v_sort, ir);
                    l!(x, "(declare-const {} {})", const_name, sort_str);
                }
            }

            l!(x);
        }

        // Integer string parsing helpers (hex / octal / binary).
        l!(x, "; Integer string parsing helpers");
        l!(x, "(define-fun rusmart_hex_char_to_int ((s String)) Int");
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
            "(define-fun-rec rusmart_from_hex_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmart_from_hex_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(
            x,
            "\t\t\t(+ (* acc 16) (rusmart_hex_char_to_int (str.at s 0))))))"
        );
        l!(x);
        l!(
            x,
            "(define-fun rusmart_from_hex_str ((s String)) Int (rusmart_from_hex_str_impl s 0))"
        );
        l!(x);
        l!(x, "(define-fun rusmart_oct_char_to_int ((s String)) Int");
        l!(
            x,
            "\t(ite (and (str.<= \"0\" s) (str.<= s \"7\")) (- (str.to_code s) 48) 0))"
        );
        l!(x);
        l!(
            x,
            "(define-fun-rec rusmart_from_oct_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmart_from_oct_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(
            x,
            "\t\t\t(+ (* acc 8) (rusmart_oct_char_to_int (str.at s 0))))))"
        );
        l!(x);
        l!(
            x,
            "(define-fun rusmart_from_oct_str ((s String)) Int (rusmart_from_oct_str_impl s 0))"
        );
        l!(x);
        l!(
            x,
            "(define-fun-rec rusmart_from_bin_str_impl ((s String) (acc Int)) Int"
        );
        l!(x, "\t(ite (= (str.len s) 0)");
        l!(x, "\t\tacc");
        l!(x, "\t\t(rusmart_from_bin_str_impl");
        l!(x, "\t\t\t(str.substr s 1 (- (str.len s) 1))");
        l!(x, "\t\t\t(+ (* acc 2) (ite (= (str.at s 0) \"1\") 1 0)))))");
        l!(x);
        l!(
            x,
            "(define-fun rusmart_from_bin_str ((s String)) Int (rusmart_from_bin_str_impl s 0))"
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
                        let param_decls: Vec<String> = sig
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

                        // Build membership constraints from vars.
                        // Each choose variable must be a member of its collection.
                        // choose!(s in array => ...) means s must be a key in array.
                        let mut membership_constraints = Vec::new();
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

                            let membership = match &coll_sort {
                                Sort::Array(_key_sort, val_sort) => {
                                    let null_name = array_null_value(val_sort, ir);
                                    format!(
                                        "(not (= (select (rarr-data {}) {}) {}))",
                                        coll_str, var.name, null_name
                                    )
                                }
                                Sort::Set(_) => {
                                    // Sets are RusmartSet wrapping (Array T Bool), membership via rset-data
                                    format!("(select (rset-data {}) {})", coll_str, var.name)
                                }
                                Sort::Seq(_) => {
                                    format!(
                                        "(and (>= {} 0) (< {} (seq.len {})))",
                                        var.name, var.name, coll_str
                                    )
                                }
                                _ => panic!(
                                    "choose! collection must be Array or Set, got {:?}",
                                    coll_sort
                                ),
                            };
                            membership_constraints.push(membership);
                        }

                        // Combine: (and membership1 ... body_condition)
                        let full_condition = if membership_constraints.is_empty() {
                            body_str
                        } else {
                            membership_constraints.push(body_str);
                            format!("(and {})", membership_constraints.join(" "))
                        };

                        l!(
                            x,
                            "(assert (forall ({}) (let ({}) {})))",
                            param_decls.join(" "),
                            let_bindings.join(" "),
                            full_condition
                        );
                    }
                }
                l!(x);
            }

            // Remaining functions: all monomorphized, non-choose functions
            let all_ids: BTreeSet<_> = mono_fids.difference(&choose_fids).copied().collect();
            // collect function call edges from the source function to the called functions inside the body
            let edges = collect_function_call_edges(&all_ids, &fn_registry);

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
                let functions_str = mk_functions_rec_str(&scc, ir);
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
        let timeout_ms = BACKEND_TIMEOUT.as_millis();
        let mut cmd = Command::new("z3");
        cmd.arg("-smt2")
            .arg(format!("-t:{}", timeout_ms))
            .arg(&path_src)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Command::new("z3") builds the command. group_spawn() launches Z3 as an OS process inside a process group. Z3's parallel solver can itself spawn worker sub-processes. A process group means child.kill() kills Z3 AND all its workers in one shot. If you used regular .spawn() + .kill(), you'd only kill the Z3 main process and leave orphaned workers running.
        let mut child = cmd.group_spawn().map_err(|e| {
            warn!("Failed to spawn z3 process: {}", e);
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
                warn!("failed to read Z3 stdout: {}", e);
            }
            output
        });

        // Read stderr in a separate thread
        let stderr_thread = thread::spawn(move || {
            let mut message = String::new();
            if let Err(e) = stderr.read_to_string(&mut message) {
                warn!("failed to read Z3 stderr: {}", e);
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
                if start.elapsed() > BACKEND_TIMEOUT {
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
            debug!("Z3 stderr: {}", stderr_output);
        }

        // Interpret the output
        let response = match status {
            None => {
                // Rust monitor killed Z3 (backup path: Z3 didn't respect its own -t:N flag).
                if !output.is_empty() {
                    warn!("Output received from timeout execution: {}", output);
                }
                Response::Timeout
            }
            Some(exit_status) => {
                if !exit_status.success() {
                    // Z3 crashed during model output (e.g. segfault in get-model).
                    warn!(
                        "Z3 exited with status {} (model may be incomplete)",
                        exit_status
                    );
                }

                let verdict = output
                    .lines()
                    .map(|l| l.trim())
                    .find(|&l| l == "sat" || l == "unsat" || l == "unknown");

                match verdict {
                    Some("unsat") => Response::Unsat,
                    Some("unknown") => Response::Unknown,
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

    fn process_error_queries(
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
                panic!(
                    "top-level function '{}' not found in function registry",
                    top_level_fn
                )
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
        let sig = ir.fn_registry.retrieve_sig(fn_id.clone());

        // Declare one fresh SMT constant per parameter of the top-level function.
        let mut query = base_code.to_string();
        let mut param_vars: Vec<String> = Vec::new();
        for (i, (_, param_sort)) in sig.params.iter().enumerate() {
            let var_name = format!("input_{}", i);
            let sort_str = format_sort_for_fn(param_sort, ir);
            query.push_str(&format!("\n(declare-const {} {})", var_name, sort_str));
            param_vars.push(var_name);
        }

        // Build the call expression: (top_level_fn input_0 input_1 ...) or just top_level_fn.
        let call_expr = if param_vars.is_empty() {
            top_level_fn.to_string()
        } else {
            format!("({} {})", top_level_fn, param_vars.join(" "))
        };

        // Build assertions: for each error ID in the target, assert set membership.
        // Single target {n}:     (select (accessor result) n)
        // Merge target {a,b,c}:  (and (select ... a) (select ... b) (select ... c))
        let member_assertions: Vec<String> = target_ids
            .iter()
            .map(|&error_id| extract_error_assertion(&sig.ret_ty, &call_expr, error_id, ir))
            .collect();

        let assertion = match member_assertions.len() {
            1 => member_assertions.into_iter().next().unwrap(),
            _ => format!("(and {})", member_assertions.join(" ")),
        };

        // Append the assertion, check-sat, and get-model to the query.
        query.push_str(&format!("(assert {})\n", assertion));
        query.push_str("(check-sat)\n");
        query.push_str("(get-model)\n");

        query
    }
}

/// Build an SMT-LIB assertion that checks whether `error_id` is reachable from
/// the result of `call_expr`.
///
/// # Background
///
/// Error in RuSmart is `(Set Int)` in Z3. Each `ErrFresh(n)` in the interpreter
/// becomes `(store ((as const (Array Int Bool)) false) n true)`, and `ErrMerge` becomes
/// `((_ map or) ...)`. To test whether a specific error site was reached, we use array select:
/// `(select result error_id)`.
///
/// # How it works
///
/// The top-level function (e.g. `parse_toml`) typically returns a user-defined
/// enum like `ParseResult<T>` rather than a bare `Error`. This function inspects
/// the return sort and generates the appropriate assertion:
///
/// - **`Sort::Error`** — the function returns `Error` directly.
///   Emits: `(select (fn input_0 ...) error_id)`.
///
/// - **`Sort::User` (enum)** — scans each variant for fields of type `Sort::Error`.
///   For each such field, emits a guarded assertion:
///   `(and (is-VariantName result) (select (accessor result) error_id))`.
///   If multiple variants carry Error fields, the assertions are OR'd together.
///
/// # Limitations
///
/// - **Enums only**: only `DataType::Enum` is handled. Structs, tuples, and
///   primitives will panic.
/// - **One level deep**: only checks direct fields of the enum variants. If Error
///   is nested inside a struct inside an enum variant, it will not be found.
/// - **No recursion**: does not follow nested user-defined types to find deeply
///   buried Error fields.
///
/// These limitations are acceptable because interpreter entry points return
/// flat enums like `ParseResult<T>` where Error is a direct field.
///
/// # Panics
///
/// Panics if the return sort does not contain a reachable `Sort::Error` field.
/// This indicates a bug: `error_targets` was populated but the top-level function
/// does not surface errors in its return type.
fn extract_error_assertion(
    ret_sort: &Sort,
    call_expr: &str,
    error_id: usize,
    ir: &IRContext,
) -> String {
    match ret_sort {
        // Function returns Error directly — just check membership.
        // Example: (select (parse_toml input_0) 3)
        Sort::Error => format!("(select {} {})", call_expr, error_id),
        // Function returns a user-defined enum — find the variant(s) that carry an Error field.
        // Example for ParseResult with Err(Error):
        //   (and (is-Err (parse_toml input_0))
        //        (select (field_ParseResult_Err_1_ (parse_toml input_0)) 3))
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
                                    if *slot_sort == Sort::Error {
                                        let tester = format!("is-{}_{}", type_name, vname);
                                        let accessor =
                                            format!("field_{}_{}_{}_", type_name, vname, i + 1);
                                        assertions.push(format!(
                                            "(and ({} {}) (select ({} {}) {}))",
                                            tester, call_expr, accessor, call_expr, error_id
                                        ));
                                    }
                                }
                            }
                            Variant::Record(fields) => {
                                for (field_key, field_sort) in fields {
                                    if *field_sort == Sort::Error {
                                        let tester = format!("is-{}_{}", type_name, vname);
                                        let accessor = format!(
                                            "record_{}_{}_{}_",
                                            type_name, vname, field_key
                                        );
                                        assertions.push(format!(
                                            "(and ({} {}) (select ({} {}) {}))",
                                            tester, call_expr, accessor, call_expr, error_id
                                        ));
                                    }
                                }
                            }
                            Variant::Unit => {} // Unit variants carry no data.
                        }
                    }
                    assert!(
                        !assertions.is_empty(),
                        "return type '{}' has no Error fields in any variant",
                        type_name
                    );
                    if assertions.len() == 1 {
                        assertions.remove(0)
                    } else {
                        format!("(or {})", assertions.join(" "))
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

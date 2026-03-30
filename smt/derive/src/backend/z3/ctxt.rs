//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::ContentBuilder;
use crate::backend::codegen::l;
use crate::backend::error::{BackendError, BackendResult};
use crate::backend::response::BACKEND_TIMEOUT;
use crate::backend::response::num_cpu_cores;
use crate::backend::response::Response;
use crate::backend::z3::fun::collect_function_call_edges;
use crate::backend::z3::fun::format_sort_for_fn;
use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::fun::{mk_function_str, mk_functions_rec_str};
use crate::backend::z3::intrinsics::array_null_const_name;
use crate::backend::z3::sort::get_generic_param_count;
use crate::backend::z3::sort::{collect_type_edges, resolve_type_name, scc_from_edges};
use crate::backend::z3::sort::{
    mk_enum_str, mk_named_tuple_str, mk_record_str, mk_unnamed_tuple_str,
};
use crate::ir::ctxt::IRContext;
use crate::ir::exp::Expression;
use crate::ir::fun::{FunDef, FunSig};
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
            undef_sorts,
            ty_registry,
            fn_registry,
            error_count: _,
            error_targets: _,
        } = ir;

        // Z3 prints "success" after every command by default.
        // 500 declarations = 500 "success" lines before the actual sat/unsat result.
        // Turn it off so our response parser only sees the verdict.
        l!(x, "(set-option :print-success false)");
        // When Z3 returns "sat", we need the actual concrete values (e.g., input_0 = "hello"),
        // not just the fact that a solution exists. Without this, (get-model) fails.
        l!(x, "(set-option :produce-models true)");
        // Proof objects explain WHY something is unsatisfiable. Building them requires extra
        // bookkeeping during search, slowing Z3 down. We only care whether sat/unsat, not why.
        l!(x, "(set-option :produce-proofs false)");
        // Unsat cores identify WHICH of your assertions conflict (e.g., "assertions 3 and 7
        // are contradictory"). Computing this requires tracking assertion contributions.
        // We don't debug unsatisfiability — we just move to the next error target.
        l!(x, "(set-option :produce-unsat-cores false)");
        // ── Reproducibility ──
        // Z3 has two layers: an inner SAT solver (raw boolean variables) and an outer SMT solver
        // (theories: integers, strings, sets). Both use randomization to decide things like
        // "which variable to try next" or "which theory to check first."
        //
        // Without fixed seeds, the same input can produce different results across runs:
        //   Monday: sat in 2 seconds (lucky variable ordering)
        //   Tuesday: timeout at 60 seconds (unlucky ordering)
        //
        // Fixed seeds make the search deterministic: same input → same path → same result.
        l!(x, "(set-option :sat.random_seed 42)");
        l!(x, "(set-option :smt.random_seed 42)");
        // ── Parallelism ──
        // Z3 can split its search across multiple threads, each exploring a different part
        // of the search space. If any thread finds a solution, Z3 returns sat.
        // N threads on N cores = each thread gets a dedicated core, no contention.
        l!(x, "(set-option :parallel.enable true)");
        l!(x, "(set-option :parallel.threads.max {})", num_cpu_cores());
        // "Cube and conquer": the main thread works alone, then after `delay` milliseconds
        // splits the problem into subproblems for worker threads.
        //   delay=10: "try alone for 10ms, if still stuck, split across threads"
        //   delay=5000: waits 5s before splitting — if the problem solves in 3s, threads sat idle
        //   delay=0: splits immediately, but splitting has overhead (copying state, coordination)
        // 10ms is a good default for hard problems — start parallelizing quickly.
        l!(x, "(set-option :parallel.conquer.delay 10)");
        // ── SAT Solver: Restarts ──
        // A restart throws away the current guess and starts over, but keeps learned facts.
        // Example: Z3 guesses x=50000. Doesn't work. Nearby values don't work either.
        // Restart: forget the guess, remember "50000 area is bad", try a different region.
        // Each restart is smarter because of accumulated knowledge from previous attempts.
        //
        // 100,000 restarts allowed. Each carries over learned facts, so later restarts are
        // much more targeted than early ones. Too low = gives up before finding a good path.
        // Restarts are sequential (one thread retrying). Conquer is parallel (multiple threads
        // searching different regions). They work together: each thread can restart within
        // its own subproblem.
        l!(x, "(set-option :sat.restart.max 100000)");
        // ── SMT Solver: Arithmetic ──
        //
        // Z3 has multiple arithmetic engines (values: 2, 3, 6).
        //   2 = old Simplex. Linear only (x + y = 5).
        //   3 = adds basic integer support.
        //   6 = newest. Handles mod, div, nonlinear (x * y), mixed int/real.
        // We use 6 because our interpreter uses mod (bitvector ops), integer division,
        // and set membership checks.
        l!(x, "(set-option :smt.arith.solver 6)");
        // ── SMT Solver: Case Splitting ──
        //
        // When Z3 encounters (or A B C), it must decide which branch to explore.
        // Values: 0, 1, 2, 3, 5.
        //   0 = try all branches blindly
        //   3 = relevancy-based: only split on branches relevant to the current goal
        //
        // Example: (assert (or (= x 1) (= x 2) (= x 3))) with (assert (> x 100))
        //   Mode 0: tries x=1, fails. Tries x=2, fails. Tries x=3, fails. Returns unsat.
        //   Mode 3: sees (> x 100) immediately rules out all three. Returns unsat without trying.
        //
        // Mode 3 is critical for us: our interpreter has hundreds of ite branches (every
        // if/match in Rust becomes an ite). Most are irrelevant to any given error target.
        l!(x, "(set-option :smt.case_split 3)");
        // ── SMT Solver: Phase Selection ──
        //
        // When Z3 picks a boolean variable to branch on, should it try true or false first?
        // Values: 0 (always true), 1 (always false), 3 (always false + caching), 4 (random).
        //
        // Example: (assert (=> p (= x 1))) (assert (=> (not p) (= x 2))) (assert (> x 1))
        //   Mode 0 (true first): tries p=true → x=1 → fails (> x 1). Backtracks, tries p=false.
        //   Mode 3 (false first): tries p=false → x=2 → sat immediately.
        //
        // Mode 3 tends to prune faster for verification problems because negative assignments
        // eliminate more possibilities. In our ite chains, the "else" branch is usually the
        // common/fast path, and the "then" branch is the error/special case.
        l!(x, "(set-option :smt.phase_selection 3)");
        // ── Quantifier Handling ──
        //
        // Z3 can't check every integer for (forall ((k Int)) ...). These options control
        // HOW Z3 picks which values to try.
        //
        // MBQI (Model-Based Quantifier Instantiation) — trial and error approach:
        //   1. Z3 GUESSES a model (ignoring the forall)
        //   2. CHECKS: does the forall hold in this model? Looks for a counterexample k.
        //   3. If counterexample found, adds it as a constraint, goes back to step 1.
        //   4. If no counterexample, the model is valid → sat.
        //
        // Example:
        //   (forall ((k Int)) (>= (select arr k) 0))   ; all elements non-negative
        //   (= (select arr 5) (- 3))                    ; but arr[5] = -3
        //
        //   Round 1: guess arr = all zeros. Forall holds. But arr[5]=0 conflicts with arr[5]=-3.
        //   Round 2: fix arr[5]=-3. Check forall with k=5: -3 >= 0? No! Contradiction → unsat.
        //
        // Without MBQI, Z3 stares at the infinite forall and returns "unknown".
        l!(x, "(set-option :smt.mbqi true)");
        //
        // E-matching — forward pattern matching approach (complementary to MBQI):
        //   Z3 looks at terms it already knows (e.g., (select arr 7) from another assertion)
        //   and matches them against quantifier patterns. If (select arr 7) matches the
        //   forall's pattern (select arr k), Z3 instantiates k=7 and checks that case.
        //
        //   MBQI works BACKWARD from models. E-matching works FORWARD from known terms.
        //   Both enabled = two independent strategies to attack quantifiers.
        l!(x, "(set-option :smt.ematching true)");
        //
        // Eager threshold: how aggressively to instantiate quantifiers before they're needed.
        //   Low (0.5) = only instantiate when stuck and need it (conservative)
        //   High (10.0) = instantiate all matching terms immediately (aggressive)
        // We use 10.0 because we have few quantifiers (ArrayIsEmpty) but they're critical.
        // Instantiate eagerly → avoid backtracking later.
        l!(x, "(set-option :smt.qi.eager_threshold 10.0)");
        //
        // Multi-patterns match multiple terms simultaneously in a forall.
        // If Z3 knows 100 (select arr _) terms and 10 (> _ 0) terms, there are 1000 possible
        // combinations. This cap controls how many combinations Z3 is allowed to try.
        // 1000 is generous enough for our use case with few quantifiers.
        l!(x, "(set-option :smt.qi.max_multi_patterns 1000)");
        // ── Auto-configuration ──
        //
        // Z3 normally analyzes your input and auto-tunes its settings. This would override
        // everything above. We disable it because we've tuned specifically for our problem
        // structure: recursive ADTs, strings, sets, bitvectors, and few quantifiers.
        l!(x, "(set-option :smt.auto_config false)");
        l!(x);

        // write the type parameters
        if !&undef_sorts.is_empty() {
            l!(x, "; Define Type Parameters of Function Signatures:");
            for sort in undef_sorts {
                l!(x, "(declare-sort {} 0)", sort);
            }
            l!(x); // add new line
        }

        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            l!(x, "; Define user-defined types");
            let edges = collect_type_edges(ty_registry.data_types());
            let mut sccs = scc_from_edges(&edges);

            // include truly isolated types
            let all_ids: BTreeSet<_> = ty_registry.data_types().keys().copied().collect();
            let covered: BTreeSet<_> = sccs.iter().flat_map(|s| s.iter().copied()).collect();
            for sid in all_ids.difference(&covered) {
                sccs.push(BTreeSet::from([*sid]));
            }

            // Deduplicate SCCs by type names to avoid declaring the same type multiple times
            // This happens with mutually recursive generic types that create multiple instantiations
            // We group by type name and prefer instances where type parameters match the type name
            let mut seen_type_names: HashSet<String> = HashSet::new();
            let mut name_to_best_sid: HashMap<String, UsrSortId> = HashMap::new();

            // find the best representative for each type name
            for scc in sccs.iter() {
                for &sid in scc {
                    let type_name = resolve_type_name(ir, sid);
                    let (_, type_params) = ir.ty_registry.reverse_lookup(sid);

                    // Check if this instance has "matching" type parameters
                    // (i.e., type parameters that start with the type name prefix)
                    let has_matching_params = type_params.iter().any(|sort| {
                        if let Sort::Uninterpreted(smt_name) = sort {
                            let name_str = smt_name.as_ref();
                            let prefix = format!("{}_", type_name);
                            name_str.starts_with(&prefix)
                        } else {
                            false
                        }
                    });

                    // If we haven't seen this type name, or this instance has matching params,
                    // update the best representative
                    if !seen_type_names.contains(&type_name) || has_matching_params {
                        seen_type_names.insert(type_name.clone());
                        name_to_best_sid.insert(type_name, sid);
                    }
                }
            }

            // Second pass: build deduplicated SCCs using the best representatives
            let mut seen_scc_signatures: HashSet<Vec<String>> = HashSet::new();
            let mut deduplicated_sccs: Vec<BTreeSet<UsrSortId>> = Vec::new();

            for scc in sccs.iter().rev() {
                // Map each sid to its best representative's sid
                let canonical_scc: BTreeSet<UsrSortId> = scc
                    .iter()
                    .map(|&sid| {
                        let type_name = resolve_type_name(ir, sid);
                        *name_to_best_sid.get(&type_name).unwrap_or(&sid)
                    })
                    .collect();

                // Create a signature for this SCC based on type names (sorted)
                let mut type_names: Vec<String> = canonical_scc
                    .iter()
                    .map(|&sid| resolve_type_name(ir, sid))
                    .collect();
                type_names.sort();

                // If we haven't seen this signature before, keep this SCC
                if seen_scc_signatures.insert(type_names) {
                    deduplicated_sccs.push(canonical_scc);
                }
            }

            for scc in deduplicated_sccs.iter() {
                let mut decl_headers = Vec::new();
                let mut decl_bodies = Vec::new();

                // Convert SCC to BTreeSet for efficient lookup
                let scc_set: BTreeSet<_> = scc.iter().copied().collect();

                for sid in scc {
                    let dt = ir.ty_registry.retrieve(*sid);
                    let type_name_str = resolve_type_name(ir, *sid);
                    let generic_param_count = get_generic_param_count(ir, *sid);

                    // Get the type parameters for this type
                    let (_, type_params) = ir.ty_registry.reverse_lookup(*sid);

                    // Extract type parameter names from the uninterpreted sorts
                    // For type parameters created with SmtSortName::new_type_param,
                    // the format is "{type_name}_{param_name}"
                    let type_param_names: Vec<String> = type_params
                        .iter()
                        .filter_map(|sort| {
                            if let Sort::Uninterpreted(smt_name) = sort {
                                let name_str = smt_name.as_ref();
                                let prefix = format!("{}_", type_name_str);
                                if name_str.starts_with(&prefix) {
                                    Some(name_str[prefix.len()..].to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();

                    decl_headers.push(format!("({} {})", type_name_str, generic_param_count));
                    let body_str = match dt {
                        DataType::Tuple(elems)
                            if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                        {
                            mk_unnamed_tuple_str(
                                type_name_str,
                                elems,
                                ir,
                                type_params,
                                &type_param_names,
                                &scc_set,
                            )
                        }
                        DataType::Tuple(elems) => mk_named_tuple_str(
                            type_name_str,
                            elems,
                            ir,
                            type_params,
                            &type_param_names,
                            &scc_set,
                        ),
                        DataType::Record(fields) => mk_record_str(
                            type_name_str,
                            fields,
                            ir,
                            type_params,
                            &type_param_names,
                            &scc_set,
                        ),
                        DataType::Enum(variants) => mk_enum_str(
                            type_name_str,
                            variants,
                            ir,
                            type_params,
                            &type_param_names,
                            &scc_set,
                        ),
                    };
                    decl_bodies.push(body_str);
                }

                // Output the combined command
                l!(
                    x,
                    "(declare-datatypes\n
                    \t({})\n
                    \t(\n
                    \t\t{}\n
                    \t)\n
                    )",
                    decl_headers.join(" "),
                    decl_bodies.join(" ")
                );
            }

            l!(x); // Empty line after types
        }

        // Declare null sentinel constants for every concrete user-defined sort.
        // Z3's @default is an output-only symbol (not valid input) for user-defined datatypes,
        // so ArrayEmpty / ArrayRemove / ArrayContainsKey use these symbolic constants instead.
        // Must appear BEFORE function definitions since function bodies may reference them.
        // Skip abstract/polymorphic sorts (those whose SMT names include uninterpreted sorts).
        {
            let mut declared_null_names: HashSet<String> = HashSet::new();
            let mut null_decls: Vec<(String, String)> = Vec::new();
            for sid in ty_registry.data_types().keys() {
                let (_, type_args) = ty_registry.reverse_lookup(*sid);
                // Skip abstract instantiations (those with uninterpreted sort arguments)
                if type_args
                    .iter()
                    .any(|s| matches!(s, Sort::Uninterpreted(_)))
                {
                    continue;
                }
                let v_sort = Sort::User(*sid);
                let const_name = array_null_const_name(&v_sort, ir);
                if declared_null_names.insert(const_name.clone()) {
                    let sort_str = format_sort_for_fn(&v_sort, ir);
                    null_decls.push((const_name, sort_str));
                }
            }
            if !null_decls.is_empty() {
                l!(x, "; Null sentinel constants for array default values");
                for (const_name, sort_str) in null_decls {
                    l!(x, "(declare-const {} {})", const_name, sort_str);
                }
                l!(x);
            }
        }

        // Integer string parsing helpers (hex / octal / binary).
        // Z3's built-in str.to_int only handles decimal; these define-fun-rec helpers
        // implement the correct base conversions.  They must be emitted BEFORE any
        // user-defined function that calls rusmart_from_hex/oct/bin_str.
        l!(x, "; Integer string parsing helpers");
        // --- hex ---
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
        // --- octal ---
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
        // --- binary ---
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
        if !fn_registry.lookup.is_empty() {
            l!(x, "; Define user-defined functions");

            // Detect IterChoose functions (those whose root body is IterChoose).
            // These use Hilbert choice semantics which can't be expressed as a define-fun
            // body in standard SMT-LIB2. Instead we generate them as uninterpreted
            // declare-fun declarations so callers can reference them with the right types.
            let mut choose_fids: BTreeSet<UsrFunId> = BTreeSet::new();
            for (_, instantiations) in &fn_registry.lookup {
                for (_, &fid) in instantiations {
                    let def = fn_registry.retrieve_def(fid);
                    let root_exp = def.body.lookup_exp(&def.root_exp_id);
                    if matches!(root_exp, Expression::IterChoose { .. }) {
                        choose_fids.insert(fid);
                    }
                }
            }

            // Emit choose functions as uninterpreted declare-fun BEFORE define-funs-rec
            // so they are available when called from within the recursive block.
            if !choose_fids.is_empty() {
                l!(
                    x,
                    "; Uninterpreted functions (Hilbert choice / epsilon semantics)"
                );
                let mut sorted_choose: Vec<UsrFunId> = choose_fids.iter().copied().collect();
                sorted_choose.sort();
                for fid in &sorted_choose {
                    let (function_name, _type_params) = resolve_function_name(ir, *fid);
                    let sig = fn_registry.retrieve_sig(*fid);
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
                }
                l!(x);
            }

            let edges = collect_function_call_edges(fn_registry);

            // Get all function IDs, excluding choose functions (already declared above)
            let all_ids: BTreeSet<_> = fn_registry
                .lookup
                .values()
                .flat_map(|insts| insts.iter().map(|(_, fn_id)| *fn_id))
                .filter(|fid| !choose_fids.contains(fid))
                .collect();

            // Group functions that have dependencies on each other
            // We'll use define-funs-rec for any group of interdependent functions
            // and define-fun only for truly isolated functions with no dependencies

            // Build a dependency graph: which functions have edges to/from other functions
            let mut has_dependencies: HashSet<UsrFunId> = HashSet::new();
            let all_fids: HashSet<UsrFunId> = all_ids.iter().copied().collect();

            for &(from, to) in &edges {
                if all_fids.contains(&from) && all_fids.contains(&to) {
                    has_dependencies.insert(from);
                    has_dependencies.insert(to);
                }
            }

            // Separate functions into two groups:
            // 1. Interdependent functions (use define-funs-rec for all)
            // 2. Truly isolated functions (use define-fun)
            let has_dependencies_set: BTreeSet<UsrFunId> =
                has_dependencies.iter().copied().collect();
            let mut interdependent_fids: Vec<UsrFunId> = has_dependencies.iter().copied().collect();
            let mut isolated_fids: Vec<UsrFunId> =
                all_ids.difference(&has_dependencies_set).copied().collect();

            // Sort for deterministic output
            interdependent_fids.sort();
            isolated_fids.sort();

            // Generate interdependent functions using define-funs-rec
            if !interdependent_fids.is_empty() {
                let mut function_data: Vec<(UsrFunId, String, Vec<Sort>, &FunSig, &FunDef)> =
                    Vec::new();
                let interdependent_set: BTreeSet<_> = interdependent_fids.iter().copied().collect();

                for fid in &interdependent_fids {
                    let (function_name, type_params) = resolve_function_name(ir, *fid);
                    let sig = ir.fn_registry.retrieve_sig(*fid);
                    let def = ir.fn_registry.retrieve_def(*fid);
                    function_data.push((*fid, function_name.to_string(), type_params, sig, def));
                }

                let functions: Vec<_> = function_data
                    .iter()
                    .map(|(fid, name, type_params, sig, def)| {
                        (*fid, name.as_str(), type_params.as_slice(), *sig, *def)
                    })
                    .collect();

                let functions_str = mk_functions_rec_str(&functions, ir, &interdependent_set);
                l!(x, "{}", functions_str);
            }

            // Generate isolated functions using define-fun
            for fid in isolated_fids {
                let (function_name, type_params) = resolve_function_name(ir, fid);
                let sig = ir.fn_registry.retrieve_sig(fid);
                let def = ir.fn_registry.retrieve_def(fid);
                let scc_set = BTreeSet::from([fid]);

                let function_str =
                    mk_function_str(function_name.as_ref(), &type_params, sig, def, ir, &scc_set);
                l!(x, "{}", function_str);
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
            stdout.read_to_string(&mut output).ok();
            output
        });

        // Read stderr in a separate thread
        let stderr_thread = thread::spawn(move || {
            let mut message = String::new();
            stderr.read_to_string(&mut message).ok();
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

        // Get the outputs from the reader threads
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
                let verdict = output
                    .lines()
                    .map(|l| l.trim())
                    .find(|&l| l == "sat" || l == "unsat" || l == "unknown");

                match verdict {
                    Some("unsat") => Response::Unsat,
                    Some("unknown") => Response::Unknown,
                    Some("sat") => {
                        if !exit_status.success() {
                            // Z3 found sat but crashed during model output (e.g. segfault in
                            // get-model). The "sat" verdict is still valid; model may be partial.
                            warn!(
                                "Z3 exited with status {} after reporting sat (model may be incomplete)",
                                exit_status
                            );
                        }
                        Response::Sat(output)
                    }
                    Some(other) => unreachable!("unexpected verdict: {}", other),
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
            .lookup
            .iter()
            .find(|(name, _)| name.as_ref() == top_level_fn)
            .unwrap_or_else(|| {
                panic!("top-level function '{}' not found in function registry", top_level_fn)
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
        // Single target {n}:     (set.member n (accessor result))
        // Merge target {a,b,c}:  (and (set.member a ...) (set.member b ...) (set.member c ...))
        let member_assertions: Vec<String> = target_ids
            .iter()
            .map(|&error_id| extract_error_assertion(&sig.ret_ty, &call_expr, error_id, ir))
            .collect();

        let assertion = match member_assertions.len() {
            1 => member_assertions.into_iter().next().unwrap(),
            _ => format!("(and {})", member_assertions.join(" ")),
        };

        // Append the comment, assertion, check-sat, and get-model to the query.
        let ids_str: Vec<String> = target_ids.iter().map(|id| id.to_string()).collect();
        query.push_str(&format!(
            "\n; Find input to {} that triggers error IDs {{{}}}\n",
            top_level_fn,
            ids_str.join(", ")
        ));
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
/// becomes `(set.singleton n)`, and `ErrMerge` becomes `(set.union ...)`.
/// To test whether a specific error site was reached, we check set membership:
/// `(set.member error_id result)`.
///
/// # How it works
///
/// The top-level function (e.g. `parse_toml`) typically returns a user-defined
/// enum like `ParseResult<T>` rather than a bare `Error`. This function inspects
/// the return sort and generates the appropriate assertion:
///
/// - **`Sort::Error`** — the function returns `Error` directly.
///   Emits: `(set.member error_id (fn input_0 ...))`.
///
/// - **`Sort::User` (enum)** — scans each variant for fields of type `Sort::Error`.
///   For each such field, emits a guarded assertion:
///   `(and (is-VariantName result) (set.member error_id (accessor result)))`.
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
        // Example: (set.member 3 (parse_toml input_0))
        Sort::Error => format!("(set.member {} {})", error_id, call_expr),
        // Function returns a user-defined enum — find the variant(s) that carry an Error field.
        // Example for ParseResult with Err(Error):
        //   (and (is-Err (parse_toml input_0))
        //        (set.member 3 (field_ParseResult_Err_1_ (parse_toml input_0))))
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
                                        let tester = format!("is-{}", vname);
                                        let accessor =
                                            format!("field_{}_{}_{}_", type_name, vname, i + 1);
                                        assertions.push(format!(
                                            "(and ({} {}) (set.member {} ({} {})))",
                                            tester, call_expr, error_id, accessor, call_expr
                                        ));
                                    }
                                }
                            }
                            Variant::Record(fields) => {
                                for (field_key, field_sort) in fields {
                                    if *field_sort == Sort::Error {
                                        let tester = format!("is-{}", vname);
                                        let accessor = format!(
                                            "record_{}_{}_{}_",
                                            type_name, vname, field_key
                                        );
                                        assertions.push(format!(
                                            "(and ({} {}) (set.member {} ({} {})))",
                                            tester, call_expr, error_id, accessor, call_expr
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

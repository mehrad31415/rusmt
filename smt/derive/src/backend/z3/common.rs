//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::ContentBuilder;
use crate::backend::codegen::l;
use crate::backend::error::{BackendError, BackendResult};
use crate::backend::response::BACKEND_TIMEOUT;
use crate::backend::response::Response;
use crate::backend::z3::fun::collect_function_call_edges;
use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::fun::scc_from_edges_fn;
use crate::backend::z3::fun::{mk_function_rec_str, mk_function_str, mk_functions_rec_str};
use crate::backend::z3::ty::get_generic_param_count;
use crate::backend::z3::ty::{collect_type_edges, resolve_type_name, scc_from_edges};
use crate::backend::z3::ty::{
    mk_enum_str, mk_named_tuple_str, mk_record_str, mk_unnamed_tuple_str,
};
use crate::ir::ctxt::IRContext;
use crate::ir::fun::{FunDef, FunSig};
use crate::ir::index::{UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort};
use command_group::CommandGroup;
use log::{debug, warn};
use std::collections::{BTreeSet, HashSet};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

/// A wrapper for Z3 backends that implements the `CodeGen` trait.
pub struct CodeGenZ3;

impl CodeGen for CodeGenZ3 {
    /// Constructs a new `CodeGenZ3` wrapper.
    fn new() -> Self {
        Self
    }

    /// Returns the name of the backend code generator, which is "z3_chc".
    fn name(&self) -> String {
        "z3_chc".to_string()
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
            error_locations,
        } = ir;

        // disable success messages
        l!(x, "(set-option :print-success false)");
        // enable model generation in case of satisfiability for debugging
        l!(x, "(set-option :produce-models true)");
        // disable proof generation to save resources
        l!(x, "(set-option :produce-proofs false)");
        // disable unsat core generation to save resources
        l!(x, "(set-option :produce-unsat-cores false)");

        // === Reproducibility ===
        l!(x, "(set-option :sat.random_seed 42)");
        l!(x, "(set-option :smt.random_seed 42)");

        // === Parallelism ===
        l!(x, "(set-option :parallel.enable true)");
        l!(x, "(set-option :parallel.threads.max 8)"); // adjust to your CPU cores
        l!(x, "(set-option :parallel.conquer.delay 10)");

        // === SAT Solver Optimizations ===
        l!(x, "(set-option :sat.restart.max 100000)");

        // === SMT Solver Optimizations ===
        l!(x, "(set-option :smt.arith.solver 6)"); // most advanced arithmetic solver
        l!(x, "(set-option :smt.case_split 3)"); // more aggressive case splitting
        l!(x, "(set-option :smt.phase_selection 3)"); // phase caching

        // === Quantifier Handling (IMPORTANT: You have quantifiers!) ===
        l!(x, "(set-option :smt.mbqi true)"); // KEEP enabled for quantifiers
        l!(x, "(set-option :smt.qi.eager_threshold 10.0)"); // control eager instantiation
        l!(x, "(set-option :smt.qi.max_multi_patterns 1000)"); // limit pattern matching
        l!(x, "(set-option :smt.ematching true)"); // enable E-matching for quantifiers

        // === Arithmetic Optimizations ===
        l!(x, "(set-option :smt.arith.nl false)"); // disable nonlinear if you don't need it

        // === Auto-configuration ===
        l!(x, "(set-option :smt.auto_config false)"); // manual control for consistency
        l!(x); // add new line

        // Define the Error datatype
        // Error is represented as a set of error IDs (integers)
        // We model it as a recursive datatype that can be empty or contain error markers
        l!(x, "; Define Error type (set of error markers)");
        l!(x, "(declare-datatypes");
        l!(x, "\t((Error 0))");
        l!(x, "\t(");
        l!(x, "\t\t((ErrEmpty)"); // No error
        l!(x, "\t\t (ErrSingle (err_id Int))"); // Single error with ID
        l!(x, "\t\t (ErrMerge (err_left Error) (err_right Error)))"); // Union of two errors
        l!(x, "\t)");
        l!(x, ")");
        l!(x); // add new line

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
            let mut name_to_best_sid: std::collections::HashMap<String, UsrSortId> = std::collections::HashMap::new();
            
            // First pass: find the best representative for each type name
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

        // Function registry
        if !fn_registry.lookup.is_empty() {
            l!(x, "; Define user-defined functions");

            let edges = collect_function_call_edges(fn_registry);
            let mut sccs = scc_from_edges_fn(&edges);

            // include truly isolated functions
            let all_ids: BTreeSet<_> = fn_registry
                .lookup
                .values()
                .flat_map(|insts| insts.iter().map(|(_, fn_id)| *fn_id))
                .collect();
            let covered: BTreeSet<_> = sccs.iter().flat_map(|s| s.iter().copied()).collect();
            for fid in all_ids.difference(&covered) {
                sccs.push(BTreeSet::from([*fid]));
            }

            // Build a set of edges for quick lookup (to check for self-loops)
            let edge_set: HashSet<(UsrFunId, UsrFunId)> = edges.iter().copied().collect();

            // Convert SCC to BTreeSet for efficient lookup
            // Iterate over all functions and instantiations
            for scc in sccs.iter().rev() {
                let scc_set: BTreeSet<_> = scc.iter().copied().collect();

                if scc.len() > 1 {
                    // Mutually recursive functions: use define-funs-rec
                    let mut function_data: Vec<(UsrFunId, String, Vec<Sort>, &FunSig, &FunDef)> =
                        Vec::new();
                    for fid in scc {
                        let (function_name, type_params) = resolve_function_name(ir, *fid);
                        let sig = ir.fn_registry.retrieve_sig(*fid);
                        let def = ir.fn_registry.retrieve_def(*fid);
                        function_data.push((
                            *fid,
                            function_name.to_string(),
                            type_params,
                            sig,
                            def,
                        ));
                    }

                    let functions: Vec<_> = function_data
                        .iter()
                        .map(|(fid, name, type_params, sig, def)| {
                            (*fid, name.as_str(), type_params.as_slice(), *sig, *def)
                        })
                        .collect();

                    let functions_str = mk_functions_rec_str(&functions, ir, &scc_set);
                    l!(x, "{}", functions_str);
                } else {
                    // Single function in SCC
                    let fid = scc.iter().next().unwrap();
                    let (function_name, type_params) = resolve_function_name(ir, *fid);
                    let sig = ir.fn_registry.retrieve_sig(*fid);
                    let def = ir.fn_registry.retrieve_def(*fid);

                    // Check if it's self-recursive (has edge to itself)
                    if edge_set.contains(&(*fid, *fid)) {
                        // Self-recursive: use define-fun-rec
                        let function_str = mk_function_rec_str(
                            function_name.as_ref(),
                            &type_params,
                            sig,
                            def,
                            ir,
                            &scc_set,
                        );
                        l!(x, "{}", function_str);
                    } else {
                        // Non-recursive: use define-fun
                        let function_str = mk_function_str(
                            function_name.as_ref(),
                            &type_params,
                            sig,
                            def,
                            ir,
                            &scc_set,
                        );
                        l!(x, "{}", function_str);
                    }
                }
            }

            l!(x); // Empty line after functions
        }

        // Add helper functions for error handling
        l!(x, "; Helper functions for error handling");
        l!(x, "(define-fun err-fresh ((id Int)) Error");
        l!(x, "\t(ErrSingle id))");
        l!(x);
        l!(x, "(define-fun err-merge ((e1 Error) (e2 Error)) Error");
        l!(x, "\t(ErrMerge e1 e2))");
        l!(x);
        l!(x, "(define-fun err-is-empty ((e Error)) Bool");
        l!(x, "\t(is-ErrEmpty e))");
        l!(x);
        
        // Recursive function to check if error contains a specific ID
        l!(x, "(define-fun-rec err-contains ((e Error) (id Int)) Bool");
        l!(x, "\t(or");
        l!(x, "\t\t(and (is-ErrSingle e) (= (err_id e) id))");
        l!(x, "\t\t(and (is-ErrMerge e)");
        l!(x, "\t\t\t(or (err-contains (err_left e) id)");
        l!(x, "\t\t\t    (err-contains (err_right e) id)))))");
        l!(x);
        
        // Don't add check-sat or get-model here - they will be added by error discovery
        l!(x, "; Base SMT-LIB definitions complete");
        l!(x, "; Add error-specific assertions below");
        l!(x);

        Ok(x.build())
    }

    /// Execute the backend solver on the generated SMTLIB2 file.
    fn invoke_backend(&self, path_src: &Path) -> BackendResult<Response> {
        let mut cmd = Command::new("z3");
        cmd.arg("-smt2")
            .arg(&path_src)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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
        
        let timestamp = SystemTime::now();

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
                match timestamp.elapsed() {
                    Ok(elapsed) if elapsed > BACKEND_TIMEOUT => {
                        let _ = child.kill();
                        return None;
                    }
                    Err(_) => {
                        // Time measurement error, treat as timeout
                        let _ = child.kill();
                        return None;
                    }
                    _ => {}
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
                if !output.is_empty() {
                    warn!("Output received from timeout execution: {}", output);
                }
                Response::Timeout
            }
            Some(exit_status) => {
                if !exit_status.success() {
                    warn!("Backend execution failed with status: {}", exit_status);
                    if !output.is_empty() {
                        warn!("Stdout: {}", output);
                    }
                    if !stderr_output.is_empty() {
                        warn!("Stderr: {}", stderr_output);
                    }
                    return Err(BackendError);
                }
                
                let trimmed = output.trim();

                if trimmed == "unknown" {
                    Response::Unknown
                } else if trimmed == "unsat" {
                    Response::Unsat
                } else if trimmed.starts_with("sat") {
                    Response::Sat(output)
                } else if trimmed.is_empty() {
                    warn!("Z3 returned empty output (likely only declarations, no queries)");
                    Response::Unknown
                } else {
                    warn!("Invalid Z3 response: {}", trimmed);
                    return Err(BackendError);
                }
            }
        };

        Ok(response)
    }
}

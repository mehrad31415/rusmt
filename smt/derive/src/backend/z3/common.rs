//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::Response;
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::assert_axioms;
use crate::backend::z3::fun::{create_function_declaration, process_function_body};
use crate::backend::z3::sort::sort_to_z3;
use crate::backend::z3::ty::{
    collect_type_edges, mk_enum, mk_named_tuple, mk_record, mk_unnamed_tuple, scc_from_edges,
};
use crate::backend::z3::unimplemented::{CloakManager, MapLengthManager};
use crate::ir::ctxt::IRContext;
use crate::ir::fun::FunDef;
use crate::ir::index::UsrFunId;
use crate::ir::sort::DataType;
use crate::parser::ctxt::Refinement;
use crate::{backend::codegen::CodeGen, ir::index::UsrSortId};
use core::panic;
use log::debug;
use rusmart_utils::config::NUM_CPU_CORES;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use z3::datatype_builder::create_datatypes;
use z3::{
    Config, Context, DatatypeBuilder, FuncDecl, SatResult, Solver, Sort, ast, ast::Ast,
    set_global_param,
};

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
    fn process(&self, ir: &IRContext, workspace: &Path) -> BackendResult<Response> {
        // destructure the IRContext
        let IRContext {
            desc,
            undef_sorts,
            ty_registry,
            fn_registry,
            axiom_registry,
        } = ir;

        set_global_param("timeout", "60000"); // 1 minute
        set_global_param("memory_max_size", "4096"); // 4GB
        set_global_param("rlimit", "100000");
        set_global_param("model", "true");
        set_global_param("proof", "false");
        set_global_param("unsat_core", "false");
        set_global_param("sat.random_seed", "42");
        set_global_param("smt.random_seed", "42");
        set_global_param("parallel.threads.max", NUM_CPU_CORES.to_string().as_str());

        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);

        // destructure the Refinement
        let Refinement { fn_impl, fn_spec } = desc;
        debug!("verification of impl-spec pair: {fn_impl} <-> {fn_spec}");

        // mapping from smt sort name to the corresponding z3 sort
        let mut sort_map = HashMap::new();
        // write the type parameters
        if !&undef_sorts.is_empty() {
            debug!("Define Type Parameters of Function Signatures");
            for sort in undef_sorts {
                let z3_sort = Sort::uninterpreted(&ctx, sort.as_ref().into());
                sort_map.insert(sort.clone(), z3_sort.clone());
            }
        }

        let mut ty_map = HashMap::new();
        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            debug!("Define user-defined types");
            let mutually_dependent_types =
                scc_from_edges(&collect_type_edges(ty_registry.data_types()));
            for sid_set in &mutually_dependent_types {
                // not mutually recursive
                if sid_set.len() == 1 {
                    let mutually_recursive = false;
                    let mut iter = sid_set.iter();
                    let sid = iter.next().expect("set should not be empty");
                    if iter.next().is_some() {
                        panic!("set should have only one element");
                    }
                    let dt = ir.ty_registry.retrieve(*sid);
                    let z3_sort = match dt {
                        DataType::Tuple(elems)
                            if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                        {
                            if ir.ty_registry.reverse_lookup(*sid).1 != elems {
                                panic!("Tuples elements are not consistent");
                            }
                            // it is an unnamed tuple
                            let dt = mk_unnamed_tuple(
                                &ctx,
                                *sid,
                                elems,
                                ir,
                                &ty_map,
                                mutually_recursive,
                            )
                            .finish();
                            (dt.sort, dt.variants)
                        }
                        DataType::Tuple(elems) => {
                            let dt =
                                mk_named_tuple(&ctx, *sid, elems, ir, &ty_map, mutually_recursive)
                                    .finish();
                            (dt.sort, dt.variants)
                        }
                        DataType::Record(fields) => {
                            let dt = mk_record(&ctx, *sid, fields, ir, &ty_map, mutually_recursive)
                                .finish();
                            (dt.sort, dt.variants)
                        }
                        DataType::Enum(variants) => {
                            let dt = mk_enum(&ctx, *sid, variants, ir, &ty_map, mutually_recursive)
                                .finish();
                            (dt.sort, dt.variants)
                        }
                    };
                    ty_map.insert(*sid, z3_sort);
                } else {
                    let mutually_recursive = true;
                    let mut ids: Vec<UsrSortId> = Vec::new();
                    let mut builders: Vec<DatatypeBuilder> = Vec::new();
                    for sid in sid_set {
                        let dt = ir.ty_registry.retrieve(*sid);
                        match dt {
                            DataType::Tuple(elems)
                                if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                            {
                                if ir.ty_registry.reverse_lookup(*sid).1 != elems {
                                    panic!("Tuples elements are not consistent");
                                }
                                // it is an unnamed tuple
                                let dt = mk_unnamed_tuple(
                                    &ctx,
                                    *sid,
                                    elems,
                                    ir,
                                    &ty_map,
                                    mutually_recursive,
                                );
                                ids.push(*sid);
                                builders.push(dt);
                            }
                            DataType::Tuple(elems) => {
                                let dt = mk_named_tuple(
                                    &ctx,
                                    *sid,
                                    elems,
                                    ir,
                                    &ty_map,
                                    mutually_recursive,
                                );
                                ids.push(*sid);
                                builders.push(dt);
                            }
                            DataType::Record(fields) => {
                                let dt =
                                    mk_record(&ctx, *sid, fields, ir, &ty_map, mutually_recursive);
                                ids.push(*sid);
                                builders.push(dt);
                            }
                            DataType::Enum(variants) => {
                                let dt =
                                    mk_enum(&ctx, *sid, variants, ir, &ty_map, mutually_recursive);
                                ids.push(*sid);
                                builders.push(dt);
                            }
                        };
                    }
                    // now finish the data types
                    let dts = create_datatypes(builders);
                    for (id, sorts) in ids.into_iter().zip(dts.into_iter()) {
                        let z3_sort = (sorts.sort, sorts.variants);
                        ty_map.insert(id, z3_sort);
                    }
                }
            }
        }

        let mut cloak_manager = CloakManager::new(&ctx);
        let mut map_length_manager = MapLengthManager::new();
        let mut axiomatic_parameters: HashMap<String, ast::Dynamic> = HashMap::new();

        let mut fn_map = HashMap::new();
        // function registry
        if !fn_registry.lookup.is_empty() {
            debug!("Define user-defined functions");
            // declare all function signatures
            for (fn_name, instantiations) in &fn_registry.lookup {
                for (generics, fn_id) in instantiations {
                    let sig = fn_registry.retrieve_sig(*fn_id);

                    // Create function declaration
                    let fn_decl = create_function_declaration(
                        &ctx,
                        fn_name,
                        generics,
                        sig,
                        ir,
                        &ty_map,
                        &mut sort_map,
                    );

                    fn_map.insert(*fn_id, fn_decl);
                }
            }

            // define function bodies
            for (fn_name, instantiations) in &fn_registry.lookup {
                for fn_id in instantiations.values() {
                    let def = fn_registry.retrieve_def(*fn_id);

                    match def {
                        FunDef::Defined(exp_registry, root_exp_id) => {
                            // Process the function body
                            process_function_body(
                                &ctx,
                                &solver,
                                *fn_id,
                                exp_registry,
                                *root_exp_id,
                                ir,
                                &fn_map,
                                &ty_map,
                                &sort_map,
                                &mut cloak_manager,
                                &mut map_length_manager,
                                &mut axiomatic_parameters,
                            );
                        }
                        FunDef::Uninterpreted => {
                            debug!("Function {fn_name} is uninterpreted");
                        }
                    }
                }
            }
        }

        // write the axioms
        if !axiom_registry.lookup.is_empty() {
            debug!("Define axioms");
            for axiom in axiom_registry.lookup.values() {
                for axiom_id in axiom.values() {
                    let predicate = axiom_registry.retrieve(*axiom_id);
                    assert_axioms(
                        &ctx,
                        ir,
                        &solver,
                        predicate,
                        &fn_map,
                        &ty_map,
                        &sort_map,
                        &mut cloak_manager,
                        &mut map_length_manager,
                        &mut axiomatic_parameters,
                    );
                }
            }
        }

        debug!("Prove the equivalence of the operational and denotational semantics");
        let impl_id = ir.fn_registry.get_lookup(fn_impl);
        let spec_id = ir.fn_registry.get_lookup(fn_spec);
        assert_eq!(
            impl_id.len(),
            spec_id.len(),
            "impl/spec overload counts differ"
        );
        let impl_spec_pairs: Vec<(UsrFunId, UsrFunId)> = impl_id
            .iter()
            .map(|(sig, &impl_fid)| {
                let spec_fids: Vec<_> = spec_id
                    .iter()
                    .filter(|(k, _)| *k == sig) // Assuming you're looking up by signature
                    .collect();
                match spec_fids.len() {
                    0 => panic!("spec missing overload for signature {:?}", sig),
                    1 => {
                        let &spec_fid = spec_fids[0].1;
                        (impl_fid, spec_fid)
                    }
                    _ => panic!(
                        "multiple spec overloads found for signature {:?}, expected exactly one",
                        sig
                    ),
                }
            })
            .collect();

        for (impl_id, spec_id) in impl_spec_pairs {
            let impl_func = fn_map
                .get(&impl_id)
                .unwrap_or_else(|| panic!("Implementation function not found: {:?}", impl_id));
            let spec_func = fn_map
                .get(&spec_id)
                .unwrap_or_else(|| panic!("Specification function not found: {:?}", spec_id));

            let sig = ir.fn_registry.retrieve_sig(impl_id);
            let mut args = Vec::new();

            for (i, (param_name, param_type)) in sig.params.iter().enumerate() {
                let z3_sort = sort_to_z3(param_type, &ctx, ir, None, &ty_map);
                let arg =
                    FuncDecl::new(&ctx, format!("arg_{i}_{param_name}"), &[], &z3_sort).apply(&[]);
                args.push(arg);
            }
            let arg_refs: Vec<&dyn Ast> = args.iter().map(|a| a as &dyn Ast).collect();
            let impl_call = impl_func.apply(&arg_refs);
            let spec_call = spec_func.apply(&arg_refs);
            let equivalence = impl_call._eq(&spec_call).not();

            // Assert the equivalence
            solver.assert(&equivalence);
        }

        let res = match solver.check() {
            SatResult::Unsat => {
                // let proof = solver.get_proof().expect("proof not available");
                // println!("Unsat: {proof:?}");
                Response::Unsat
            }
            SatResult::Sat => {
                let model = solver.get_model().expect("Model not found");
                debug!("Sat: {model:?}");
                Response::Sat
            }
            SatResult::Unknown => {
                // let reason = solver
                //     .get_reason_unknown()
                //     .expect("Reason unknown not available");
                // debug!("Unknown: {reason:?}");
                Response::Unknown
            }
        };

        for a in solver.get_assertions() {
            println!("Assertion: {}", a);
            let _ = a.simplify(); // will panic if an assertion is invalid/null
            println!("will panic if an assertion is invalid/null");
        }

        // let smt = solver.to_smt2();
        println!("xxx");
        println!("solver check: {:?}", solver.check());
        fs::write(workspace.join(format!("response.{}", self.flavor())), "x").unwrap();

        Ok(Response::Sat)
    }
}

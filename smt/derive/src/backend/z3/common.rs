//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::Response;
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::process_axiom_body;
use crate::backend::z3::fun::{create_function_declaration, process_function_body};
use crate::backend::z3::sort::sort_to_z3;
use crate::backend::z3::ty::{
    collect_type_edges, mk_enum, mk_named_tuple, mk_record, mk_unnamed_tuple, scc_from_edges,
};
use crate::backend::z3::unimplemented::{CloakManager, MapLengthManager};
use crate::ir::ctxt::IRContext;
use crate::ir::fun::FunDef;
use crate::ir::fun::FunSig;
use crate::ir::index::UsrFunId;
use crate::ir::sort::DataType;
use crate::ir::sort::Sort;
use crate::parser::ctxt::Refinement;
use core::panic;
use log::debug;
use rusmart_utils::config::NUM_CPU_CORES;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use z3::Model;
use z3::ast::Ast;
use z3::ast::Dynamic;
use z3::datatype_builder::create_datatypes;
use z3::{Config, Context, SatResult, Solver, ast, set_global_param};

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
    fn process(
        &self,
        ir: &IRContext,
        workspace: &Path,
    ) -> BackendResult<(Response, Option<Model>)> {
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
        set_global_param("parallel.enable", "true");
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
                let z3_sort = z3::Sort::uninterpreted(&ctx, sort.as_ref().into());
                sort_map.insert(sort.clone(), z3_sort.clone());
            }
        }

        let mut ty_map = HashMap::new();
        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            debug!("Define user-defined types");
            let edges = collect_type_edges(ty_registry.data_types());
            let mut sccs = scc_from_edges(&edges);

            // include truly isolated types
            let all_ids: BTreeSet<_> = ty_registry.data_types().keys().copied().collect();
            let covered: BTreeSet<_> = sccs.iter().flat_map(|s| s.iter().copied()).collect();
            for sid in all_ids.difference(&covered) {
                sccs.push(BTreeSet::from([*sid]));
            }

            for scc in sccs.iter().rev() {
                let mut ids = Vec::new();
                let mut builders = Vec::new();

                for sid in scc {
                    let dt = ir.ty_registry.retrieve(*sid);
                    let dt_builder = match dt {
                        DataType::Tuple(elems)
                            if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                        {
                            // unnamed tuple
                            mk_unnamed_tuple(&ctx, *sid, elems, ir, &ty_map, scc)
                        }
                        DataType::Tuple(elems) => {
                            mk_named_tuple(&ctx, *sid, elems, ir, &ty_map, scc)
                        }
                        DataType::Record(fields) => mk_record(&ctx, *sid, fields, ir, &ty_map, scc),
                        DataType::Enum(variants) => mk_enum(&ctx, *sid, variants, ir, &ty_map, scc),
                    };
                    ids.push(*sid);
                    builders.push(dt_builder);
                }

                // now finish the data types
                let dts = create_datatypes(builders);
                for (id, dt) in ids.into_iter().zip(dts.into_iter()) {
                    ty_map.insert(id, dt);
                }
            }
        }

        let mut cloak_manager = CloakManager::new(&ctx);
        let mut map_length_manager = MapLengthManager::new();
        let mut axiomatic_parameters: HashMap<String, ast::Dynamic> = HashMap::new();
        let mut fn_map = HashMap::new();
        let mut axiom_map = HashMap::new();

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
                        ir,
                        fn_name.to_string(),
                        generics,
                        sig,
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
                                ir,
                                *fn_id,
                                exp_registry,
                                *root_exp_id,
                                &ty_map,
                                &sort_map,
                                &mut cloak_manager,
                                &mut map_length_manager,
                                &mut axiomatic_parameters,
                                &fn_map,
                            );
                        }
                        FunDef::Uninterpreted => {
                            debug!("Function {fn_name} is uninterpreted");
                        }
                    }
                }
            }
        }

        if !axiom_registry.lookup.is_empty() {
            debug!("Define axioms (declare)");
            for (axiom_name, instantiations) in &axiom_registry.lookup {
                for (generics, axiom_id) in instantiations {
                    let p = axiom_registry.retrieve(*axiom_id);
                    let sig = FunSig {
                        params: p.params.clone(),
                        ret_ty: Sort::Boolean,
                    };

                    let decl = create_function_declaration(
                        &ctx,
                        ir,
                        axiom_name.to_string(),
                        generics,
                        &sig,
                        &ty_map,
                        &mut sort_map,
                    );
                    axiom_map.insert(*axiom_id, decl);
                }
            }

            for (_axiom_name, instantiations) in &axiom_registry.lookup {
                for axiom_id in instantiations.values() {
                    let p = axiom_registry.retrieve(*axiom_id);
                    process_axiom_body(
                        &ctx,
                        &solver,
                        ir,
                        *axiom_id,
                        &p.params,
                        &p.body_reg,
                        p.body_exp,
                        &ty_map,
                        &sort_map,
                        &mut cloak_manager,
                        &mut map_length_manager,
                        &mut axiomatic_parameters,
                        &axiom_map,
                        &fn_map,
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

        let impl_spec_pairs: BTreeSet<(UsrFunId, UsrFunId)> = impl_id
            .iter()
            .map(|(sig, &impl_fid)| {
                let spec_fids: Vec<_> = spec_id.iter().filter(|(k, _)| *k == sig).collect();
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

        let spec_impl_pairs: BTreeSet<(UsrFunId, UsrFunId)> = spec_id
            .iter()
            .map(|(sig, &spec_fid)| {
                let impl_fids: Vec<_> = impl_id.iter().filter(|(k, _)| *k == sig).collect();
                match impl_fids.len() {
                    0 => panic!("spec missing overload for signature {:?}", sig),
                    1 => {
                        let &impl_fid = impl_fids[0].1;
                        (impl_fid, spec_fid)
                    }
                    _ => panic!(
                        "multiple impl overloads found for signature {:?}, expected exactly one",
                        sig
                    ),
                }
            })
            .collect();

        // sanity check
        assert_eq!(spec_impl_pairs, impl_spec_pairs);

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
                    Dynamic::fresh_const(&ctx, format!("arg_{i}_{param_name}").as_str(), &z3_sort);
                args.push(arg);
            }
            let arg_refs: Vec<&dyn Ast> = args.iter().map(|a| a as &dyn Ast).collect();
            let impl_call: Dynamic = impl_func.apply(&arg_refs);
            let spec_call: Dynamic = spec_func.apply(&arg_refs);
            let equivalence = impl_call._eq(&spec_call).not();

            // Assert the equivalence
            solver.assert(&equivalence);
        }

        let (res, model) = match solver.check() {
            SatResult::Unsat => (Response::Unsat, None),
            SatResult::Sat => {
                let model = solver.get_model().expect("Model not found");
                (Response::Sat, Some(model))
            }
            SatResult::Unknown => (Response::Unknown, None),
        };

        fs::write(
            workspace.join(format!("response.{}", self.flavor())),
            solver.to_smt2(),
        )
        .unwrap();

        Ok((res, model))
    }
}

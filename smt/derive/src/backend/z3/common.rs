//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::l;
use crate::backend::error::BackendResult;
use crate::backend::z3::ty::{collect_type_edges, scc_from_edges};
use crate::ir::ctxt::IRContext;
use crate::ir::sort::DataType;
use std::collections::BTreeSet;
use std::collections::HashMap;

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
        } = ir;

        // disable success messages
        l!(x, "(set-option :print-success false)");
        // enable model generation in case of satisfiability for debugging
        l!(x, "(set-option :produce-models true)");
        // disable proof generation to save resources
        l!(x, "(set-option :produce-proofs false)");
        // disable unsat core generation to save resources
        l!(x, "(set-option :produce-unsat-cores false)");
        // set a timeout of 60000 milliseconds (1 minute)
        l!(x, "(set-option :timeout 60000)"); // 1 minute
        // set resource limit to 100000
        l!(x, "(set-option :rlimit 100000)");
        // set random seed and enable parallelism
        l!(x, "(set-option :sat.random_seed 42)");
        l!(x, "(set-option :smt.random_seed 42)");
        l!(x, "(set-option :parallel.enable true)");
        l!(x); // add new line

        // mapping from smt sort name to the corresponding z3 sort
        let mut sort_map = HashMap::new();
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

            for scc in sccs.iter().rev() {
                let mut decl_headers = Vec::new();
                let mut decl_bodies = Vec::new();

                for sid in scc {
                    let dt = ir.ty_registry.retrieve(*sid);

                    // 1. Resolve the name for the declaration header
                    let type_name_str = resolve_type_name(ir, *sid);
                    decl_headers.push(format!("({} 0)", type_name_str));

                    // 2. Build the body definition
                    let body_str = match dt {
                        DataType::Tuple(elems)
                            if ir.ty_registry.reverse_lookup(*sid).0.is_none() =>
                        {
                            mk_unnamed_tuple_str(type_name_str, elems, ir)
                        }
                        DataType::Tuple(elems) => mk_named_tuple_str(type_name_str, elems, ir),
                        DataType::Record(fields) => mk_record_str(type_name_str, fields, ir),
                        DataType::Enum(variants) => mk_enum_str(type_name_str, variants, ir),
                    };
                    decl_bodies.push(body_str);
                }

                // Output the combined command
                l!(
                    x,
                    "(declare-datatypes ({}) ({}))",
                    decl_headers.join(" "),
                    decl_bodies.join(" ")
                );
            }
            l!(x); // Empty line after types
        }

        // let mut cloak_manager = CloakManager::new(&ctx);
        // let mut map_length_manager = MapLengthManager::new();
        // let mut axiomatic_parameters: HashMap<String, ast::Dynamic> = HashMap::new();
        // let mut fn_map = HashMap::new();
        // let mut axiom_map = HashMap::new();

        // // function registry
        // if !fn_registry.lookup.is_empty() {
        //     debug!("Define user-defined functions");
        //     // declare all function signatures
        //     for (fn_name, instantiations) in &fn_registry.lookup {
        //         for (generics, fn_id) in instantiations {
        //             let sig = fn_registry.retrieve_sig(*fn_id);

        //             // Create function declaration
        //             let fn_decl = create_function_declaration(
        //                 &ctx,
        //                 ir,
        //                 fn_name.to_string(),
        //                 generics,
        //                 sig,
        //                 &ty_map,
        //                 &mut sort_map,
        //             );

        //             fn_map.insert(*fn_id, fn_decl);
        //         }
        //     }

        //     // define function bodies
        //     for (fn_name, instantiations) in &fn_registry.lookup {
        //         for fn_id in instantiations.values() {
        //             let def = fn_registry.retrieve_def(*fn_id);

        //             match def {
        //                 FunDef::Defined(exp_registry, root_exp_id) => {
        //                     // Process the function body
        //                     process_function_body(
        //                         &ctx,
        //                         &solver,
        //                         ir,
        //                         *fn_id,
        //                         exp_registry,
        //                         *root_exp_id,
        //                         &ty_map,
        //                         &sort_map,
        //                         &mut cloak_manager,
        //                         &mut map_length_manager,
        //                         &mut axiomatic_parameters,
        //                         &fn_map,
        //                     );
        //                 }
        //             }
        //         }
        //     }
        // }
        Ok(x.build())
    }
}

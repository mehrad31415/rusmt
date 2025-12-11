//! This module provides the specific implementation of the `CodeGen` trait for Z3.

use crate::backend::codegen::CodeGen;
use crate::backend::codegen::ContentBuilder;
use crate::backend::codegen::l;
use crate::backend::error::BackendResult;
use crate::backend::z3::ty::{collect_type_edges, resolve_type_name, scc_from_edges};
use crate::backend::z3::ty::{
    mk_enum_str, mk_named_tuple_str, mk_record_str, mk_unnamed_tuple_str,
};
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

        // // Function registry
        // if !fn_registry.lookup.is_empty() {
        //     l!(x, "; Define user-defined functions (Mutually Recursive)");

        //     // We need two parallel lists for `define-funs-rec`:
        //     // 1. Function headers: (name ((param type) ...) ret_type)
        //     // 2. Function bodies: (expression)
        //     let mut decl_headers = Vec::new();
        //     let mut decl_bodies = Vec::new();

        //     // Iterate over all functions and instantiations
        //     for (fn_name, instantiations) in &fn_registry.lookup {
        //         for (generics, fn_id) in instantiations {
        //             let sig = fn_registry.retrieve_sig(*fn_id);

        //             // 1. Prepare Header
        //             // Note: We use the same name string. SMT-LIB allows overloading
        //             // if the signatures (generics) differ.
        //             let header_str =
        //                 create_function_header_str(ir, fn_name.to_string(), generics, sig, &ty_map);
        //             decl_headers.push(header_str);

        //             // 2. Prepare Body
        //             let def = fn_registry.retrieve_def(*fn_id);
        //             match def {
        //                 FunDef::Defined(exp_registry, root_exp_id) => {
        //                     let body_str = process_function_body_str(
        //                         ir,
        //                         *fn_id,
        //                         exp_registry,
        //                         *root_exp_id,
        //                         &ty_map,
        //                         &mut cloak_manager,
        //                         &mut map_length_manager,
        //                         &mut axiomatic_parameters,
        //                     );
        //                     decl_bodies.push(body_str);
        //                 }
        //                 // Handle abstract/undefined functions if necessary,
        //                 // though usually these are just `declare-fun`.
        //                 _ => panic!("Unsupported function definition type for SMT export"),
        //             }
        //         }
        //     }

        //     // Write the single block command
        //     // (define-funs-rec ( (header1) (header2) ) ( (body1) (body2) ) )
        //     l!(
        //         x,
        //         "(define-funs-rec ({}) ({}))",
        //         decl_headers.join(" "),
        //         decl_bodies.join(" ")
        //     );
        //     l!(x); // Empty line
        // }
        Ok(x.build())
    }
}

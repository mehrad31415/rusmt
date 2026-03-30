// //! This module contains functions for working with Z3 function declarations and definitions.

use crate::backend::z3::exp::format_expression;
use crate::ir::{
    ctxt::IRContext,
    fun::{FunDef, FunRegistry, FunSig},
    index::UsrFunId,
    name::UsrFunName,
    sort::Sort,
};
use std::collections::BTreeSet;

/// Helper to resolve the function name and type parameters from a function ID.
/// Returns (function_name, type_parameters)
pub fn resolve_function_name(ir: &IRContext, fid: UsrFunId) -> (UsrFunName, Vec<Sort>) {
    for (name, instantiations) in &ir.fn_registry.lookup {
        for (ty_args, fn_id) in instantiations {
            if *fn_id == fid {
                return (name.clone(), ty_args.clone());
            }
        }
    }
    panic!("no such function id in resolve_function_name");
}

/// Collects function call edges from the function registry.
pub fn collect_function_call_edges(fn_registry: &FunRegistry) -> Vec<(UsrFunId, UsrFunId)> {
    let mut edges = vec![];
    for (_, instantiations) in &fn_registry.lookup {
        for (_, fn_id) in instantiations {
            let def = fn_registry.retrieve_def(*fn_id);
            let called_fns = def.body.collect_called_functions(&def.root_exp_id);
            for called_fn in called_fns {
                edges.push((*fn_id, called_fn));
            }
        }
    }
    edges
}

/// Helper to format a Sort for use in function signatures (not inside polymorphic type declarations).
/// This is simpler than format_sort because function type parameters are already declared separately.
pub fn format_sort_for_fn(sort: &Sort, ir: &IRContext) -> String {
    match sort {
        Sort::User(sid) => {
            // Use the type name from the registry
            let (ty_name_opt, type_args) = ir.ty_registry.reverse_lookup(*sid);
            if let Some(ty_name) = ty_name_opt {
                // If there are type arguments, format as (TypeName Arg1 Arg2 ...)
                if type_args.is_empty() {
                    ty_name.to_string()
                } else {
                    let args_str: Vec<String> = type_args
                        .iter()
                        .map(|t| format_sort_for_fn(t, ir))
                        .collect();
                    format!("({} {})", ty_name, args_str.join(" "))
                }
            } else {
                // Unnamed tuple
                format!(
                    "Tuple_{}",
                    ir.ty_registry
                        .reverse_lookup(*sid)
                        .1
                        .iter()
                        .map(|t| format_sort_for_fn(t, ir))
                        .collect::<Vec<_>>()
                        .join("_")
                )
            }
        }
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Real => "Real".to_string(),
        Sort::String => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", format_sort_for_fn(inner, ir)),
        Sort::Set(inner) => format!("(Set {})", format_sort_for_fn(inner, ir)),
        Sort::Array(key, value) => format!(
            "(Array {} {})",
            format_sort_for_fn(key, ir),
            format_sort_for_fn(value, ir)
        ),
        Sort::F32 => "(_ FloatingPoint 8 24)".to_string(),
        Sort::F64 => "(_ FloatingPoint 11 53)".to_string(),
        Sort::I32 => "(_ BitVec 32)".to_string(),
        Sort::I64 => "(_ BitVec 64)".to_string(),
        Sort::U32 => "(_ BitVec 32)".to_string(),
        Sort::U64 => "(_ BitVec 64)".to_string(),
        Sort::Uninterpreted(x) => x.to_string(),
        Sort::Error => "(Array Int Bool)".to_string(), // Error is a set of integer IDs
    }
}

/// Helper to format a single function signature for define-funs-rec.
/// Returns: (function_name ((param1 Type1) (param2 Type2)) ReturnType)
fn format_function_signature(function_name: &str, sig: &FunSig, ir: &IRContext) -> String {
    let param_list: Vec<String> = sig
        .params
        .iter()
        .map(|(param_name, param_sort)| {
            format!("({} {})", param_name, format_sort_for_fn(param_sort, ir))
        })
        .collect();

    let ret_type = format_sort_for_fn(&sig.ret_ty, ir);

    format!(
        "({} ({}) {})",
        function_name,
        param_list.join(" "),
        ret_type
    )
}

/// Helper to format a function body.
/// Returns the body expression as a string.
fn format_function_body(
    def: &FunDef,
    ir: &IRContext,
    sig: &FunSig,
    scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    // Build set of parameter names for variable resolution
    let param_names: std::collections::HashSet<String> = sig
        .params
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    format_expression(&def.body, def.root_exp_id, ir, &param_names, scc_fids)
}

/// Convert a non-recursive function definition to SMT-LIB string format.
/// Format: (define-fun function_name ((param1 Type1) (param2 Type2)) ReturnType body)
pub fn mk_function_str(
    function_name: &str,
    _type_params: &[Sort],
    sig: &FunSig,
    def: &FunDef,
    ir: &IRContext,
    _scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    let param_list: Vec<String> = sig
        .params
        .iter()
        .map(|(param_name, param_sort)| {
            format!("({} {})", param_name, format_sort_for_fn(param_sort, ir))
        })
        .collect();

    let ret_type = format_sort_for_fn(&sig.ret_ty, ir);
    let body = format_function_body(def, ir, sig, _scc_fids);

    format!(
        "(define-fun {} ({}) {} {})",
        function_name,
        param_list.join(" "),
        ret_type,
        body
    )
}

/// Convert mutually recursive function definitions to SMT-LIB string format.
/// Format: (define-funs-rec ((name1 ((param1 Type1)) ReturnType1) (name2 ((param2 Type2)) ReturnType2)) (body1 body2))
pub fn mk_functions_rec_str(
    functions: &[(UsrFunId, &str, &[Sort], &FunSig, &FunDef)],
    ir: &IRContext,
    _scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    let signatures: Vec<String> = functions
        .iter()
        .map(|(_, name, _type_params, sig, _)| format_function_signature(name, sig, ir))
        .collect();

    let bodies: Vec<String> = functions
        .iter()
        .map(|(_, _name, _type_params, sig, def)| format_function_body(def, ir, sig, _scc_fids))
        .collect();

    format!(
        "(define-funs-rec ({}) ({}))",
        signatures.join(" "),
        bodies.join(" ")
    )
}

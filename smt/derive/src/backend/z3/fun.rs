//! This module contains functions for working with Z3 function declarations and definitions.

use crate::backend::z3::exp::{format_expression, format_expression_renamed};
use crate::backend::z3::intrinsics::{array_null_value, collect_fn_from_intrinsic};
use crate::backend::z3::sort::resolve_type_name;
use crate::ir::exp::{ExpRegistry, Expression, VarKind, VariantCtor};
use crate::ir::index::ExpId;
use crate::ir::{
    ctxt::IRContext,
    fun::{FunDef, FunRegistry, FunSig},
    index::UsrFunId,
    sort::{DataType, Sort, Variant},
};
use std::collections::{BTreeMap, BTreeSet};

/// Helper to resolve a unique SMT function name from a function ID.
/// Non-generic functions use their base name (e.g., "parse_toml").
/// Monomorphized generic functions include type args (e.g., "parse_value_String_Int").
pub fn resolve_function_name(ir: &IRContext, fid: UsrFunId) -> String {
    for (name, instantiations) in ir.fn_registry.lookup() {
        for (ty_args, fn_id) in instantiations {
            if *fn_id == fid {
                if ty_args.is_empty() {
                    return name.to_string();
                } else {
                    return format!(
                        "{}_{}",
                        name,
                        ty_args
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("_")
                    );
                }
            }
        }
    }
    panic!("no such function id in resolve_function_name");
}

/// Helper to format a Sort for use in function signatures.
pub fn format_sort_for_fn(sort: &Sort, ir: &IRContext) -> String {
    match sort {
        Sort::User(sid) => {
            // Use the type name from the registry
            let (ty_name_opt, type_args) = ir.ty_registry.reverse_lookup(*sid);
            let type_name = resolve_type_name(ir, *sid);

            if let Some(_) = ty_name_opt {
                // If there are type arguments, format as (TypeName Arg1 Arg2 ...)
                if type_args.is_empty() {
                    type_name
                } else {
                    let args_str: Vec<String> = type_args
                        .iter()
                        .map(|t| format_sort_for_fn(t, ir))
                        .collect();
                    format!("({} {})", type_name, args_str.join(" "))
                }
            } else {
                type_name
            }
        }
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Real => "Real".to_string(),
        Sort::String => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", format_sort_for_fn(inner, ir)),
        Sort::Set(inner) => format!("(RuSmtSet {})", format_sort_for_fn(inner, ir)),
        Sort::Array(key, value) => format!(
            "(RuSmtArray {} {})",
            format_sort_for_fn(key, ir),
            format_sort_for_fn(value, ir)
        ),
        Sort::F32 => "(_ FloatingPoint 8 24)".to_string(),
        Sort::F64 => "(_ FloatingPoint 11 53)".to_string(),
        Sort::I32 => "(_ BitVec 32)".to_string(),
        Sort::I64 => "(_ BitVec 64)".to_string(),
        Sort::U32 => "(_ BitVec 32)".to_string(),
        Sort::U64 => "(_ BitVec 64)".to_string(),
        Sort::Uninterpreted(x) => panic!(
            "uninterpreted sort {} should not be used in the monomorphized version of a function",
            x
        ),
        Sort::Path => "(Array Int Bool)".to_string(), // Path is a set of integer IDs
    }
}

/// Collects function call edges from the function registry except for functions that are iterchoose.
pub fn collect_function_call_edges(
    all_ids: &BTreeSet<UsrFunId>,
    fn_registry: &FunRegistry,
) -> Vec<(UsrFunId, UsrFunId)> {
    let mut edges = vec![];
    for id in all_ids {
        let def = fn_registry.retrieve_def(*id);
        let called_fns = collect_called_functions(&def.body, &def.root_exp_id);
        for called_fn in called_fns {
            if all_ids.contains(&called_fn) {
                edges.push((*id, called_fn));
            }
        }
    }
    edges
}

/// Collect all called functions from an expression
pub(crate) fn collect_called_functions(
    exp_registry: &ExpRegistry,
    exp_id: &ExpId,
) -> Vec<UsrFunId> {
    let mut called_fns = vec![];
    let exp = exp_registry.lookup_exp(exp_id);
    // recursively traverse the expression to find called functions
    match exp {
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.kind {
                VarKind::Bound { bind } => {
                    called_fns.append(&mut collect_called_functions(exp_registry, bind));
                }
                VarKind::Param | VarKind::Quant | VarKind::Axiom => {
                    // do nothing
                }
                VarKind::Match {
                    head,
                    sort: _,
                    branch: _,
                    selector: _,
                } => {
                    called_fns.append(&mut collect_called_functions(exp_registry, head));
                }
            }
        }
        Expression::Pack { sort: _, elems } => {
            for e in elems {
                called_fns.append(&mut collect_called_functions(exp_registry, e));
            }
        }
        Expression::Tuple { sort: _, slots } => {
            for s in slots {
                called_fns.append(&mut collect_called_functions(exp_registry, s));
            }
        }
        Expression::Record { sort: _, fields } => {
            for (_, f) in fields {
                called_fns.append(&mut collect_called_functions(exp_registry, f));
            }
        }
        Expression::Enum {
            sort: _,
            branch: _,
            variant,
        } => match variant {
            VariantCtor::Unit => (),
            VariantCtor::Tuple(elems) => {
                for e in elems {
                    called_fns.append(&mut collect_called_functions(exp_registry, e));
                }
            }
            VariantCtor::Record(fields) => {
                for (_, f) in fields {
                    called_fns.append(&mut collect_called_functions(exp_registry, f));
                }
            }
        },
        Expression::AccessSlot { base, slot: _ } => {
            called_fns.append(&mut collect_called_functions(exp_registry, base));
        }
        Expression::AccessField { base, field: _ } => {
            called_fns.append(&mut collect_called_functions(exp_registry, base));
        }
        Expression::Match { cases } => {
            for case in cases {
                for atom in &case.atoms {
                    called_fns.append(&mut collect_called_functions(exp_registry, &atom.head));
                }
                called_fns.append(&mut collect_called_functions(exp_registry, &case.body));
            }
        }
        Expression::Phi { cases, default } => {
            for case in cases {
                called_fns.append(&mut collect_called_functions(exp_registry, &case.cond));
                called_fns.append(&mut collect_called_functions(exp_registry, &case.body));
            }
            called_fns.append(&mut collect_called_functions(exp_registry, default));
        }
        Expression::IterForall { vars, body } => {
            for (_, e) in vars {
                called_fns.append(&mut collect_called_functions(exp_registry, e));
            }
            called_fns.append(&mut collect_called_functions(exp_registry, body));
        }
        Expression::IterExists { vars, body } => {
            for (_, e) in vars {
                called_fns.append(&mut collect_called_functions(exp_registry, e));
            }
            called_fns.append(&mut collect_called_functions(exp_registry, body));
        }
        Expression::IterChoose {
            vars,
            body,
            rets: _,
        } => {
            for (_, e) in vars {
                called_fns.append(&mut collect_called_functions(exp_registry, e));
            }
            called_fns.append(&mut collect_called_functions(exp_registry, body));
        }
        Expression::Procedure { callee, args } => {
            called_fns.push(*callee);
            for a in args {
                called_fns.append(&mut collect_called_functions(exp_registry, a));
            }
        }
        Expression::Intrinsic(intrinsic) => {
            // collect from intrinsic
            called_fns.append(&mut collect_fn_from_intrinsic(exp_registry, intrinsic));
        }
    }
    called_fns
}

/// Convert a non-recursive function definition to SMT-LIB string format.
/// Format: (define-fun function_name ((param1 Type1) (param2 Type2)) ReturnType body)
pub fn mk_function_str(
    function_name: String,
    sig: &FunSig,
    def: &FunDef,
    ir: &IRContext,
) -> String {
    let param_list: Vec<String> = sig
        .params
        .iter()
        .map(|(param_name, param_sort)| {
            format!("({} {})", param_name, format_sort_for_fn(param_sort, ir))
        })
        .collect();

    let ret_type = format_sort_for_fn(&sig.ret_ty, ir);
    let body = format_expression(&def.body, def.root_exp_id, &ir);

    format!(
        "(define-fun {} ({}) {} {})",
        function_name,
        param_list.join(" "),
        ret_type,
        body
    )
}

/// Helper to format a single function signature.
/// Returns: (function_name ((param1 Type1) (param2 Type2)) ReturnType)
fn format_function_signature(function_name: String, sig: &FunSig, ir: &IRContext) -> String {
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

/// Convert mutually recursive function definitions to SMT-LIB string format.
/// Format: (define-funs-rec ((name1 ((param1 Type1)) ReturnType1) (name2 ((param2 Type2)) ReturnType2)) (body1 body2))
pub fn mk_functions_rec_str(scc_fids: &BTreeSet<UsrFunId>, ir: &IRContext) -> String {
    let signatures: Vec<String> = scc_fids
        .iter()
        .map(|fid| {
            format_function_signature(
                resolve_function_name(ir, *fid),
                ir.fn_registry.retrieve_sig(*fid),
                ir,
            )
        })
        .collect();

    let bodies: Vec<String> = scc_fids
        .iter()
        .map(|fid| {
            format_expression(
                &ir.fn_registry.retrieve_def(*fid).body,
                ir.fn_registry.retrieve_def(*fid).root_exp_id,
                ir,
            )
        })
        .collect();

    format!(
        "(define-funs-rec ({}) ({}))",
        signatures.join(" "),
        bodies.join(" ")
    )
}

/// Convert a recursive SCC into a sequence of non-recursive `define-fun`
/// declarations realizing bounded-recursion unrolling to depth `k` (k ≥ 1).
///
/// Soundness: a sat model under depth-k unrolling is a genuine witness that
/// the SCC reaches the asserted condition within ≤ k recursive steps. Inputs
/// whose path requires more than k steps return the terminator instead.
pub fn mk_functions_unrolled_str(
    scc_fids: &BTreeSet<UsrFunId>,
    depth: usize,
    ir: &IRContext,
) -> String {
    assert!(depth >= 1, "mk_functions_unrolled_str requires depth >= 1");

    let names: BTreeMap<UsrFunId, String> = scc_fids
        .iter()
        .map(|&fid| (fid, resolve_function_name(ir, fid)))
        .collect();

    let format_params = |sig: &FunSig| -> Vec<String> {
        sig.params
            .iter()
            .map(|(n, s)| format!("({} {})", n, format_sort_for_fn(s, ir)))
            .collect()
    };

    let mut out = String::new();

    // Depth 0: terminator (null sentinel of return type)
    for &fid in scc_fids {
        let sig = ir.fn_registry.retrieve_sig(fid);
        let params = format_params(sig);
        let ret = format_sort_for_fn(&sig.ret_ty, ir);
        let terminator = bmc_terminator(&sig.ret_ty, ir);
        out.push_str(&format!(
            "(define-fun {}_0 ({}) {} {})\n",
            names[&fid],
            params.join(" "),
            ret,
            terminator
        ));
    }

    // Depths 1..=k
    for d in 1..=depth {
        let rename: BTreeMap<UsrFunId, String> = names
            .iter()
            .map(|(&fid, name)| (fid, format!("{}_{}", name, d - 1)))
            .collect();
        for &fid in scc_fids {
            let sig = ir.fn_registry.retrieve_sig(fid);
            let def = ir.fn_registry.retrieve_def(fid);
            let params = format_params(sig);
            let ret = format_sort_for_fn(&sig.ret_ty, ir);
            let body = format_expression_renamed(&def.body, def.root_exp_id, ir, &rename);
            out.push_str(&format!(
                "(define-fun {}_{} ({}) {} {})\n",
                names[&fid],
                d,
                params.join(" "),
                ret,
                body
            ));
        }
    }

    // Aliases — route external callers through the depth-k copy.
    for &fid in scc_fids {
        let sig = ir.fn_registry.retrieve_sig(fid);
        let params = format_params(sig);
        let ret = format_sort_for_fn(&sig.ret_ty, ir);
        let arg_names: Vec<String> = sig.params.iter().map(|(n, _)| n.to_string()).collect();
        let call = if arg_names.is_empty() {
            format!("{}_{}", names[&fid], depth)
        } else {
            format!("({}_{} {})", names[&fid], depth, arg_names.join(" "))
        };
        out.push_str(&format!(
            "(define-fun {} ({}) {} {})\n",
            names[&fid],
            params.join(" "),
            ret,
            call
        ));
    }

    // Trim the final newline so the calling `l!` macro doesn't double up.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Pick the SMT-LIB terminator value to return when a bounded-recursion
/// unrolled function exhausts its depth budget.
///
/// 1. If the return sort is a user-defined enum (e.g. `ParseResult<T>`,
///    `Optional<T>`), return the constructor application of the first
///    `Variant::Unit` declared on it. For parser-shaped types this picks
///    a meaningful "ran out of budget" sentinel — `NoMatch` for
///    `ParseResult`, `None` for `Optional`.
/// 2. Otherwise (primitives, records, tuples, enums with no Unit variant)
///    fall back to `array_null_value`, which produces the type's null
///    sentinel (`0` for ints, `false` for bools, the declared
///    `null_<TypeName>` constant for records/tuples, etc.).
fn bmc_terminator(sort: &Sort, ir: &IRContext) -> String {
    if let Sort::User(sid) = sort {
        let dt = ir.ty_registry.retrieve(*sid);
        if let DataType::Enum(variants) = dt {
            if let Some(vname) = variants.iter().find_map(|(n, v)| {
                if matches!(v, Variant::Unit) {
                    Some(n.clone())
                } else {
                    None
                }
            }) {
                let type_name = resolve_type_name(ir, *sid);
                let sort_str = crate::backend::z3::sort::format_sort(sort, ir);
                return format!("(as {}_{} {})", type_name, vname, sort_str);
            }
        }
    }
    array_null_value(sort, ir)
}

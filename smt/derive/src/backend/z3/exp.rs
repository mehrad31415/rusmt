// //! This module contains the conversion of expressions to SMT-LIB format.
// //! An expression is what constitutes the body of a function or an axiom.

use crate::backend::z3::fun::{format_sort_for_fn, resolve_function_name};
use crate::backend::z3::intrinsics::format_intrinsic;
use crate::backend::z3::ty::resolve_type_name;
use crate::ir::exp::{EnumSelector, Expression, VarKind, VariantCtor};
use crate::ir::index::UsrFunId;
use crate::ir::sort::DataType;
use crate::ir::{ctxt::IRContext, exp::ExpRegistry, index::ExpId};
use std::collections::BTreeSet;

/// Convert an expression to SMT-LIB string format.
pub fn format_expression(
    exp_registry: &ExpRegistry,
    exp_id: ExpId,
    ir: &IRContext,
    param_names: &std::collections::HashSet<String>,
    scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    let exp = exp_registry.lookup_exp(&exp_id);

    match exp {
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.kind {
                VarKind::Param => var.name.to_string(),
                VarKind::Bound { bind } => {
                    format_expression(exp_registry, *bind, ir, param_names, scc_fids)
                }
                VarKind::Quant | VarKind::Axiom => var.name.to_string(),
                VarKind::Match {
                    head,
                    sort,
                    branch,
                    selector,
                } => {
                    // Match-introduced variable: access the field from the head
                    let head_str =
                        format_expression(exp_registry, *head, ir, param_names, scc_fids);
                    let type_name = resolve_type_name(ir, *sort);
                    let accessor_name = match selector {
                        EnumSelector::Tuple(idx) => {
                            format!("field_{}_{}_{}_", type_name, branch, idx + 1)
                        }
                        EnumSelector::Record(field_name) => {
                            format!("record_{}_{}_{}_", type_name, branch, field_name)
                        }
                    };
                    format!("({} {})", accessor_name, head_str)
                }
            }
        }

        Expression::Pack { sort, elems } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{}", type_name);
            let elem_strs: Vec<String> = elems
                .iter()
                .map(|e| format_expression(exp_registry, *e, ir, param_names, scc_fids))
                .collect();
            format!("({} {})", constructor_name, elem_strs.join(" "))
        }

        Expression::Tuple { sort, slots } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{}", type_name);
            let slot_strs: Vec<String> = slots
                .iter()
                .map(|s| format_expression(exp_registry, *s, ir, param_names, scc_fids))
                .collect();
            format!("({} {})", constructor_name, slot_strs.join(" "))
        }

        Expression::Record { sort, fields } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{}", type_name);
            // For records, we need to provide values in the order they appear in the type definition
            let dt = ir.ty_registry.retrieve(*sort);
            if let DataType::Record(type_fields) = dt {
                let ordered_values: Vec<String> = type_fields
                    .keys()
                    .map(|type_field_name| {
                        let exp_id = fields
                            .get(type_field_name)
                            .expect("field missing in record expression");
                        format_expression(exp_registry, *exp_id, ir, param_names, scc_fids)
                    })
                    .collect();
                format!("({} {})", constructor_name, ordered_values.join(" "))
            } else {
                // Fallback (shouldn't happen for records)
                let field_values: Vec<String> = fields
                    .values()
                    .map(|exp_id| {
                        format_expression(exp_registry, *exp_id, ir, param_names, scc_fids)
                    })
                    .collect();
                format!("({} {})", constructor_name, field_values.join(" "))
            }
        }

        Expression::Enum {
            sort,
            branch,
            variant,
        } => {
            let _type_name = resolve_type_name(ir, *sort);
            match variant {
                VariantCtor::Unit => branch.clone(),
                VariantCtor::Tuple(elems) => {
                    let elem_strs: Vec<String> = elems
                        .iter()
                        .map(|e| format_expression(exp_registry, *e, ir, param_names, scc_fids))
                        .collect();
                    format!("({} {})", branch, elem_strs.join(" "))
                }
                VariantCtor::Record(fields) => {
                    // For enum record variants, we need to provide values in order
                    let dt = ir.ty_registry.retrieve(*sort);
                    if let DataType::Enum(variants) = dt {
                        if let Some(variant_def) = variants.get(branch) {
                            if let crate::ir::sort::Variant::Record(type_fields) = variant_def {
                                let ordered_values: Vec<String> = type_fields
                                    .keys()
                                    .map(|type_field_name| {
                                        let exp_id = fields
                                            .get(type_field_name)
                                            .expect("field missing in enum record expression");
                                        format_expression(
                                            exp_registry,
                                            *exp_id,
                                            ir,
                                            param_names,
                                            scc_fids,
                                        )
                                    })
                                    .collect();
                                format!("({} {})", branch, ordered_values.join(" "))
                            } else {
                                branch.clone()
                            }
                        } else {
                            branch.clone()
                        }
                    } else {
                        branch.clone()
                    }
                }
            }
        }

        Expression::AccessSlot { base, slot } => {
            let base_str = format_expression(exp_registry, *base, ir, param_names, scc_fids);
            let base_sort = {
                let base_exp = exp_registry.lookup_exp(base);
                match base_exp {
                    Expression::Pack { sort, .. } | Expression::Tuple { sort, .. } => *sort,
                    _ => panic!("AccessSlot base must be Pack or Tuple"),
                }
            };
            let type_name = resolve_type_name(ir, base_sort);
            let accessor_name = format!("field_{}_{}_", type_name, slot + 1);
            format!("({} {})", accessor_name, base_str)
        }

        Expression::AccessField { base, field } => {
            let base_str = format_expression(exp_registry, *base, ir, param_names, scc_fids);
            let (base_sort, variant_name) = {
                let base_exp = exp_registry.lookup_exp(base);
                match base_exp {
                    Expression::Record { sort, .. } => (*sort, None),
                    Expression::Enum { sort, branch, .. } => (*sort, Some(branch.clone())),
                    _ => panic!("AccessField base must be Record or Enum"),
                }
            };
            let type_name = resolve_type_name(ir, base_sort);
            let accessor_name = if let Some(variant) = variant_name {
                format!("record_{}_{}_{}_", type_name, variant, field)
            } else {
                format!("record_{}_{}_", type_name, field)
            };
            format!("({} {})", accessor_name, base_str)
        }

        Expression::Match { cases } => {
            // Match expression: (ite condition1 body1 (ite condition2 body2 ... default))
            // For enums, we use tester functions: (is-VariantName expr)
            if cases.is_empty() {
                return "true".to_string(); // Fallback
            }

            let mut result = format_expression(
                exp_registry,
                cases.last().unwrap().body,
                ir,
                param_names,
                scc_fids,
            );

            // Build nested ite expressions from last to first (excluding the last case)
            for case in cases.iter().rev().skip(1) {
                let condition = if case.atoms.len() == 1 {
                    let atom = &case.atoms[0];
                    let head_str =
                        format_expression(exp_registry, atom.head, ir, param_names, scc_fids);
                    // Use tester function for enum
                    format!("(is-{} {})", atom.branch, head_str)
                } else {
                    // Multiple atoms: combine with and
                    let conditions: Vec<String> = case
                        .atoms
                        .iter()
                        .map(|atom| {
                            let head_str = format_expression(
                                exp_registry,
                                atom.head,
                                ir,
                                param_names,
                                scc_fids,
                            );
                            format!("(is-{} {})", atom.branch, head_str)
                        })
                        .collect();
                    format!("(and {})", conditions.join(" "))
                };
                let body_str =
                    format_expression(exp_registry, case.body, ir, param_names, scc_fids);
                result = format!("(ite {} {} {})", condition, body_str, result);
            }
            result
        }

        Expression::Phi { cases, default } => {
            // If-then-else chain: (ite condition1 body1 (ite condition2 body2 ... default))
            let mut result = format_expression(exp_registry, *default, ir, param_names, scc_fids);
            for case in cases.iter().rev() {
                let cond_str =
                    format_expression(exp_registry, case.cond, ir, param_names, scc_fids);
                let body_str =
                    format_expression(exp_registry, case.body, ir, param_names, scc_fids);
                result = format!("(ite {} {} {})", cond_str, body_str, result);
            }
            result
        }

        Expression::Forall { vars, body } => {
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(forall ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::Exists { vars, body } => {
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(exists ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::Choose {
            vars,
            body,
            rets: _,
        } => {
            // Choose is similar to exists but returns values
            // For now, treat as exists
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(exists ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::IterForall { vars, body } => {
            // IterForall: forall with iteration over collections
            // Format: (forall ((var Type)) (=> (guard condition) body))
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, _coll_exp_id)| {
                    let var = exp_registry.lookup_var(var_id);
                    // Use the variable's sort (which should be the element type of the collection)
                    format!("({} {})", var.name, format_sort_for_fn(&var.sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(forall ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::IterExists { vars, body } => {
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, _coll_exp_id)| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(&var.sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(exists ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::IterChoose {
            vars,
            body,
            rets: _,
        } => {
            let var_decls: Vec<String> = vars
                .iter()
                .map(|(var_id, _coll_exp_id)| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(&var.sort, ir))
                })
                .collect();
            let body_str = format_expression(exp_registry, *body, ir, param_names, scc_fids);
            format!("(exists ({}) {})", var_decls.join(" "), body_str)
        }

        Expression::Procedure { callee, args } => {
            // Function call
            let (function_name, type_params) = resolve_function_name(ir, *callee);
            // For functions in the same SCC (e.g., recursive calls), use the base name without type suffix
            // Otherwise, use the instance name with type parameters
            let instance_name = if scc_fids.contains(callee) {
                // Same SCC: use base function name (recursive or mutually recursive call)
                function_name.to_string()
            } else if type_params.is_empty() {
                function_name.to_string()
            } else {
                let type_suffix = type_params
                    .iter()
                    .map(|s| format_sort_for_fn(s, ir))
                    .collect::<Vec<_>>()
                    .join("_");
                format!("{}_{}", function_name, type_suffix)
            };
            let arg_strs: Vec<String> = args
                .iter()
                .map(|a| format_expression(exp_registry, *a, ir, param_names, scc_fids))
                .collect();
            format!("({} {})", instance_name, arg_strs.join(" "))
        }

        Expression::Intrinsic(intrinsic) => {
            format_intrinsic(intrinsic, exp_registry, ir, param_names, scc_fids)
        }
    }
}

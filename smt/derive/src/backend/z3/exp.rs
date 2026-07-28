//! This module contains the conversion of expressions to SMT-LIB format.
//! An expression is what constitutes the body of a function or an axiom.

use crate::backend::z3::fun::{format_sort_for_fn, resolve_function_name};
use crate::backend::z3::intrinsics::format_intrinsic;
use crate::backend::z3::sort::resolve_type_name;
use crate::ir::exp::{EnumSelector, Expression, VarKind, VariantCtor};
use crate::ir::index::UsrFunId;
use crate::ir::sort::Sort;
use crate::ir::{ctxt::IRContext, exp::ExpRegistry, index::ExpId};
use std::collections::BTreeMap;

/// Convert an expression to SMT-LIB string format.
pub fn format_expression(exp_registry: &ExpRegistry, exp_id: ExpId, ir: &IRContext) -> String {
    format_expression_renamed(exp_registry, exp_id, ir, &BTreeMap::new())
}

/// Convert an expression to SMT-LIB string format, redirecting calls to
/// callees in `rename` to the renamed function name. Used by bounded-recursion unrolling.
pub fn format_expression_renamed(
    exp_registry: &ExpRegistry,
    exp_id: ExpId,
    ir: &IRContext,
    rename: &BTreeMap<UsrFunId, String>,
) -> String {
    let exp = exp_registry.lookup_exp(&exp_id);

    match exp {
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.kind {
                VarKind::Param => var.name.to_string(),
                VarKind::Bound { bind } => {
                    format_expression_renamed(exp_registry, *bind, ir, rename)
                }
                VarKind::Quant | VarKind::Axiom => var.name.to_string(),
                VarKind::Match {
                    head,
                    sort,
                    branch,
                    selector,
                } => {
                    // Match-introduced variable: access the field from the head
                    let head_str = format_expression_renamed(exp_registry, *head, ir, rename);
                    let type_name = resolve_type_name(ir, *sort);
                    let accessor_name = match selector {
                        EnumSelector::Tuple(idx) => {
                            format!("field_{}_{}_{}_", type_name, branch, idx + 1)
                        }
                        EnumSelector::Record(field_name) => {
                            format!("record_{type_name}_{branch}_{field_name}_")
                        }
                    };
                    format!("({accessor_name} {head_str})")
                }
            }
        }
        Expression::Pack { sort, elems } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{type_name}");
            let elem_strs: Vec<String> = elems
                .iter()
                .map(|e| format_expression_renamed(exp_registry, *e, ir, rename))
                .collect();
            format!("({} {})", constructor_name, elem_strs.join(" "))
        }
        Expression::Tuple { sort, slots } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{type_name}");
            let slot_strs: Vec<String> = slots
                .iter()
                .map(|s| format_expression_renamed(exp_registry, *s, ir, rename))
                .collect();
            format!("({} {})", constructor_name, slot_strs.join(" "))
        }
        Expression::Record { sort, fields } => {
            let type_name = resolve_type_name(ir, *sort);
            let constructor_name = format!("mk-{type_name}");
            let ordered_values: Vec<String> = fields
                .values()
                .map(|exp_id| format_expression_renamed(exp_registry, *exp_id, ir, rename))
                .collect();
            format!("({} {})", constructor_name, ordered_values.join(" "))
        }
        Expression::Enum {
            sort,
            branch,
            variant,
        } => match variant {
            VariantCtor::Unit => {
                let type_name = resolve_type_name(ir, *sort);
                let constructor = format!("{type_name}_{branch}");
                let sort_str = format_sort_for_fn(&Sort::User(*sort), ir);
                format!("(as {constructor} {sort_str})")
            }
            VariantCtor::Tuple(elems) => {
                let type_name = resolve_type_name(ir, *sort);
                let constructor = format!("{type_name}_{branch}");
                let sort_str = format_sort_for_fn(&Sort::User(*sort), ir);
                let elem_strs: Vec<String> = elems
                    .iter()
                    .map(|e| format_expression_renamed(exp_registry, *e, ir, rename))
                    .collect();
                format!(
                    "((as {} {}) {})",
                    constructor,
                    sort_str,
                    elem_strs.join(" ")
                )
            }
            VariantCtor::Record(fields) => {
                let type_name = resolve_type_name(ir, *sort);
                let constructor = format!("{type_name}_{branch}");
                let sort_str = format_sort_for_fn(&Sort::User(*sort), ir);
                let values: Vec<String> = fields
                    .values()
                    .map(|exp_id| format_expression_renamed(exp_registry, *exp_id, ir, rename))
                    .collect();
                format!("((as {} {}) {})", constructor, sort_str, values.join(" "))
            }
        },
        Expression::AccessSlot { base, slot } => {
            let base_str = format_expression_renamed(exp_registry, *base, ir, rename);
            let base_sort = exp_registry.derive_type(*base, ir);
            let type_name = resolve_type_name(ir, ExpRegistry::expect_sort_user(&base_sort));
            let accessor_name = format!("field_{}_{}_", type_name, slot + 1);
            format!("({accessor_name} {base_str})")
        }
        Expression::AccessField { base, field } => {
            let base_str = format_expression_renamed(exp_registry, *base, ir, rename);
            let base_sort = exp_registry.derive_type(*base, ir);
            let type_name = resolve_type_name(ir, ExpRegistry::expect_sort_user(&base_sort));
            let accessor_name = format!("record_{type_name}_{field}_");
            format!("({accessor_name} {base_str})")
        }
        Expression::Phi { cases, default } => {
            // If-then-else chain: (ite condition1 body1 (ite condition2 body2 ... default))
            let mut result = format_expression_renamed(exp_registry, *default, ir, rename);
            for case in cases.iter().rev() {
                let cond_str = format_expression_renamed(exp_registry, case.cond, ir, rename);
                let body_str = format_expression_renamed(exp_registry, case.body, ir, rename);
                result = format!("(ite {cond_str} {body_str} {result})");
            }
            result
        }
        Expression::Procedure { callee, args } => {
            let function_name = match rename.get(callee) {
                Some(renamed) => renamed.clone(),
                None => resolve_function_name(ir, *callee),
            };
            if args.is_empty() {
                function_name.to_string()
            } else {
                let arg_strs: Vec<String> = args
                    .iter()
                    .map(|a| format_expression_renamed(exp_registry, *a, ir, rename))
                    .collect();
                format!("({} {})", function_name, arg_strs.join(" "))
            }
        }
        Expression::Intrinsic(intrinsic) => format_intrinsic(intrinsic, exp_registry, ir, rename),
        Expression::Match { cases } => {
            // Match expression: (ite condition1 body1 (ite condition2 body2 ... default))
            // For enums, we use tester functions: (is-VariantName expr)
            if cases.is_empty() {
                return "true".to_string(); // Fallback
            }

            let mut result =
                format_expression_renamed(exp_registry, cases.last().unwrap().body, ir, rename);

            // Build nested ite expressions from last to first (excluding the last case)
            for case in cases.iter().rev().skip(1) {
                let condition = if case.atoms.len() == 1 {
                    let atom = &case.atoms[0];
                    let head_str = format_expression_renamed(exp_registry, atom.head, ir, rename);
                    let type_name = resolve_type_name(ir, atom.sort);
                    // Use tester function for enum — prefixed with type name to match declaration
                    format!("(is-{}_{} {})", type_name, atom.branch, head_str)
                } else {
                    // Multiple atoms: combine with and
                    let conditions: Vec<String> = case
                        .atoms
                        .iter()
                        .map(|atom| {
                            let head_str =
                                format_expression_renamed(exp_registry, atom.head, ir, rename);
                            let type_name = resolve_type_name(ir, atom.sort);
                            format!("(is-{}_{} {})", type_name, atom.branch, head_str)
                        })
                        .collect();
                    format!("(and {})", conditions.join(" "))
                };
                let body_str = format_expression_renamed(exp_registry, case.body, ir, rename);
                result = format!("(ite {condition} {body_str} {result})");
            }
            result
        }
        Expression::IterChoose { .. } => {
            panic!("choose! must be the sole expression in a function body, not a sub-expression")
        }
        Expression::IterForall { vars, body } => {
            let var_decls: Vec<String> = vars
                .keys()
                .map(|var_id| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(&var.sort, ir))
                })
                .collect();
            let body_str = format_expression_renamed(exp_registry, *body, ir, rename);

            // Build membership guards: forall uses (=> guard body)
            let mut guards = Vec::new();
            for (var_id, coll_eid) in vars {
                let var = exp_registry.lookup_var(var_id);
                let coll_exp = exp_registry.lookup_exp(coll_eid);
                let coll_sort = match coll_exp {
                    // without losing expressiveness we can assume that the collection is a variable
                    Expression::Var(coll_vid) => exp_registry.lookup_var(coll_vid).sort.clone(),
                    _ => panic!("forall! collection must be a variable"),
                };
                let coll_str = format_expression_renamed(exp_registry, *coll_eid, ir, rename);
                let guard = match &coll_sort {
                    Sort::Array(_, _) => {
                        format!("(select (rarr-pres {}) {})", coll_str, var.name)
                    }
                    Sort::Set(_) => format!("(select (rset-data {}) {})", coll_str, var.name),
                    Sort::Seq(_) => format!(
                        "(and (>= {} 0) (< {} (seq.len {})))",
                        var.name, var.name, coll_str
                    ),
                    _ => panic!("forall! collection must be Array, Set, or Seq"),
                };
                guards.push(guard);
            }
            let guard_str = if guards.len() == 1 {
                guards.into_iter().next().unwrap()
            } else {
                format!("(and {})", guards.join(" "))
            };
            format!(
                "(forall ({}) (=> {} {}))",
                var_decls.join(" "),
                guard_str,
                body_str
            )
        }
        Expression::IterExists { vars, body } => {
            let var_decls: Vec<String> = vars
                .keys()
                .map(|var_id| {
                    let var = exp_registry.lookup_var(var_id);
                    format!("({} {})", var.name, format_sort_for_fn(&var.sort, ir))
                })
                .collect();
            let body_str = format_expression_renamed(exp_registry, *body, ir, rename);

            // Build membership guards: exists uses (and guard body)
            let mut guards = Vec::new();
            for (var_id, coll_eid) in vars {
                let var = exp_registry.lookup_var(var_id);
                let coll_exp = exp_registry.lookup_exp(coll_eid);
                let coll_sort = match coll_exp {
                    Expression::Var(coll_vid) => exp_registry.lookup_var(coll_vid).sort.clone(),
                    _ => panic!("exists! collection must be a variable"),
                };
                let coll_str = format_expression_renamed(exp_registry, *coll_eid, ir, rename);
                let guard = match &coll_sort {
                    Sort::Array(_, _) => {
                        format!("(select (rarr-pres {}) {})", coll_str, var.name)
                    }
                    Sort::Set(_) => format!("(select (rset-data {}) {})", coll_str, var.name),
                    Sort::Seq(_) => format!(
                        "(and (>= {} 0) (< {} (seq.len {})))",
                        var.name, var.name, coll_str
                    ),
                    _ => panic!("exists! collection must be Array, Set, or Seq"),
                };
                guards.push(guard);
            }
            guards.push(body_str);
            format!(
                "(exists ({}) (and {}))",
                var_decls.join(" "),
                guards.join(" ")
            )
        }
    }
}

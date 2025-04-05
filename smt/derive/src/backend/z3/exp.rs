//! This module contains the conversion of expressions to SMT-LIB format.
//! An expression is the body of a function or an axiom.

use crate::backend::z3::intrinsics::intrinsics_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::{EnumSelector, VarKind};
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, MatchCase, Variable, VariantCtor};
use crate::ir::index::{ExpId, VarId};
use crate::IRContext;
use core::panic;
use std::collections::BTreeMap;

/// Converts an expression into the corresponding SMT-LIB as a `String`.
/// This function takes an expression registry, an expression ID, an IR context, dependencies set,
/// and a mapping of variables to their SMT-LIB names.
/// It recursively converts the expression and its components into SMT-LIB format.
pub fn expr_to_smt(
    exp_registry: &ExpRegistry,
    id: &ExpId,
    ir: &IRContext,
    dependencies: &mut Vec<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    // destruct ExpRegistry
    let ExpRegistry { vars, exps } = exp_registry;

    let exp = exps.get(id).expect("expression not found in registry");
    expr_to_smt_inner(vars, exp_registry, exp, id, ir, dependencies, mapping_vars)
}

pub fn expr_to_smt_inner(
    vars: &BTreeMap<VarId, Variable>,
    exp_registry: &ExpRegistry,
    exp: &Expression,
    id: &ExpId,
    ir: &IRContext,
    dependencies: &mut Vec<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    match exp {
        Expression::Var(var_id) => {
            // if the variable is an smt variable, we return its name
            for (varid, name) in mapping_vars.iter() {
                if varid == var_id {
                    return name.clone();
                }
            }
            // if the variable is inside the match
            let varkind = vars
                .get(var_id)
                .expect("variable not found in registry")
                .kind
                .clone();
            if let VarKind::Match {
                head,
                sort: _,
                branch,
                selector,
            } = varkind
            {
                // get the name of the head variable
                let head_smt = expr_to_smt(exp_registry, &head, ir, dependencies, mapping_vars);
                let branch_name = match selector {
                    EnumSelector::Tuple(x) => format!("field_{}_{}_", branch, x + 1),
                    EnumSelector::Record(x) => format!("record_{}_", x),
                };
                return format!("({} {})", branch_name, head_smt);
            }

            // bound variables
            if let VarKind::Bound { bind } = varkind {
                return expr_to_smt(exp_registry, &bind, ir, dependencies, mapping_vars);
            }

            // if Param, we return its name
            if let VarKind::Param = varkind {
                return vars
                    .get(var_id)
                    .expect("variable not found in registry")
                    .name
                    .to_string();
            }

            // if quant
            if let VarKind::Quant = varkind {
                let v = vars
                    .get(var_id)
                    .expect("variable not found in registry")
                    .name
                    .to_string();
                return format!("{}_{}", v, var_id);
            }

            // if axiom
            return vars
                .get(var_id)
                .expect("variable not found in registry")
                .name
                .to_string();
        }
        Expression::Pack { sort, elems } => {
            let (sort_name, ty_args) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(_) = sort_name {
                panic!("tuples are unnamed types");
            }
            let tuple_name = format!(
                "Tuple_{}",
                ty_args // for tuples it is the elements list
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );
            let constructor_name = format!("mk-{}", tuple_name);
            let elems = elems
                .iter()
                .map(|e| {
                    format!(
                        "{}",
                        expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                    )
                })
                .collect::<Vec<_>>();
            format!("({} {})", constructor_name, elems.join(" "))
        }
        Expression::Tuple { sort, slots } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = slots
                    .iter()
                    .map(|e| {
                        format!(
                            "{}",
                            expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                        )
                    })
                    .collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("struct tuple must have a name");
            }
        }
        Expression::Record { sort, fields } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = fields
                    .iter()
                    .map(|(_, e)| {
                        format!(
                            "{}",
                            expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                        )
                    })
                    .collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("record struct must have a name");
            }
        }
        Expression::Enum {
            sort,
            branch,
            variant,
        } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(_) = ty {
                let constructor_name = format!("{}", branch);
                match variant {
                    VariantCtor::Unit => format!("{}", constructor_name),
                    VariantCtor::Tuple(t) => {
                        let elems = t
                            .iter()
                            .map(|e| {
                                format!(
                                    "{}",
                                    expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                                )
                            })
                            .collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                    VariantCtor::Record(r) => {
                        let elems = r
                            .iter()
                            .map(|(_, e)| {
                                format!(
                                    "{}",
                                    expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                                )
                            })
                            .collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                }
            } else {
                panic!("enum has no name")
            }
        }
        Expression::AccessSlot { base, slot } => {
            let base_smt = expr_to_smt(exp_registry, base, ir, dependencies, mapping_vars);
            let field_name = format!("field_{}_{}_", base, slot + 1);
            format!("({} {})", field_name, base_smt)
        }
        Expression::AccessField { base, field } => {
            let base_smt = expr_to_smt(exp_registry, base, ir, dependencies, mapping_vars);
            format!("({} {})", field, base_smt)
        }
        Expression::Match { cases } => {
            // if the cases is empty, we panic
            if cases.is_empty() {
                panic!("no cases in match");
            }
            // otherwise, take the first case
            let fist_case = cases.iter().next().expect("no cases in match");
            // take the rest of the cases
            let rest: Vec<MatchCase> = cases.iter().skip(1).cloned().collect();
            // if the rest has only one element, no need to create ite
            let x: String = if rest.len() == 1 {
                let case = rest.first().expect("no cases in match");
                let body = case.body;
                let body_smt = expr_to_smt(exp_registry, &body, ir, dependencies, mapping_vars);
                format!("{}", body_smt)
            } else {
                expr_to_smt_inner(
                    vars,
                    exp_registry,
                    &Expression::Match { cases: rest },
                    id,
                    ir,
                    dependencies,
                    mapping_vars,
                )
            };
            // let MatchCase { atoms, body } = case;
            let atoms = &fist_case.atoms; // atoms gives the condition
                                          // the condition
            let mut cond_smt = Vec::new();
            for atom in atoms {
                let MatchAtom {
                    head,
                    sort: _,
                    branch,
                    variant: _,
                } = atom;

                // get the condition
                let head_smt = expr_to_smt(exp_registry, head, ir, dependencies, mapping_vars);
                cond_smt.push(format!("(is-{} {})", branch, head_smt));
            }

            // construct the condition
            let final_condition = if cond_smt.len() == 1 {
                cond_smt.pop().unwrap()
            } else {
                format!("(and {})", cond_smt.join(" "))
            };

            // get the body of the first case
            let body = fist_case.body;
            let body_smt = expr_to_smt(exp_registry, &body, ir, dependencies, mapping_vars);
            format!("(ite {} {} {})", final_condition, body_smt, x)
        }
        Expression::Phi { cases, default } => {
            let first = cases.iter().next();
            if first.is_none() {
                panic!("no cases in phi");
            }
            let first_case = first.unwrap();
            // let PhiCase { cond, body } = first_case;
            let cond = first_case.cond;
            let body = first_case.body;
            let cond_smt = expr_to_smt(exp_registry, &cond, ir, dependencies, mapping_vars);
            let body_smt = expr_to_smt(exp_registry, &body, ir, dependencies, mapping_vars);
            let default = expr_to_smt(exp_registry, default, ir, dependencies, mapping_vars);
            format!("(ite {} {} {})", cond_smt, body_smt, default)
        }
        Expression::Intrinsic(intrinsic) => {
            intrinsics_to_smt(intrinsic, exp_registry, id, ir, dependencies, mapping_vars)
        }
        Expression::Procedure { callee, args } => {
            let callee_smt = ir.fn_registry.get_name(callee);
            let args_smt = args
                .iter()
                .map(|e| {
                    format!(
                        "{}",
                        expr_to_smt(exp_registry, e, ir, dependencies, mapping_vars)
                    )
                })
                .collect::<Vec<_>>();
            format!("({} {})", callee_smt, args_smt.join(" "))
        }
        Expression::Forall { vars, body } => {
            let vars_string = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var_name = format!("x_{}", var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            format!(
                "(forall ({}) {})",
                vars_string.join(" "),
                expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars)
            )
        }
        Expression::Exists { vars, body } => {
            let vars_string = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var_name = format!("x_{}", var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            format!(
                "(exists ({}) {})",
                vars_string.join(" "),
                expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars)
            )
        }
        _ => panic!("expression not supported in SMT-LIB"),
    }
}

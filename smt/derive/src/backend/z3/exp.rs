//! This module contains the conversion of expressions to SMT-LIB format.
//! An expression is the body of a function or an axiom.

use crate::backend::z3::intrinsics::intrinsics_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, MatchCase, Variable, VariantCtor};
use crate::ir::index::{ExpId, VarId};
use crate::IRContext;
use std::collections::{BTreeMap, BTreeSet};

/// Converts an expression into the corresponding SMT-LIB as a `String`.
/// This function takes an expression registry, an expression ID, an IR context, dependencies set,
/// and a mapping of variables to their SMT-LIB names.
/// It recursively converts the expression and its components into SMT-LIB format.
pub fn expr_to_smt(
    exp_registry: &ExpRegistry,
    id: &ExpId,
    ir: &IRContext,
    dependencies: &mut BTreeSet<String>,
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
    dependencies: &mut BTreeSet<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    match exp {
        Expression::Var(var_id) => {
            for (varid, name) in mapping_vars.iter() {
                if varid == var_id {
                    return name.clone();
                }
            }
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
                    VariantCtor::Unit => format!("({})", constructor_name),
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
            let field_name = format!("field{}_", slot + 1);
            format!("({} {})", field_name, base_smt)
        }
        Expression::AccessField { base, field } => {
            let base_smt = expr_to_smt(exp_registry, base, ir, dependencies, mapping_vars);
            format!("({} {})", field, base_smt)
        }
        Expression::Match { cases } => {
            if cases.is_empty() {
                panic!("no cases in match");
            }
            let case = cases.iter().next().expect("no cases in match");
            // take out the first case from cases
            let new_match: Vec<MatchCase> = cases.iter().skip(1).cloned().collect();
            // let MatchCase { atoms, body } = case;
            let atoms = &case.atoms;
            let body = case.body;
            let body_smt = expr_to_smt(exp_registry, &body, ir, dependencies, mapping_vars);
            let cond_smt = String::new();
            for atom in atoms {
                let MatchAtom {
                    head,
                    sort,
                    branch,
                    variant,
                } = atom;
                let head_smt = expr_to_smt(exp_registry, head, ir, dependencies, mapping_vars);
            }
            format!(
                "(ite {} {} {})",
                cond_smt,
                body_smt,
                expr_to_smt_inner(
                    vars,
                    exp_registry,
                    &Expression::Match { cases: new_match },
                    id,
                    ir,
                    dependencies,
                    mapping_vars,
                )
            )
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
            let vars = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var_name = format!("x_{}", var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            let ret = format!(
                "(assert (forall ({}) {}))",
                vars.join(" "),
                expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars)
            );
            dependencies.insert(ret);
            "".to_string()
        }
        Expression::Exists { vars, body } => {
            let vars = vars
                .iter()
                .map(|(var_id, sort)| {
                    let var_name = format!("x_{}", var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            let ret = format!(
                "(assert (not (forall ({}) {})))",
                vars.join(" "),
                expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars)
            );
            dependencies.insert(ret);
            "".to_string()
        }
        _ => panic!("expression not supported in SMT-LIB"),
    }
}

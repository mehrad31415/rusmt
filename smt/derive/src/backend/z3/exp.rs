//! This module contains the conversion of expressions to SMT-LIB format.
//! An expression is the body of a function or an axiom.

use crate::backend::z3::intrinsics::intrinsics_to_smt;
use crate::backend::z3::sort::derive_type;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::{EnumSelector, VarKind};
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, MatchCase, Variable, VariantCtor};
use crate::ir::index::{ExpId, VarId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::sort::Sort;
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
        Expression::Intrinsic(intrinsic) => {
            intrinsics_to_smt(intrinsic, exp_registry, id, ir, dependencies, mapping_vars)
        }
        Expression::Var(var_id) => {
            // if the variable is an smt variable, we return its name
            for (varid, name) in mapping_vars.iter() {
                if varid == var_id {
                    return name.clone();
                }
            }
            // get the kind of the variable
            let varkind = vars
                .get(var_id)
                .expect("variable not found in registrya")
                .kind
                .clone();
            match varkind {
                // if it is a match variable
                VarKind::Match {
                    head,
                    sort,
                    branch,
                    selector,
                } => {
                    // get the name of the head variable
                    let head_smt = expr_to_smt(exp_registry, &head, ir, dependencies, mapping_vars);
                    // get the sort
                    let (sort_name, _) = ir.ty_registry.reverse_lookup(sort);
                    let sort_name = sort_name.expect("sort name not found");
                    let branch_name = match selector {
                        EnumSelector::Tuple(x) => {
                            format!("field_{}_{}_{}_", sort_name, branch, x + 1)
                        }
                        EnumSelector::Record(x) => {
                            format!("record_{}_{}_{}_", sort_name, branch, x)
                        }
                    };
                    return format!("({} {})", branch_name, head_smt);
                }
                // bound variables
                VarKind::Bound { bind } => {
                    let o = expr_to_smt_inner(
                        vars,
                        exp_registry,
                        exp_registry.lookup_exp(bind),
                        &bind,
                        ir,
                        dependencies,
                        mapping_vars,
                    );
                    return o;
                }
                // if Param, we return its name
                VarKind::Param => {
                    return vars
                        .get(var_id)
                        .expect("variable not found in registryD")
                        .name
                        .to_string();
                }
                // if quant
                VarKind::Quant => {
                    let v = vars
                        .get(var_id)
                        .expect("variable not found in registryQ")
                        .name
                        .to_string();
                    return format!("{}_{}", v, var_id);
                }
                // if axiom
                VarKind::Axiom => {
                    let v = vars
                        .get(var_id)
                        .expect("variable not found in registryE")
                        .name
                        .to_string();
                    return format!("{}_{}", v, var_id);
                }
            }
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
            let ty = sort_to_smt(&derive_type(exp_registry, ir, base), ir);
            let base_smt = expr_to_smt(exp_registry, base, ir, dependencies, mapping_vars);
            format!("(field_{}_{}_ {})", ty, slot + 1, base_smt)
        }
        Expression::AccessField { base, field } => {
            let ty = sort_to_smt(&derive_type(exp_registry, ir, base), ir);
            let base_smt = expr_to_smt(exp_registry, base, ir, dependencies, mapping_vars);
            format!("(record_{}_{}_ {})", ty, field, base_smt)
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
            if args_smt.is_empty() {
                return format!("{}", callee_smt);
            }
            format!("({} {})", callee_smt, args_smt.join(" "))
        }
        // these can only be used in the spec body
        Expression::Forall { vars, body } => {
            let vars_string = vars
                .iter()
                .map(|(var_id, sort)| {
                    // get the varname
                    let var_name = exp_registry
                        .vars
                        .get(var_id)
                        .expect("variable not found in registry")
                        .name
                        .to_string();
                    let var_name = format!("{}_{}", var_name, var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            // the condition
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            format!("(forall ({}) {})", vars_string.join(" "), body_smt)
        }
        Expression::Exists { vars, body } => {
            let vars_string = vars
                .iter()
                .map(|(var_id, sort)| {
                    // get the varname
                    let var_name = exp_registry
                        .vars
                        .get(var_id)
                        .expect("variable not found in registry")
                        .name
                        .to_string();
                    let var_name = format!("{}_{}", var_name, var_id);
                    format!("({} {})", var_name, sort_to_smt(sort, ir))
                })
                .collect::<Vec<_>>();
            // the condition
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            format!("(exists ({}) {})", vars_string.join(" "), body_smt)
        }
        Expression::Choose {
            vars,
            body,
            rets: _,
        } => {
            let vars_string = vars
                .iter()
                .map(|(var_id, sort)| {
                    // get the varname
                    let var_name = exp_registry
                        .vars
                        .get(var_id)
                        .expect("variable not found in registry")
                        .name
                        .to_string();
                    let var_name = format!("{}_choose_{}", var_name, var_id);
                    mapping_vars.insert(*var_id, var_name.clone());
                    dependencies.push(format!(
                        "(declare-const {} ({}))",
                        var_name,
                        sort_to_smt(sort, ir)
                    ));
                    format!("{}", var_name)
                })
                .collect::<Vec<_>>();
            // the condition
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            dependencies.push(format!("(assert {})", body_smt));
            format!("({})", vars_string.join(" "))
        }
        Expression::IterExists { vars, body } => {
            let mut var_bindings = Vec::new(); // e.g. "(x_42 Int)"
            let mut membership = Vec::new(); // e.g. "(member x_42 xs)"

            for (var_id, domain_exp_id) in vars.iter() {
                let var_info = exp_registry
                    .vars
                    .get(var_id)
                    .unwrap_or_else(|| panic!("VarId {var_id} not found in registry"));

                let Variable {
                    name,
                    kind: _,
                    sort,
                } = var_info.clone();
                let var_name = name.to_string();
                let smt_sort = sort_to_smt(&sort, ir);
                let collection =
                    expr_to_smt(exp_registry, domain_exp_id, ir, dependencies, mapping_vars);

                // check whether the collection is a set or a seq or a map
                if exp_registry.exps.get(domain_exp_id).is_none() {
                    panic!("domain_exp_id not found in registry");
                }
                let s = derive_type(exp_registry, ir, domain_exp_id);
                match s {
                    Sort::Set(s) => {
                        let intrinsic = Intrinsic::SetEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!("(select {} {}_{})", collection, var_name, var_id));
                    }
                    Sort::Seq(s) => {
                        let intrinsic = Intrinsic::SeqEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(seq.contains {} (seq.unit {}_{}))",
                            collection, var_name, var_id
                        ));
                    }
                    Sort::Map(k, v) => {
                        let intrinsic = Intrinsic::MapEmpty {
                            k: k.as_ref().clone(),
                            v: v.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(distinct (select {} {}_{}) not_present_{})",
                            collection, var_name, var_id, v
                        ));
                    }
                    _ => panic!("domain_exp_id is not a set or a seq or a map"),
                }
                var_bindings.push(format!("({}_{} {})", var_name, var_id, smt_sort));
            }
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            let members = if membership.len() == 1 {
                membership[0].clone()
            } else {
                format!("(and {})", membership.join(" "))
            };
            format!(
                "(exists ({}) (=> {} {}))",
                var_bindings.join(" "),
                members,
                body_smt
            )
        }
        Expression::IterForall { vars, body } => {
            let mut var_bindings = Vec::new(); // e.g. "(x_42 Int)"
            let mut membership = Vec::new(); // e.g. "(member x_42 xs)"

            for (var_id, domain_exp_id) in vars.iter() {
                let var_info = exp_registry
                    .vars
                    .get(var_id)
                    .unwrap_or_else(|| panic!("VarId {var_id} not found in registry"));

                let Variable {
                    name,
                    kind: _,
                    sort,
                } = var_info.clone();
                let var_name = name.to_string();
                let smt_sort = sort_to_smt(&sort, ir);
                let collection =
                    expr_to_smt(exp_registry, domain_exp_id, ir, dependencies, mapping_vars);

                // check whether the collection is a set or a seq or a map
                if exp_registry.exps.get(domain_exp_id).is_none() {
                    panic!("domain_exp_id not found in registry");
                }
                let s = derive_type(exp_registry, ir, domain_exp_id);
                match s {
                    Sort::Set(s) => {
                        let intrinsic = Intrinsic::SetEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!("(select {} {}_{})", collection, var_name, var_id));
                    }
                    Sort::Seq(s) => {
                        let intrinsic = Intrinsic::SeqEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(seq.contains {} (seq.unit {}_{}))",
                            collection, var_name, var_id
                        ));
                    }
                    Sort::Map(k, v) => {
                        let intrinsic = Intrinsic::MapEmpty {
                            k: k.as_ref().clone(),
                            v: v.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(distinct (select {} {}_{}) not_present_{})",
                            collection, var_name, var_id, v
                        ));
                    }
                    _ => panic!("domain_exp_id is not a set or a seq or a map"),
                }
                var_bindings.push(format!("({}_{} {})", var_name, var_id, smt_sort));
            }
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            let members = if membership.len() == 1 {
                membership[0].clone()
            } else {
                format!("(and {})", membership.join(" "))
            };
            format!(
                "(forall ({}) (=> {} {}))",
                var_bindings.join(" "),
                members,
                body_smt
            )
        }
        Expression::IterChoose {
            vars,
            body,
            rets: _,
        } => {
            let mut var_bindings = Vec::new(); // e.g. "(x_42 Int)"
            let mut membership = Vec::new(); // e.g. "(member x_42 xs)"

            for (var_id, domain_exp_id) in vars.iter() {
                let var_info = exp_registry
                    .vars
                    .get(var_id)
                    .unwrap_or_else(|| panic!("VarId {var_id} not found in registry"));

                let Variable {
                    name,
                    kind: _,
                    sort,
                } = var_info.clone();
                let var_name = name.to_string();
                let var_name = format!("{}_choose_{}", var_name, var_id);
                let smt_sort = sort_to_smt(&sort, ir);
                let collection =
                    expr_to_smt(exp_registry, domain_exp_id, ir, dependencies, mapping_vars);

                // check whether the collection is a set or a seq or a map
                if exp_registry.exps.get(domain_exp_id).is_none() {
                    panic!("domain_exp_id not found in registry");
                }
                let s = derive_type(exp_registry, ir, domain_exp_id);
                match s {
                    Sort::Set(s) => {
                        let intrinsic = Intrinsic::SetEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!("(select {} {})", collection, var_name));
                    }
                    Sort::Seq(s) => {
                        let intrinsic = Intrinsic::SeqEmpty {
                            t: s.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(seq.contains {} (seq.unit {}))",
                            collection, var_name
                        ));
                    }
                    Sort::Map(k, v) => {
                        let intrinsic = Intrinsic::MapEmpty {
                            k: k.as_ref().clone(),
                            v: v.as_ref().clone(),
                        };
                        // add the dependencies and throw away the result
                        let _s = intrinsics_to_smt(
                            &intrinsic,
                            exp_registry,
                            &domain_exp_id,
                            ir,
                            dependencies,
                            mapping_vars,
                        );
                        membership.push(format!(
                            "(distinct (select {} {}) not_present_{})",
                            collection, var_name, v
                        ));
                    }
                    _ => panic!("domain_exp_id is not a set or a seq or a map"),
                }
                mapping_vars.insert(*var_id, var_name.clone());
                dependencies.push(format!("(declare-const {} ({}))", var_name, smt_sort));
                var_bindings.push(format!("{}", var_name));
            }
            let body_smt = expr_to_smt(exp_registry, body, ir, dependencies, mapping_vars);
            let members = if membership.len() == 1 {
                membership[0].clone()
            } else {
                format!("(and {})", membership.join(" "))
            };
            dependencies.push(format!("(assert (=> {} {}))", members, body_smt));
            format!("({})", var_bindings.join(" "))
        }
    }
}

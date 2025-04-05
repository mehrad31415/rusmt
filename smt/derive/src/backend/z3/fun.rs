//! This module contains the logic for converting function definitions to SMT-LIB
//! It has the following functions:
//! - `fundef_in_smt`: Converts a function definition into the corresponding SMT-LIB function definition as a `String`.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, VariantCtor};
use crate::ir::fun::{FunDef, FunSig};
use crate::ir::index::{ExpId, UsrFunId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::name::UsrFunName;
use crate::ir::sort::Sort;
use crate::IRContext;
use std::collections::{BTreeMap, BTreeSet};
use std::panic;

/// Converts a function definition into the corresponding SMT-LIB function definition as a `String`.
/// The function definition can be either a defined function or an uninterpreted function.
/// The function signature is used to determine the types of the parameters and the return type.
/// The function definition is used to determine the body of the function.
/// The Generics are already registered in `undef_sorts`.
pub fn fundef_in_smt(
    ir: &IRContext,
    funcs: &BTreeMap<UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>>,
) -> String {
    let mut dependencies = Vec::new();
    let mut mapping_vars = BTreeMap::new();

    let mut sigs = Vec::new();
    let mut bodies = Vec::new();
    let mut ret = String::new();
    for (name, generics_id) in funcs {
        let generics_id = generics_id.as_ref().expect("generics_id is None");
        for (_generics, id) in generics_id {
            let sig = ir.fn_registry.retrieve_sig(*id);
            let def = ir.fn_registry.retrieve_def(*id);

            // depending on whether the function is defined or uninterpreted, the function signature is different
            let FunSig { params, ret_ty } = sig;

            let return_type = sort_to_smt(ret_ty, ir);

            match def {
                FunDef::Defined(reg, id) => {
                    // convert the function body to SMT-LIB
                    let body_expr = expr_to_smt(reg, id, ir, &mut dependencies, &mut mapping_vars);

                    let field_defs: Vec<String> = params
                        .iter()
                        .map(|(field_name, sort)| {
                            format!("({} {})", field_name, sort_to_smt(sort, ir))
                        })
                        .collect();

                    // define the function with define-fun-rec - "(define-fun-rec {} ({}) {} {})"
                    // add the dependencies
                    for dep in dependencies.iter() {
                        ret += dep.as_str();
                        ret += "\n";
                    }
                    sigs.push(format!(
                        "{} ({}) {}\n",
                        name,
                        field_defs.join(" "),
                        return_type
                    ));
                    bodies.push(format!("{}\n", body_expr));
                }
                FunDef::Uninterpreted => {
                    let field_defs: Vec<String> = params
                        .iter()
                        .map(|(_, sort)| format!("{}", sort_to_smt(sort, ir)))
                        .collect();

                    // declare the function with declare-fun
                    // mutually dependent functions must have a body so no need to check here!
                    return format!(
                        "(declare-fun {} ({}) {})",
                        name,
                        field_defs.join(" "),
                        return_type
                    );
                }
            }
        }
    }
    if sigs.len() == 1 {
        ret += format!("(define-fun-rec {} {})", sigs[0], bodies[0]).as_str();
    } else {
        ret += format!(
            "(define-funs-rec ({}) ({}))",
            sigs.iter()
                .map(|s| format!("({})", s))
                .collect::<Vec<_>>()
                .join(" "),
            bodies
                .iter()
                .map(|s| format!("{}", s))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .as_str();
    }

    // done
    ret
}

pub fn group_dependent_funcs(
    func_name: UsrFunName,
    def: &FunDef,
    ir: &IRContext,
    generics_id: &BTreeMap<Vec<Sort>, UsrFunId>,
    func_deps: &mut BTreeSet<BTreeMap<UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>>>,
) {
    match def {
        FunDef::Defined(reg, id) => {
            if let Some(old_map) = func_deps
                .iter()
                .find(|map| map.contains_key(&func_name))
                .cloned()
            {
                func_deps.take(&old_map);
                if old_map.get(&func_name.clone()).unwrap().is_some() {
                    panic!("Function {:?} is already defined in the map!", func_name);
                }
                let mut new_map = old_map;
                new_map.insert(func_name.clone(), Some(generics_id.clone()));
                func_deps.insert(new_map.clone());
                return;
            }

            func_deps.insert(BTreeMap::from([(
                func_name.clone(),
                Some(generics_id.clone()),
            )]));
            analyze_expression(func_name, id, reg, ir, generics_id, func_deps);
        }
        FunDef::Uninterpreted => {
            if func_deps
                .iter()
                .any(|map| map.keys().any(|key| key == &func_name))
            {
                panic!(
                    "Function {:?} is uninterpreted but already exists in the map!",
                    func_name
                );
            }

            // Otherwise, insert a new set with just fun_name
            let mut new_map = BTreeMap::new();
            new_map.insert(func_name, Some(generics_id.clone()));
            func_deps.insert(new_map);
        }
    }
}

fn analyze_expression(
    func_name: UsrFunName,
    id: &ExpId,
    reg: &ExpRegistry,
    ir: &IRContext,
    generics_id: &BTreeMap<Vec<Sort>, UsrFunId>,
    func_deps: &mut BTreeSet<BTreeMap<UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>>>,
) {
    // destruct ExpRegistry
    let ExpRegistry { vars: _, exps } = reg;
    let exp = exps.get(id).expect("expression not found in registry");

    match exp {
        Expression::Var(var_id) => (),
        Expression::Pack { sort, elems } => {
            elems.iter().for_each(|e| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
        Expression::Tuple { sort, slots } => {
            slots.iter().for_each(|e| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
        Expression::Record { sort, fields } => {
            fields.iter().for_each(|(s, e)| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
        Expression::Enum {
            sort,
            branch,
            variant,
        } => match variant {
            VariantCtor::Unit => (),
            VariantCtor::Tuple(t) => {
                t.iter().for_each(|e| {
                    analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
                });
            }
            VariantCtor::Record(r) => {
                r.iter().for_each(|(s, e)| {
                    analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
                });
            }
        },
        Expression::AccessSlot { base, slot } => {
            analyze_expression(func_name, base, reg, ir, generics_id, func_deps);
        }
        Expression::AccessField { base, field } => {
            analyze_expression(func_name, base, reg, ir, generics_id, func_deps);
        }
        Expression::Match { cases } => {
            for case in cases {
                let atoms = &case.atoms;
                let body = case.body;
                analyze_expression(func_name.clone(), &body, reg, ir, generics_id, func_deps);
                for atom in atoms {
                    let MatchAtom {
                        head,
                        sort,
                        branch,
                        variant,
                    } = atom;
                    analyze_expression(func_name.clone(), head, reg, ir, generics_id, func_deps);
                }
            }
        }
        Expression::Phi { cases, default } => {
            analyze_expression(func_name.clone(), default, reg, ir, generics_id, func_deps);
            for case in cases {
                let cond = &case.cond;
                let body = case.body;
                analyze_expression(func_name.clone(), cond, reg, ir, generics_id, func_deps);
                analyze_expression(func_name.clone(), &body, reg, ir, generics_id, func_deps);
            }
        }
        Expression::Intrinsic(intrinsic) => {
            analyze_intrinsic(func_name, intrinsic, reg, ir, generics_id, func_deps);
        }
        Expression::Procedure { callee, args } => {
            let callee_smt = ir.fn_registry.get_name(callee);
            if let Some(old_map) = func_deps
                .iter()
                .find(|map| map.contains_key(&func_name))
                .cloned()
            {
                func_deps.take(&old_map);

                let mut new_map = old_map;
                new_map.insert(callee_smt.clone(), None);

                // Reinsert the modified map back into the set.
                func_deps.insert(new_map.clone());
            }
            for arg in args {
                analyze_expression(func_name.clone(), arg, reg, ir, generics_id, func_deps);
            }
        }
        Expression::Forall { vars, body } => {
            analyze_expression(func_name, body, reg, ir, generics_id, func_deps);
        }
        Expression::Exists { vars, body } => {
            analyze_expression(func_name, body, reg, ir, generics_id, func_deps);
        }
        Expression::Choose { vars, body, rets } => {
            analyze_expression(func_name, body, reg, ir, generics_id, func_deps);
        }
        Expression::IterForall { vars, body } => {
            analyze_expression(func_name.clone(), body, reg, ir, generics_id, func_deps);
            vars.iter().for_each(|(s, e)| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
        Expression::IterExists { vars, body } => {
            analyze_expression(func_name.clone(), body, reg, ir, generics_id, func_deps);
            vars.iter().for_each(|(s, e)| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
        Expression::IterChoose { vars, body, rets } => {
            analyze_expression(func_name.clone(), body, reg, ir, generics_id, func_deps);
            vars.iter().for_each(|(s, e)| {
                analyze_expression(func_name.clone(), e, reg, ir, generics_id, func_deps);
            });
        }
    }
}

fn analyze_intrinsic(
    func_name: UsrFunName,
    intrinsic: &Intrinsic,
    reg: &ExpRegistry,
    ir: &IRContext,
    generics_id: &BTreeMap<Vec<Sort>, UsrFunId>,
    func_deps: &mut BTreeSet<BTreeMap<UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>>>,
) {
    use crate::ir::intrinsics::Intrinsic::*;

    match intrinsic {
        BoolVal(b) => (),
        BoolNot { val } => {
            analyze_expression(func_name.clone(), val, reg, ir, generics_id, func_deps);
        }
        BoolAnd { lhs, rhs }
        | BoolOr { lhs, rhs }
        | BoolXor { lhs, rhs }
        | BoolImplies { lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        IntVal(i) => (),
        IntLt { lhs, rhs }
        | IntLe { lhs, rhs }
        | IntGe { lhs, rhs }
        | IntGt { lhs, rhs }
        | IntAdd { lhs, rhs }
        | IntSub { lhs, rhs }
        | IntMul { lhs, rhs }
        | IntDiv { lhs, rhs }
        | IntRem { lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        NumVal(i) => (),
        NumLt { lhs, rhs }
        | NumLe { lhs, rhs }
        | NumGe { lhs, rhs }
        | NumGt { lhs, rhs }
        | NumAdd { lhs, rhs }
        | NumSub { lhs, rhs }
        | NumMul { lhs, rhs }
        | NumDiv { lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        StrVal(s) => (),
        StrLt { lhs, rhs } | StrLe { lhs, rhs } | StrGe { lhs, rhs } | StrGt { lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        BoxShield { t, val } => {
            analyze_expression(func_name.clone(), val, reg, ir, generics_id, func_deps);
        }
        BoxReveal { t, val } => {
            analyze_expression(func_name.clone(), val, reg, ir, generics_id, func_deps);
        }
        SeqEmpty { t } => (),
        SeqLength { t: _, seq } => {
            analyze_expression(func_name.clone(), seq, reg, ir, generics_id, func_deps);
        }
        SeqAppend { t: _, seq, item } => {
            analyze_expression(func_name.clone(), seq, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), item, reg, ir, generics_id, func_deps);
        }
        SeqAt { t: _, seq, idx } => {
            analyze_expression(func_name.clone(), seq, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), idx, reg, ir, generics_id, func_deps);
        }
        SeqIncludes { t: _, seq, item } => {
            analyze_expression(func_name.clone(), seq, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), item, reg, ir, generics_id, func_deps);
        }
        SetEmpty { t } => (),
        SetLength { t, set } => {
            analyze_expression(func_name.clone(), set, reg, ir, generics_id, func_deps);
        }
        SetInsert { t: _, set, item } => {
            analyze_expression(func_name.clone(), set, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), item, reg, ir, generics_id, func_deps);
        }
        SetRemove { t: _, set, item } => {
            analyze_expression(func_name.clone(), set, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), item, reg, ir, generics_id, func_deps);
        }
        SetContains { t: _, set, item } => {
            analyze_expression(func_name.clone(), set, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), item, reg, ir, generics_id, func_deps);
        }
        // --- Map ---
        MapEmpty { k, v } => (),
        MapLength { k, v, map } => {
            analyze_expression(func_name.clone(), map, reg, ir, generics_id, func_deps);
        }
        MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            analyze_expression(func_name.clone(), map, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), key, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), val, reg, ir, generics_id, func_deps);
        }
        MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            analyze_expression(func_name.clone(), map, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), key, reg, ir, generics_id, func_deps);
        }
        MapDel { k: _, v, map, key } => {
            analyze_expression(func_name.clone(), map, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), key, reg, ir, generics_id, func_deps);
        }
        MapContainsKey { k: _, v, map, key } => {
            analyze_expression(func_name.clone(), map, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), key, reg, ir, generics_id, func_deps);
        }
        ErrFresh => (),
        ErrMerge { lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        SmtEq { t: _, lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
        SmtNe { t: _, lhs, rhs } => {
            analyze_expression(func_name.clone(), lhs, reg, ir, generics_id, func_deps);
            analyze_expression(func_name.clone(), rhs, reg, ir, generics_id, func_deps);
        }
    }
}

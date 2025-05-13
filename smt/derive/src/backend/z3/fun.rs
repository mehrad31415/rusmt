//! This module contains the logic for converting function definitions to SMT-LIB
//! It has the following functions:
//! - `fundef_in_smt`: Converts a function definition into the corresponding SMT-LIB function definition as a `String`.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, VariantCtor};
use crate::ir::fun::{FunDef, FunSig};
use crate::ir::index::{ExpId, UsrFunId, VarId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::name::UsrFunName;
use crate::ir::sort::Sort;
use crate::IRContext;
use std::collections::BTreeMap;
use std::panic;

/// Converts a function definition into the corresponding SMT-LIB function definition as a `String`.
/// The function definition can be either a defined function or an uninterpreted function.
/// The function signature is used to determine the types of the parameters and the return type.
/// The function definition is used to determine the body of the function.
/// The Generics are already registered in `undef_sorts`.
pub fn fundef_in_smt(
    ir: &IRContext,
    funcs: &Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>,
    dependencies: &mut Vec<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    let mut sigs = Vec::new();
    let mut bodies = Vec::new();
    let mut ret = String::new();
    let mut num_of_uninterpreted: i32 = 0;
    let mut uninterpreted_ret: String = String::new();
    for (name, generics_id) in funcs {
        let generics_id = generics_id.as_ref().expect("generics_id is None");
        for (_generics, id) in generics_id {
            let sig = ir.fn_registry.retrieve_sig(*id);
            let def = ir.fn_registry.retrieve_def(*id);

            // depending on whether the function is defined or uninterpreted, the function signature is different
            let FunSig { params, ret_ty } = sig;

            let return_type = sort_to_smt(ret_ty, ir, None);

            match def {
                FunDef::Defined(reg, id) => {
                    // convert the function body to SMT-LIB
                    let body_expr =
                        expr_to_smt(name.to_string(), reg, id, ir, dependencies, mapping_vars);

                    let field_defs: Vec<String> = params
                        .iter()
                        .map(|(field_name, sort)| {
                            format!(
                                "({}_{} {})",
                                name.to_string(),
                                field_name,
                                sort_to_smt(sort, ir, None)
                            )
                        })
                        .collect();

                    // define the function with define-fun-rec - "(define-fun-rec {} ({}) {} {})"
                    sigs.push(format!(
                        "{} ({}) {}\n",
                        name,
                        field_defs.join(" "),
                        return_type
                    ));
                    bodies.push(format!("{}\n", body_expr));
                }
                FunDef::Uninterpreted => {
                    num_of_uninterpreted += 1;
                    let field_defs: Vec<String> = params
                        .iter()
                        .map(|(_, sort)| format!("{}", sort_to_smt(sort, ir, None)))
                        .collect();

                    // declare the function with declare-fun
                    // mutually dependent functions must have a body so no need to check here!
                    uninterpreted_ret = format!(
                        "(declare-fun {} ({}) {})",
                        name,
                        field_defs.join(" "),
                        return_type
                    );
                }
            }
        }
    }
    if num_of_uninterpreted > 1 {
        panic!(
            "There are {} mutually depedendent uninterpreted functions, but maximum one is allowed!",
            num_of_uninterpreted
        );
    } else if num_of_uninterpreted == 1 {
        if sigs.len() != 0 {
            panic!(
                "when there is one uninterpreted function, there should be no defined functions!"
            );
        }
        return uninterpreted_ret;
    } else {
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
    }
    // done
    ret
}

/// Groups dependent functions in the function definition.
/// This is so that cyclic dependent functions are defined differently in SMT-LIB.
pub fn group_dependent_funcs(
    func_name: &UsrFunName,
    generics_id: &BTreeMap<Vec<Sort>, UsrFunId>,
    ir: &IRContext,
    func_deps: &mut Vec<Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>>,
) {
    for (_, id) in generics_id {
        let def = ir.fn_registry.retrieve_def(*id).clone();

        if let Some(inner) = func_deps
            .iter_mut()
            .find(|v| v.iter().any(|(name, _)| name == func_name))
        {
            // grab the (name, value) pair itself
            let (_, slot) = inner
                .iter_mut()
                .find(|(name, _)| name == func_name)
                .unwrap();

            // same‐name definition already present
            if slot.is_some() {
                panic!("Function {:?} is already defined in the map!", func_name);
            }

            // otherwise fill the slot
            *slot = Some(generics_id.clone());
        } else {
            // no inner vec had `func_name`
            func_deps.push(vec![(func_name.clone(), Some(generics_id.clone()))]);
        }
        if let FunDef::Defined(reg, id) = def {
            // analyzes the body of the function
            receive_funcall(func_name, &id, &reg, ir, func_deps);
        }
    }
}

/// Analyzes the body expression and its dependencies to determine the function calls inside the expression.
fn receive_funcall(
    func_name: &UsrFunName,
    id: &ExpId,
    reg: &ExpRegistry,
    ir: &IRContext,
    func_deps: &mut Vec<Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>>,
) {
    // destruct ExpRegistry
    let ExpRegistry { vars: _, exps } = reg;
    let exp = exps.get(id).expect("expression not found in registry");

    match exp {
        Expression::Var(_) => (),
        Expression::Pack { sort: _, elems } => {
            elems.iter().for_each(|e| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
        Expression::Tuple { sort: _, slots } => {
            slots.iter().for_each(|e| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
        Expression::Record { sort: _, fields } => {
            fields.iter().for_each(|(_, e)| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
        Expression::Enum {
            sort: _,
            branch: _,
            variant,
        } => match variant {
            VariantCtor::Unit => (),
            VariantCtor::Tuple(t) => {
                t.iter().for_each(|e| {
                    receive_funcall(func_name, e, reg, ir, func_deps);
                });
            }
            VariantCtor::Record(r) => {
                r.iter().for_each(|(_, e)| {
                    receive_funcall(func_name, e, reg, ir, func_deps);
                });
            }
        },
        Expression::AccessSlot { base, slot: _ } => {
            receive_funcall(func_name, base, reg, ir, func_deps);
        }
        Expression::AccessField { base, field: _ } => {
            receive_funcall(func_name, base, reg, ir, func_deps);
        }
        Expression::Match { cases } => {
            for case in cases {
                let atoms = &case.atoms;
                let body = case.body;
                receive_funcall(func_name, &body, reg, ir, func_deps);
                for atom in atoms {
                    let MatchAtom {
                        head,
                        sort: _,
                        branch: _,
                        variant: _,
                    } = atom;
                    receive_funcall(func_name, head, reg, ir, func_deps);
                }
            }
        }
        Expression::Phi { cases, default } => {
            receive_funcall(func_name, default, reg, ir, func_deps);
            for case in cases {
                let cond = &case.cond;
                let body = case.body;
                receive_funcall(func_name, cond, reg, ir, func_deps);
                receive_funcall(func_name, &body, reg, ir, func_deps);
            }
        }
        Expression::Intrinsic(intrinsic) => {
            analyze_intrinsic(func_name, intrinsic, reg, ir, func_deps);
        }
        Expression::Procedure { callee, args } => {
            let callee_smt = ir.fn_registry.get_name(callee);

            // Find the index of the group that contains the caller - if any.
            let pos_caller_opt = func_deps
                .iter()
                .position(|group| group.iter().any(|(name, _)| name == func_name));
            // Find the index of the group that already contains the callee - if any.
            let pos_callee_opt = func_deps
                .iter()
                .position(|group| group.iter().any(|(name, _)| name == &callee_smt));

            match (pos_caller_opt, pos_callee_opt) {
                // Both caller and callee are in some groups.
                (Some(pos_caller), Some(pos_callee)) => {
                    if pos_caller != pos_callee {
                        // Merge the two groups.
                        // We'll remove both groups and then push back a union of their members.
                        let idx_low = pos_caller.min(pos_callee);
                        let idx_high = pos_caller.max(pos_callee);
                        let mut group_low = func_deps.remove(idx_low);
                        // Removing idx_high: note that after removing the lower-index group, the higher index shifts by -1.
                        let group_high = func_deps.remove(idx_high - 1);
                        for (n, dep) in group_high {
                            if !group_low.iter().any(|(name, _)| name == &n) {
                                group_low.push((n, dep));
                            } else {
                                // here we have the same function in two groups, so we see:
                                // if the dep is none in group_low and some in group_high, we take the one in group_high
                                // if the dep is none in both do nothing
                                // if the dep is some in group_low and none in group_high, we take the one in group_low
                                // if the dep is some in both, they should be equal and do nothing
                                if group_low
                                    .iter()
                                    .find(|(name, _)| name == &n)
                                    .unwrap()
                                    .1
                                    .is_none()
                                    && dep.is_some()
                                {
                                    // replace the None with the Some
                                    let pos =
                                        group_low.iter().position(|(name, _)| name == &n).unwrap();
                                    group_low[pos].1 = dep.clone();
                                }

                                // if both are Some, we should check if they are equal
                                if group_low
                                    .iter()
                                    .find(|(name, _)| name == &n)
                                    .unwrap()
                                    .1
                                    .is_some()
                                    && dep.is_some()
                                {
                                    let pos =
                                        group_low.iter().position(|(name, _)| name == &n).unwrap();
                                    if group_low[pos].1 != dep {
                                        panic!("Function {:?} has different dependencies in two groups!", n);
                                    }
                                }
                            }
                        }
                        func_deps.push(group_low);
                    }
                    // Else: same group already — do nothing.
                }
                // Caller is in a group but callee is new.
                (Some(pos_caller), None) => {
                    // Add callee to the caller's group.
                    let mut group = func_deps.remove(pos_caller);
                    // Only add callee if not already present.
                    if !group.iter().any(|(name, _)| name == &callee_smt) {
                        // add the new (callee, None) entry at the start
                        // this is because we want the callee to be first defined before being used
                        group.insert(0, (callee_smt.clone(), None));
                    }
                    // put the vec back where it was
                    func_deps.insert(pos_caller, group);
                }
                // caller is not in a group, this should not happen because the caller is added before entering the function
                (None, _) => {
                    panic!("Caller {:?} is not in a group", func_name);
                }
            }

            // Process the procedure arguments as before.
            for arg in args {
                receive_funcall(func_name, arg, reg, ir, func_deps);
            }
        }
        Expression::Forall { vars: _, body } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
        }
        Expression::Exists { vars: _, body } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
        }
        Expression::Choose {
            vars: _,
            body,
            rets: _,
        } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
        }
        Expression::IterForall { vars, body } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
            vars.iter().for_each(|(_, e)| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
        Expression::IterExists { vars, body } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
            vars.iter().for_each(|(_, e)| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
        Expression::IterChoose {
            vars,
            body,
            rets: _,
        } => {
            receive_funcall(func_name, body, reg, ir, func_deps);
            vars.iter().for_each(|(_, e)| {
                receive_funcall(func_name, e, reg, ir, func_deps);
            });
        }
    }
}

/// Analyze the function calls in the intrinsic.
fn analyze_intrinsic(
    func_name: &UsrFunName,
    intrinsic: &Intrinsic,
    reg: &ExpRegistry,
    ir: &IRContext,
    func_deps: &mut Vec<Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>>,
) {
    use crate::ir::intrinsics::Intrinsic::*;

    match intrinsic {
        BoolVal(_) => (),
        BoolNot { val } => {
            receive_funcall(func_name, val, reg, ir, func_deps);
        }
        BoolAnd { lhs, rhs }
        | BoolOr { lhs, rhs }
        | BoolXor { lhs, rhs }
        | BoolImplies { lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        IntVal(_) => (),
        IntLt { lhs, rhs }
        | IntLe { lhs, rhs }
        | IntGe { lhs, rhs }
        | IntGt { lhs, rhs }
        | IntAdd { lhs, rhs }
        | IntSub { lhs, rhs }
        | IntMul { lhs, rhs }
        | IntDiv { lhs, rhs }
        | IntRem { lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        NumVal(_) => (),
        NumLt { lhs, rhs }
        | NumLe { lhs, rhs }
        | NumGe { lhs, rhs }
        | NumGt { lhs, rhs }
        | NumAdd { lhs, rhs }
        | NumSub { lhs, rhs }
        | NumMul { lhs, rhs }
        | NumDiv { lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        StrVal(_) => (),
        StrLt { lhs, rhs } | StrLe { lhs, rhs } | StrGe { lhs, rhs } | StrGt { lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        BoxShield { t: _, val } => {
            receive_funcall(func_name, val, reg, ir, func_deps);
        }
        BoxReveal { t: _, val } => {
            receive_funcall(func_name, val, reg, ir, func_deps);
        }
        SeqEmpty { t: _ } => (),
        SeqLength { t: _, seq } => {
            receive_funcall(func_name, seq, reg, ir, func_deps);
        }
        SeqAppend { t: _, seq, item } => {
            receive_funcall(func_name, seq, reg, ir, func_deps);
            receive_funcall(func_name, item, reg, ir, func_deps);
        }
        SeqAt { t: _, seq, idx } => {
            receive_funcall(func_name, seq, reg, ir, func_deps);
            receive_funcall(func_name, idx, reg, ir, func_deps);
        }
        SeqIncludes { t: _, seq, item } => {
            receive_funcall(func_name, seq, reg, ir, func_deps);
            receive_funcall(func_name, item, reg, ir, func_deps);
        }
        SetEmpty { t: _ } => (),
        SetLength { t: _, set } => {
            receive_funcall(func_name, set, reg, ir, func_deps);
        }
        SetInsert { t: _, set, item } => {
            receive_funcall(func_name, set, reg, ir, func_deps);
            receive_funcall(func_name, item, reg, ir, func_deps);
        }
        SetRemove { t: _, set, item } => {
            receive_funcall(func_name, set, reg, ir, func_deps);
            receive_funcall(func_name, item, reg, ir, func_deps);
        }
        SetContains { t: _, set, item } => {
            receive_funcall(func_name, set, reg, ir, func_deps);
            receive_funcall(func_name, item, reg, ir, func_deps);
        }
        // --- Map ---
        MapEmpty { k: _, v: _ } => (),
        MapLength { k: _, v: _, map } => {
            receive_funcall(func_name, map, reg, ir, func_deps);
        }
        MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            receive_funcall(func_name, map, reg, ir, func_deps);
            receive_funcall(func_name, key, reg, ir, func_deps);
            receive_funcall(func_name, val, reg, ir, func_deps);
        }
        MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            receive_funcall(func_name, map, reg, ir, func_deps);
            receive_funcall(func_name, key, reg, ir, func_deps);
        }
        MapDel {
            k: _,
            v: _,
            map,
            key,
        } => {
            receive_funcall(func_name, map, reg, ir, func_deps);
            receive_funcall(func_name, key, reg, ir, func_deps);
        }
        MapContainsKey {
            k: _,
            v: _,
            map,
            key,
        } => {
            receive_funcall(func_name, map, reg, ir, func_deps);
            receive_funcall(func_name, key, reg, ir, func_deps);
        }
        ErrFresh => (),
        ErrMerge { lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        SmtEq { t: _, lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
        SmtNe { t: _, lhs, rhs } => {
            receive_funcall(func_name, lhs, reg, ir, func_deps);
            receive_funcall(func_name, rhs, reg, ir, func_deps);
        }
    }
}

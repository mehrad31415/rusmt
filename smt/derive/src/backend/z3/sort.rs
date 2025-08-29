//! Converts a `Sort` to a Z3 `Sort`.

use crate::ir::{
    ctxt::IRContext,
    exp::{ExpRegistry, Expression},
    index::{ExpId, UsrSortId},
    intrinsics::Intrinsic,
    name::UsrSortName,
    sort::{DataType, Sort},
};
use std::collections::HashMap;
use z3::{Context, DatatypeVariant};

/// Converts a `Sort` to a Z3 `Sort`.
pub fn sort_to_z3(
    s: &Sort,
    ctx: &Context,
    ir: &IRContext,
    user_sort_name: Option<&UsrSortName>,
    ty_map: &HashMap<UsrSortId, (z3::Sort, Vec<DatatypeVariant>)>,
) -> z3::Sort {
    match s {
        Sort::Boolean => z3::Sort::bool(ctx),
        Sort::Integer => z3::Sort::int(ctx),
        Sort::Rational => z3::Sort::real(ctx),
        Sort::Text => z3::Sort::string(ctx),

        Sort::Seq(inner) => {
            let inner_sort = sort_to_z3(inner, ctx, ir, user_sort_name, ty_map);
            z3::Sort::seq(ctx, &inner_sort)
        }

        Sort::Set(inner) => {
            let inner_sort = sort_to_z3(inner, ctx, ir, user_sort_name, ty_map);
            z3::Sort::set(ctx, &inner_sort)
        }

        Sort::Map(key, val) => {
            let key_sort = sort_to_z3(key, ctx, ir, user_sort_name, ty_map);
            let val_sort = sort_to_z3(val, ctx, ir, user_sort_name, ty_map);
            z3::Sort::array(ctx, &key_sort, &val_sort)
        }

        Sort::Error => panic!("cannot convert error sort to Z3 API"),

        Sort::User(sid) => ty_map
            .get(sid)
            .expect("user sort not defined before it is used")
            .0
            .clone(),

        Sort::Uninterpreted(name) => {
            let full = if let Some(parent) = user_sort_name {
                let n = format!("{parent}_{name}");
                z3::Sort::uninterpreted(ctx, n.clone().into());
                n
            } else {
                name.to_string()
            };
            z3::Sort::uninterpreted(ctx, full.into())
        }
    }
}

/// Derive the type of an expression for inside quantifiers for example forall (x in xs => x > 0) the type of xs is defined by this function
pub fn derive_type(exp_registry: &ExpRegistry, ir: &IRContext, eid: &ExpId) -> Sort {
    let sort = match exp_registry.lookup_exp(eid) {
        Expression::Var(vid) => exp_registry.lookup_var(vid).sort.clone(),
        Expression::Pack { sort, elems: _ }
        | Expression::Tuple { sort, slots: _ }
        | Expression::Record { sort, fields: _ }
        | Expression::Enum {
            sort,
            branch: _,
            variant: _,
        } => Sort::User(*sort),
        Expression::AccessSlot { base, slot } => {
            let base_sort = derive_type(exp_registry, ir, base);
            let base_tuple = match ir.ty_registry.retrieve(match &base_sort {
                Sort::User(sid) => *sid,
                _ => panic!("type mismatch: expect $? | actual {base_sort}"),
            }) {
                DataType::Tuple(tuple) => tuple.clone(),
                dt => panic!("type mismatch: expect <tuple> | actual {dt}"),
            };
            base_tuple
                .into_iter()
                .nth(*slot)
                .unwrap_or_else(|| panic!("type mismatch: no slot {slot} in tuple {base_sort}"))
        }
        Expression::AccessField { base, field } => {
            let base_sort = derive_type(exp_registry, ir, base);
            let mut base_record = match ir.ty_registry.retrieve(match &base_sort {
                Sort::User(sid) => *sid,
                _ => panic!("type mismatch: expect $? | actual {base_sort}"),
            }) {
                DataType::Record(record) => record.clone(),
                dt => panic!("type mismatch: expect <record> | actual {dt}"),
            };
            base_record
                .remove(field)
                .unwrap_or_else(|| panic!("type mismatch: no field {field} in record {base_sort}"))
                .clone()
        }
        Expression::Match { cases } => {
            let mut case_sort = None;
            for case in cases {
                let sort = derive_type(exp_registry, ir, &case.body);
                match &case_sort {
                    None => {
                        case_sort = Some(sort);
                    }
                    Some(s) => {
                        if s != &sort {
                            panic!("type mismatch: expect {s} | actual {sort}");
                        }
                    }
                }
            }
            match case_sort {
                None => panic!("expect at least one match arm"),
                Some(sort) => sort,
            }
        }
        Expression::Phi { cases, default } => {
            if cases.is_empty() {
                panic!("expect at least one phi case");
            }
            let case_sort = derive_type(exp_registry, ir, default);
            for case in cases {
                let sort = derive_type(exp_registry, ir, &case.body);
                if case_sort != sort {
                    panic!("type mismatch: expect {case_sort} | actual {sort}");
                }
            }
            case_sort
        }
        Expression::Forall { .. }
        | Expression::Exists { .. }
        | Expression::IterForall { .. }
        | Expression::IterExists { .. } => Sort::Boolean,
        Expression::Choose {
            vars,
            body: _,
            rets,
        } => {
            let mut inst = vec![];
            for vid in rets {
                match vars.get(vid) {
                    None => panic!("invalid axiom variable to return"),
                    Some(sort) => {
                        inst.push(sort.clone());
                    }
                }
            }
            // unwrap the single-element tuple for choose
            if inst.len() == 1 {
                inst.into_iter().next().unwrap()
            } else {
                Sort::User(lookup_type(ir, None, &inst))
            }
        }
        Expression::IterChoose {
            vars,
            body: _,
            rets,
        } => {
            let mut inst = vec![];
            for vid in rets {
                match vars.get(vid) {
                    None => panic!("invalid iterator variable to return"),
                    Some(eid) => {
                        let vty = match derive_type(exp_registry, ir, eid) {
                            Sort::Seq(_) => Sort::Integer,
                            Sort::Set(e) => *e,
                            Sort::Map(k, _) => *k,
                            _ => panic!("not a collection sort"),
                        };
                        inst.push(vty);
                    }
                }
            }
            // unwrap the single-element tuple for choose
            if inst.len() == 1 {
                inst.into_iter().next().unwrap()
            } else {
                Sort::User(lookup_type(ir, None, &inst))
            }
        }
        Expression::Intrinsic(intrinsic) => match intrinsic.as_ref() {
            // boolean
            Intrinsic::BoolVal(_)
            | Intrinsic::BoolNot { .. }
            | Intrinsic::BoolAnd { .. }
            | Intrinsic::BoolOr { .. }
            | Intrinsic::BoolXor { .. }
            | Intrinsic::BoolImplies { .. }
            | Intrinsic::BoolIff { .. } => Sort::Boolean,
            // integer
            Intrinsic::IntVal(_)
            | Intrinsic::IntAdd { .. }
            | Intrinsic::IntSub { .. }
            | Intrinsic::IntMul { .. }
            | Intrinsic::IntDiv { .. }
            | Intrinsic::IntRem { .. }
            | Intrinsic::IntPow { .. }
            | Intrinsic::IntAbs { .. } => Sort::Integer,
            Intrinsic::IntLt { .. }
            | Intrinsic::IntLe { .. }
            | Intrinsic::IntGe { .. }
            | Intrinsic::IntGt { .. } => Sort::Boolean,
            Intrinsic::IntToRational { .. } => Sort::Rational,
            // rational
            Intrinsic::NumVal(_)
            | Intrinsic::NumAdd { .. }
            | Intrinsic::NumSub { .. }
            | Intrinsic::NumMul { .. }
            | Intrinsic::NumDiv { .. }
            | Intrinsic::NumAbs { .. }
            | Intrinsic::NumPow { .. } => Sort::Rational,
            Intrinsic::NumRound { .. } | Intrinsic::NumFloor { .. } | Intrinsic::NumCeil { .. } => {
                Sort::Integer
            }
            Intrinsic::NumLt { .. }
            | Intrinsic::NumLe { .. }
            | Intrinsic::NumGe { .. }
            | Intrinsic::NumGt { .. } => Sort::Boolean,
            // string
            Intrinsic::StrVal(_) | Intrinsic::StrConcat { .. } | Intrinsic::StrAt { .. } => {
                Sort::Text
            }
            Intrinsic::StrLt { .. }
            | Intrinsic::StrLe { .. }
            | Intrinsic::StrGe { .. }
            | Intrinsic::StrGt { .. }
            | Intrinsic::StrIncludes { .. }
            | Intrinsic::StrStartsWith { .. }
            | Intrinsic::StrEndsWith { .. } => Sort::Boolean,
            Intrinsic::StrLength { .. } => Sort::Integer,
            // cloak
            Intrinsic::BoxShield { t, .. } | Intrinsic::BoxReveal { t, .. } => t.clone(),
            // seq
            Intrinsic::SeqEmpty { t } | Intrinsic::SeqAppend { t, .. } => {
                Sort::Seq(t.clone().into())
            }
            Intrinsic::SeqLength { .. } => Sort::Integer,
            Intrinsic::SeqAt { t, .. } => t.clone(),
            Intrinsic::SeqIncludes { .. } | Intrinsic::SeqIsEmpty { .. } => Sort::Boolean,
            // set
            Intrinsic::SetEmpty { t }
            | Intrinsic::SetInsert { t, .. }
            | Intrinsic::SetRemove { t, .. }
            | Intrinsic::SetUnion { t, .. }
            | Intrinsic::SetIntersection { t, .. }
            | Intrinsic::SetDifference { t, .. } => Sort::Set(t.clone().into()),
            Intrinsic::SetLength { .. } => Sort::Integer,
            Intrinsic::SetContains { .. }
            | Intrinsic::SetIsEmpty { .. }
            | Intrinsic::SetIsSubset { .. } => Sort::Boolean,
            // map
            Intrinsic::MapEmpty { k, v }
            | Intrinsic::MapPut { k, v, .. }
            | Intrinsic::MapDel { k, v, .. } => Sort::Map(k.clone().into(), v.clone().into()),
            Intrinsic::MapGet { v, .. } => v.clone(),
            Intrinsic::MapLength { .. } => Sort::Integer,
            Intrinsic::MapContainsKey { .. } | Intrinsic::MapIsEmpty { .. } => Sort::Boolean,
            // error
            Intrinsic::ErrFresh | Intrinsic::ErrMerge { .. } => Sort::Error,
            // smt
            Intrinsic::SmtEq { .. } | Intrinsic::SmtNe { .. } => Sort::Boolean,
        },
        Expression::Procedure { callee, args: _ } => {
            ir.fn_registry.retrieve_sig(*callee).ret_ty.clone()
        }
    };
    sort
}

/// Lookup the type of a user-defined sort
fn lookup_type(ir: &IRContext, name: Option<&UsrSortName>, inst: &[Sort]) -> UsrSortId {
    match ir.ty_registry.get_index(name, inst) {
        None => {
            let inst_content = inst
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            match name {
                None => panic!("anonymous sort not registered ({inst_content})"),
                Some(n) => panic!("user-defined sort not registered {n}<{inst_content}>"),
            }
        }
        Some(sid) => sid,
    }
}

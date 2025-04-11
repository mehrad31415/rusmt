//! This module contains the conversion functions for converting Rusmart types to SMT-LIB types

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::ty::tyuse_in_smt;
use crate::ir::exp::{ExpRegistry, Expression};
use crate::ir::index::{ExpId, UsrSortId, VarId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::name::UsrSortName;
use crate::ir::sort::{DataType, Sort};
use crate::IRContext;
use core::panic;
use std::collections::BTreeMap;
use std::f32::consts::PI;
use std::sync::atomic::Ordering;

use super::intrinsics::COUNTER;

/// Converts a Rust `Sort` into the corresponding SMT-LIB sort as a `String`
pub fn sort_to_smt(s: &Sort, ir: &IRContext) -> String {
    match s {
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Rational => "Real".to_string(),
        Sort::Text => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", sort_to_smt(inner, ir)),
        Sort::Set(inner) => format!("(Set {})", sort_to_smt(inner, ir)),
        Sort::Map(key, value) => {
            format!(
                "(Array {} {})",
                sort_to_smt(key, ir),
                sort_to_smt(value, ir)
            )
        }
        Sort::Error => "undefined_function".to_string(), // triggers an undefined function which leads to a crash assuming that `undefined_function` is not defined!
        Sort::User(usr_sort_id) => tyuse_in_smt(*usr_sort_id, ir),
        Sort::Uninterpreted(name) => format!("{}", name),
    }
}

pub fn sort_to_smt_name(s: &Sort, ir: &IRContext) -> String {
    match s {
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Rational => "Real".to_string(),
        Sort::Text => "String".to_string(),
        Sort::Seq(inner) => format!("Seq_{}", sort_to_smt(inner, ir)),
        Sort::Set(inner) => format!("Set_{}", sort_to_smt(inner, ir)),
        Sort::Map(key, value) => {
            format!("Array_{}_{}", sort_to_smt(key, ir), sort_to_smt(value, ir))
        }
        Sort::Error => "undefined_function".to_string(), // triggers an undefined function which leads to a crash assuming that `undefined_function` is not defined!
        Sort::User(usr_sort_id) => tyuse_in_smt(*usr_sort_id, ir),
        Sort::Uninterpreted(name) => format!("{}", name),
    }
}

/// This function gives the default value for a given sort (type)
pub fn sort_default_value(
    exp_registry: &ExpRegistry,
    var_id: &VarId,
    s: &Sort,
    ir: &IRContext,
    dependencies: &mut Vec<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    match s {
        Sort::Boolean => "false".to_string(),
        Sort::Integer => "0".to_string(),
        Sort::Rational => "0.0".to_string(),
        Sort::Text => "\"\"".to_string(), // empty string
        Sort::Seq(ty) => {
            let ty_name = sort_to_smt(ty, ir);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let decl = format!("(declare-const seq_{} (Seq {}))", id, ty_name);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (= seq_{} (as seq.empty (Seq {})))) ; seq.empty",
                id, ty_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            // inside the function body we need to use the name seq_<id> instead of the original name
            mapping_vars.insert(*var_id, format!("seq_{}", id));
            format!("seq_{}", id)
        }
        Sort::Set(ty) => {
            let ty_name = sort_to_smt(ty, ir);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let decl = format!("(declare-const set_{} (Set {}))", id, ty_name);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((x {})) (= (select set_{} x) false))) ; set.empty",
                ty_name, id
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            mapping_vars.insert(*var_id, format!("set_{}", id));
            // sets do not have a length in SMT-LIB, so we need a function
            let decl = format!("(declare-fun len ((Set {})) Int)", ty_name);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!("(assert (= (len set_{}) 0)) ; length of empty set is 0", id);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (not (select m i)) (= (len (store m i true)) (+ (len m) 1))))) ; length of set after adding an element",
                ty_name,
                ty_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (select m i) (= (len (store m i true)) (len m)))))",
                ty_name, ty_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (select m i) (= (len (store m i false)) (- (len m) 1)))))",
                ty_name, ty_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (not (select m i)) (= (len (store m i false)) (len m)))))",
                ty_name, ty_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("set_{}", id)
        }
        Sort::Map(k, v) => {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let decl = format!(
                "(declare-const map_{} (Array {} {}))",
                id,
                sort_to_smt(k, ir),
                sort_to_smt(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-const not_present_{} {})",
                sort_to_smt_name(v, ir),
                sort_to_smt(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((x {})) (= (select map_{} x) not_present_{}))) ; array.empty",
                sort_to_smt(k, ir),
                id,
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            // inside the function body we need to use the name map_<id> instead of the original name
            mapping_vars.insert(*var_id, format!("map_{}", id));
            // arrays do not have a length in SMT-LIB, so we need a function
            // also we need some semantics for the length of the map (even though a full definition is not possible)
            let decl = format!(
                "(declare-fun len_map ((Array {} {})) Int)",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (= (len_map map_{}) 0)) ; length of empty map is 0",
                id
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(define-fun in_map ((m (Array {} {})) (i {})) Bool
                (not (= (select m i) not_present_{})))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}) (v {}))
            (=> (and (not (in_map m i)) (not (= v not_present_{}))) (= (len_map (store m i v)) (+ (len_map m) 1)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}) (v {}))
                (=> (and (in_map m i) (not (= v not_present_{}))) (= (len_map (store m i v)) (len_map m)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}))
                        (=> (in_map m i)
                            (= (len_map (store m i not_present_{})) (- (len_map m) 1)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}))
                        (=> (not (in_map m i))
                            (= (len_map (store m i not_present_{})) (len_map m)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt_name(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("map_{}", id)
        }
        Sort::Error => "undefined_function".to_string(), // just crash on purpose
        Sort::User(usr_sort_id) => {
            // give the first element of the user defined type as a default value
            format!("")
        }
        Sort::Uninterpreted(name) => {
            panic!("no default value for {}", name);
        }
    }
}

/// Derive the type of an expression for inside quantifiers for example forall (x in xs => x > 0) the type of xs is defined by this function
pub fn derive_type(exp_registry: &ExpRegistry, ir: &IRContext, eid: &ExpId) -> Sort {
    let sort = match exp_registry.lookup_exp(*eid) {
        Expression::Var(vid) => exp_registry.lookup_var(*vid).sort.clone(),
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
                _ => panic!("type mismatch: expect $? | actual {}", base_sort),
            }) {
                DataType::Tuple(tuple) => tuple.clone(),
                dt => panic!("type mismatch: expect <tuple> | actual {}", dt),
            };
            base_tuple
                .into_iter()
                .nth(*slot)
                .unwrap_or_else(|| panic!("type mismatch: no slot {} in tuple {}", slot, base_sort))
        }
        Expression::AccessField { base, field } => {
            let base_sort = derive_type(exp_registry, ir, base);
            let mut base_record = match ir.ty_registry.retrieve(match &base_sort {
                Sort::User(sid) => *sid,
                _ => panic!("type mismatch: expect $? | actual {}", base_sort),
            }) {
                DataType::Record(record) => record.clone(),
                dt => panic!("type mismatch: expect <record> | actual {}", dt),
            };
            base_record
                .remove(field)
                .unwrap_or_else(|| {
                    panic!("type mismatch: no field {} in record {}", field, base_sort)
                })
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
                            panic!("type mismatch: expect {} | actual {}", s, sort);
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
                    panic!("type mismatch: expect {} | actual {}", case_sort, sort);
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
        Expression::Intrinsic(intrinsic) => match intrinsic {
            // boolean
            Intrinsic::BoolVal(_)
            | Intrinsic::BoolNot { .. }
            | Intrinsic::BoolAnd { .. }
            | Intrinsic::BoolOr { .. }
            | Intrinsic::BoolXor { .. }
            | Intrinsic::BoolImplies { .. } => Sort::Boolean,
            // integer
            Intrinsic::IntVal(_)
            | Intrinsic::IntAdd { .. }
            | Intrinsic::IntSub { .. }
            | Intrinsic::IntMul { .. }
            | Intrinsic::IntDiv { .. }
            | Intrinsic::IntRem { .. } => Sort::Integer,
            Intrinsic::IntLt { .. }
            | Intrinsic::IntLe { .. }
            | Intrinsic::IntGe { .. }
            | Intrinsic::IntGt { .. } => Sort::Boolean,
            // rational
            Intrinsic::NumVal(_)
            | Intrinsic::NumAdd { .. }
            | Intrinsic::NumSub { .. }
            | Intrinsic::NumMul { .. }
            | Intrinsic::NumDiv { .. } => Sort::Rational,
            Intrinsic::NumLt { .. }
            | Intrinsic::NumLe { .. }
            | Intrinsic::NumGe { .. }
            | Intrinsic::NumGt { .. } => Sort::Boolean,
            // string
            Intrinsic::StrVal(_) => Sort::Text,
            Intrinsic::StrLt { .. }
            | Intrinsic::StrLe { .. }
            | Intrinsic::StrGe { .. }
            | Intrinsic::StrGt { .. } => Sort::Boolean,
            // cloak
            Intrinsic::BoxShield { t, .. } | Intrinsic::BoxReveal { t, .. } => t.clone(),
            // seq
            Intrinsic::SeqEmpty { t } | Intrinsic::SeqAppend { t, .. } => {
                Sort::Seq(t.clone().into())
            }
            Intrinsic::SeqLength { .. } => Sort::Integer,
            Intrinsic::SeqAt { t, .. } => t.clone(),
            Intrinsic::SeqIncludes { .. } => Sort::Boolean,
            // set
            Intrinsic::SetEmpty { t }
            | Intrinsic::SetInsert { t, .. }
            | Intrinsic::SetRemove { t, .. } => Sort::Set(t.clone().into()),
            Intrinsic::SetLength { .. } => Sort::Integer,
            Intrinsic::SetContains { .. } => Sort::Boolean,
            // map
            Intrinsic::MapEmpty { k, v }
            | Intrinsic::MapPut { k, v, .. }
            | Intrinsic::MapDel { k, v, .. } => Sort::Map(k.clone().into(), v.clone().into()),
            Intrinsic::MapGet { v, .. } => v.clone(),
            Intrinsic::MapLength { .. } => Sort::Integer,
            Intrinsic::MapContainsKey { .. } => Sort::Boolean,
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
                None => panic!("anonymous sort not registered ({})", inst_content),
                Some(n) => panic!("user-defined sort not registered {}<{}>", n, inst_content),
            }
        }
        Some(sid) => sid,
    }
}

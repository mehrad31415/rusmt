//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_not_present;
use crate::ir::exp::ExpRegistry;
use crate::ir::exp::VarKind;
use crate::ir::index::ExpId;
use crate::ir::index::VarId;
use crate::ir::intrinsics::Intrinsic;
use crate::IRContext;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};

// counter for unique names in SMT-LIB
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Converts an system default function in Rusmart into the corresponding SMT-LIB as a `String`.
pub fn intrinsics_to_smt(
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    id: &ExpId,
    ir: &IRContext,
    dependencies: &mut BTreeSet<String>,
    mapping_vars: &mut BTreeMap<VarId, String>,
) -> String {
    use crate::ir::intrinsics::Intrinsic::*;

    match intrinsic {
        // --- Boolean ---
        // `Boolean::from`
        BoolVal(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        // `Boolean::not`
        BoolNot { val } => {
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            format!("(not {})", v)
        }
        // `Boolean::and`
        BoolAnd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(and {} {})", l, r)
        }
        // `Boolean::or`
        BoolOr { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(or {} {})", l, r)
        }
        // `Boolean::xor`
        BoolXor { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(or (and (not {}) {}) (and {} (not {})))", l, r, l, r)
        }
        // `Boolean::implies`
        BoolImplies { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(=> {} {})", l, r)
        }

        // --- Integer ---
        // `Integer::from`
        IntVal(i) => {
            format!("{}", i)
        }
        // `Integer::lt`
        IntLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(< {} {})", l, r)
        }
        // `Integer::le`
        IntLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(<= {} {})", l, r)
        }
        // `Integer::ge`
        IntGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(>= {} {})", l, r)
        }
        // `Integer::gt`
        IntGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(> {} {})", l, r)
        }
        // `Integer::add`
        IntAdd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(+ {} {})", l, r)
        }
        // `Integer::sub`
        IntSub { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(- {} {})", l, r)
        }
        // `Integer::mul`
        IntMul { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("( {} {})", l, r)
        }
        // `Integer::div` - integer division
        IntDiv { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(div {} {})", l, r)
        }
        // `Integer::rem` - integer remainder
        IntRem { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(mod {} {})", l, r)
        }
        // --- Rational ---
        // `Rational::from`
        NumVal(i) => {
            format!("(to_real {})", i)
        }
        // `Rational::lt`
        NumLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(< {} {})", l, r)
        }
        // `Rational::le`
        NumLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(<= {} {})", l, r)
        }
        // `Rational::ge`
        NumGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(>= {} {})", l, r)
        }
        // `Rational::gt`
        NumGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(> {} {})", l, r)
        }
        // `Rational::add`
        NumAdd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(+ {} {})", l, r)
        }
        // `Rational::sub`
        NumSub { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(- {} {})", l, r)
        }
        // `Rational::mul`
        NumMul { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("( {} {})", l, r)
        }
        // `Rational::div` - rational division
        NumDiv { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(/ {} {})", l, r)
        }
        // --- Text ---
        // `Text::from`
        StrVal(s) => {
            format!("\"{}\"", s)
        }
        // `Text::lt` - lexicographic string comparison
        StrLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(simplify (str.< {} {}))", l, r) // simplify is needed for Z3 otherwise unsupported error
        }
        // `Text::le`
        StrLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(simplify (str.<= {} {}))", l, r)
        }
        // `Text::ge`
        StrGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(simplify (str.<= {} {}))", r, l)
        }
        // `Text::gt`
        StrGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(simplify (str.< {} {}))", r, l)
        }
        // --- Cloak (box) ---
        // `Cloak::shield`
        BoxShield { t, val } => {
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            dependencies.insert(format!("(declare-sort Cloak 1) ; Cloak<T>"));
            dependencies.insert(format!("(declare-fun shield ({}) (Cloak {}))", t, t));
            dependencies.insert(format!("(declare-fun reveal ((Cloak {})) {})", t, t));
            dependencies.insert(format!("(assert (forall ((x (Cloak {}))) (= (shield (reveal x)) x))) ; shield(reveal(x)) = x", t));
            dependencies.insert(format!(
                "(assert (forall ((x {})) (= (reveal (shield x)) x))) ; reveal(shield(x)) = x",
                t
            ));
            format!("(shield {})", v) // shield(x) - needs to be the same function as in the assert
        }
        // `Cloak::reveal`
        BoxReveal { t, val } => {
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            dependencies.insert(format!("(declare-sort Cloak 1) ; Cloak<T>"));
            dependencies.insert(format!("(declare-fun shield ({}) (Cloak {}))", t, t));
            dependencies.insert(format!("(declare-fun reveal ((Cloak {})) {})", t, t));
            dependencies.insert(format!("(assert (forall ((x (Cloak {}))) (= (shield (reveal x)) x))) ; shield(reveal(x)) = x", t));
            dependencies.insert(format!(
                "(assert (forall ((x {})) (= (reveal (shield x)) x))) ; reveal(shield(x)) = x",
                t
            ));
            format!("(reveal {})", v) // reveal(x) - needs to be the same function as in the assert
        }
        // --- Sequence ---
        // `Seq::empty` - (declare-const <name> (Seq <type>))
        SeqEmpty { t } => {
            for (varid, var) in exp_registry.vars.iter() {
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        // because the asssertions go on the top level, there might be the case where variables with the same names are defined in different functions (for example let a = Seq::new())
                        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                        dependencies.insert(format!("(declare-const seq_{} (Seq {}))", id, t));
                        dependencies.insert(format!(
                            "(assert (= seq_{} (as seq.empty (Seq {})))) ; seq.empty",
                            id, t
                        ));
                        mapping_vars.insert(*varid, format!("seq_{}", id));
                        break;
                    }
                }
            }
            format!("")
        }
        // `Seq::length`
        SeqLength { t: _, seq } => {
            let s = expr_to_smt(exp_registry, seq, ir, dependencies, mapping_vars);
            format!("(seq.len {})", s)
        }
        // `Seq::append`
        SeqAppend { t: _, seq, item } => {
            let s = expr_to_smt(exp_registry, seq, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, item, ir, dependencies, mapping_vars);
            format!("(seq.++ {} (seq.unit {}))", s, i)
        }
        // `Seq::at_unchecked`
        SeqAt { t: _, seq, idx } => {
            let s = expr_to_smt(exp_registry, seq, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, idx, ir, dependencies, mapping_vars);
            dependencies.insert(format!(
                "(assert (and (<= 0 {}) (< {} (seq.len {}))))",
                i, i, s
            ));
            format!("(seq.nth {} {})", s, i)
        }
        // `Seq::includes`
        SeqIncludes { t: _, seq, item } => {
            let s = expr_to_smt(exp_registry, seq, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, item, ir, dependencies, mapping_vars);
            format!("(seq.contains {} (seq.unit {}))", s, i)
        }
        // --- Set ---
        // `Set::empty` - The type constructor (Set T) is a macro for (Array T Bool).
        SetEmpty { t } => {
            for (varid, var) in exp_registry.vars.iter() {
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                        dependencies.insert(format!("(declare-const set_{} (Set {}))", id, t));
                        dependencies.insert(format!(
                            "(assert (forall ((x {})) (= (select set_{} x) false))) ; set.empty",
                            t, id
                        ));
                        mapping_vars.insert(*varid, format!("set_{}", id));
                        // sets do not have a length in SMT-LIB, so we need a function
                        dependencies.insert(format!("(declare-fun len ((Set {})) Int)", t));
                        dependencies.insert(format!(
                            "(assert (= (len set_{}) 0)) ; length of empty set is 0",
                            id
                        ));
                        break;
                    }
                }
            }
            format!("")
        }
        // `Set::length`
        SetLength { t, set } => {
            let s = expr_to_smt(exp_registry, set, ir, dependencies, mapping_vars);
            // the definition should already be made but just to be sure
            dependencies.insert(format!("(declare-fun len ((Set {})) Int)", t));
            format!("(len {})", s)
        }
        SetInsert { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, item, ir, dependencies, mapping_vars);
            format!("(store {} {} true)", s, i)
        }
        SetRemove { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, item, ir, dependencies, mapping_vars);
            format!("(store {} {} false)", s, i)
        }
        SetContains { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir, dependencies, mapping_vars);
            let i = expr_to_smt(exp_registry, item, ir, dependencies, mapping_vars);
            format!("select {} {}", s, i)
        }
        // --- Map ---
        MapEmpty { k, v } => {
            for (varid, var) in exp_registry.vars.iter() {
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                        dependencies
                            .insert(format!("(declare-const map_{} (Array {} {}))", id, k, v));
                        dependencies.insert(sort_not_present(v, ir));
                        dependencies.insert(format!(
                            "(assert (forall ((x {})) (= (select map_{} x) not_present_{}))) ; array.empty",
                            k,
                            id,
                            v
                        ));
                        mapping_vars.insert(*varid, format!("map_{}", id));
                        // sets do not have a length in SMT-LIB, so we need a function
                        dependencies
                            .insert(format!("(declare-fun len_map ((Array {} {})) Int)", k, v));
                        dependencies.insert(format!(
                            "(assert (= (len_map map_{}) 0)) ; length of empty map is 0",
                            id
                        ));
                        break;
                    }
                }
            }
            format!("")
        }
        MapLength { k, v, map } => {
            let s = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            // the definition should already be made but just to be sure
            dependencies.insert(format!("(declare-fun len_map ((Array {} {})) Int)", k, v));
            format!("(len_map {})", s)
        }
        MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            let m = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let k = expr_to_smt(exp_registry, key, ir, dependencies, mapping_vars);
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            format!("(store {} {} {})", m, k, v)
        }
        MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let k = expr_to_smt(exp_registry, key, ir, dependencies, mapping_vars);
            format!("(select {} {})", m, k)
        }
        MapDel { k: _, v, map, key } => {
            let m = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let k = expr_to_smt(exp_registry, key, ir, dependencies, mapping_vars);
            format!("(store {} {} not_present_{})", m, k, v)
        }
        MapContainsKey {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let k = expr_to_smt(exp_registry, key, ir, dependencies, mapping_vars);
            format!("(select {} {})", m, k)
        }
        // --- Error ---
        ErrFresh => {
            format!("(error \"something went wrong in error fresh\")")
        }
        ErrMerge { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!(
                "(error \"something went wrong in error merge between {} {}\")",
                l, r
            )
        }
        // --- Generic eq/ne ---
        SmtEq { t: _, lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(= {} {})", l, r)
        }
        SmtNe { t: _, lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            // (distinct ...) is equivalent to != in SMT but distinct can have more than two args
            // distinct a b c means that all three are mutually different
            format!("(distinct {} {})", l, r)
        }
    }
}

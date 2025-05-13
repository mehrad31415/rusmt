//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::backend::z3::sort::sort_to_smt_name;
use crate::ir::exp::ExpRegistry;
use crate::ir::exp::VarKind;
use crate::ir::index::ExpId;
use crate::ir::index::VarId;
use crate::ir::intrinsics::Intrinsic;
use crate::IRContext;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// counter for unique names in SMT-LIB
pub static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Converts a system default function in Rusmart into the corresponding SMT-LIB as a `String`.
pub fn intrinsics_to_smt(
    name: String,
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    id: &ExpId,
    ir: &IRContext,
    dependencies: &mut Vec<String>, // we use vec because the order is important
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
            let v = expr_to_smt(
                name.clone(),
                exp_registry,
                val,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(not {})", v)
        }
        // `Boolean::and`
        BoolAnd { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(and {} {})", l, r)
        }
        // `Boolean::or`
        BoolOr { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(or {} {})", l, r)
        }
        // `Boolean::xor`
        BoolXor { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(or (and (not {}) {}) (and {} (not {})))", l, r, l, r)
        }
        // `Boolean::implies`
        BoolImplies { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(=> {} {})", l, r)
        }

        // --- Integer ---
        // `Integer::from`
        IntVal(i) => {
            format!("{}", i)
        }
        // `Integer::lt`
        IntLt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(< {} {})", l, r)
        }
        // `Integer::le`
        IntLe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(<= {} {})", l, r)
        }
        // `Integer::ge`
        IntGe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(>= {} {})", l, r)
        }
        // `Integer::gt`
        IntGt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(> {} {})", l, r)
        }
        // `Integer::add`
        IntAdd { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(+ {} {})", l, r)
        }
        // `Integer::sub`
        IntSub { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(- {} {})", l, r)
        }
        // `Integer::mul`
        IntMul { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(* {} {})", l, r)
        }
        // `Integer::div` - integer division
        IntDiv { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(div {} {})", l, r)
        }
        // `Integer::rem` - integer remainder
        IntRem { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(mod {} {})", l, r)
        }
        // --- Rational ---
        // `Rational::from`
        NumVal(i) => {
            format!("(to_real {})", i)
        }
        // `Rational::lt`
        NumLt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(< {} {})", l, r)
        }
        // `Rational::le`
        NumLe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(<= {} {})", l, r)
        }
        // `Rational::ge`
        NumGe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(>= {} {})", l, r)
        }
        // `Rational::gt`
        NumGt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(> {} {})", l, r)
        }
        // `Rational::add`
        NumAdd { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(+ {} {})", l, r)
        }
        // `Rational::sub`
        NumSub { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(- {} {})", l, r)
        }
        // `Rational::mul`
        NumMul { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(* {} {})", l, r)
        }
        // `Rational::div` - rational division
        NumDiv { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(/ {} {})", l, r)
        }
        // --- Text ---
        // `Text::from`
        StrVal(s) => {
            format!("{}", s)
        }
        // `Text::lt` - lexicographic string comparison
        StrLt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(str.< {} {})", l, r)
        }
        // `Text::le`
        StrLe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(str.<= {} {})", l, r)
        }
        // `Text::ge`
        StrGe { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(str.<= {} {})", r, l)
        }
        // `Text::gt`
        StrGt { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(str.< {} {})", r, l)
        }
        // --- Cloak (box) ---
        // `Cloak::shield`
        BoxShield { t, val } => {
            let v = expr_to_smt(
                name.clone(),
                exp_registry,
                val,
                ir,
                dependencies,
                mapping_vars,
            );
            let decl = format!("(declare-sort Cloak 1) ; Cloak<T>");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun shield ({}) (Cloak {}))",
                sort_to_smt(t, ir, None),
                sort_to_smt(t, ir, None)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun reveal ((Cloak {})) {})",
                sort_to_smt(t, ir, None),
                sort_to_smt(t, ir, None)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!("(assert (= (reveal (shield {})) {}))", v, v);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("(shield {})", v) // shield(x) - needs to be the same function as in the assert
        }
        // `Cloak::reveal`
        BoxReveal { t, val } => {
            let v = expr_to_smt(
                name.clone(),
                exp_registry,
                val,
                ir,
                dependencies,
                mapping_vars,
            );
            let decl = format!("(declare-sort Cloak 1) ; Cloak<T>");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun shield ({}) (Cloak {}))",
                sort_to_smt(t, ir, None),
                sort_to_smt(t, ir, None)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun reveal ((Cloak {})) {})",
                sort_to_smt(t, ir, None),
                sort_to_smt(t, ir, None)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!("(assert (= (shield (reveal {})) {}))", v, v);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("(reveal {})", v) // reveal(x) - needs to be the same function as in the assert
        }
        // --- Sequence ---
        // `Seq::empty` - (declare-const <name> (Seq <type>))
        SeqEmpty { t } => {
            for (varid, var) in exp_registry.vars.iter() {
                // let x = Seq::empty() is the only place where we have a Seq::empty()
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        // because the asssertions go on the top level, there might be the case where variables with the same names are defined in different functions (for example let a = Seq::new())
                        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
                        let sort = sort_to_smt(t, ir, None);
                        let decl = format!(
                            "(define-fun seq_{id} () (Seq {sort}) (as seq.empty (Seq {sort})))"
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        // inside the function body we need to use the name seq_<id> instead of the original name
                        mapping_vars.insert(*varid, format!("seq_{id}"));
                        return format!("seq_{}", id);
                    }
                } else {
                    panic!("Seq::new() is not a bound variable");
                }
            }
            panic!("no Seq::new() found");
        }
        // `Seq::length`
        SeqLength { t: _, seq } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                seq,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(seq.len {})", s)
        }
        // `Seq::append`
        SeqAppend { t: _, seq, item } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                seq,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                item,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(seq.++ {} (seq.unit {}))", s, i)
        }
        // `Seq::at_unchecked`
        SeqAt { t: _, seq, idx } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                seq,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                idx,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(seq.nth {} {})", s, i)
        }
        // `Seq::includes`
        SeqIncludes { t: _, seq, item } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                seq,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                item,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(seq.contains {} (seq.unit {}))", s, i)
        }
        // --- Set ---
        // `Set::empty` - The type constructor (Set T) is a macro for (Array T Bool).
        SetEmpty { t } => {
            for (varid, var) in exp_registry.vars.iter() {
                // a Set::new() is always used like let x = Set::new() in the code
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
                        let sort = sort_to_smt(t, ir, None);
                        let set_ty = format!("(Set {sort})");

                        let decl = format!(
                            "(define-fun set_{id} () {set_ty} \
                              ((as const {set_ty}) false))"
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        mapping_vars.insert(*varid, format!("set_{}", id));
                        // sets do not have a length in SMT-LIB, so we need a function
                        let decl =
                            format!("(declare-fun len ((Set {})) Int)", sort_to_smt(t, ir, None));
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl =
                            format!("(assert (= (len set_{}) 0)) ; length of empty set is 0", id);
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        return format!("set_{}", id);
                    }
                } else {
                    panic!("Set::new() is not a bound variable");
                }
            }
            panic!("no Set::new() found");
        }
        // `Set::length`
        SetLength { t, set } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                set,
                ir,
                dependencies,
                mapping_vars,
            );
            let decl = format!("(declare-fun len ((Set {})) Int)", sort_to_smt(t, ir, None));
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {}))
                (=> (not (select {} i)) (= (len (store {} i true)) (+ (len {}) 1)))))",
                sort_to_smt(t, ir, None),
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {}))
                (=> (select {} i) (= (len (store {} i true)) (len {})))))",
                sort_to_smt(t, ir, None),
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {}))
                (=> (select {} i) (= (len (store {} i false)) (- (len {}) 1)))))",
                sort_to_smt(t, ir, None),
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {}))
                (=> (not (select {} i)) (= (len (store {} i false)) (len {})))))",
                sort_to_smt(t, ir, None),
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("(len {})", s)
        }
        SetInsert { t: _, set, item } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                set,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                item,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(store {} {} true)", s, i)
        }
        SetRemove { t: _, set, item } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                set,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                item,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(store {} {} false)", s, i)
        }
        SetContains { t: _, set, item } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                set,
                ir,
                dependencies,
                mapping_vars,
            );
            let i = expr_to_smt(
                name.clone(),
                exp_registry,
                item,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(select {} {})", s, i)
        }
        // --- Map ---
        MapEmpty { k, v } => {
            for (varid, var) in exp_registry.vars.iter() {
                // a Map::new() is always used like let x = Map::new() in the code
                if let VarKind::Bound { bind: expid } = var.kind {
                    // get the variable id
                    if id == &expid {
                        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
                        let k_sort = sort_to_smt(k, ir, None);
                        let v_sort = sort_to_smt(v, ir, None);
                        let v_name = sort_to_smt_name(v, ir);

                        let decl = format!("(declare-const not_present_{v_name} {v_sort})");
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        let map_ty = format!("(Array {k_sort} {v_sort})");
                        let decl = format!(
                            "(define-fun map_{id} () {map_ty} \
                               ((as const {map_ty}) not_present_{v_name}))"
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        // inside the function body we need to use the name map_<id> instead of the original name
                        mapping_vars.insert(*varid, format!("map_{}", id));
                        // arrays do not have a length in SMT-LIB, so we need a function
                        // also we need some semantics for the length of the map (even though a full definition is not possible)
                        let decl = format!("(declare-fun len_map ({map_ty}) Int)");
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
                        return format!("map_{}", id);
                    }
                } else {
                    panic!("Map::new() is not a bound variable");
                }
            }
            panic!("no Map::new() found");
        }
        MapLength { k, v, map } => {
            let s = expr_to_smt(
                name.clone(),
                exp_registry,
                map,
                ir,
                dependencies,
                mapping_vars,
            );
            let k_sort = sort_to_smt(k, ir, None);
            let v_sort = sort_to_smt(v, ir, None);
            let v_name = sort_to_smt_name(v, ir);

            let decl = format!("(declare-const not_present_{v_name} {v_sort})");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            let map_ty = format!("(Array {k_sort} {v_sort})");
            let decl = format!("(declare-fun len_map ({map_ty}) Int)");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(define-fun in_map ((m {map_ty}) (i {})) Bool
                (not (= (select m i) not_present_{})))",
                k_sort, v_name
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            let decl = format!(
                "(assert (forall ((i {k_sort}) (v {v_sort}))
            (=> (not (in_map {} i)) (= (len_map (store {} i v)) (+ (len_map {}) 1)))))",
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {k_sort}) (v {v_sort}))
                (=> (in_map {} i) (= (len_map (store {} i v)) (len_map {})))))",
                s,
                s,
                s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {k_sort}))
                        (=> (in_map {} i)
                            (= (len_map (store {} i not_present_{v_name})) (- (len_map {}) 1)))))",
                s, s, s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((i {k_sort}))
                        (=> (not (in_map {} i))
                            (= (len_map (store {} i not_present_{v_name})) (len_map {})))))",
                s, s, s
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }

            format!("(len_map {})", s)
        }
        MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            let m = expr_to_smt(
                name.clone(),
                exp_registry,
                map,
                ir,
                dependencies,
                mapping_vars,
            );
            let k = expr_to_smt(
                name.clone(),
                exp_registry,
                key,
                ir,
                dependencies,
                mapping_vars,
            );
            let v = expr_to_smt(
                name.clone(),
                exp_registry,
                val,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(store {} {} {})", m, k, v)
        }
        MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(
                name.clone(),
                exp_registry,
                map,
                ir,
                dependencies,
                mapping_vars,
            );
            let k = expr_to_smt(
                name.clone(),
                exp_registry,
                key,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(select {} {})", m, k)
        }
        MapDel { k: _, v, map, key } => {
            let m = expr_to_smt(
                name.clone(),
                exp_registry,
                map,
                ir,
                dependencies,
                mapping_vars,
            );
            let k = expr_to_smt(
                name.clone(),
                exp_registry,
                key,
                ir,
                dependencies,
                mapping_vars,
            );
            format!(
                "(store {} {} not_present_{})",
                m,
                k,
                sort_to_smt_name(v, ir)
            )
        }
        MapContainsKey { k: _, v, map, key } => {
            let m = expr_to_smt(
                name.clone(),
                exp_registry,
                map,
                ir,
                dependencies,
                mapping_vars,
            );
            let k = expr_to_smt(
                name.clone(),
                exp_registry,
                key,
                ir,
                dependencies,
                mapping_vars,
            );
            format!(
                "(distinct (select {} {}) not_present_{})",
                m,
                k,
                sort_to_smt_name(v, ir)
            )
        }
        // --- Error ---
        ErrFresh => {
            format!("error fresh")
        }
        ErrMerge { lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(error merge between {} {}\")", l, r)
        }
        // --- Generic eq/ne ---
        SmtEq { t: _, lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            format!("(= {} {})", l, r)
        }
        SmtNe { t: _, lhs, rhs } => {
            let l = expr_to_smt(
                name.clone(),
                exp_registry,
                lhs,
                ir,
                dependencies,
                mapping_vars,
            );
            let r = expr_to_smt(
                name.clone(),
                exp_registry,
                rhs,
                ir,
                dependencies,
                mapping_vars,
            );
            // (distinct ...) is equivalent to != in SMT but distinct can have more than two args
            // distinct a b c means that all three are mutually different
            format!("(distinct {} {})", l, r)
        }
    }
}

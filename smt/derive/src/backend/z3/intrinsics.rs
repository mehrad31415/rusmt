//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::exp::ExpRegistry;
use crate::ir::exp::VarKind;
use crate::ir::index::ExpId;
use crate::ir::index::VarId;
use crate::ir::intrinsics::Intrinsic;
use crate::IRContext;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// counter for unique names in SMT-LIB
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Converts a system default function in Rusmart into the corresponding SMT-LIB as a `String`.
pub fn intrinsics_to_smt(
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
            format!("(* {} {})", l, r)
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
            format!("(* {} {})", l, r)
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
            format!("(str.< {} {})", l, r)
        }
        // `Text::le`
        StrLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(str.<= {} {})", l, r)
        }
        // `Text::ge`
        StrGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(str.<= {} {})", r, l)
        }
        // `Text::gt`
        StrGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(str.< {} {})", r, l)
        }
        // --- Cloak (box) ---
        // `Cloak::shield`
        BoxShield { t, val } => {
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            let decl = format!("(declare-sort Cloak 1) ; Cloak<T>");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun shield ({}) (Cloak {}))",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun reveal ((Cloak {})) {})",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!("(assert (forall ((x (Cloak {}))) (= (shield (reveal x)) x))) ; shield(reveal(x)) = x", sort_to_smt(t,ir));
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((x {})) (= (reveal (shield x)) x))) ; reveal(shield(x)) = x",
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            format!("(shield {})", v) // shield(x) - needs to be the same function as in the assert
        }
        // `Cloak::reveal`
        BoxReveal { t, val } => {
            let v = expr_to_smt(exp_registry, val, ir, dependencies, mapping_vars);
            let decl = format!("(declare-sort Cloak 1) ; Cloak<T>");
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun shield ({}) (Cloak {}))",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(declare-fun reveal ((Cloak {})) {})",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!("(assert (forall ((x (Cloak {}))) (= (shield (reveal x)) x))) ; shield(reveal(x)) = x", sort_to_smt(t,ir));
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((x {})) (= (reveal (shield x)) x))) ; reveal(shield(x)) = x",
                sort_to_smt(t, ir)
            );
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
                        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                        let decl =
                            format!("(declare-const seq_{} (Seq {}))", id, sort_to_smt(t, ir));
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (= seq_{} (as seq.empty (Seq {})))) ; seq.empty",
                            id,
                            sort_to_smt(t, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        // inside the function body we need to use the name seq_<id> instead of the original name
                        mapping_vars.insert(*varid, format!("seq_{}", id));
                        break;
                    }
                }
            }
            format!("")
        }
        // `Seq::length`
        SeqLength { t, seq } => {
            let a = exp_registry.lookup_exp(*seq);
            println!("SAAAA: {:?}", a);
            let b = exp_registry.lookup_var(VarId { index: 0 });
            println!("SABBB: {:?}", b);
            let s = expr_to_smt(exp_registry, seq, ir, dependencies, mapping_vars);
            let c = exp_registry.lookup_exp(ExpId { index: 0 });
            println!("SACCC: {:?}", c);
            println!("seqXXX: {}", s);
            println!("seqXXXSORT: {}", sort_to_smt(t, ir));
            println!("seqXXXseq: {}", seq);
            println!("seqXXXID: {}", id);
            println!("intrinsic: {:?}", intrinsic);
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
                // a Set::new() is always used like let x = Set::new() in the code
                if let VarKind::Bound { bind: expid } = var.kind {
                    if id == &expid {
                        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
                        let decl = format!("(declare-const set_{} (Set {}))", id, t);
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((x {})) (= (select set_{} x) false))) ; set.empty",
                            sort_to_smt(t, ir),
                            id
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        mapping_vars.insert(*varid, format!("set_{}", id));
                        // sets do not have a length in SMT-LIB, so we need a function
                        let decl = format!("(declare-fun len ((Set {})) Int)", t);
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl =
                            format!("(assert (= (len set_{}) 0)) ; length of empty set is 0", id);
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((m (Set {})) (i {}))
                            (=> (not (select m i)) (= (len (store m i true)) (+ (len m) 1))))) ; length of set after adding an element",
                            sort_to_smt(t, ir),
                            sort_to_smt(t, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((m (Set {})) (i {}))
                            (=> (select m i) (= (len (store m i true)) (len m)))))",
                            sort_to_smt(t, ir),
                            sort_to_smt(t, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        break;
                    }
                }
            }
            format!("")
        }
        // `Set::length`
        SetLength { t, set } => {
            let s = expr_to_smt(exp_registry, set, ir, dependencies, mapping_vars);
            let decl = format!("(declare-fun len ((Set {})) Int)", t);
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (not (select m i)) (= (len (store m i true)) (+ (len m) 1)))))",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Set {})) (i {}))
                (=> (select m i) (= (len (store m i true)) (len m)))))",
                sort_to_smt(t, ir),
                sort_to_smt(t, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
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
                // a Map::new() is always used like let x = Map::new() in the code
                if let VarKind::Bound { bind: expid } = var.kind {
                    // get the variable id
                    if id == &expid {
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
                            sort_to_smt(v, ir),
                            sort_to_smt(v, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((x {})) (= (select map_{} x) not_present_{}))) ; array.empty",
                            sort_to_smt(k,ir),
                            id,
                            sort_to_smt(v, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }

                        // inside the function body we need to use the name map_<id> instead of the original name
                        mapping_vars.insert(*varid, format!("map_{}", id));
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
                            sort_to_smt(v, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((m (Array {} {})) (i {}) (v {}))
                        (=> (not (in_map m i)) (= (len_map (store m i v)) (+ (len_map m) 1)))))",
                            sort_to_smt(k, ir),
                            sort_to_smt(v, ir),
                            sort_to_smt(k, ir),
                            sort_to_smt(v, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        let decl = format!(
                            "(assert (forall ((m (Array {} {})) (i {}) (v {}))
                            (=> (in_map m i) (= (len_map (store m i v)) (len_map m)))))",
                            sort_to_smt(k, ir),
                            sort_to_smt(v, ir),
                            sort_to_smt(k, ir),
                            sort_to_smt(v, ir)
                        );
                        if !dependencies.contains(&decl) {
                            dependencies.push(decl);
                        }
                        break;
                    }
                }
            }
            format!("")
        }
        MapLength { k, v, map } => {
            let s = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let decl = format!(
                "(declare-fun len_map ((Array {} {})) Int)",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir)
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
                sort_to_smt(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}) (v {}))
                (=> (not (in_map m i)) (= (len_map (store m i v)) (+ (len_map m) 1)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt(v, ir)
            );
            if !dependencies.contains(&decl) {
                dependencies.push(decl);
            }
            let decl = format!(
                "(assert (forall ((m (Array {} {})) (i {}) (v {}))
                (=> (in_map m i) (= (len_map (store m i v)) (len_map m)))))",
                sort_to_smt(k, ir),
                sort_to_smt(v, ir),
                sort_to_smt(k, ir),
                sort_to_smt(v, ir)
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
            format!("(store {} {} not_present_{})", m, k, sort_to_smt(v, ir))
        }
        MapContainsKey { k: _, v, map, key } => {
            let m = expr_to_smt(exp_registry, map, ir, dependencies, mapping_vars);
            let k = expr_to_smt(exp_registry, key, ir, dependencies, mapping_vars);
            format!("(distinct (select {} {}) not_present_{})", m, k, v)
        }
        // --- Error ---
        ErrFresh => {
            format!("error fresh")
        }
        ErrMerge { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir, dependencies, mapping_vars);
            let r = expr_to_smt(exp_registry, rhs, ir, dependencies, mapping_vars);
            format!("(error merge between {} {}\")", l, r)
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

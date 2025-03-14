//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::expr_to_smt;
use crate::ir::exp::ExpRegistry;
use crate::ir::intrinsics::Intrinsic;
use crate::IRContext;

/// Converts an system default function in Rusmart into the corresponding SMT-LIB as a `String`.
pub fn intrinsics_to_smt(
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    ir: &IRContext,
) -> String {
    match intrinsic {
        // --- Boolean ---
        Intrinsic::BoolVal(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Intrinsic::BoolNot { val } => {
            let v = expr_to_smt(exp_registry, val, ir);
            format!("(not {})", v)
        }
        Intrinsic::BoolAnd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(and {} {})", l, r)
        }
        Intrinsic::BoolOr { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(or {} {})", l, r)
        }
        Intrinsic::BoolXor { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            //? is xor implemented in all solvers?
            format!("(or (and (not {}) {}) (and {} (not {})))", l, r, l, r)
        }
        Intrinsic::BoolImplies { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(=> {} {})", l, r)
        }

        // --- Integer ---
        Intrinsic::IntVal(i) => {
            format!("{}", i)
        }
        Intrinsic::IntLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(< {} {})", l, r)
        }
        Intrinsic::IntLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(<= {} {})", l, r)
        }
        Intrinsic::IntGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(>= {} {})", l, r)
        }
        Intrinsic::IntGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(> {} {})", l, r)
        }
        Intrinsic::IntAdd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(+ {} {})", l, r)
        }
        Intrinsic::IntSub { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(- {} {})", l, r)
        }
        Intrinsic::IntMul { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("( {} {})", l, r)
        }
        Intrinsic::IntDiv { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(div {} {})", l, r)
        }
        Intrinsic::IntRem { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(mod {} {})", l, r)
        }

        // --- Rational ---
        Intrinsic::NumVal(i) => {
            format!("(to_real {})", i)
        }
        Intrinsic::NumLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(< {} {})", l, r)
        }
        Intrinsic::NumLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(<= {} {})", l, r)
        }
        Intrinsic::NumGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(>= {} {})", l, r)
        }
        Intrinsic::NumGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(> {} {})", l, r)
        }
        Intrinsic::NumAdd { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(+ {} {})", l, r)
        }
        Intrinsic::NumSub { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(- {} {})", l, r)
        }
        Intrinsic::NumMul { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("( {} {})", l, r)
        }
        Intrinsic::NumDiv { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(/ {} {})", l, r)
        }

        // --- Text ---
        Intrinsic::StrVal(s) => {
            format!("\"{}\"", s)
        }
        Intrinsic::StrLt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            // There's no standard "string <" in SMT-LIB
            format!("(strLT {} {})", l, r)
        }
        Intrinsic::StrLe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(strLE {} {})", l, r)
        }
        Intrinsic::StrGe { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(strGE {} {})", l, r)
        }
        Intrinsic::StrGt { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(strGT {} {})", l, r)
        }

        // --- Cloak (box) ---
        Intrinsic::BoxShield { t, val } => {
            // Custom function:
            let s = expr_to_smt(exp_registry, val, ir);
            panic!("BoxShield not implemented yet, please use a custom function",);
        }
        Intrinsic::BoxReveal { t: _, val } => {
            let r = expr_to_smt(exp_registry, val, ir);
            panic!("BoxReveal not implemented yet, please use a custom function",);
        }

        // --- Sequence ---
        Intrinsic::SeqEmpty { t } => {
            format!("(declare-const {}_{} (Seq {}))", "seq_empty", t, t)
        }
        Intrinsic::SeqLength { t: _, seq } => {
            let s = expr_to_smt(exp_registry, seq, ir);
            format!("(seqLength {})", s)
        }
        Intrinsic::SeqAppend { t: _, seq, item } => {
            let s = expr_to_smt(exp_registry, seq, ir);
            let i = expr_to_smt(exp_registry, item, ir);
            format!("(seqAppend {} {})", s, i)
        }
        Intrinsic::SeqAt { t: _, seq, idx } => {
            let s = expr_to_smt(exp_registry, seq, ir);
            let i = expr_to_smt(exp_registry, idx, ir);
            format!("(seqAt {} {})", s, i)
        }
        Intrinsic::SeqIncludes { t: _, seq, item } => {
            let s = expr_to_smt(exp_registry, seq, ir);
            let i = expr_to_smt(exp_registry, item, ir);
            format!("(seqIncludes {} {})", s, i)
        }

        // --- Set ---
        Intrinsic::SetEmpty { t: _ } => "(setEmpty)".to_string(),
        Intrinsic::SetLength { t: _, set } => {
            let s = expr_to_smt(exp_registry, set, ir);
            format!("(setLength {})", s)
        }
        Intrinsic::SetInsert { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir);
            let i = expr_to_smt(exp_registry, item, ir);
            format!("(setInsert {} {})", s, i)
        }
        Intrinsic::SetRemove { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir);
            let i = expr_to_smt(exp_registry, item, ir);
            format!("(setRemove {} {})", s, i)
        }
        Intrinsic::SetContains { t: _, set, item } => {
            let s = expr_to_smt(exp_registry, set, ir);
            let i = expr_to_smt(exp_registry, item, ir);
            format!("(setContains {} {})", s, i)
        }

        // --- Map ---
        Intrinsic::MapEmpty { k: _, v: _ } => "(mapEmpty)".to_string(),
        Intrinsic::MapLength { k: _, v: _, map } => {
            let m = expr_to_smt(exp_registry, map, ir);
            format!("(mapLength {})", m)
        }
        Intrinsic::MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            let m = expr_to_smt(exp_registry, map, ir);
            let k = expr_to_smt(exp_registry, key, ir);
            let v = expr_to_smt(exp_registry, val, ir);
            format!("(mapPut {} {} {})", m, k, v)
        }
        Intrinsic::MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(exp_registry, map, ir);
            let k = expr_to_smt(exp_registry, key, ir);
            format!("(mapGet {} {})", m, k)
        }
        Intrinsic::MapDel {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(exp_registry, map, ir);
            let k = expr_to_smt(exp_registry, key, ir);
            format!("(mapDel {} {})", m, k)
        }
        Intrinsic::MapContainsKey {
            k: _,
            v: _,
            map,
            key,
        } => {
            let m = expr_to_smt(exp_registry, map, ir);
            let k = expr_to_smt(exp_registry, key, ir);
            format!("(mapContainsKey {} {})", m, k)
        }

        // --- Error ---
        Intrinsic::ErrFresh => {
            // Something custom, e.g. a fresh error symbol
            "(errFresh)".to_string()
        }
        Intrinsic::ErrMerge { lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(errMerge {} {})", l, r)
        }

        // --- Generic eq/ne ---
        Intrinsic::SmtEq { t: _, lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            format!("(= {} {})", l, r)
        }
        Intrinsic::SmtNe { t: _, lhs, rhs } => {
            let l = expr_to_smt(exp_registry, lhs, ir);
            let r = expr_to_smt(exp_registry, rhs, ir);
            // (distinct ...) is a common way to express != in SMT
            format!("(distinct {} {})", l, r)
        }
    }
}

//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::format_expression;
use crate::backend::z3::fun::format_sort_for_fn;
use crate::ir::ctxt::IRContext;
use crate::ir::exp::ExpRegistry;
use crate::ir::index::UsrFunId;
use crate::ir::intrinsics::Intrinsic;
use crate::ir::sort::Sort;
use std::collections::BTreeSet;

/// Convert an intrinsic operation to SMT-LIB string format.
pub fn format_intrinsic(
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    ir: &IRContext,
    param_names: &std::collections::HashSet<String>,
    scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    // Helper to format sub-expressions
    let fmt = |id| format_expression(exp_registry, id, ir, param_names, scc_fids);

    match intrinsic {
        // --- Boolean Operations ---
        Intrinsic::BoolVal(b) => b.to_string(),
        Intrinsic::BoolNot { val } => format!("(not {})", fmt(*val)),
        Intrinsic::BoolAnd { lhs, rhs } => format!("(and {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolOr { lhs, rhs } => format!("(or {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolXor { lhs, rhs } => format!("(xor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolNand { lhs, rhs } => format!("(not (and {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolNor { lhs, rhs } => format!("(not (or {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolXnor { lhs, rhs } => format!("(not (xor {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolImplies { lhs, rhs } => format!("(=> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolIff { lhs, rhs } => format!("(= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolIte { cond, then, else_ } => {
            format!("(ite {} {} {})", fmt(*cond), fmt(*then), fmt(*else_))
        }

        // --- Integer Operations ---
        Intrinsic::IntVal(n) => n.to_string(),
        Intrinsic::IntNeg { val } => format!("(- {})", fmt(*val)),
        Intrinsic::IntAdd { lhs, rhs } => format!("(+ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntSub { lhs, rhs } => format!("(- {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntMul { lhs, rhs } => format!("(* {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntDiv { lhs, rhs } => format!("(div {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntDivTrunc { lhs, rhs } => {
            let l = fmt(*lhs);
            let r = fmt(*rhs);
            // C-style truncation: (ite (>= n 0) (div n d) (- (div (- n) d)))
            format!("(ite (>= {l} 0) (div {l} {r}) (- (div (- {l}) {r})))")
        }
        Intrinsic::IntMod { lhs, rhs } => format!("(mod {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntRem { lhs, rhs } => format!("(rem {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntPow { base, exp } => format!("(^ {} {})", fmt(*base), fmt(*exp)),
        Intrinsic::IntAbs { val } => format!("(abs {})", fmt(*val)),
        Intrinsic::IntDivides { lhs, rhs } => format!("(= (mod {} {}) 0)", fmt(*rhs), fmt(*lhs)),
        Intrinsic::IntLt { lhs, rhs } => format!("(< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntLe { lhs, rhs } => format!("(<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntGt { lhs, rhs } => format!("(> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntGe { lhs, rhs } => format!("(>= {} {})", fmt(*lhs), fmt(*rhs)),

        // --- Integer Conversions ---
        Intrinsic::IntToReal { val } => format!("(to_real {})", fmt(*val)),
        Intrinsic::IntToI32 { val } => format!("((_ int2bv 32) {})", fmt(*val)),
        Intrinsic::IntToI64 { val } => format!("((_ int2bv 64) {})", fmt(*val)),
        Intrinsic::IntToU32 { val } => format!("((_ int2bv 32) {})", fmt(*val)),
        Intrinsic::IntToU64 { val } => format!("((_ int2bv 64) {})", fmt(*val)),
        Intrinsic::IntToF32 { val } => format!("((_ to_fp 8 24) RNE (to_real {}))", fmt(*val)),
        Intrinsic::IntToF64 { val } => format!("((_ to_fp 11 53) RNE (to_real {}))", fmt(*val)),

        // --- Integer Parsing ---
        Intrinsic::IntFromHex { val } | Intrinsic::IntFromOct { val } | Intrinsic::IntFromBin { val } => {
            format!("(str.to_int {})", fmt(*val))
        }

        // --- Integer Range Checks ---
        Intrinsic::IntIsGtI64Max { val } => format!("(> {} 9223372036854775807)", fmt(*val)),
        Intrinsic::IntIsLtI64Min { val } => format!("(< {} (- 9223372036854775808))", fmt(*val)),
        Intrinsic::IntIsGtU64Max { val } => format!("(> {} 18446744073709551615)", fmt(*val)),
        Intrinsic::IntIsLtU64Min { val } => format!("(< {} 0)", fmt(*val)),
        Intrinsic::IntIsLtI32Min { val } => format!("(< {} (- 2147483648))", fmt(*val)),
        Intrinsic::IntIsGtI32Max { val } => format!("(> {} 2147483647)", fmt(*val)),
        Intrinsic::IntIsLtU32Min { val } => format!("(< {} 0)", fmt(*val)),
        Intrinsic::IntIsGtU32Max { val } => format!("(> {} 4294967295)", fmt(*val)),

        // --- Real Operations ---
        Intrinsic::RealVal(r) => format!("(/ {} {})", r.numer(), r.denom()),
        Intrinsic::RealNeg { val } => format!("(- {})", fmt(*val)),
        Intrinsic::RealAdd { lhs, rhs } => format!("(+ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealSub { lhs, rhs } => format!("(- {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealMul { lhs, rhs } => format!("(* {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealDiv { lhs, rhs } => format!("(/ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealPow { base, exp } => format!("(^ {} {})", fmt(*base), fmt(*exp)),
        Intrinsic::RealAbs { val } => format!("(abs {})", fmt(*val)),
        Intrinsic::RealRound { val } => format!("(to_real (to_int (+ {} 0.5)))", fmt(*val)),
        Intrinsic::RealFloor { val } => format!("(to_real (to_int {}))", fmt(*val)),
        Intrinsic::RealCeil { val } => format!("(- (to_real (to_int (- {}))))", fmt(*val)),
        Intrinsic::RealIsInt { val } => format!("(is_int {})", fmt(*val)),
        Intrinsic::RealLt { lhs, rhs } => format!("(< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealLe { lhs, rhs } => format!("(<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealGt { lhs, rhs } => format!("(> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealGe { lhs, rhs } => format!("(>= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealToInt { val } => format!("(to_int {})", fmt(*val)),
        Intrinsic::RealToF32 { val } => format!("((_ to_fp 8 24) RNE {})", fmt(*val)),
        Intrinsic::RealToF64 { val } => format!("((_ to_fp 11 53) RNE {})", fmt(*val)),

        // --- String Operations ---
        Intrinsic::StrVal(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        Intrinsic::StrNew => "\"\"".to_string(),
        Intrinsic::StrLen { seq } => format!("(str.len {})", fmt(*seq)),
        Intrinsic::StrConcat { lhs, rhs } => format!("(str.++ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrAt { seq, idx } => format!("(str.at {} {})", fmt(*seq), fmt(*idx)),
        Intrinsic::StrIndexOf { seq, sub, offset } => format!("(str.indexof {} {} {})", fmt(*seq), fmt(*sub), fmt(*offset)),
        Intrinsic::StrIndexOfDefault { seq, sub } => format!("(str.indexof {} {} 0)", fmt(*seq), fmt(*sub)),
        Intrinsic::StrSubstr { seq, start, len } => format!("(str.substr {} {} {})", fmt(*seq), fmt(*start), fmt(*len)),
        Intrinsic::StrIsEmpty { seq } => format!("(= (str.len {}) 0)", fmt(*seq)),
        Intrinsic::StrContains { seq, item } => format!("(str.contains {} {})", fmt(*seq), fmt(*item)),
        Intrinsic::StrStartsWith { seq, item } => format!("(str.prefixof {} {})", fmt(*item), fmt(*seq)),
        Intrinsic::StrEndsWith { seq, item } => format!("(str.suffixof {} {})", fmt(*item), fmt(*seq)),
        Intrinsic::StrIsDigit { seq } => format!("(str.in_re {} (re.range \"0\" \"9\"))", fmt(*seq)),
        Intrinsic::StrLe { lhs, rhs } => format!("(str.<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrLt { lhs, rhs } => format!("(str.< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrGe { lhs, rhs } => format!("(str.<= {} {})", fmt(*rhs), fmt(*lhs)),
        Intrinsic::StrGt { lhs, rhs } => format!("(str.< {} {})", fmt(*rhs), fmt(*lhs)),
        Intrinsic::StrReplace { seq, src, dst } => format!("(str.replace {} {} {})", fmt(*seq), fmt(*src), fmt(*dst)),
        Intrinsic::StrReplaceAll { seq, src, dst } => format!("(str.replace_all {} {} {})", fmt(*seq), fmt(*src), fmt(*dst)),
        Intrinsic::StrToInt { val } => format!("(str.to_int {})", fmt(*val)),
        Intrinsic::StrFromInt { val } => format!("(str.from_int {})", fmt(*val)),
        Intrinsic::StrFromCode { val } => format!("(str.from_code {})", fmt(*val)),
        Intrinsic::StrToCode { val } => format!("(str.to_code {})", fmt(*val)),

        // --- Cloak Operations ---
        Intrinsic::BoxShield { val, .. } | Intrinsic::BoxReveal { val, .. } => fmt(*val),

        // --- Sequence Operations ---
        Intrinsic::SeqEmpty { t } => format!("(as seq.empty (Seq {}))", format_sort_for_fn(t, ir)),
        Intrinsic::SeqUnit { val, .. } => format!("(seq.unit {})", fmt(*val)),
        Intrinsic::SeqLen { seq, .. } => format!("(seq.len {})", fmt(*seq)),
        Intrinsic::SeqPush { seq, item, .. } => format!("(seq.++ {} (seq.unit {}))", fmt(*seq), fmt(*item)),
        Intrinsic::SeqConcat { lhs, rhs, .. } => format!("(seq.++ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SeqNth { seq, idx, .. } => format!("(seq.nth {} {})", fmt(*seq), fmt(*idx)),
        Intrinsic::SeqAtSeq { seq, idx, .. } => format!("(seq.extract {} {} 1)", fmt(*seq), fmt(*idx)),
        Intrinsic::SeqExtract { seq, offset, len, .. } => format!("(seq.extract {} {} {})", fmt(*seq), fmt(*offset), fmt(*len)),
        Intrinsic::SeqIndexOf { seq, sub, offset, .. } => format!("(seq.indexof {} {} {})", fmt(*seq), fmt(*sub), fmt(*offset)),
        Intrinsic::SeqIndexOfDefault { seq, sub, .. } => format!("(seq.indexof {} {} 0)", fmt(*seq), fmt(*sub)),
        Intrinsic::SeqContains { seq, item, .. } => format!("(seq.contains {} (seq.unit {}))", fmt(*seq), fmt(*item)),
        Intrinsic::SeqPrefixOf { lhs, rhs, .. } => format!("(seq.prefixof {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SeqSuffixOf { lhs, rhs, .. } => format!("(seq.suffixof {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SeqReplace { seq, src, dst, .. } => format!("(seq.replace {} {} {})", fmt(*seq), fmt(*src), fmt(*dst)),
        Intrinsic::SeqIsEmpty { seq, .. } => format!("(= (seq.len {}) 0)", fmt(*seq)),

        // --- Set Operations (Z3 Theory of Sets) ---
        Intrinsic::SetEmpty { t } => format!("(as set.empty (Set {}))", format_sort_for_fn(t, ir)),
        Intrinsic::SetLen { set, .. } => format!("(set.card {})", fmt(*set)),
        Intrinsic::SetInsert { set, item, .. } => format!("(set.insert {} {})", fmt(*item), fmt(*set)),
        Intrinsic::SetRemove { set, item, .. } => format!("(set.setminus {} (set.singleton {}))", fmt(*set), fmt(*item)),
        Intrinsic::SetContains { set, item, .. } => format!("(set.member {} {})", fmt(*item), fmt(*set)),
        Intrinsic::SetIsEmpty { set, .. } => format!("(= (set.card {}) 0)", fmt(*set)),
        Intrinsic::SetIntersect { lhs, rhs, .. } => format!("(set.inter {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SetUnion { lhs, rhs, .. } => format!("(set.union {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SetDiff { lhs, rhs, .. } => format!("(set.setminus {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SetSymDiff { lhs, rhs, .. } => {
            let (l, r) = (fmt(*lhs), fmt(*rhs));
            format!("(set.union (set.setminus {l} {r}) (set.setminus {r} {l}))")
        }
        Intrinsic::SetIsSubset { lhs, rhs, .. } => format!("(set.subset {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SetIsProperSubset { lhs, rhs, .. } => {
            let (l, r) = (fmt(*lhs), fmt(*rhs));
            format!("(and (set.subset {l} {r}) (not (= {l} {r})))")
        }
        Intrinsic::SetIsDisjoint { lhs, rhs, .. } => format!("(= (set.inter {} {}) set.empty)", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SetHasSize { set, size, .. } => format!("(= (set.card {}) {})", fmt(*set), fmt(*size)),

        // --- Array Operations ---
        Intrinsic::ArrayEmpty { k, v } => {
            let (ks, vs) = (format_sort_for_fn(k, ir), format_sort_for_fn(v, ir));
            format!("((as const (Array {ks} {vs})) (as @default {vs}))")
        }
        Intrinsic::ArrayLen { arr, .. } => format!("(array.size {})", fmt(*arr)), // Z3 extension
        Intrinsic::ArrayStore { arr, key, val, .. } => format!("(store {} {} {})", fmt(*arr), fmt(*key), fmt(*val)),
        Intrinsic::ArraySelect { arr, key, .. } => format!("(select {} {})", fmt(*arr), fmt(*key)),
        Intrinsic::ArrayRemove { arr, key, .. } => format!("(store {} {} @default)", fmt(*arr), fmt(*key)),
        Intrinsic::ArrayContainsKey { arr, key, .. } => format!("(not (= (select {} {}) @default))", fmt(*arr), fmt(*key)),
        Intrinsic::ArrayIsEmpty { arr, .. } => format!("(= (array.size {}) 0)", fmt(*arr)),

        // --- BitVector Operations ---
        Intrinsic::BvVal { t, val } => {
            let width = match t {
                Sort::I32 | Sort::U32 => 32,
                _ => 64,
            };
            format!("(_ bv{} {})", val, width)
        }
        Intrinsic::BvNot { val, .. } => format!("(bvnot {})", fmt(*val)),
        Intrinsic::BvAnd { lhs, rhs, .. } => format!("(bvand {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvOr { lhs, rhs, .. } => format!("(bvor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvXor { lhs, rhs, .. } => format!("(bvxor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvNand { lhs, rhs, .. } => format!("(bvnand {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvNor { lhs, rhs, .. } => format!("(bvnor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvXnor { lhs, rhs, .. } => format!("(bvxnor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvRedAnd { val, .. } => format!("(bvredand {})", fmt(*val)),
        Intrinsic::BvRedOr { val, .. } => format!("(bvredor {})", fmt(*val)),
        Intrinsic::BvNeg { val, .. } => format!("(bvneg {})", fmt(*val)),
        Intrinsic::BvAdd { lhs, rhs, .. } => format!("(bvadd {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvSub { lhs, rhs, .. } => format!("(bvsub {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvMul { lhs, rhs, .. } => format!("(bvmul {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvDiv { lhs, rhs, .. } => format!("(bvudiv {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvRem { lhs, rhs, .. } => format!("(bvurem {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvMod { lhs, rhs, .. } => format!("(bvsmod {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvShl { lhs, rhs, .. } => format!("(bvshl {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvLshr { lhs, rhs, .. } => format!("(bvlshr {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvAshr { lhs, rhs, .. } => format!("(bvashr {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvRotLeft { lhs, rhs, .. } => format!("(ext_rotate_left {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvRotRight { lhs, rhs, .. } => format!("(ext_rotate_right {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvLt { lhs, rhs, .. } => format!("(bvult {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvLe { lhs, rhs, .. } => format!("(bvule {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvGt { lhs, rhs, .. } => format!("(bvugt {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvGe { lhs, rhs, .. } => format!("(bvuge {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvToInt { val, .. } => format!("(bv2int {})", fmt(*val)),

        // --- Floating-Point Operations ---
        Intrinsic::FloatVal { t, val } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("((_ to_fp {} {}) RNE (/ {} {}))", eb, sb, val.numer(), val.denom())
        }
        Intrinsic::FloatAdd { lhs, rhs, .. } => format!("(fp.add RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatSub { lhs, rhs, .. } => format!("(fp.sub RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatMul { lhs, rhs, .. } => format!("(fp.mul RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatDiv { lhs, rhs, .. } => format!("(fp.div RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatNeg { val, .. } => format!("(fp.neg {})", fmt(*val)),
        Intrinsic::FloatAbs { val, .. } => format!("(fp.abs {})", fmt(*val)),
        Intrinsic::FloatRem { lhs, rhs, .. } => format!("(fp.rem {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatSqrt { val, .. } => format!("(fp.sqrt RNE {})", fmt(*val)),
        Intrinsic::FloatMin { lhs, rhs, .. } => format!("(fp.min {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatMax { lhs, rhs, .. } => format!("(fp.max {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatIsNaN { val, .. } => format!("(fp.isNaN {})", fmt(*val)),
        Intrinsic::FloatIsInf { val, .. } => format!("(fp.isInfinite {})", fmt(*val)),
        Intrinsic::FloatIsZero { val, .. } => format!("(fp.isZero {})", fmt(*val)),
        Intrinsic::FloatIsNormal { val, .. } => format!("(fp.isNormal {})", fmt(*val)),
        Intrinsic::FloatIsSubnormal { val, .. } => format!("(fp.isSubnormal {})", fmt(*val)),
        Intrinsic::FloatIsNeg { val, .. } => format!("(fp.isNegative {})", fmt(*val)),
        Intrinsic::FloatIsPos { val, .. } => format!("(fp.isPositive {})", fmt(*val)),
        Intrinsic::FloatLt { lhs, rhs, .. } => format!("(fp.lt {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatLe { lhs, rhs, .. } => format!("(fp.leq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatGt { lhs, rhs, .. } => format!("(fp.gt {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatGe { lhs, rhs, .. } => format!("(fp.geq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatNaN { t } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("(_ NaN {eb} {sb})")
        }
        Intrinsic::FloatPosInf { t } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("(_ +oo {eb} {sb})")
        }
        Intrinsic::FloatNegInf { t } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("(_ -oo {eb} {sb})")
        }
        Intrinsic::FloatPosZero { t } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("(_ +zero {eb} {sb})")
        }
        Intrinsic::FloatNegZero { t } => {
            let (eb, sb) = match t { Sort::F32 => (8, 24), _ => (11, 53) };
            format!("(_ -zero {eb} {sb})")
        }
        Intrinsic::FloatToInt { val, .. } => format!("(to_int (fp.to_real {}))", fmt(*val)),
        Intrinsic::FloatToReal { val, .. } => format!("(fp.to_real {})", fmt(*val)),
        Intrinsic::FloatToU32 { val, .. } => format!("((_ fp.to_ubv 32) RNE {})", fmt(*val)),
        Intrinsic::FloatToI32 { val, .. } => format!("((_ fp.to_sbv 32) RNE {})", fmt(*val)),
        Intrinsic::FloatToU64 { val, .. } => format!("((_ fp.to_ubv 64) RNE {})", fmt(*val)),
        Intrinsic::FloatToI64 { val, .. } => format!("((_ fp.to_sbv 64) RNE {})", fmt(*val)),
        Intrinsic::FloatCeil { val, .. } => format!("(fp.roundToIntegral RTP {})", fmt(*val)),
        Intrinsic::FloatFloor { val, .. } => format!("(fp.roundToIntegral RTN {})", fmt(*val)),
        Intrinsic::FloatTrunc { val, .. } => format!("(fp.roundToIntegral RTZ {})", fmt(*val)),
        Intrinsic::FloatNearest { val, .. } => format!("(fp.roundToIntegral RNE {})", fmt(*val)),
        Intrinsic::FloatFqEq { lhs, rhs, .. } => format!("(fp.eq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatFromHexStr { val, .. } => format!("(fp.from_str {})", fmt(*val)), // Z3 extension

        // --- Error & Generic Operations ---
        Intrinsic::ErrFresh => "ErrFresh".to_string(),
        Intrinsic::ErrMerge { lhs, rhs } => format!("(ErrMerge {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SmtEq { lhs, rhs, .. } => format!("(= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SmtNe { lhs, rhs, .. } => format!("(not (= {} {}))", fmt(*lhs), fmt(*rhs)),
    }
}
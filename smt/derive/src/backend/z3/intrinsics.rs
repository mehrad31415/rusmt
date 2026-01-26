//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::backend::z3::exp::format_expression;
use crate::backend::z3::fun::format_sort_for_fn;
use crate::ir::ctxt::IRContext;
use crate::ir::exp::ExpRegistry;
use crate::ir::index::UsrFunId;
use crate::ir::intrinsics::Intrinsic;
use std::collections::BTreeSet;

/// Convert an intrinsic operation to SMT-LIB string format.
pub fn format_intrinsic(
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    ir: &IRContext,
    param_names: &std::collections::HashSet<String>,
    scc_fids: &BTreeSet<UsrFunId>,
) -> String {
    match intrinsic {
        // Boolean Operations
        Intrinsic::BoolVal(b) => b.to_string(),
        Intrinsic::BoolNot { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(not {})", val_str)
        }
        Intrinsic::BoolAnd { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(and {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BoolOr { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(or {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BoolXor { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(xor {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BoolImplies { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(=> {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BoolIff { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BoolNand { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (and {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::BoolNor { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (or {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::BoolXnor { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (xor {} {}))", lhs_str, rhs_str)
        }

        // Integer Operations - Values & Comparisons
        Intrinsic::IntVal(n) => n.to_string(),
        Intrinsic::IntNeg { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(- {})", val_str)
        }
        Intrinsic::IntLt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(< {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntLe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(<= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntGe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(>= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntGt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(> {} {})", lhs_str, rhs_str)
        }
        
        // Integer Arithmetic
        Intrinsic::IntAdd { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(+ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntSub { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(- {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntMul { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(* {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntDiv { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(div {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntMod { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(mod {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntRem { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(rem {} {})", lhs_str, rhs_str)
        }
        Intrinsic::IntPow { base, exp } => {
            let base_str = format_expression(exp_registry, *base, ir, param_names, scc_fids);
            let exp_str = format_expression(exp_registry, *exp, ir, param_names, scc_fids);
            format!("(^ {} {})", base_str, exp_str)
        }
        Intrinsic::IntAbs { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(abs {})", val_str)
        }
        Intrinsic::IntDivides { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(= (mod {} {}) 0)", rhs_str, lhs_str)
        }

        // Integer Conversions
        Intrinsic::IntoToReal { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(to_real {})", val_str)
        }
        Intrinsic::IntToI32 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ int2bv 32) {})", val_str)
        }
        Intrinsic::IntToI64 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ int2bv 64) {})", val_str)
        }
        Intrinsic::IntToU32 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ int2bv 32) {})", val_str)
        }
        Intrinsic::IntToU64 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ int2bv 64) {})", val_str)
        }
        Intrinsic::IntToF32 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ to_fp 8 24) RTZ (to_real {}))", val_str)
        }
        Intrinsic::IntToF64 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ to_fp 11 53) RTZ (to_real {}))", val_str)
        }

        // Integer Parsing (placeholder)
        Intrinsic::IntFromHex { val } | Intrinsic::IntFromOct { val } | Intrinsic::IntFromBin { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(str.to_int {})", val_str)
        }

        // Integer Range Checks
        Intrinsic::IntIsGtI64Max { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(> {} 9223372036854775807)", val_str)
        }
        Intrinsic::IntIsLtI64Min { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(< {} (- 9223372036854775808))", val_str)
        }
        Intrinsic::IntIsGtU64Max { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(> {} 18446744073709551615)", val_str)
        }
        Intrinsic::IntIsLtU64Min { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(< {} 0)", val_str)
        }
        Intrinsic::IntIsLtI32Min { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(< {} (- 2147483648))", val_str)
        }
        Intrinsic::IntIsGtI32Max { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(> {} 2147483647)", val_str)
        }
        Intrinsic::IntIsLtU32Min { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(< {} 0)", val_str)
        }
        Intrinsic::IntIsGtU32Max { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(> {} 4294967295)", val_str)
        }

        // Real (Rational) Operations
        Intrinsic::RealVal(r) => format!("(/ {} {})", r.numer(), r.denom()),
        Intrinsic::RealNeg { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(- {})", val_str)
        }
        Intrinsic::RealLt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(< {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealLe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(<= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealGe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(>= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealGt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(> {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealAdd { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(+ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealSub { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(- {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealMul { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(* {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealDiv { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(/ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::RealPow { base, exp } => {
            let base_str = format_expression(exp_registry, *base, ir, param_names, scc_fids);
            let exp_str = format_expression(exp_registry, *exp, ir, param_names, scc_fids);
            format!("(^ {} {})", base_str, exp_str)
        }
        Intrinsic::RealAbs { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(abs {})", val_str)
        }
        Intrinsic::RealRound { val } | Intrinsic::RealFloor { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(to_real (to_int {}))", val_str)
        }
        Intrinsic::RealCeil { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(- (to_real (to_int (- {}))))", val_str)
        }
        Intrinsic::RealIsInt { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(is_int {})", val_str)
        }
        Intrinsic::RealToInt { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(to_int {})", val_str)
        }
        Intrinsic::RealToF32 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ to_fp 8 24) RTZ {})", val_str)
        }
        Intrinsic::RealToF64 { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("((_ to_fp 11 53) RTZ {})", val_str)
        }
        Intrinsic::RealRealer { val } | Intrinsic::RealDenom { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(to_real (to_int {}))", val_str)
        }

        // String Operations
        Intrinsic::StrVal(s) => {
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\"", escaped)
        }
        Intrinsic::StrLt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(str.< {} {})", lhs_str, rhs_str)
        }
        Intrinsic::StrLe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(str.<= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::StrGt { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (str.<= {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::StrGe { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (str.< {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::StrConcat { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(str.++ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::StrAt { seq, idx } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let idx_str = format_expression(exp_registry, *idx, ir, param_names, scc_fids);
            format!("(str.at {} {})", seq_str, idx_str)
        }
        Intrinsic::StrLength { seq } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            format!("(str.len {})", seq_str)
        }
        Intrinsic::StrIsEmpty { seq } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            format!("(= {} \"\")", seq_str)
        }
        Intrinsic::StrIncludes { seq, item } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(str.contains {} {})", seq_str, item_str)
        }
        Intrinsic::StrStartsWith { seq, item } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(str.prefixof {} {})", item_str, seq_str)
        }
        Intrinsic::StrEndsWith { seq, item } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(str.suffixof {} {})", item_str, seq_str)
        }
        Intrinsic::StrIsDigit { seq } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            format!("(str.is_digit {})", seq_str)
        }
        Intrinsic::StrIndexOf { seq, sub, offset } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let sub_str = format_expression(exp_registry, *sub, ir, param_names, scc_fids);
            let offset_str = format_expression(exp_registry, *offset, ir, param_names, scc_fids);
            format!("(str.indexof {} {} {})", seq_str, sub_str, offset_str)
        }
        Intrinsic::StrReplace { seq, src, dst } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let src_str = format_expression(exp_registry, *src, ir, param_names, scc_fids);
            let dst_str = format_expression(exp_registry, *dst, ir, param_names, scc_fids);
            format!("(str.replace {} {} {})", seq_str, src_str, dst_str)
        }
        Intrinsic::StrReplaceAll { seq, src, dst } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let src_str = format_expression(exp_registry, *src, ir, param_names, scc_fids);
            let dst_str = format_expression(exp_registry, *dst, ir, param_names, scc_fids);
            format!("(str.replace_all {} {} {})", seq_str, src_str, dst_str)
        }
        Intrinsic::StrToInt { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(str.to_int {})", val_str)
        }
        Intrinsic::StrFromInt { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(str.from_int {})", val_str)
        }
        Intrinsic::StrFromCode { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(str.from_code {})", val_str)
        }
        Intrinsic::StrToCode { val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(str.to_code {})", val_str)
        }

        // Cloak Operations (transparent in SMT)
        Intrinsic::BoxShield { t: _, val } | Intrinsic::BoxReveal { t: _, val } => {
            format_expression(exp_registry, *val, ir, param_names, scc_fids)
        }

        // Sequence Operations
        Intrinsic::SeqEmpty { t } => {
            let elem_type = format_sort_for_fn(t, ir);
            format!("(as seq.empty (Seq {}))", elem_type)
        }
        Intrinsic::SeqUnit { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(seq.unit {})", val_str)
        }
        Intrinsic::SeqLength { t: _, seq } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            format!("(seq.len {})", seq_str)
        }
        Intrinsic::SeqNth { t: _, seq, idx } | Intrinsic::SeqAt { t: _, seq, idx } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let idx_str = format_expression(exp_registry, *idx, ir, param_names, scc_fids);
            format!("(seq.nth {} {})", seq_str, idx_str)
        }
        Intrinsic::SeqExtract { t: _, seq, offset, len } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let offset_str = format_expression(exp_registry, *offset, ir, param_names, scc_fids);
            let len_str = format_expression(exp_registry, *len, ir, param_names, scc_fids);
            format!("(seq.extract {} {} {})", seq_str, offset_str, len_str)
        }
        Intrinsic::SeqAppend { t: _, seq, item } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(seq.++ {} (seq.unit {}))", seq_str, item_str)
        }
        Intrinsic::SeqConcat { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(seq.++ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SeqIncludes { t: _, seq, item } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(seq.contains {} (seq.unit {}))", seq_str, item_str)
        }
        Intrinsic::SeqPrefixOf { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(seq.prefixof {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SeqSuffixOf { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(seq.suffixof {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SeqReplace { t: _, seq, src, dst } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            let src_str = format_expression(exp_registry, *src, ir, param_names, scc_fids);
            let dst_str = format_expression(exp_registry, *dst, ir, param_names, scc_fids);
            format!("(seq.replace {} {} {})", seq_str, src_str, dst_str)
        }
        Intrinsic::SeqIsEmpty { t: _, seq } => {
            let seq_str = format_expression(exp_registry, *seq, ir, param_names, scc_fids);
            format!("(= (seq.len {}) 0)", seq_str)
        }

        // Set Operations (using Array model)
        Intrinsic::SetEmpty { t } => {
            let elem_type = format_sort_for_fn(t, ir);
            format!("((as const (Array {} Bool)) false)", elem_type)
        }
        Intrinsic::SetLength { t: _, set } | Intrinsic::SetIsEmpty { t: _, set } => {
            let set_str = format_expression(exp_registry, *set, ir, param_names, scc_fids);
            format!("(set-card {})", set_str)
        }
        Intrinsic::SetInsert { t: _, set, item } => {
            let set_str = format_expression(exp_registry, *set, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(store {} {} true)", set_str, item_str)
        }
        Intrinsic::SetRemove { t: _, set, item } => {
            let set_str = format_expression(exp_registry, *set, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(store {} {} false)", set_str, item_str)
        }
        Intrinsic::SetContains { t: _, set, item } => {
            let set_str = format_expression(exp_registry, *set, ir, param_names, scc_fids);
            let item_str = format_expression(exp_registry, *item, ir, param_names, scc_fids);
            format!("(select {} {})", set_str, item_str)
        }
        Intrinsic::SetUnion { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(set-union {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SetIntersection { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(set-intersect {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SetDifference { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(set-minus {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SetSymDiff { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(set-symmetric-diff {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SetIsSubset { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(subset {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SetIsProperSubset { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(and (subset {} {}) (not (= {} {})))", lhs_str, rhs_str, lhs_str, rhs_str)
        }
        Intrinsic::SetIsSuperset { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(subset {} {})", rhs_str, lhs_str)
        }
        Intrinsic::SetIsDisjoint { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(= (set-card (set-intersect {} {})) 0)", lhs_str, rhs_str)
        }
        Intrinsic::SetHasSize { t: _, set, size } => {
            let set_str = format_expression(exp_registry, *set, ir, param_names, scc_fids);
            let size_str = format_expression(exp_registry, *size, ir, param_names, scc_fids);
            format!("(= (set-card {}) {})", set_str, size_str)
        }

        // Array Operations

        // Map Operations
        Intrinsic::MapEmpty { k, v } => {
            let key_type = format_sort_for_fn(k, ir);
            let val_type = format_sort_for_fn(v, ir);
            format!("((as const (Array {} {})) none)", key_type, val_type)
        }
        Intrinsic::MapPut { k: _, v: _, map, key, val } => {
            let map_str = format_expression(exp_registry, *map, ir, param_names, scc_fids);
            let key_str = format_expression(exp_registry, *key, ir, param_names, scc_fids);
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(store {} {} {})", map_str, key_str, val_str)
        }
        Intrinsic::MapGet { k: _, v: _, map, key } => {
            let map_str = format_expression(exp_registry, *map, ir, param_names, scc_fids);
            let key_str = format_expression(exp_registry, *key, ir, param_names, scc_fids);
            format!("(select {} {})", map_str, key_str)
        }
        Intrinsic::MapDel { k: _, v: _, map, key } => {
            let map_str = format_expression(exp_registry, *map, ir, param_names, scc_fids);
            let key_str = format_expression(exp_registry, *key, ir, param_names, scc_fids);
            format!("(store {} {} none)", map_str, key_str)
        }
        Intrinsic::MapContainsKey { k: _, v: _, map, key } => {
            let map_str = format_expression(exp_registry, *map, ir, param_names, scc_fids);
            let key_str = format_expression(exp_registry, *key, ir, param_names, scc_fids);
            format!("(is-some (select {} {}))", map_str, key_str)
        }
        Intrinsic::MapLength { k: _, v: _, map } | Intrinsic::MapIsEmpty { k: _, v: _, map } => {
            let map_str = format_expression(exp_registry, *map, ir, param_names, scc_fids);
            format!("(map-card {})", map_str)
        }
        Intrinsic::ErrFresh { error_id } => format!("(ErrSingle {})", error_id),
        Intrinsic::ErrMerge { lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(ErrMerge {} {})", lhs_str, rhs_str)
        }

        // Equality Operations
        Intrinsic::SmtEq { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(= {} {})", lhs_str, rhs_str)
        }
        Intrinsic::SmtNe { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (= {} {}))", lhs_str, rhs_str)
        }

        // BitVector Operations
        Intrinsic::BvVal { t, val } => {
            // Get bit-width from the sort
            let width = match t {
                crate::ir::sort::Sort::I32 => 32,
                crate::ir::sort::Sort::I64 => 64,
                crate::ir::sort::Sort::U32 => 32,
                crate::ir::sort::Sort::U64 => 64,
                _ => 64, // default
            };
            format!("(_ bv{} {})", val, width)
        }
        Intrinsic::BvNot { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(bvnot {})", val_str)
        }
        Intrinsic::BvNeg { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(bvneg {})", val_str)
        }
        Intrinsic::BvRedAnd { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(bvredand {})", val_str)
        }
        Intrinsic::BvRedOr { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(bvredor {})", val_str)
        }
        Intrinsic::BvAnd { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvand {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvOr { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvor {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvXor { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvxor {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvNand { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvnand {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvNor { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvnor {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvXnor { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvxnor {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvAdd { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvadd {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvSub { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvsub {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvMul { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvmul {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvDiv { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvudiv {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvRem { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvurem {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvMod { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvsmod {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvShl { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvshl {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvLshr { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvlshr {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvAshr { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvashr {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvRotLeft { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("((_ rotate_left {}) {})", rhs_str, lhs_str)
        }
        Intrinsic::BvRotRight { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("((_ rotate_right {}) {})", rhs_str, lhs_str)
        }
        Intrinsic::BvLt { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvult {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvLe { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvule {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvGt { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvugt {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvGe { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(bvuge {} {})", lhs_str, rhs_str)
        }
        Intrinsic::BvToInt { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(bv2int {})", val_str)
        }
        Intrinsic::BvAddNoOverflow { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (bvuaddo {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::BvSubNoOverflow { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (bvsubo {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::BvNegNoOverflow { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(not (bvnego {}))", val_str)
        }
        Intrinsic::BvMulNoOverflow { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (bvumulo {} {}))", lhs_str, rhs_str)
        }
        Intrinsic::BvDivNoOverflow { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(not (bvsdivo {} {}))", lhs_str, rhs_str)
        }

        // Floating-Point Operations
        Intrinsic::FloatVal { t, val } => {
            // Get exponent and significand bits from the sort
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53), // default to F64
            };
            // Convert rational to float literal
            format!("((_ to_fp {} {}) RTZ (/ {} {}))", eb, sb, val.numer(), val.denom())
        }
        Intrinsic::FloatNaN { t } => {
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53),
            };
            format!("(_ NaN {} {})", eb, sb)
        }
        Intrinsic::FloatPosInf { t } => {
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53),
            };
            format!("(_ +oo {} {})", eb, sb)
        }
        Intrinsic::FloatNegInf { t } => {
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53),
            };
            format!("(_ -oo {} {})", eb, sb)
        }
        Intrinsic::FloatPosZero { t } => {
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53),
            };
            format!("(_ +zero {} {})", eb, sb)
        }
        Intrinsic::FloatNegZero { t } => {
            let (eb, sb) = match t {
                crate::ir::sort::Sort::F32 => (8, 24),
                crate::ir::sort::Sort::F64 => (11, 53),
                _ => (11, 53),
            };
            format!("(_ -zero {} {})", eb, sb)
        }
        Intrinsic::FloatNeg { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.neg {})", val_str)
        }
        Intrinsic::FloatAbs { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.abs {})", val_str)
        }
        Intrinsic::FloatSqrt { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.sqrt RTZ {})", val_str)
        }
        Intrinsic::FloatAdd { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.add RTZ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatSub { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.sub RTZ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatMul { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.mul RTZ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatDiv { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.div RTZ {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatRem { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.rem {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatMin { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.min {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatMax { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.max {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatIsNaN { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isNaN {})", val_str)
        }
        Intrinsic::FloatIsInf { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isInfinite {})", val_str)
        }
        Intrinsic::FloatIsZero { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isZero {})", val_str)
        }
        Intrinsic::FloatIsNormal { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isNormal {})", val_str)
        }
        Intrinsic::FloatIsSubnormal { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isSubnormal {})", val_str)
        }
        Intrinsic::FloatIsNeg { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isNegative {})", val_str)
        }
        Intrinsic::FloatIsPos { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.isPositive {})", val_str)
        }
        Intrinsic::FloatLt { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.lt {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatLe { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.leq {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatGt { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.gt {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatGe { t: _, lhs, rhs } => {
            let lhs_str = format_expression(exp_registry, *lhs, ir, param_names, scc_fids);
            let rhs_str = format_expression(exp_registry, *rhs, ir, param_names, scc_fids);
            format!("(fp.geq {} {})", lhs_str, rhs_str)
        }
        Intrinsic::FloatToInt { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.to_sbv 64 RTZ {})", val_str)
        }
        Intrinsic::FloatToReal { t: _, val } => {
            let val_str = format_expression(exp_registry, *val, ir, param_names, scc_fids);
            format!("(fp.to_real {})", val_str)
        }
    }
}

//! Translate IR intrinsic operations to Z3 API calls.

use crate::backend::z3::fun::format_sort_for_fn;
use crate::backend::z3::intrinsics::array_null_const_name;
use crate::backend::z3_api::Z3Ast;
use crate::backend::z3_api::context::Z3ApiContext;
use crate::backend::z3_api::mk_string_symbol;
use crate::backend::z3_api::translate::translate_expression;
use crate::ir::exp::ExpRegistry;
use crate::ir::index::{ExpId, UsrFunId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::sort::Sort;
use num_bigint::BigInt;
use std::collections::{BTreeSet, HashMap};

/// Translate an IR Intrinsic to a Z3 AST.
pub fn translate_intrinsic<'ctx>(
    api_ctx: &mut Z3ApiContext<'ctx>,
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    var_map: &HashMap<String, z3_sys::Z3_ast>,
    scc_fids: &BTreeSet<UsrFunId>,
) -> Z3Ast<'ctx> {
    let ctx = api_ctx.ctx;

    // Macro to translate sub-expressions (avoids mutable borrow issues with closures)
    macro_rules! tr {
        ($id:expr) => {
            translate_expression(api_ctx, exp_registry, $id, var_map, scc_fids)
        };
    }

    unsafe {
        match intrinsic {
            // --- Boolean ---
            Intrinsic::BoolVal(b) => {
                let ast = if *b {
                    z3_sys::Z3_mk_true(ctx).expect("Z3_mk_true")
                } else {
                    z3_sys::Z3_mk_false(ctx).expect("Z3_mk_false")
                };
                Z3Ast::new(ctx, ast)
            }
            Intrinsic::BoolNot { val } => {
                let v = tr!(*val);
                Z3Ast::new(ctx, z3_sys::Z3_mk_not(ctx, v.raw()).expect("Z3_mk_not"))
            }
            Intrinsic::BoolAnd { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_and(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_and"),
                )
            }
            Intrinsic::BoolOr { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_or(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_or"),
                )
            }
            Intrinsic::BoolXor { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_xor(ctx, l.raw(), r.raw()).expect("Z3_mk_xor"),
                )
            }
            Intrinsic::BoolNand { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let and =
                    z3_sys::Z3_mk_and(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_and");
                Z3Ast::new(ctx, z3_sys::Z3_mk_not(ctx, and).expect("Z3_mk_not"))
            }
            Intrinsic::BoolNor { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let or = z3_sys::Z3_mk_or(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_or");
                Z3Ast::new(ctx, z3_sys::Z3_mk_not(ctx, or).expect("Z3_mk_not"))
            }
            Intrinsic::BoolXnor { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let xor = z3_sys::Z3_mk_xor(ctx, l.raw(), r.raw()).expect("Z3_mk_xor");
                Z3Ast::new(ctx, z3_sys::Z3_mk_not(ctx, xor).expect("Z3_mk_not"))
            }
            Intrinsic::BoolImplies { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_implies(ctx, l.raw(), r.raw()).expect("Z3_mk_implies"),
                )
            }
            Intrinsic::BoolIff { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_eq(ctx, l.raw(), r.raw()).expect("Z3_mk_eq"),
                )
            }
            Intrinsic::BoolIte { cond, then, else_ } => {
                let (c, t, e) = (tr!(*cond), tr!(*then), tr!(*else_));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ite(ctx, c.raw(), t.raw(), e.raw()).expect("Z3_mk_ite"),
                )
            }

            // --- Integer ---
            Intrinsic::IntVal(n) => {
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let s = std::ffi::CString::new(n.to_string()).unwrap();
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_numeral(ctx, s.as_ptr(), int_sort).expect("Z3_mk_numeral"),
                )
            }
            Intrinsic::IntNeg { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_unary_minus(ctx, v.raw()).expect("Z3_mk_unary_minus"),
                )
            }
            Intrinsic::IntAdd { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_add(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_add"),
                )
            }
            Intrinsic::IntSub { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_sub(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_sub"),
                )
            }
            Intrinsic::IntMul { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_mul(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_mul"),
                )
            }
            Intrinsic::IntDiv { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_div(ctx, l.raw(), r.raw()).expect("Z3_mk_div"),
                )
            }
            Intrinsic::IntDivTrunc { lhs, rhs } => {
                // C-style truncation: ite (>= n 0) (div n d) (- (div (- n) d))
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                let ge_zero = z3_sys::Z3_mk_ge(ctx, l.raw(), zero).expect("Z3_mk_ge");
                let neg_l = z3_sys::Z3_mk_unary_minus(ctx, l.raw()).expect("Z3_mk_unary_minus");
                let div_pos = z3_sys::Z3_mk_div(ctx, l.raw(), r.raw()).expect("Z3_mk_div");
                let div_neg_raw = z3_sys::Z3_mk_div(ctx, neg_l, r.raw()).expect("Z3_mk_div");
                let div_neg =
                    z3_sys::Z3_mk_unary_minus(ctx, div_neg_raw).expect("Z3_mk_unary_minus");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ite(ctx, ge_zero, div_pos, div_neg).expect("Z3_mk_ite"),
                )
            }
            Intrinsic::IntMod { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_mod(ctx, l.raw(), r.raw()).expect("Z3_mk_mod"),
                )
            }
            Intrinsic::IntRem { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_rem(ctx, l.raw(), r.raw()).expect("Z3_mk_rem"),
                )
            }
            Intrinsic::IntPow { base, exp } => {
                let (b, e) = (tr!(*base), tr!(*exp));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_power(ctx, b.raw(), e.raw()).expect("Z3_mk_power"),
                )
            }
            Intrinsic::IntAbs { val } => {
                let v = tr!(*val);
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                let neg = z3_sys::Z3_mk_unary_minus(ctx, v.raw()).expect("Z3_mk_unary_minus");
                let ge = z3_sys::Z3_mk_ge(ctx, v.raw(), zero).expect("Z3_mk_ge");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ite(ctx, ge, v.raw(), neg).expect("Z3_mk_ite"),
                )
            }
            Intrinsic::IntDivides { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                let not_zero =
                    z3_sys::Z3_mk_not(ctx, z3_sys::Z3_mk_eq(ctx, l.raw(), zero).expect("Z3_mk_eq"))
                        .expect("Z3_mk_not");
                let mod_eq_zero = z3_sys::Z3_mk_eq(
                    ctx,
                    z3_sys::Z3_mk_mod(ctx, r.raw(), l.raw()).expect("Z3_mk_mod"),
                    zero,
                )
                .expect("Z3_mk_eq");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_and(ctx, 2, [not_zero, mod_eq_zero].as_ptr()).expect("Z3_mk_and"),
                )
            }
            Intrinsic::IntLt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_lt(ctx, l.raw(), r.raw()).expect("Z3_mk_lt"),
                )
            }
            Intrinsic::IntLe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_le(ctx, l.raw(), r.raw()).expect("Z3_mk_le"),
                )
            }
            Intrinsic::IntGt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_gt(ctx, l.raw(), r.raw()).expect("Z3_mk_gt"),
                )
            }
            Intrinsic::IntGe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ge(ctx, l.raw(), r.raw()).expect("Z3_mk_ge"),
                )
            }

            // --- Integer Conversions ---
            Intrinsic::IntToReal { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_int2real(ctx, v.raw()).expect("Z3_mk_int2real"),
                )
            }
            Intrinsic::IntToI32 { val } | Intrinsic::IntToU32 { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_int2bv(ctx, 32, v.raw()).expect("Z3_mk_int2bv"),
                )
            }
            Intrinsic::IntToI64 { val } | Intrinsic::IntToU64 { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_int2bv(ctx, 64, v.raw()).expect("Z3_mk_int2bv"),
                )
            }
            Intrinsic::IntToF32 { val } => {
                let v = tr!(*val);
                let real = z3_sys::Z3_mk_int2real(ctx, v.raw()).expect("Z3_mk_int2real");
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                let fp_sort = z3_sys::Z3_mk_fpa_sort(ctx, 8, 24).expect("Z3_mk_fpa_sort");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_fp_real(ctx, rne, real, fp_sort)
                        .expect("Z3_mk_fpa_to_fp_real"),
                )
            }
            Intrinsic::IntToF64 { val } => {
                let v = tr!(*val);
                let real = z3_sys::Z3_mk_int2real(ctx, v.raw()).expect("Z3_mk_int2real");
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                let fp_sort = z3_sys::Z3_mk_fpa_sort(ctx, 11, 53).expect("Z3_mk_fpa_sort");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_fp_real(ctx, rne, real, fp_sort)
                        .expect("Z3_mk_fpa_to_fp_real"),
                )
            }

            // --- Integer Parsing ---
            Intrinsic::IntFromHex { val }
            | Intrinsic::IntFromOct { val }
            | Intrinsic::IntFromBin { val } => {
                let v = tr!(*val);
                let func_name = match intrinsic {
                    Intrinsic::IntFromHex { .. } => "rusmart_from_hex_str",
                    Intrinsic::IntFromOct { .. } => "rusmart_from_oct_str",
                    Intrinsic::IntFromBin { .. } => "rusmart_from_bin_str",
                    _ => unreachable!(),
                };
                // Look up the helper function declared by build_string_parsing_helpers.
                let decl = api_ctx.get_helper_func_decl(func_name);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_app(ctx, decl, 1, [v.raw()].as_ptr()).expect("Z3_mk_app"),
                )
            }

            // --- Integer Range Checks ---
            Intrinsic::IntIsGtI64Max { val } => {
                mk_int_cmp(ctx, &tr!(*val), ">", "9223372036854775807")
            }
            Intrinsic::IntIsLtI64Min { val } => {
                mk_int_cmp(ctx, &tr!(*val), "<", "-9223372036854775808")
            }
            Intrinsic::IntIsGtU64Max { val } => {
                mk_int_cmp(ctx, &tr!(*val), ">", "18446744073709551615")
            }
            Intrinsic::IntIsLtU64Min { val } => mk_int_cmp(ctx, &tr!(*val), "<", "0"),
            Intrinsic::IntIsLtI32Min { val } => mk_int_cmp(ctx, &tr!(*val), "<", "-2147483648"),
            Intrinsic::IntIsGtI32Max { val } => mk_int_cmp(ctx, &tr!(*val), ">", "2147483647"),
            Intrinsic::IntIsLtU32Min { val } => mk_int_cmp(ctx, &tr!(*val), "<", "0"),
            Intrinsic::IntIsGtU32Max { val } => mk_int_cmp(ctx, &tr!(*val), ">", "4294967295"),

            // --- Real ---
            Intrinsic::RealVal(r) => {
                let real_sort = z3_sys::Z3_mk_real_sort(ctx).expect("Z3_mk_real_sort");
                let num = std::ffi::CString::new(r.numer().to_string()).unwrap();
                let den = std::ffi::CString::new(r.denom().to_string()).unwrap();
                let n = z3_sys::Z3_mk_numeral(ctx, num.as_ptr(), real_sort).expect("Z3_mk_numeral");
                let d = z3_sys::Z3_mk_numeral(ctx, den.as_ptr(), real_sort).expect("Z3_mk_numeral");
                Z3Ast::new(ctx, z3_sys::Z3_mk_div(ctx, n, d).expect("Z3_mk_div"))
            }
            Intrinsic::RealNeg { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_unary_minus(ctx, v.raw()).expect("Z3_mk_unary_minus"),
                )
            }
            Intrinsic::RealAdd { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_add(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_add"),
                )
            }
            Intrinsic::RealSub { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_sub(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_sub"),
                )
            }
            Intrinsic::RealMul { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_mul(ctx, 2, [l.raw(), r.raw()].as_ptr()).expect("Z3_mk_mul"),
                )
            }
            Intrinsic::RealDiv { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_div(ctx, l.raw(), r.raw()).expect("Z3_mk_div"),
                )
            }
            Intrinsic::RealPow { base, exp } => {
                let (b, e) = (tr!(*base), tr!(*exp));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_power(ctx, b.raw(), e.raw()).expect("Z3_mk_power"),
                )
            }
            Intrinsic::RealAbs { val } => {
                let v = tr!(*val);
                let real_sort = z3_sys::Z3_mk_real_sort(ctx).expect("Z3_mk_real_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, real_sort).expect("Z3_mk_int");
                let neg = z3_sys::Z3_mk_unary_minus(ctx, v.raw()).expect("Z3_mk_unary_minus");
                let ge = z3_sys::Z3_mk_ge(ctx, v.raw(), zero).expect("Z3_mk_ge");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ite(ctx, ge, v.raw(), neg).expect("Z3_mk_ite"),
                )
            }
            Intrinsic::RealRound { val } => {
                // floor(x + 0.5)
                let v = tr!(*val);
                let real_sort = z3_sys::Z3_mk_real_sort(ctx).expect("Z3_mk_real_sort");
                let half = z3_sys::Z3_mk_real(ctx, 1, 2).expect("Z3_mk_real");
                let sum = z3_sys::Z3_mk_add(ctx, 2, [v.raw(), half].as_ptr()).expect("Z3_mk_add");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_real2int(ctx, sum).expect("Z3_mk_real2int"),
                )
            }
            Intrinsic::RealFloor { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_real2int(ctx, v.raw()).expect("Z3_mk_real2int"),
                )
            }
            Intrinsic::RealCeil { val } => {
                // -floor(-x)
                let v = tr!(*val);
                let neg = z3_sys::Z3_mk_unary_minus(ctx, v.raw()).expect("Z3_mk_unary_minus");
                let floor = z3_sys::Z3_mk_real2int(ctx, neg).expect("Z3_mk_real2int");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_unary_minus(ctx, floor).expect("Z3_mk_unary_minus"),
                )
            }
            Intrinsic::RealIsInt { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_is_int(ctx, v.raw()).expect("Z3_mk_is_int"),
                )
            }
            Intrinsic::RealLt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_lt(ctx, l.raw(), r.raw()).expect("Z3_mk_lt"),
                )
            }
            Intrinsic::RealLe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_le(ctx, l.raw(), r.raw()).expect("Z3_mk_le"),
                )
            }
            Intrinsic::RealGt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_gt(ctx, l.raw(), r.raw()).expect("Z3_mk_gt"),
                )
            }
            Intrinsic::RealGe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ge(ctx, l.raw(), r.raw()).expect("Z3_mk_ge"),
                )
            }
            Intrinsic::RealToInt { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_real2int(ctx, v.raw()).expect("Z3_mk_real2int"),
                )
            }
            Intrinsic::RealToF32 { val } => {
                let v = tr!(*val);
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                let fp = z3_sys::Z3_mk_fpa_sort(ctx, 8, 24).expect("Z3_mk_fpa_sort");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_fp_real(ctx, rne, v.raw(), fp)
                        .expect("Z3_mk_fpa_to_fp_real"),
                )
            }
            Intrinsic::RealToF64 { val } => {
                let v = tr!(*val);
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                let fp = z3_sys::Z3_mk_fpa_sort(ctx, 11, 53).expect("Z3_mk_fpa_sort");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_fp_real(ctx, rne, v.raw(), fp)
                        .expect("Z3_mk_fpa_to_fp_real"),
                )
            }

            // --- String ---
            Intrinsic::StrVal(s) => {
                let c = std::ffi::CString::new(s.as_str()).unwrap();
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("Z3_mk_string"),
                )
            }
            Intrinsic::StrNew => {
                let c = std::ffi::CString::new("").unwrap();
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("Z3_mk_string"),
                )
            }
            Intrinsic::StrLen { seq } => {
                let s = tr!(*seq);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_length(ctx, s.raw()).expect("Z3_mk_seq_length"),
                )
            }
            Intrinsic::StrConcat { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_concat(ctx, 2, [l.raw(), r.raw()].as_ptr())
                        .expect("Z3_mk_seq_concat"),
                )
            }
            Intrinsic::StrAt { seq, idx } => {
                let (s, i) = (tr!(*seq), tr!(*idx));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_at(ctx, s.raw(), i.raw()).expect("Z3_mk_seq_at"),
                )
            }
            Intrinsic::StrIndexOf { seq, sub, offset } => {
                let (s, sub_v, off) = (tr!(*seq), tr!(*sub), tr!(*offset));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_index(ctx, s.raw(), sub_v.raw(), off.raw())
                        .expect("Z3_mk_seq_index"),
                )
            }
            Intrinsic::StrIndexOfDefault { seq, sub } => {
                let (s, sub_v) = (tr!(*seq), tr!(*sub));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_index(ctx, s.raw(), sub_v.raw(), zero)
                        .expect("Z3_mk_seq_index"),
                )
            }
            Intrinsic::StrSubstr { seq, start, len } => {
                let (s, st, ln) = (tr!(*seq), tr!(*start), tr!(*len));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_extract(ctx, s.raw(), st.raw(), ln.raw())
                        .expect("Z3_mk_seq_extract"),
                )
            }
            Intrinsic::StrIsEmpty { seq } => {
                let s = tr!(*seq);
                let len = z3_sys::Z3_mk_seq_length(ctx, s.raw()).expect("Z3_mk_seq_length");
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                Z3Ast::new(ctx, z3_sys::Z3_mk_eq(ctx, len, zero).expect("Z3_mk_eq"))
            }
            Intrinsic::StrContains { seq, item } => {
                let (s, i) = (tr!(*seq), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_contains(ctx, s.raw(), i.raw()).expect("Z3_mk_seq_contains"),
                )
            }
            Intrinsic::StrStartsWith { seq, item } => {
                let (s, i) = (tr!(*seq), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_prefix(ctx, i.raw(), s.raw()).expect("Z3_mk_seq_prefix"),
                )
            }
            Intrinsic::StrEndsWith { seq, item } => {
                let (s, i) = (tr!(*seq), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_suffix(ctx, i.raw(), s.raw()).expect("Z3_mk_seq_suffix"),
                )
            }
            Intrinsic::StrIsDigit { seq } => {
                let s = tr!(*seq);
                let lo = {
                    let c = std::ffi::CString::new("0").unwrap();
                    z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("Z3_mk_string")
                };
                let hi = {
                    let c = std::ffi::CString::new("9").unwrap();
                    z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("Z3_mk_string")
                };
                let re = z3_sys::Z3_mk_re_range(ctx, lo, hi).expect("Z3_mk_re_range");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_in_re(ctx, s.raw(), re).expect("Z3_mk_seq_in_re"),
                )
            }
            Intrinsic::StrLe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_str_le(ctx, l.raw(), r.raw()),
                )
            }
            Intrinsic::StrLt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_str_lt(ctx, l.raw(), r.raw()),
                )
            }
            Intrinsic::StrGe { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_str_le(ctx, r.raw(), l.raw()),
                )
            }
            Intrinsic::StrGt { lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_str_lt(ctx, r.raw(), l.raw()),
                )
            }
            Intrinsic::StrReplace { seq, src, dst } => {
                let (s, sr, ds) = (tr!(*seq), tr!(*src), tr!(*dst));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_replace(ctx, s.raw(), sr.raw(), ds.raw())
                        .expect("Z3_mk_seq_replace"),
                )
            }
            Intrinsic::StrReplaceAll { seq, src, dst } => {
                // Z3 doesn't have replace_all in the C API; approximate with single replace
                let (s, sr, ds) = (tr!(*seq), tr!(*src), tr!(*dst));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_replace(ctx, s.raw(), sr.raw(), ds.raw())
                        .expect("Z3_mk_seq_replace"),
                )
            }
            Intrinsic::StrToInt { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_str_to_int(ctx, v.raw()).expect("Z3_mk_str_to_int"),
                )
            }
            Intrinsic::StrFromInt { val } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_int_to_str(ctx, v.raw()).expect("Z3_mk_int_to_str"),
                )
            }
            Intrinsic::StrFromCode { val } => {
                let v = tr!(*val);
                let bv2int = z3_sys::Z3_mk_bv2int(ctx, v.raw(), false).expect("Z3_mk_bv2int");
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_string_from_code(ctx, bv2int),
                )
            }
            Intrinsic::StrToCode { val } => {
                let v = tr!(*val);
                let code = crate::backend::z3_api::Z3_mk_string_to_code(ctx, v.raw());
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_int2bv(ctx, 32, code).expect("Z3_mk_int2bv"),
                )
            }

            // --- Box (passthrough) ---
            Intrinsic::BoxShield { val, .. } | Intrinsic::BoxReveal { val, .. } => tr!(*val),

            // --- Sequence ---
            Intrinsic::SeqEmpty { t } => {
                let seq_sort = api_ctx.translate_sort(&Sort::Seq(Box::new(t.clone())));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_empty(ctx, seq_sort).expect("Z3_mk_seq_empty"),
                )
            }
            Intrinsic::SeqUnit { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_unit(ctx, v.raw()).expect("Z3_mk_seq_unit"),
                )
            }
            Intrinsic::SeqLen { seq, .. } => {
                let s = tr!(*seq);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_length(ctx, s.raw()).expect("Z3_mk_seq_length"),
                )
            }
            Intrinsic::SeqPush { seq, item, .. } => {
                let (s, i) = (tr!(*seq), tr!(*item));
                let unit = z3_sys::Z3_mk_seq_unit(ctx, i.raw()).expect("Z3_mk_seq_unit");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_concat(ctx, 2, [s.raw(), unit].as_ptr())
                        .expect("Z3_mk_seq_concat"),
                )
            }
            Intrinsic::SeqConcat { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_concat(ctx, 2, [l.raw(), r.raw()].as_ptr())
                        .expect("Z3_mk_seq_concat"),
                )
            }
            Intrinsic::SeqNth { seq, idx, .. } => {
                let (s, i) = (tr!(*seq), tr!(*idx));
                Z3Ast::new(
                    ctx,
                    crate::backend::z3_api::Z3_mk_seq_nth(ctx, s.raw(), i.raw()),
                )
            }
            Intrinsic::SeqAtSeq { seq, idx, .. } => {
                let (s, i) = (tr!(*seq), tr!(*idx));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let one = z3_sys::Z3_mk_int(ctx, 1, int_sort).expect("Z3_mk_int");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_extract(ctx, s.raw(), i.raw(), one)
                        .expect("Z3_mk_seq_extract"),
                )
            }
            Intrinsic::SeqExtract {
                seq, offset, len, ..
            } => {
                let (s, o, l) = (tr!(*seq), tr!(*offset), tr!(*len));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_extract(ctx, s.raw(), o.raw(), l.raw())
                        .expect("Z3_mk_seq_extract"),
                )
            }
            Intrinsic::SeqIndexOf {
                seq, sub, offset, ..
            } => {
                let (s, su, o) = (tr!(*seq), tr!(*sub), tr!(*offset));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_index(ctx, s.raw(), su.raw(), o.raw())
                        .expect("Z3_mk_seq_index"),
                )
            }
            Intrinsic::SeqIndexOfDefault { seq, sub, .. } => {
                let (s, su) = (tr!(*seq), tr!(*sub));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_index(ctx, s.raw(), su.raw(), zero).expect("Z3_mk_seq_index"),
                )
            }
            Intrinsic::SeqContains { seq, item, .. } => {
                let (s, i) = (tr!(*seq), tr!(*item));
                let unit = z3_sys::Z3_mk_seq_unit(ctx, i.raw()).expect("Z3_mk_seq_unit");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_contains(ctx, s.raw(), unit).expect("Z3_mk_seq_contains"),
                )
            }
            Intrinsic::SeqPrefixOf { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_prefix(ctx, l.raw(), r.raw()).expect("Z3_mk_seq_prefix"),
                )
            }
            Intrinsic::SeqSuffixOf { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_suffix(ctx, l.raw(), r.raw()).expect("Z3_mk_seq_suffix"),
                )
            }
            Intrinsic::SeqReplace { seq, src, dst, .. } => {
                let (s, sr, ds) = (tr!(*seq), tr!(*src), tr!(*dst));
                let src_unit = z3_sys::Z3_mk_seq_unit(ctx, sr.raw()).expect("Z3_mk_seq_unit");
                let dst_unit = z3_sys::Z3_mk_seq_unit(ctx, ds.raw()).expect("Z3_mk_seq_unit");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_seq_replace(ctx, s.raw(), src_unit, dst_unit)
                        .expect("Z3_mk_seq_replace"),
                )
            }
            Intrinsic::SeqIsEmpty { seq, .. } => {
                let s = tr!(*seq);
                let len = z3_sys::Z3_mk_seq_length(ctx, s.raw()).expect("Z3_mk_seq_length");
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                Z3Ast::new(ctx, z3_sys::Z3_mk_eq(ctx, len, zero).expect("Z3_mk_eq"))
            }

            // --- Set ---
            Intrinsic::SetEmpty { t } => {
                let elem_sort = api_ctx.translate_sort(t);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_empty_set(ctx, elem_sort).expect("Z3_mk_empty_set"),
                )
            }
            Intrinsic::SetLen { set, .. } => {
                // set.card - no direct z3-sys binding, use mk_set_card if available
                // For now approximate: this isn't commonly used
                let s = tr!(*set);
                // Z3 doesn't have set.card in the C API in older versions
                // We'll create an uninterpreted function for it
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let set_sort = z3_sys::Z3_get_sort(ctx, s.raw()).expect("Z3_get_sort");
                let name = mk_string_symbol(ctx, "set.card");
                let decl = z3_sys::Z3_mk_func_decl(ctx, name, 1, [set_sort].as_ptr(), int_sort)
                    .expect("Z3_mk_func_decl");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_app(ctx, decl, 1, [s.raw()].as_ptr()).expect("Z3_mk_app"),
                )
            }
            Intrinsic::SetInsert { set, item, .. } => {
                let (s, i) = (tr!(*set), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_add(ctx, s.raw(), i.raw()).expect("Z3_mk_set_add"),
                )
            }
            Intrinsic::SetRemove { set, item, .. } => {
                let (s, i) = (tr!(*set), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_del(ctx, s.raw(), i.raw()).expect("Z3_mk_set_del"),
                )
            }
            Intrinsic::SetContains { set, item, .. } => {
                let (s, i) = (tr!(*set), tr!(*item));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_member(ctx, i.raw(), s.raw()).expect("Z3_mk_set_member"),
                )
            }
            Intrinsic::SetIsEmpty { set, .. } => {
                // card == 0
                let s = tr!(*set);
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let set_sort = z3_sys::Z3_get_sort(ctx, s.raw()).expect("Z3_get_sort");
                let name = mk_string_symbol(ctx, "set.card");
                let decl = z3_sys::Z3_mk_func_decl(ctx, name, 1, [set_sort].as_ptr(), int_sort)
                    .expect("Z3_mk_func_decl");
                let card = z3_sys::Z3_mk_app(ctx, decl, 1, [s.raw()].as_ptr()).expect("Z3_mk_app");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                Z3Ast::new(ctx, z3_sys::Z3_mk_eq(ctx, card, zero).expect("Z3_mk_eq"))
            }
            Intrinsic::SetIntersect { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_intersect(ctx, 2, [l.raw(), r.raw()].as_ptr())
                        .expect("Z3_mk_set_intersect"),
                )
            }
            Intrinsic::SetUnion { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_union(ctx, 2, [l.raw(), r.raw()].as_ptr())
                        .expect("Z3_mk_set_union"),
                )
            }
            Intrinsic::SetDiff { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_difference(ctx, l.raw(), r.raw())
                        .expect("Z3_mk_set_difference"),
                )
            }
            Intrinsic::SetSymDiff { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let diff1 = z3_sys::Z3_mk_set_difference(ctx, l.raw(), r.raw())
                    .expect("Z3_mk_set_difference");
                let diff2 = z3_sys::Z3_mk_set_difference(ctx, r.raw(), l.raw())
                    .expect("Z3_mk_set_difference");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_union(ctx, 2, [diff1, diff2].as_ptr())
                        .expect("Z3_mk_set_union"),
                )
            }
            Intrinsic::SetIsSubset { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_set_subset(ctx, l.raw(), r.raw()).expect("Z3_mk_set_subset"),
                )
            }
            Intrinsic::SetIsProperSubset { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let subset =
                    z3_sys::Z3_mk_set_subset(ctx, l.raw(), r.raw()).expect("Z3_mk_set_subset");
                let neq = z3_sys::Z3_mk_not(
                    ctx,
                    z3_sys::Z3_mk_eq(ctx, l.raw(), r.raw()).expect("Z3_mk_eq"),
                )
                .expect("Z3_mk_not");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_and(ctx, 2, [subset, neq].as_ptr()).expect("Z3_mk_and"),
                )
            }
            Intrinsic::SetIsDisjoint { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let inter = z3_sys::Z3_mk_set_intersect(ctx, 2, [l.raw(), r.raw()].as_ptr())
                    .expect("Z3_mk_set_intersect");
                let elem_sort = api_ctx.translate_sort(t);
                let empty = z3_sys::Z3_mk_empty_set(ctx, elem_sort).expect("Z3_mk_empty_set");
                Z3Ast::new(ctx, z3_sys::Z3_mk_eq(ctx, inter, empty).expect("Z3_mk_eq"))
            }
            Intrinsic::SetHasSize { set, size, .. } => {
                let (s, sz) = (tr!(*set), tr!(*size));
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let set_sort = z3_sys::Z3_get_sort(ctx, s.raw()).expect("Z3_get_sort");
                let name = mk_string_symbol(ctx, "set.card");
                let decl = z3_sys::Z3_mk_func_decl(ctx, name, 1, [set_sort].as_ptr(), int_sort)
                    .expect("Z3_mk_func_decl");
                let card = z3_sys::Z3_mk_app(ctx, decl, 1, [s.raw()].as_ptr()).expect("Z3_mk_app");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_eq(ctx, card, sz.raw()).expect("Z3_mk_eq"),
                )
            }

            // --- Array ---
            Intrinsic::ArrayEmpty { k, v } => {
                let k_sort = api_ctx.translate_sort(k);
                let null = api_ctx.get_null_const(&array_null_const_name(v, api_ctx.ir));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_const_array(ctx, k_sort, null).expect("Z3_mk_const_array"),
                )
            }
            Intrinsic::ArrayLen { arr, v, .. } => {
                let a = tr!(*arr);
                let null = api_ctx.get_null_const(&array_null_const_name(v, api_ctx.ir));
                // Approximate: ite forall(k, select(arr,k)==null) 0 1
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("Z3_mk_int");
                let one = z3_sys::Z3_mk_int(ctx, 1, int_sort).expect("Z3_mk_int");
                // Simplified: just return 0 or 1 based on whether array is empty
                // This matches the text backend's approximation
                let str_sort = z3_sys::Z3_mk_string_sort(ctx).expect("Z3_mk_string_sort");
                let k_sym = mk_string_symbol(ctx, "_ak_");
                let k_var = z3_sys::Z3_mk_const(ctx, k_sym, str_sort).expect("Z3_mk_const");
                let sel = z3_sys::Z3_mk_select(ctx, a.raw(), k_var).expect("Z3_mk_select");
                let eq_null = z3_sys::Z3_mk_eq(ctx, sel, null).expect("Z3_mk_eq");
                let k_app = z3_sys::Z3_to_app(ctx, k_var).expect("Z3_to_app");
                let forall = z3_sys::Z3_mk_forall_const(
                    ctx,
                    0,
                    1,
                    [k_app].as_ptr(),
                    0,
                    std::ptr::null(),
                    eq_null,
                )
                .expect("Z3_mk_forall_const");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ite(ctx, forall, zero, one).expect("Z3_mk_ite"),
                )
            }
            Intrinsic::ArrayStore { arr, key, val, .. } => {
                let (a, k, v) = (tr!(*arr), tr!(*key), tr!(*val));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_store(ctx, a.raw(), k.raw(), v.raw()).expect("Z3_mk_store"),
                )
            }
            Intrinsic::ArraySelect { arr, key, .. } => {
                let (a, k) = (tr!(*arr), tr!(*key));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_select(ctx, a.raw(), k.raw()).expect("Z3_mk_select"),
                )
            }
            Intrinsic::ArrayRemove { arr, key, v, .. } => {
                let (a, k) = (tr!(*arr), tr!(*key));
                let null = api_ctx.get_null_const(&array_null_const_name(v, api_ctx.ir));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_store(ctx, a.raw(), k.raw(), null).expect("Z3_mk_store"),
                )
            }
            Intrinsic::ArrayContainsKey { arr, key, v, .. } => {
                let (a, k) = (tr!(*arr), tr!(*key));
                let null = api_ctx.get_null_const(&array_null_const_name(v, api_ctx.ir));
                let sel = z3_sys::Z3_mk_select(ctx, a.raw(), k.raw()).expect("Z3_mk_select");
                let eq = z3_sys::Z3_mk_eq(ctx, sel, null).expect("Z3_mk_eq");
                Z3Ast::new(ctx, z3_sys::Z3_mk_not(ctx, eq).expect("Z3_mk_not"))
            }
            Intrinsic::ArrayIsEmpty { arr, k, v } => {
                let a = tr!(*arr);
                let null = api_ctx.get_null_const(&array_null_const_name(v, api_ctx.ir));
                let k_sort = api_ctx.translate_sort(k);
                let k_sym = mk_string_symbol(ctx, "_ak_");
                let k_var = z3_sys::Z3_mk_const(ctx, k_sym, k_sort).expect("Z3_mk_const");
                let sel = z3_sys::Z3_mk_select(ctx, a.raw(), k_var).expect("Z3_mk_select");
                let eq = z3_sys::Z3_mk_eq(ctx, sel, null).expect("Z3_mk_eq");
                let k_app = z3_sys::Z3_to_app(ctx, k_var).expect("Z3_to_app");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_forall_const(
                        ctx,
                        0,
                        1,
                        [k_app].as_ptr(),
                        0,
                        std::ptr::null(),
                        eq,
                    )
                    .expect("Z3_mk_forall_const"),
                )
            }

            // --- Bitvector ---
            Intrinsic::BvVal { t, val } => {
                let width: u32 = match t {
                    Sort::I32 | Sort::U32 => 32,
                    _ => 64,
                };
                let unsigned_val = if val.sign() == num_bigint::Sign::Minus {
                    val + (BigInt::from(1u64) << width)
                } else {
                    val.clone()
                };
                let bv_sort = z3_sys::Z3_mk_bv_sort(ctx, width).expect("Z3_mk_bv_sort");
                let s = std::ffi::CString::new(unsigned_val.to_string()).unwrap();
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_numeral(ctx, s.as_ptr(), bv_sort).expect("Z3_mk_numeral"),
                )
            }
            Intrinsic::BvNot { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(ctx, z3_sys::Z3_mk_bvnot(ctx, v.raw()).expect("Z3_mk_bvnot"))
            }
            Intrinsic::BvAnd { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvand(ctx, l.raw(), r.raw()).expect("Z3_mk_bvand"),
                )
            }
            Intrinsic::BvOr { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvor(ctx, l.raw(), r.raw()).expect("Z3_mk_bvor"),
                )
            }
            Intrinsic::BvXor { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvxor(ctx, l.raw(), r.raw()).expect("Z3_mk_bvxor"),
                )
            }
            Intrinsic::BvNand { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvnand(ctx, l.raw(), r.raw()).expect("Z3_mk_bvnand"),
                )
            }
            Intrinsic::BvNor { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvnor(ctx, l.raw(), r.raw()).expect("Z3_mk_bvnor"),
                )
            }
            Intrinsic::BvXnor { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvxnor(ctx, l.raw(), r.raw()).expect("Z3_mk_bvxnor"),
                )
            }
            Intrinsic::BvRedAnd { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvredand(ctx, v.raw()).expect("Z3_mk_bvredand"),
                )
            }
            Intrinsic::BvRedOr { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvredor(ctx, v.raw()).expect("Z3_mk_bvredor"),
                )
            }
            Intrinsic::BvNeg { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(ctx, z3_sys::Z3_mk_bvneg(ctx, v.raw()).expect("Z3_mk_bvneg"))
            }
            Intrinsic::BvAdd { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvadd(ctx, l.raw(), r.raw()).expect("Z3_mk_bvadd"),
                )
            }
            Intrinsic::BvSub { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvsub(ctx, l.raw(), r.raw()).expect("Z3_mk_bvsub"),
                )
            }
            Intrinsic::BvMul { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvmul(ctx, l.raw(), r.raw()).expect("Z3_mk_bvmul"),
                )
            }
            Intrinsic::BvDiv { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsdiv,
                    _ => z3_sys::Z3_mk_bvudiv,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvdiv"))
            }
            Intrinsic::BvRem { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsrem,
                    _ => z3_sys::Z3_mk_bvurem,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvrem"))
            }
            Intrinsic::BvMod { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsmod,
                    _ => z3_sys::Z3_mk_bvurem,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvmod"))
            }
            Intrinsic::BvShl { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvshl(ctx, l.raw(), r.raw()).expect("Z3_mk_bvshl"),
                )
            }
            Intrinsic::BvLshr { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvlshr(ctx, l.raw(), r.raw()).expect("Z3_mk_bvlshr"),
                )
            }
            Intrinsic::BvAshr { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_bvashr(ctx, l.raw(), r.raw()).expect("Z3_mk_bvashr"),
                )
            }
            Intrinsic::BvRotLeft { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ext_rotate_left(ctx, l.raw(), r.raw())
                        .expect("Z3_mk_ext_rotate_left"),
                )
            }
            Intrinsic::BvRotRight { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_ext_rotate_right(ctx, l.raw(), r.raw())
                        .expect("Z3_mk_ext_rotate_right"),
                )
            }
            Intrinsic::BvLt { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvslt,
                    _ => z3_sys::Z3_mk_bvult,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvcmp"))
            }
            Intrinsic::BvLe { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsle,
                    _ => z3_sys::Z3_mk_bvule,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvcmp"))
            }
            Intrinsic::BvGt { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsgt,
                    _ => z3_sys::Z3_mk_bvugt,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvcmp"))
            }
            Intrinsic::BvGe { t, lhs, rhs } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let f = match t {
                    Sort::I32 | Sort::I64 => z3_sys::Z3_mk_bvsge,
                    _ => z3_sys::Z3_mk_bvuge,
                };
                Z3Ast::new(ctx, f(ctx, l.raw(), r.raw()).expect("Z3_mk_bvcmp"))
            }
            Intrinsic::BvToInt { t, val } => {
                let v = tr!(*val);
                match t {
                    Sort::I32 => {
                        // signed: ite (bvslt val 0) (- (bv2int val) 2^32) (bv2int val)
                        let bv_sort = z3_sys::Z3_mk_bv_sort(ctx, 32).expect("Z3_mk_bv_sort");
                        let zero_bv = z3_sys::Z3_mk_int(ctx, 0, bv_sort).expect("Z3_mk_int");
                        let is_neg =
                            z3_sys::Z3_mk_bvslt(ctx, v.raw(), zero_bv).expect("Z3_mk_bvslt");
                        let unsigned =
                            z3_sys::Z3_mk_bv2int(ctx, v.raw(), false).expect("Z3_mk_bv2int");
                        let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                        let two32 = {
                            let s = std::ffi::CString::new("4294967296").unwrap();
                            z3_sys::Z3_mk_numeral(ctx, s.as_ptr(), int_sort).expect("Z3_mk_numeral")
                        };
                        let signed = z3_sys::Z3_mk_sub(ctx, 2, [unsigned, two32].as_ptr())
                            .expect("Z3_mk_sub");
                        Z3Ast::new(
                            ctx,
                            z3_sys::Z3_mk_ite(ctx, is_neg, signed, unsigned).expect("Z3_mk_ite"),
                        )
                    }
                    Sort::I64 => {
                        let bv_sort = z3_sys::Z3_mk_bv_sort(ctx, 64).expect("Z3_mk_bv_sort");
                        let zero_bv = z3_sys::Z3_mk_int(ctx, 0, bv_sort).expect("Z3_mk_int");
                        let is_neg =
                            z3_sys::Z3_mk_bvslt(ctx, v.raw(), zero_bv).expect("Z3_mk_bvslt");
                        let unsigned =
                            z3_sys::Z3_mk_bv2int(ctx, v.raw(), false).expect("Z3_mk_bv2int");
                        let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                        let two64 = {
                            let s = std::ffi::CString::new("18446744073709551616").unwrap();
                            z3_sys::Z3_mk_numeral(ctx, s.as_ptr(), int_sort).expect("Z3_mk_numeral")
                        };
                        let signed = z3_sys::Z3_mk_sub(ctx, 2, [unsigned, two64].as_ptr())
                            .expect("Z3_mk_sub");
                        Z3Ast::new(
                            ctx,
                            z3_sys::Z3_mk_ite(ctx, is_neg, signed, unsigned).expect("Z3_mk_ite"),
                        )
                    }
                    _ => Z3Ast::new(
                        ctx,
                        z3_sys::Z3_mk_bv2int(ctx, v.raw(), false).expect("Z3_mk_bv2int"),
                    ),
                }
            }

            // --- Float ---
            Intrinsic::FloatVal { t, val } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                let fp_sort = z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort");
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                let real_sort = z3_sys::Z3_mk_real_sort(ctx).expect("Z3_mk_real_sort");
                let num = std::ffi::CString::new(val.numer().to_string()).unwrap();
                let den = std::ffi::CString::new(val.denom().to_string()).unwrap();
                let n = z3_sys::Z3_mk_numeral(ctx, num.as_ptr(), real_sort).expect("Z3_mk_numeral");
                let d = z3_sys::Z3_mk_numeral(ctx, den.as_ptr(), real_sort).expect("Z3_mk_numeral");
                let real_val = z3_sys::Z3_mk_div(ctx, n, d).expect("Z3_mk_div");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_fp_real(ctx, rne, real_val, fp_sort)
                        .expect("Z3_mk_fpa_to_fp_real"),
                )
            }
            Intrinsic::FloatAdd { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_add(ctx, rne, l.raw(), r.raw()).expect("Z3_mk_fpa_add"),
                )
            }
            Intrinsic::FloatSub { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_sub(ctx, rne, l.raw(), r.raw()).expect("Z3_mk_fpa_sub"),
                )
            }
            Intrinsic::FloatMul { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_mul(ctx, rne, l.raw(), r.raw()).expect("Z3_mk_fpa_mul"),
                )
            }
            Intrinsic::FloatDiv { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_div(ctx, rne, l.raw(), r.raw()).expect("Z3_mk_fpa_div"),
                )
            }
            Intrinsic::FloatNeg { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_neg(ctx, v.raw()).expect("Z3_mk_fpa_neg"),
                )
            }
            Intrinsic::FloatAbs { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_abs(ctx, v.raw()).expect("Z3_mk_fpa_abs"),
                )
            }
            Intrinsic::FloatRem { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_rem(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_rem"),
                )
            }
            Intrinsic::FloatSqrt { val, .. } => {
                let v = tr!(*val);
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_sqrt(ctx, rne, v.raw()).expect("Z3_mk_fpa_sqrt"),
                )
            }
            Intrinsic::FloatMin { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_min(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_min"),
                )
            }
            Intrinsic::FloatMax { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_max(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_max"),
                )
            }
            Intrinsic::FloatIsNaN { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_nan(ctx, v.raw()).expect("Z3_mk_fpa_is_nan"),
                )
            }
            Intrinsic::FloatIsInf { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_infinite(ctx, v.raw()).expect("Z3_mk_fpa_is_infinite"),
                )
            }
            Intrinsic::FloatIsZero { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_zero(ctx, v.raw()).expect("Z3_mk_fpa_is_zero"),
                )
            }
            Intrinsic::FloatIsNormal { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_normal(ctx, v.raw()).expect("Z3_mk_fpa_is_normal"),
                )
            }
            Intrinsic::FloatIsSubnormal { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_subnormal(ctx, v.raw()).expect("Z3_mk_fpa_is_subnormal"),
                )
            }
            Intrinsic::FloatIsNeg { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_negative(ctx, v.raw()).expect("Z3_mk_fpa_is_negative"),
                )
            }
            Intrinsic::FloatIsPos { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_is_positive(ctx, v.raw()).expect("Z3_mk_fpa_is_positive"),
                )
            }
            Intrinsic::FloatLt { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_lt(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_lt"),
                )
            }
            Intrinsic::FloatLe { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_leq(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_leq"),
                )
            }
            Intrinsic::FloatGt { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_gt(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_gt"),
                )
            }
            Intrinsic::FloatGe { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_geq(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_geq"),
                )
            }
            Intrinsic::FloatNaN { t } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_nan(
                        ctx,
                        z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort"),
                    )
                    .expect("Z3_mk_fpa_nan"),
                )
            }
            Intrinsic::FloatPosInf { t } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_inf(
                        ctx,
                        z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort"),
                        false,
                    )
                    .expect("Z3_mk_fpa_inf"),
                )
            }
            Intrinsic::FloatNegInf { t } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_inf(
                        ctx,
                        z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort"),
                        true,
                    )
                    .expect("Z3_mk_fpa_inf"),
                )
            }
            Intrinsic::FloatPosZero { t } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_zero(
                        ctx,
                        z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort"),
                        false,
                    )
                    .expect("Z3_mk_fpa_zero"),
                )
            }
            Intrinsic::FloatNegZero { t } => {
                let (eb, sb) = match t {
                    Sort::F32 => (8u32, 24u32),
                    _ => (11, 53),
                };
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_zero(
                        ctx,
                        z3_sys::Z3_mk_fpa_sort(ctx, eb, sb).expect("Z3_mk_fpa_sort"),
                        true,
                    )
                    .expect("Z3_mk_fpa_zero"),
                )
            }
            Intrinsic::FloatToInt { val, .. } => {
                let v = tr!(*val);
                let real = z3_sys::Z3_mk_fpa_to_real(ctx, v.raw()).expect("Z3_mk_fpa_to_real");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_real2int(ctx, real).expect("Z3_mk_real2int"),
                )
            }
            Intrinsic::FloatToReal { val, .. } => {
                let v = tr!(*val);
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_real(ctx, v.raw()).expect("Z3_mk_fpa_to_real"),
                )
            }
            Intrinsic::FloatToU32 { val, .. } => {
                let v = tr!(*val);
                let rtz =
                    z3_sys::Z3_mk_fpa_round_toward_zero(ctx).expect("Z3_mk_fpa_round_toward_zero");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_ubv(ctx, rtz, v.raw(), 32).expect("Z3_mk_fpa_to_ubv"),
                )
            }
            Intrinsic::FloatToI32 { val, .. } => {
                let v = tr!(*val);
                let rtz =
                    z3_sys::Z3_mk_fpa_round_toward_zero(ctx).expect("Z3_mk_fpa_round_toward_zero");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_sbv(ctx, rtz, v.raw(), 32).expect("Z3_mk_fpa_to_sbv"),
                )
            }
            Intrinsic::FloatToU64 { val, .. } => {
                let v = tr!(*val);
                let rtz =
                    z3_sys::Z3_mk_fpa_round_toward_zero(ctx).expect("Z3_mk_fpa_round_toward_zero");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_ubv(ctx, rtz, v.raw(), 64).expect("Z3_mk_fpa_to_ubv"),
                )
            }
            Intrinsic::FloatToI64 { val, .. } => {
                let v = tr!(*val);
                let rtz =
                    z3_sys::Z3_mk_fpa_round_toward_zero(ctx).expect("Z3_mk_fpa_round_toward_zero");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_to_sbv(ctx, rtz, v.raw(), 64).expect("Z3_mk_fpa_to_sbv"),
                )
            }
            Intrinsic::FloatCeil { val, .. } => {
                let v = tr!(*val);
                let rtp = z3_sys::Z3_mk_fpa_round_toward_positive(ctx)
                    .expect("Z3_mk_fpa_round_toward_positive");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_round_to_integral(ctx, rtp, v.raw())
                        .expect("Z3_mk_fpa_round_to_integral"),
                )
            }
            Intrinsic::FloatFloor { val, .. } => {
                let v = tr!(*val);
                let rtn = z3_sys::Z3_mk_fpa_round_toward_negative(ctx)
                    .expect("Z3_mk_fpa_round_toward_negative");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_round_to_integral(ctx, rtn, v.raw())
                        .expect("Z3_mk_fpa_round_to_integral"),
                )
            }
            Intrinsic::FloatTrunc { val, .. } => {
                let v = tr!(*val);
                let rtz =
                    z3_sys::Z3_mk_fpa_round_toward_zero(ctx).expect("Z3_mk_fpa_round_toward_zero");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_round_to_integral(ctx, rtz, v.raw())
                        .expect("Z3_mk_fpa_round_to_integral"),
                )
            }
            Intrinsic::FloatNearest { val, .. } => {
                let v = tr!(*val);
                let rne = z3_sys::Z3_mk_fpa_round_nearest_ties_to_even(ctx)
                    .expect("Z3_mk_fpa_round_nearest_ties_to_even");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_round_to_integral(ctx, rne, v.raw())
                        .expect("Z3_mk_fpa_round_to_integral"),
                )
            }
            Intrinsic::FloatFqEq { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_fpa_eq(ctx, l.raw(), r.raw()).expect("Z3_mk_fpa_eq"),
                )
            }

            // --- Error ---
            Intrinsic::ErrFresh(id) => {
                let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
                let bool_sort = z3_sys::Z3_mk_bool_sort(ctx).expect("Z3_mk_bool_sort");
                let false_val = z3_sys::Z3_mk_false(ctx).expect("Z3_mk_false");
                let empty =
                    z3_sys::Z3_mk_const_array(ctx, int_sort, false_val).expect("Z3_mk_const_array");
                let idx = z3_sys::Z3_mk_int(ctx, *id as i32, int_sort).expect("Z3_mk_int");
                let true_val = z3_sys::Z3_mk_true(ctx).expect("Z3_mk_true");
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_store(ctx, empty, idx, true_val).expect("Z3_mk_store"),
                )
            }
            Intrinsic::ErrMerge { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                let bool_sort = z3_sys::Z3_mk_bool_sort(ctx).expect("Z3_mk_bool_sort");
                let or_decl = z3_sys::Z3_mk_func_decl(
                    ctx,
                    mk_string_symbol(ctx, "or"),
                    2,
                    [bool_sort, bool_sort].as_ptr(),
                    bool_sort,
                )
                .expect("Z3_mk_func_decl");
                // Z3_mk_map applies a function pointwise to arrays
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_map(ctx, or_decl, 2, [l.raw(), r.raw()].as_ptr())
                        .expect("Z3_mk_map"),
                )
            }

            // --- Equality ---
            Intrinsic::SmtEq { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_eq(ctx, l.raw(), r.raw()).expect("Z3_mk_eq"),
                )
            }
            Intrinsic::SmtNe { lhs, rhs, .. } => {
                let (l, r) = (tr!(*lhs), tr!(*rhs));
                Z3Ast::new(
                    ctx,
                    z3_sys::Z3_mk_not(
                        ctx,
                        z3_sys::Z3_mk_eq(ctx, l.raw(), r.raw()).expect("Z3_mk_eq"),
                    )
                    .expect("Z3_mk_not"),
                )
            }
        }
    }
}

/// Helper to create an integer comparison against a constant.
unsafe fn mk_int_cmp<'ctx>(
    ctx: z3_sys::Z3_context,
    val: &Z3Ast<'ctx>,
    op: &str,
    bound: &str,
) -> Z3Ast<'ctx> {
    let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("Z3_mk_int_sort");
    let c = std::ffi::CString::new(bound).unwrap();
    let bound_val = z3_sys::Z3_mk_numeral(ctx, c.as_ptr(), int_sort).expect("Z3_mk_numeral");
    let result = match op {
        ">" => z3_sys::Z3_mk_gt(ctx, val.raw(), bound_val).expect("Z3_mk_gt"),
        "<" => z3_sys::Z3_mk_lt(ctx, val.raw(), bound_val).expect("Z3_mk_lt"),
        _ => unreachable!(),
    };
    Z3Ast::new(ctx, result)
}

//! System types and functions of Rusmart

use crate::parser::expr::Expr;
use crate::parser::infer::TypeRef;
use crate::parser::name::UsrFuncName;
use crate::parser::ty::SysTypeName;
use crate::{bail_if_exists, bail_if_missing, bail_on};
use anyhow::bail;
use num_bigint::BigInt;
use num_rational::BigRational;
use std::fmt::{Display, Formatter};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Expr as Exp, ExprLit, ExprUnary, Lit, Result};

/// Intrinsic procedure
#[derive(Clone, Debug)]
pub enum Intrinsic {
    /// `Boolean::from`
    BoolVal(bool),
    /// `Boolean::not`
    BoolNot { val: Expr },
    /// `Boolean::and`
    BoolAnd { lhs: Expr, rhs: Expr },
    /// `Boolean::or`
    BoolOr { lhs: Expr, rhs: Expr },
    /// `Boolean::xor`
    BoolXor { lhs: Expr, rhs: Expr },
    /// `Boolean::implies`
    BoolImplies { lhs: Expr, rhs: Expr },
    /// `Boolean::iff`
    BoolIff { lhs: Expr, rhs: Expr },
    /// `Boolean::nand`
    BoolNand { lhs: Expr, rhs: Expr },
    /// `Boolean::nor`
    BoolNor { lhs: Expr, rhs: Expr },
    /// `Boolean::xnor`
    BoolXnor { lhs: Expr, rhs: Expr },
    /// `Integer::from` (Literal)
    IntVal(BigInt),
    // Arithmetic
    /// `Integer::neg`
    IntNeg { val: Expr },
    /// `Integer::add`
    IntAdd { lhs: Expr, rhs: Expr },
    /// `Integer::sub`
    IntSub { lhs: Expr, rhs: Expr },
    /// `Integer::mul`
    IntMul { lhs: Expr, rhs: Expr },
    /// `Integer::div`
    IntDiv { lhs: Expr, rhs: Expr },
    /// `Integer::modulo`
    IntMod { lhs: Expr, rhs: Expr },
    /// `Integer::rem` (Rust % operator behavior)
    IntRem { lhs: Expr, rhs: Expr },
    /// `Integer::pow`
    IntPow { base: Expr, exp: Expr },
    /// `Integer::abs`
    IntAbs { val: Expr },
    // Predicates
    /// `Integer::divides` (Check if lhs divides rhs)
    IntDivides { lhs: Expr, rhs: Expr },
    /// `Integer::lt`
    IntLt { lhs: Expr, rhs: Expr },
    /// `Integer::le`
    IntLe { lhs: Expr, rhs: Expr },
    /// `Integer::gt`
    IntGt { lhs: Expr, rhs: Expr },
    /// `Integer::ge`
    IntGe { lhs: Expr, rhs: Expr },
    // Type Conversions
    /// `Integer::to_real`
    IntToReal { val: Expr },
    /// `Integer::to_i32`
    IntToI32 { val: Expr },
    /// `Integer::to_i64`
    IntToI64 { val: Expr },
    /// `Integer::to_u32`
    IntToU32 { val: Expr },
    /// `Integer::to_u64`
    IntToU64 { val: Expr },
    /// `Integer::to_f32`
    IntToF32 { val: Expr },
    /// `Integer::to_f64`
    IntToF64 { val: Expr },
    // String Parsing Constructors
    /// `Integer::from_hex_str`
    IntFromHex { val: Expr },
    /// `Integer::from_oct_str`
    IntFromOct { val: Expr },
    /// `Integer::from_bin_str`
    IntFromBin { val: Expr },
    // Range Checks
    /// `Integer::is_gt_i64_max`
    IntIsGtI64Max { val: Expr },
    /// `Integer::is_lt_i64_min`
    IntIsLtI64Min { val: Expr },
    /// `Integer::is_gt_u64_max`
    IntIsGtU64Max { val: Expr },
    /// `Integer::is_lt_u64_min`
    IntIsLtU64Min { val: Expr },
    /// `Integer::is_lt_i32_min`
    IntIsLtI32Min { val: Expr },
    /// `Integer::is_gt_i32_max`
    IntIsGtI32Max { val: Expr },
    /// `Integer::is_lt_u32_min`
    IntIsLtU32Min { val: Expr },
    /// `Integer::is_gt_u32_max`
    IntIsGtU32Max { val: Expr },
    /// `Real::from` (Literal)
    RealVal(BigRational),
    // Arithmetic
    /// `Real::neg`
    RealNeg { val: Expr },
    /// `Real::add`
    RealAdd { lhs: Expr, rhs: Expr },
    /// `Real::sub`
    RealSub { lhs: Expr, rhs: Expr },
    /// `Real::mul`
    RealMul { lhs: Expr, rhs: Expr },
    /// `Real::div`
    RealDiv { lhs: Expr, rhs: Expr },
    /// `Real::pow`
    RealPow { base: Expr, exp: Expr },
    /// `Real::abs`
    RealAbs { val: Expr },
    /// `Real::round`
    RealRound { val: Expr },
    /// `Real::floor`
    RealFloor { val: Expr },
    /// `Real::ceil`
    RealCeil { val: Expr },
    /// `Real::is_integer`
    RealIsInt { val: Expr },
    /// `Real::lt`
    RealLt { lhs: Expr, rhs: Expr },
    /// `Real::le`
    RealLe { lhs: Expr, rhs: Expr },
    /// `Real::gt`
    RealGt { lhs: Expr, rhs: Expr },
    /// `Real::ge`
    RealGe { lhs: Expr, rhs: Expr },
    /// `Real::to_int` (Truncate to Integer)
    RealToInt { val: Expr },
    /// `Real::to_f32`
    RealToF32 { val: Expr },
    /// `Real::to_f64`
    RealToF64 { val: Expr },
    /// `Real::numerator`
    RealNumer { val: Expr },
    /// `Real::denominator`
    RealDenom { val: Expr },
    /// `String::from` (Literal)
    StrVal(String),
    /// `String::length`
    StrLen { seq: Expr },
    /// `String::concat`
    StrConcat { lhs: Expr, rhs: Expr },
    /// `String::at`
    StrAt { seq: Expr, idx: Expr },
    /// `String::is_empty`
    StrIsEmpty { seq: Expr },
    /// `String::contains`
    StrContains { seq: Expr, item: Expr },
    /// `String::starts_with`
    StrStartsWith { seq: Expr, item: Expr },
    /// `String::ends_with`
    StrEndsWith { seq: Expr, item: Expr },
    /// `String::is_digit`
    StrIsDigit { seq: Expr },
    /// `String::le`
    StrLe { lhs: Expr, rhs: Expr },
    /// `String::lt`
    StrLt { lhs: Expr, rhs: Expr },
    /// `String::ge`
    StrGe { lhs: Expr, rhs: Expr },
    /// `String::gt`
    StrGt { lhs: Expr, rhs: Expr },
    /// `String::index_of`
    StrIndexOf { seq: Expr, sub: Expr, offset: Expr },
    /// `String::replace` (Single occurrence)
    StrReplace { seq: Expr, src: Expr, dst: Expr },
    /// `String::replace_all` (All occurrences)
    StrReplaceAll { seq: Expr, src: Expr, dst: Expr },
    /// `String::to_int`
    StrToInt { val: Expr },
    /// `String::from_int`
    StrFromInt { val: Expr },
    /// `String::from_code`
    StrFromCode { val: Expr },
    /// `String::to_code`
    StrToCode { val: Expr },
    /// `Cloak::shield`
    BoxShield { t: TypeRef, val: Expr },
    /// `Cloak::reveal`
    BoxReveal { t: TypeRef, val: Expr },
    /// `Seq::new` (Empty sequence)
    SeqEmpty { t: TypeRef },
    /// `Seq::unit` (Create singleton sequence [e])
    SeqUnit { t: TypeRef, val: Expr },
    /// `Seq::length`
    SeqLen { t: TypeRef, seq: Expr },
    /// `Seq::at` (Get element at index, corresponds to `seq.nth`)
    SeqNth { t: TypeRef, seq: Expr, idx: Expr },
    /// `Seq::extract` (Sub-sequence, corresponds to `seq.extract`)
    SeqExtract {
        t: TypeRef,
        seq: Expr,
        offset: Expr,
        len: Expr,
    },
    /// `Seq::append` (Push single element to end)
    SeqPush { t: TypeRef, seq: Expr, item: Expr },
    /// `Seq::concat` (Join two sequences)
    SeqConcat { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Seq::contains` (Check if sequence contains element)
    SeqContains { t: TypeRef, seq: Expr, item: Expr },
    /// `Seq::prefix_of` (Check if self is prefix of other)
    SeqPrefixOf { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Seq::suffix_of` (Check if self is suffix of other)
    SeqSuffixOf { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Seq::replace` (Replace first occurrence of element `src` with `dst`)
    SeqReplace {
        t: TypeRef,
        seq: Expr,
        src: Expr,
        dst: Expr,
    },
    /// `Seq::is_empty`
    SeqIsEmpty { t: TypeRef, seq: Expr },
    /// `Set::new` (Empty set)
    SetEmpty { t: TypeRef },
    /// `Set::length` (Cardinality)
    SetLen { t: TypeRef, set: Expr },
    /// `Set::insert` (Functional insert)
    SetInsert { t: TypeRef, set: Expr, item: Expr },
    /// `Set::remove` (Functional remove)
    SetRemove { t: TypeRef, set: Expr, item: Expr },
    /// `Set::contains` (Membership check)
    SetContains { t: TypeRef, set: Expr, item: Expr },
    /// `Set::is_empty`
    SetIsEmpty { t: TypeRef, set: Expr },
    /// `Set::intersection`
    SetIntersect { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::union`
    SetUnion { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::difference` (Set minus)
    SetDiff { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::symmetric_difference`
    SetSymDiff { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_subset` (Subset or equal)
    SetIsSubset { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_proper_subset` (Strict subset)
    SetIsProperSubset { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_superset`
    SetIsSuperset { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_disjoint` (No common elements)
    SetIsDisjoint { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::has_size` (Check if cardinality equals specific integer)
    SetHasSize { t: TypeRef, set: Expr, size: Expr },
    /// `Array::new`
    ArrayEmpty { k: TypeRef, v: TypeRef },
    /// `Array::length`
    ArrayLen { k: TypeRef, v: TypeRef, arr: Expr },
    /// `Array::store`
    ArrayStore {
        k: TypeRef,
        v: TypeRef,
        arr: Expr,
        key: Expr,
        val: Expr,
    },
    /// `Array::select`
    ArraySelect {
        k: TypeRef,
        v: TypeRef,
        arr: Expr,
        key: Expr,
    },
    /// `Array::del`
    ArrayRemove {
        k: TypeRef,
        v: TypeRef,
        arr: Expr,
        key: Expr,
    },
    /// `Array::contains_key`
    ArrayContainsKey {
        k: TypeRef,
        v: TypeRef,
        arr: Expr,
        key: Expr,
    },
    /// `Array::is_empty`
    ArrayIsEmpty { k: TypeRef, v: TypeRef, arr: Expr },
    /// `bv_val` (Literal)
    BvVal { t: TypeRef, val: BigInt },
    /// `bv_not`
    BvNot { t: TypeRef, val: Expr },
    /// `bv_and`
    BvAnd { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_or`
    BvOr { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_xor`
    BvXor { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_nand`
    BvNand { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_nor`
    BvNor { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_xnor`
    BvXnor { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_redand`
    BvRedAnd { t: TypeRef, val: Expr },
    /// `bv_redor`
    BvRedOr { t: TypeRef, val: Expr },
    /// `bv_neg`
    BvNeg { t: TypeRef, val: Expr },
    /// `bv_add`
    BvAdd { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_sub`
    BvSub { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_mul`
    BvMul { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_div` (Signed/Unsigned handled by type `t`)
    BvDiv { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_rem`
    BvRem { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_mod`
    BvMod { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `checked_bvadd_no_overflow`
    BvAddNoOverflow { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `checked_bvsub_no_overflow`
    BvSubNoOverflow { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `checked_bvneg_no_overflow`
    BvNegNoOverflow { t: TypeRef, val: Expr },
    /// `checked_bvmul_no_overflow`
    BvMulNoOverflow { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `checked_bvsdiv_no_overflow`
    BvDivNoOverflow { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_shl`
    BvShl { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_lshr`
    BvLshr { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_ashr`
    BvAshr { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_rotate_left`
    BvRotLeft { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_rotate_right`
    BvRotRight { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_lt`
    BvLt { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_le`
    BvLe { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_gt`
    BvGt { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `bv_ge`
    BvGe { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `to_int`
    BvToInt { t: TypeRef, val: Expr },
    /// `FloatOps::nan`
    FloatNaN { t: TypeRef },
    /// `FloatOps::infinity`
    FloatPosInf { t: TypeRef },
    /// `FloatOps::neg_infinity`
    FloatNegInf { t: TypeRef },
    /// `FloatOps::pos_zero`
    FloatPosZero { t: TypeRef },
    /// `FloatOps::neg_zero`
    FloatNegZero { t: TypeRef },
    /// Literal Value
    FloatVal { t: TypeRef, val: BigRational },
    /// `FloatOps::add`
    FloatAdd { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::sub`
    FloatSub { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::mul`
    FloatMul { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::div`
    FloatDiv { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::neg`
    FloatNeg { t: TypeRef, val: Expr },
    /// `FloatOps::abs`
    FloatAbs { t: TypeRef, val: Expr },
    /// `FloatOps::rem`
    FloatRem { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::sqrt`
    FloatSqrt { t: TypeRef, val: Expr },
    /// `FloatOps::min`
    FloatMin { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::max`
    FloatMax { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::is_nan`
    FloatIsNaN { t: TypeRef, val: Expr },
    /// `FloatOps::is_infinite`
    FloatIsInf { t: TypeRef, val: Expr },
    /// `FloatOps::is_zero`
    FloatIsZero { t: TypeRef, val: Expr },
    /// `FloatOps::is_normal`
    FloatIsNormal { t: TypeRef, val: Expr },
    /// `FloatOps::is_subnormal`
    FloatIsSubnormal { t: TypeRef, val: Expr },
    /// `FloatOps::is_negative`
    FloatIsNeg { t: TypeRef, val: Expr },
    /// `FloatOps::is_positive`
    FloatIsPos { t: TypeRef, val: Expr },
    /// `FloatOps::lt`
    FloatLt { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::le`
    FloatLe { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::gt`
    FloatGt { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::ge`
    FloatGe { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `FloatOps::to_integer`
    FloatToInt { t: TypeRef, val: Expr },
    /// `FloatOps::to_real`
    FloatToReal { t: TypeRef, val: Expr },
    /// `Error::fresh`
    ErrFresh,
    /// `Error::merge`
    ErrMerge { lhs: Expr, rhs: Expr },
    /// `<any-smt-type>::eq`
    SmtEq { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `<any-smt-type>::ne`
    SmtNe { t: TypeRef, lhs: Expr, rhs: Expr },
}

// mk1!(BoolNot, ty_args, args) //no type arguments expected for this function.
// the above expands to:
// {
//     Intrinsic::unpack_ty_arg_0(ty_args)?;
//     let e1 = Intrinsic::unpack_expr_1(args)?;
//     Intrinsic::BoolNot { val: e1 }
macro_rules! mk1 {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        Intrinsic::unpack_ty_arg_0($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?; // a.not() expects 1 argument including the receiver
        Intrinsic::$op { val: e1 }
    }};
}

macro_rules! mk2 {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        Intrinsic::unpack_ty_arg_0($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op { lhs: e1, rhs: e2 }
    }};
}

macro_rules! mk0_t {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        Intrinsic::unpack_expr_0($args)?;
        Intrinsic::$op { t: t1 }
    }};
}

macro_rules! mk1_t {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?;
        Intrinsic::$op { t: t1, $n1: e1 }
    }};
}

macro_rules! mk2_t {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op {
            t: t1,
            $n1: e1,
            $n2: e2,
        }
    }};
}

macro_rules! mk0_kv {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        Intrinsic::unpack_expr_0($args)?;
        Intrinsic::$op { k: t1, v: t2 }
    }};
}

macro_rules! mk1_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
        }
    }};
}

macro_rules! mk2_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
            $n2: e2,
        }
    }};
}

macro_rules! mk3_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident, $n3:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let (e1, e2, e3) = Intrinsic::unpack_expr_3($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
            $n2: e2,
            $n3: e3,
        }
    }};
}

impl Intrinsic {
    /// Convert an argument list to a boolean literal
    pub fn unpack_lit_bool(args: &Punctuated<Exp, Comma>) -> Result<bool> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Bool(val) => val.value,
                    _ => bail_on!(lit, "not a boolean literal"),
                }
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an argument list to an integer literal
    pub fn unpack_lit_int(args: &Punctuated<Exp, Comma>) -> Result<BigInt> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Int(val) => match val.token().to_string().parse() {
                        Ok(v) => v,
                        Err(_) => bail_on!(val, "unable to parse"),
                    },
                    _ => bail_on!(lit, "not an integer literal"),
                }
            }
            Exp::Unary(unary) => {
                let ExprUnary { attrs: _, op, expr } = unary;
                let val = match op {
                    syn::UnOp::Neg(_) => -Self::unpack_lit_int(&Punctuated::from_iter(vec![
                        (*expr).as_ref().clone(),
                    ]))?,
                    _ => bail_on!(op, "not a unary negation operator"),
                };
                val
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an argument list to a floating-point literal
    pub fn unpack_lit_float(args: &Punctuated<Exp, Comma>) -> Result<BigRational> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Float(val) => match val.token().to_string().parse() {
                        Ok(v) => v,
                        Err(_) => bail_on!(val, "unable to parse"),
                    },
                    Lit::Int(val) => match val.token().to_string().parse::<BigInt>() {
                        Ok(v) => BigRational::from_integer(v),
                        Err(_) => bail_on!(val, "unable to parse"),
                    },
                    _ => bail_on!(lit, "not a float literal"),
                }
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an argument list to a string literal
    pub fn unpack_lit_str(args: &Punctuated<Exp, Comma>) -> Result<String> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Str(val) => val.token().to_string(),
                    _ => bail_on!(lit, "not a string literal"),
                }
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an expression to a literal
    pub fn parse_literal_into(receiver: &Exp) -> Result<(Self, TypeRef)> {
        let (intrinsic, ty) = match receiver {
            // A literal in place of an expression: `1`, `"foo"`.
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Bool(val) => (Self::BoolVal(val.value), TypeRef::Boolean),
                    Lit::Str(val) => (Self::StrVal(val.value()), TypeRef::String),
                    Lit::Int(val) => {
                        // Extract value string (digits) and suffix (type)
                        let raw_digits = val.base10_digits();
                        let suffix = val.suffix();

                        // Parse the number into BigInt
                        let big_int = match raw_digits.parse::<BigInt>() {
                            Ok(v) => v,
                            Err(_) => bail_on!(val, "unable to parse literal integer"),
                        };

                        // Decide TypeRef based on suffix
                        match suffix {
                            "" => (Self::IntVal(big_int), TypeRef::Integer),
                            "u32" => (
                                Self::BvVal {
                                    t: TypeRef::U32,
                                    val: big_int,
                                },
                                TypeRef::U32,
                            ),
                            "u64" => (
                                Self::BvVal {
                                    t: TypeRef::U64,
                                    val: big_int,
                                },
                                TypeRef::U64,
                            ),
                            "i32" => (
                                Self::BvVal {
                                    t: TypeRef::I32,
                                    val: big_int,
                                },
                                TypeRef::I32,
                            ),
                            "i64" => (
                                Self::BvVal {
                                    t: TypeRef::I64,
                                    val: big_int,
                                },
                                TypeRef::I64,
                            ),
                            // otherwise bail
                            _ => bail_on!(val, "unsupported integer suffix: '{}'", suffix),
                        }
                    }
                    Lit::Float(val) => {
                        // Extract value string and suffix
                        let raw_digits = val.base10_digits();
                        let suffix = val.suffix();

                        // Parse into BigRational (Standard for SMT Reals)
                        let big_rat = match raw_digits.parse::<BigRational>() {
                            Ok(v) => v,
                            // Fallback: try parsing as f64 then converting if BigRational string parse fails
                            Err(_) => match raw_digits.parse::<f64>() {
                                Ok(f) => BigRational::from_float(f)
                                    .unwrap_or_else(|| BigRational::from_float(0.0).unwrap()),
                                Err(_) => bail_on!(val, "unable to parse literal float"),
                            },
                        };

                        match suffix {
                            "" => (Self::RealVal(big_rat), TypeRef::Real),
                            "f32" => (
                                Self::FloatVal {
                                    t: TypeRef::F32,
                                    val: big_rat,
                                },
                                TypeRef::F32,
                            ),
                            "f64" => (
                                Self::FloatVal {
                                    t: TypeRef::F64,
                                    val: big_rat,
                                },
                                TypeRef::F64,
                            ),
                            _ => bail_on!(val, "unsupported float suffix: '{}'", suffix),
                        }
                    }
                    _ => bail_on!(
                        lit,
                        "not an expected literal type (char, byte, etc. not supported)"
                    ),
                }
            }
            // if not a literal, bail
            _ => bail_on!(receiver, "not a literal"),
        };
        Ok((intrinsic, ty))
    }

    /// Create an intrinsic
    pub fn new(
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
        ty_args: Vec<TypeRef>,
        args: Vec<Expr>,
    ) -> anyhow::Result<Self> {
        use SysTypeName as Q;

        // =====================================================================
        // LOCAL HELPER MACROS (Extensions to cover missing cases)
        // =====================================================================

        // Matches mk2 but allows custom field names (for IntPow etc.)
        macro_rules! mk2_named {
            ($op:ident, $n1:ident, $n2:ident) => {{
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let ($n1, $n2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::$op { $n1, $n2 }
            }};
        }

        // Matches mk3 (Ternary)
        macro_rules! mk3_named {
            ($op:ident, $n1:ident, $n2:ident, $n3:ident) => {{
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let ($n1, $n2, $n3) = Intrinsic::unpack_expr_3(args)?;
                Intrinsic::$op { $n1, $n2, $n3 }
            }};
        }

        // Matches mk3_t (Ternary with Type Arg)
        macro_rules! mk3_t {
            ($op:ident, $n1:ident, $n2:ident, $n3:ident) => {{
                let t1 = Intrinsic::unpack_ty_arg_1(ty_args)?;
                let ($n1, $n2, $n3) = Intrinsic::unpack_expr_3(args)?;
                Intrinsic::$op {
                    t: t1,
                    $n1,
                    $n2,
                    $n3,
                }
            }};
        }

        // Helper to infer TypeRef from SysTypeName (for I32, F64, etc.)
        let get_impl_type = || -> anyhow::Result<TypeRef> {
            match ty_name {
                Q::I32 => Ok(TypeRef::I32),
                Q::I64 => Ok(TypeRef::I64),
                Q::U32 => Ok(TypeRef::U32),
                Q::U64 => Ok(TypeRef::U64),
                Q::F32 => Ok(TypeRef::F32),
                Q::F64 => Ok(TypeRef::F64),
                _ => anyhow::bail!("Type {:?} does not have a static TypeRef", ty_name),
            }
        };

        // Implicit Type Macros (t comes from name, not args)
        macro_rules! mk0_impl {
            ($op:ident) => {{
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                Intrinsic::unpack_expr_0(args)?;
                Intrinsic::$op {
                    t: get_impl_type()?,
                }
            }};
        }
        macro_rules! mk1_impl {
            ($op:ident, $n1:ident) => {{
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let e1 = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::$op {
                    t: get_impl_type()?,
                    $n1: e1,
                }
            }};
        }
        macro_rules! mk2_impl {
            ($op:ident, $n1:ident, $n2:ident) => {{
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (e1, e2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::$op {
                    t: get_impl_type()?,
                    $n1: e1,
                    $n2: e2,
                }
            }};
        }

        // =====================================================================
        // MATCH LOGIC
        // =====================================================================

        let intrinsic = match (ty_name, fn_name.as_ref()) {
            // -----------------------------------------------------------------
            // Generic / Error
            // -----------------------------------------------------------------
            (_, "eq") => {
                let t = Intrinsic::unpack_ty_arg_1(ty_args)?;
                let (lhs, rhs) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::SmtEq { t, lhs, rhs }
            }
            (_, "ne") => {
                let t = Intrinsic::unpack_ty_arg_1(ty_args)?;
                let (lhs, rhs) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::SmtNe { t, lhs, rhs }
            }
            (Q::Error, "fresh") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                Intrinsic::unpack_expr_0(args)?;
                Intrinsic::ErrFresh
            }
            (Q::Error, "merge") => mk2!(ErrMerge, ty_args, args),

            // -----------------------------------------------------------------
            // Boolean
            // -----------------------------------------------------------------
            (Q::Boolean, "not") => mk1!(BoolNot, ty_args, args),
            (Q::Boolean, "and") => mk2!(BoolAnd, ty_args, args),
            (Q::Boolean, "or") => mk2!(BoolOr, ty_args, args),
            (Q::Boolean, "xor") => mk2!(BoolXor, ty_args, args),
            (Q::Boolean, "implies") => mk2!(BoolImplies, ty_args, args),
            (Q::Boolean, "iff") => mk2!(BoolIff, ty_args, args),
            (Q::Boolean, "nand") => mk2!(BoolNand, ty_args, args),
            (Q::Boolean, "nor") => mk2!(BoolNor, ty_args, args),
            (Q::Boolean, "xnor") => mk2!(BoolXnor, ty_args, args),

            // -----------------------------------------------------------------
            // Integer
            // -----------------------------------------------------------------
            (Q::Integer, "neg") => mk1!(IntNeg, ty_args, args),
            (Q::Integer, "add") => mk2!(IntAdd, ty_args, args),
            (Q::Integer, "sub") => mk2!(IntSub, ty_args, args),
            (Q::Integer, "mul") => mk2!(IntMul, ty_args, args),
            (Q::Integer, "div") => mk2!(IntDiv, ty_args, args),
            (Q::Integer, "mod") => mk2!(IntMod, ty_args, args),
            (Q::Integer, "rem") => mk2!(IntRem, ty_args, args),
            (Q::Integer, "pow") => mk2_named!(IntPow, base, exp),
            (Q::Integer, "abs") => mk1!(IntAbs, ty_args, args),

            (Q::Integer, "divides") => mk2!(IntDivides, ty_args, args),
            (Q::Integer, "lt") => mk2!(IntLt, ty_args, args),
            (Q::Integer, "le") => mk2!(IntLe, ty_args, args),
            (Q::Integer, "ge") => mk2!(IntGe, ty_args, args),
            (Q::Integer, "gt") => mk2!(IntGt, ty_args, args),

            (Q::Integer, "to_real") => mk1!(IntToReal, ty_args, args),
            (Q::Integer, "to_i32") => mk1!(IntToI32, ty_args, args),
            (Q::Integer, "to_i64") => mk1!(IntToI64, ty_args, args),
            (Q::Integer, "to_u32") => mk1!(IntToU32, ty_args, args),
            (Q::Integer, "to_u64") => mk1!(IntToU64, ty_args, args),
            (Q::Integer, "to_f32") => mk1!(IntToF32, ty_args, args),
            (Q::Integer, "to_f64") => mk1!(IntToF64, ty_args, args),

            (Q::Integer, "from_hex_str") => mk1!(IntFromHex, ty_args, args),
            (Q::Integer, "from_oct_str") => mk1!(IntFromOct, ty_args, args),
            (Q::Integer, "from_bin_str") => mk1!(IntFromBin, ty_args, args),

            (Q::Integer, "is_gt_i64_max") => mk1!(IntIsGtI64Max, ty_args, args),
            (Q::Integer, "is_lt_i64_min") => mk1!(IntIsLtI64Min, ty_args, args),
            (Q::Integer, "is_gt_u64_max") => mk1!(IntIsGtU64Max, ty_args, args),
            (Q::Integer, "is_lt_u64_min") => mk1!(IntIsLtU64Min, ty_args, args),
            (Q::Integer, "is_lt_i32_min") => mk1!(IntIsLtI32Min, ty_args, args),
            (Q::Integer, "is_gt_i32_max") => mk1!(IntIsGtI32Max, ty_args, args),
            (Q::Integer, "is_lt_u32_min") => mk1!(IntIsLtU32Min, ty_args, args),
            (Q::Integer, "is_gt_u32_max") => mk1!(IntIsGtU32Max, ty_args, args),

            // -----------------------------------------------------------------
            // Real (Rational)
            // -----------------------------------------------------------------
            (Q::Real, "neg") => mk1!(RealNeg, ty_args, args),
            (Q::Real, "add") => mk2!(RealAdd, ty_args, args),
            (Q::Real, "sub") => mk2!(RealSub, ty_args, args),
            (Q::Real, "mul") => mk2!(RealMul, ty_args, args),
            (Q::Real, "div") => mk2!(RealDiv, ty_args, args),
            (Q::Real, "pow") => mk2_named!(RealPow, base, exp),
            (Q::Real, "abs") => mk1!(RealAbs, ty_args, args),
            (Q::Real, "round") => mk1!(RealRound, ty_args, args),
            (Q::Real, "floor") => mk1!(RealFloor, ty_args, args),
            (Q::Real, "ceil") => mk1!(RealCeil, ty_args, args),

            (Q::Real, "is_integer") => mk1!(RealIsInt, ty_args, args),
            (Q::Real, "lt") => mk2!(RealLt, ty_args, args),
            (Q::Real, "le") => mk2!(RealLe, ty_args, args),
            (Q::Real, "ge") => mk2!(RealGe, ty_args, args),
            (Q::Real, "gt") => mk2!(RealGt, ty_args, args),

            (Q::Real, "to_int") => mk1!(RealToInt, ty_args, args),
            (Q::Real, "to_f32") => mk1!(RealToF32, ty_args, args),
            (Q::Real, "to_f64") => mk1!(RealToF64, ty_args, args),
            (Q::Real, "numerator") => mk1!(RealNumer, ty_args, args),
            (Q::Real, "denominator") => mk1!(RealDenom, ty_args, args),

            // -----------------------------------------------------------------
            // String (Text)
            // -----------------------------------------------------------------
            // mk1 uses `val`, but StrLen uses `seq`. Must use manual or named helper.
            (Q::String, "length") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let seq = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::StrLen { seq }
            }
            (Q::String, "concat") => mk2!(StrConcat, ty_args, args),
            (Q::String, "at") => mk2_named!(StrAt, seq, idx),
            (Q::String, "is_empty") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let seq = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::StrIsEmpty { seq }
            }
            (Q::String, "contains") => mk2_named!(StrContains, seq, item),
            (Q::String, "starts_with") => mk2_named!(StrStartsWith, seq, item),
            (Q::String, "ends_with") => mk2_named!(StrEndsWith, seq, item),
            (Q::String, "is_digit") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let seq = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::StrIsDigit { seq }
            }

            (Q::String, "le") => mk2!(StrLe, ty_args, args),
            (Q::String, "lt") => mk2!(StrLt, ty_args, args),
            (Q::String, "ge") => mk2!(StrGe, ty_args, args),
            (Q::String, "gt") => mk2!(StrGt, ty_args, args),

            (Q::String, "index_of") => mk3_named!(StrIndexOf, seq, sub, offset),
            (Q::String, "replace") => mk3_named!(StrReplace, seq, src, dst),
            (Q::String, "replace_all") => mk3_named!(StrReplaceAll, seq, src, dst),

            (Q::String, "to_int") => mk1!(StrToInt, ty_args, args),
            (Q::String, "from_int") => mk1!(StrFromInt, ty_args, args),
            (Q::String, "from_code") => mk1!(StrFromCode, ty_args, args),
            (Q::String, "to_code") => mk1!(StrToCode, ty_args, args),

            // -----------------------------------------------------------------
            // Collections (Generic)
            // -----------------------------------------------------------------
            // Cloak - mk1_t uses `val` and `t`. This matches BoxShield { t, val }.
            (Q::Cloak, "shield") => mk1_t!(BoxShield, ty_args, args, val),
            (Q::Cloak, "reveal") => mk1_t!(BoxReveal, ty_args, args, val),

            // Sequence
            (Q::Seq, "new") => mk0_t!(SeqEmpty, ty_args, args),
            (Q::Seq, "unit") => mk1_t!(SeqUnit, ty_args, args, val),
            (Q::Seq, "length") => mk1_t!(SeqLen, ty_args, args, seq),
            (Q::Seq, "append") => mk2_t!(SeqPush, ty_args, args, seq, item),
            (Q::Seq, "at") => mk2_t!(SeqNth, ty_args, args, seq, idx),
            (Q::Seq, "contains") => mk2_t!(SeqContains, ty_args, args, seq, item),
            (Q::Seq, "concat") => mk2_t!(SeqConcat, ty_args, args, lhs, rhs),
            (Q::Seq, "prefix_of") => mk2_t!(SeqPrefixOf, ty_args, args, lhs, rhs),
            (Q::Seq, "suffix_of") => mk2_t!(SeqSuffixOf, ty_args, args, lhs, rhs),
            (Q::Seq, "is_empty") => mk1_t!(SeqIsEmpty, ty_args, args, seq),
            (Q::Seq, "extract") => mk3_t!(SeqExtract, seq, offset, len),
            (Q::Seq, "replace") => mk3_t!(SeqReplace, seq, src, dst),

            // Set
            (Q::Set, "new") => mk0_t!(SetEmpty, ty_args, args),
            (Q::Set, "length") => mk1_t!(SetLen, ty_args, args, set),
            (Q::Set, "insert") => mk2_t!(SetInsert, ty_args, args, set, item),
            (Q::Set, "remove") => mk2_t!(SetRemove, ty_args, args, set, item),
            (Q::Set, "contains") => mk2_t!(SetContains, ty_args, args, set, item),
            (Q::Set, "is_empty") => mk1_t!(SetIsEmpty, ty_args, args, set),
            (Q::Set, "intersection") => mk2_t!(SetIntersect, ty_args, args, lhs, rhs),
            (Q::Set, "union") => mk2_t!(SetUnion, ty_args, args, lhs, rhs),
            (Q::Set, "difference") => mk2_t!(SetDiff, ty_args, args, lhs, rhs),
            (Q::Set, "symmetric_difference") => mk2_t!(SetSymDiff, ty_args, args, lhs, rhs),
            (Q::Set, "is_subset") => mk2_t!(SetIsSubset, ty_args, args, lhs, rhs),
            (Q::Set, "is_proper_subset") => mk2_t!(SetIsProperSubset, ty_args, args, lhs, rhs),
            (Q::Set, "is_superset") => mk2_t!(SetIsSuperset, ty_args, args, lhs, rhs),
            (Q::Set, "is_disjoint") => mk2_t!(SetIsDisjoint, ty_args, args, lhs, rhs),
            (Q::Set, "has_size") => mk2_t!(SetHasSize, ty_args, args, set, size),

            // Array / Map
            (Q::Array, "new") => mk0_kv!(ArrayEmpty, ty_args, args),
            (Q::Array, "length") => mk1_kv!(ArrayLen, ty_args, args, arr),
            (Q::Array, "store") => mk3_kv!(ArrayStore, ty_args, args, arr, key, val),
            (Q::Array, "select") => mk2_kv!(ArraySelect, ty_args, args, arr, key),
            (Q::Array, "del") => mk2_kv!(ArrayRemove, ty_args, args, arr, key),
            (Q::Array, "contains_key") => mk2_kv!(ArrayContainsKey, ty_args, args, arr, key),
            (Q::Array, "is_empty") => mk1_kv!(ArrayIsEmpty, ty_args, args, arr),

            // -----------------------------------------------------------------
            // Bitvectors (I32, I64, U32, U64)
            // -----------------------------------------------------------------
            // Unary
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_not") => mk1_impl!(BvNot, val),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_neg") => mk1_impl!(BvNeg, val),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_redand") => mk1_impl!(BvRedAnd, val),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_redor") => mk1_impl!(BvRedOr, val),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "to_int") => mk1_impl!(BvToInt, val),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "checked_bvneg_no_overflow") => {
                mk1_impl!(BvNegNoOverflow, val)
            }

            // Binary
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_and") => mk2_impl!(BvAnd, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_or") => mk2_impl!(BvOr, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_xor") => mk2_impl!(BvXor, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_nand") => mk2_impl!(BvNand, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_nor") => mk2_impl!(BvNor, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_xnor") => mk2_impl!(BvXnor, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_add") => mk2_impl!(BvAdd, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_sub") => mk2_impl!(BvSub, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_mul") => mk2_impl!(BvMul, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_div") => mk2_impl!(BvDiv, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_rem") => mk2_impl!(BvRem, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_mod") => mk2_impl!(BvMod, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_shl") => mk2_impl!(BvShl, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_lshr") => mk2_impl!(BvLshr, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_ashr") => mk2_impl!(BvAshr, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_rotate_left") => mk2_impl!(BvRotLeft, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_rotate_right") => {
                mk2_impl!(BvRotRight, lhs, rhs)
            }
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_lt") => mk2_impl!(BvLt, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_le") => mk2_impl!(BvLe, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_gt") => mk2_impl!(BvGt, lhs, rhs),
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "bv_ge") => mk2_impl!(BvGe, lhs, rhs),

            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "checked_bvadd_no_overflow") => {
                mk2_impl!(BvAddNoOverflow, lhs, rhs)
            }
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "checked_bvsub_no_overflow") => {
                mk2_impl!(BvSubNoOverflow, lhs, rhs)
            }
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "checked_bvmul_no_overflow") => {
                mk2_impl!(BvMulNoOverflow, lhs, rhs)
            }
            (Q::I32 | Q::I64 | Q::U32 | Q::U64, "checked_bvsdiv_no_overflow") => {
                mk2_impl!(BvDivNoOverflow, lhs, rhs)
            }

            // -----------------------------------------------------------------
            // Floats (F32, F64)
            // -----------------------------------------------------------------
            // Constants
            (Q::F32 | Q::F64, "nan") => mk0_impl!(FloatNaN),
            (Q::F32 | Q::F64, "infinity") => mk0_impl!(FloatPosInf),
            (Q::F32 | Q::F64, "neg_infinity") => mk0_impl!(FloatNegInf),
            (Q::F32 | Q::F64, "pos_zero") => mk0_impl!(FloatPosZero),
            (Q::F32 | Q::F64, "neg_zero") => mk0_impl!(FloatNegZero),

            // Unary
            (Q::F32 | Q::F64, "neg") => mk1_impl!(FloatNeg, val),
            (Q::F32 | Q::F64, "abs") => mk1_impl!(FloatAbs, val),
            (Q::F32 | Q::F64, "sqrt") => mk1_impl!(FloatSqrt, val),
            (Q::F32 | Q::F64, "to_integer") => mk1_impl!(FloatToInt, val),
            (Q::F32 | Q::F64, "to_real") => mk1_impl!(FloatToReal, val),

            // Predicates
            (Q::F32 | Q::F64, "is_nan") => mk1_impl!(FloatIsNaN, val),
            (Q::F32 | Q::F64, "is_infinite") => mk1_impl!(FloatIsInf, val),
            (Q::F32 | Q::F64, "is_zero") => mk1_impl!(FloatIsZero, val),
            (Q::F32 | Q::F64, "is_normal") => mk1_impl!(FloatIsNormal, val),
            (Q::F32 | Q::F64, "is_subnormal") => mk1_impl!(FloatIsSubnormal, val),
            (Q::F32 | Q::F64, "is_negative") => mk1_impl!(FloatIsNeg, val),
            (Q::F32 | Q::F64, "is_positive") => mk1_impl!(FloatIsPos, val),

            // Binary
            (Q::F32 | Q::F64, "add") => mk2_impl!(FloatAdd, lhs, rhs),
            (Q::F32 | Q::F64, "sub") => mk2_impl!(FloatSub, lhs, rhs),
            (Q::F32 | Q::F64, "mul") => mk2_impl!(FloatMul, lhs, rhs),
            (Q::F32 | Q::F64, "div") => mk2_impl!(FloatDiv, lhs, rhs),
            (Q::F32 | Q::F64, "rem") => mk2_impl!(FloatRem, lhs, rhs),
            (Q::F32 | Q::F64, "min") => mk2_impl!(FloatMin, lhs, rhs),
            (Q::F32 | Q::F64, "max") => mk2_impl!(FloatMax, lhs, rhs),
            (Q::F32 | Q::F64, "lt") => mk2_impl!(FloatLt, lhs, rhs),
            (Q::F32 | Q::F64, "le") => mk2_impl!(FloatLe, lhs, rhs),
            (Q::F32 | Q::F64, "gt") => mk2_impl!(FloatGt, lhs, rhs),
            (Q::F32 | Q::F64, "ge") => mk2_impl!(FloatGe, lhs, rhs),

            _ => anyhow::bail!("unknown intrinsic: {:?}::{}", ty_name, fn_name),
        };

        Ok(intrinsic)
    }
    /// Utility to unpack 0 type argument
    fn unpack_ty_arg_0(ty_args: Vec<TypeRef>) -> anyhow::Result<()> {
        let mut iter = ty_args.into_iter();
        if iter.next().is_some() {
            bail!("expect 0 type argument");
        }
        Ok(())
    }

    /// Utility to unpack 1 type argument
    fn unpack_ty_arg_1(ty_args: Vec<TypeRef>) -> anyhow::Result<TypeRef> {
        let mut iter = ty_args.into_iter();
        let t1 = match iter.next() {
            None => bail!("expect 1 type argument"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail!("expect 1 type argument");
        }
        Ok(t1)
    }

    /// Utility to unpack 2 type arguments
    fn unpack_ty_arg_2(ty_args: Vec<TypeRef>) -> anyhow::Result<(TypeRef, TypeRef)> {
        let mut iter = ty_args.into_iter();
        let t1 = match iter.next() {
            None => bail!("expect 2 type arguments"),
            Some(t) => t,
        };
        let t2 = match iter.next() {
            None => bail!("expect 2 type arguments"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail!("expect 2 type arguments");
        }
        Ok((t1, t2))
    }

    /// Utility to unpack 0 argument
    fn unpack_expr_0(exprs: Vec<Expr>) -> anyhow::Result<()> {
        let mut iter = exprs.into_iter();
        if iter.next().is_some() {
            bail!("expect 0 argument");
        }
        Ok(())
    }

    /// Utility to unpack 1 argument
    fn unpack_expr_1(exprs: Vec<Expr>) -> anyhow::Result<Expr> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 1 argument"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 1 argument");
        }
        Ok(e1)
    }

    /// Utility to unpack 2 arguments
    fn unpack_expr_2(exprs: Vec<Expr>) -> anyhow::Result<(Expr, Expr)> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 2 arguments"),
            Some(e) => e,
        };
        let e2 = match iter.next() {
            None => bail!("expect 2 arguments"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 2 arguments");
        }
        Ok((e1, e2))
    }

    /// Utility to unpack 3 arguments
    fn unpack_expr_3(exprs: Vec<Expr>) -> anyhow::Result<(Expr, Expr, Expr)> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        let e2 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        let e3 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 3 arguments");
        }
        Ok((e1, e2, e3))
    }
}

impl Display for Intrinsic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // -----------------------------------------------------------------
            // Values / Literals
            // -----------------------------------------------------------------
            Self::BoolVal(v) => write!(f, "{v}"),
            Self::IntVal(v) => write!(f, "{v}"),
            Self::RealVal(v) => write!(f, "{v}"),
            Self::StrVal(v) => write!(f, "\"{v}\""), // Wrap string in quotes
            Self::BvVal { val, .. } => write!(f, "{val}"), // Type inferred from context usually
            Self::FloatVal { val, .. } => write!(f, "{val}"),

            // -----------------------------------------------------------------
            // Infix Operators (Arithmetic & Logic)
            // -----------------------------------------------------------------
            // Equality (Generic)
            Self::SmtEq { lhs, rhs, .. } => write!(f, "{lhs} == {rhs}"),
            Self::SmtNe { lhs, rhs, .. } => write!(f, "{lhs} != {rhs}"),

            // Boolean Logic
            Self::BoolAnd { lhs, rhs } => write!(f, "({lhs} && {rhs})"),
            Self::BoolOr { lhs, rhs } => write!(f, "({lhs} || {rhs})"),
            Self::BoolImplies { lhs, rhs } => write!(f, "({lhs} => {rhs})"),
            Self::BoolIff { lhs, rhs } => write!(f, "({lhs} <=> {rhs})"),
            Self::BoolXor { lhs, rhs } => write!(f, "({lhs} ^ {rhs})"),

            // Addition (+)
            Self::IntAdd { lhs, rhs }
            | Self::RealAdd { lhs, rhs }
            | Self::BvAdd { lhs, rhs, .. }
            | Self::FloatAdd { lhs, rhs, .. } => write!(f, "({lhs} + {rhs})"),

            // Subtraction (-)
            Self::IntSub { lhs, rhs }
            | Self::RealSub { lhs, rhs }
            | Self::BvSub { lhs, rhs, .. }
            | Self::FloatSub { lhs, rhs, .. } => write!(f, "({lhs} - {rhs})"),

            // Multiplication (*)
            Self::IntMul { lhs, rhs }
            | Self::RealMul { lhs, rhs }
            | Self::BvMul { lhs, rhs, .. }
            | Self::FloatMul { lhs, rhs, .. } => write!(f, "({lhs} * {rhs})"),

            // Division (/)
            Self::IntDiv { lhs, rhs }
            | Self::RealDiv { lhs, rhs }
            | Self::BvDiv { lhs, rhs, .. }
            | Self::FloatDiv { lhs, rhs, .. } => write!(f, "({lhs} / {rhs})"),

            // Remainder (%)
            Self::IntRem { lhs, rhs }
            | Self::BvRem { lhs, rhs, .. }
            | Self::FloatRem { lhs, rhs, .. } => write!(f, "({lhs} % {rhs})"),

            // Bitwise Logic (&, |, ^)
            Self::BvAnd { lhs, rhs, .. } => write!(f, "({lhs} & {rhs})"),
            Self::BvOr { lhs, rhs, .. } => write!(f, "({lhs} | {rhs})"),
            Self::BvXor { lhs, rhs, .. } => write!(f, "({lhs} ^ {rhs})"),
            Self::BvShl { lhs, rhs, .. } => write!(f, "({lhs} << {rhs})"),
            Self::BvLshr { lhs, rhs, .. } => write!(f, "({lhs} >> {rhs})"),
            Self::BvAshr { lhs, rhs, .. } => write!(f, "({lhs} a>> {rhs})"), // Arithmetic shift distinction

            // String Concatenation (++)
            Self::StrConcat { lhs, rhs } | Self::SeqConcat { lhs, rhs, .. } => {
                write!(f, "({lhs} ++ {rhs})")
            }

            // -----------------------------------------------------------------
            // Comparisons (<, <=, >, >=)
            // -----------------------------------------------------------------
            Self::IntLt { lhs, rhs }
            | Self::RealLt { lhs, rhs }
            | Self::StrLt { lhs, rhs }
            | Self::BvLt { lhs, rhs, .. }
            | Self::FloatLt { lhs, rhs, .. } => write!(f, "({lhs} < {rhs})"),

            Self::IntLe { lhs, rhs }
            | Self::RealLe { lhs, rhs }
            | Self::StrLe { lhs, rhs }
            | Self::BvLe { lhs, rhs, .. }
            | Self::FloatLe { lhs, rhs, .. } => write!(f, "({lhs} <= {rhs})"),

            Self::IntGt { lhs, rhs }
            | Self::RealGt { lhs, rhs }
            | Self::StrGt { lhs, rhs }
            | Self::BvGt { lhs, rhs, .. }
            | Self::FloatGt { lhs, rhs, .. } => write!(f, "({lhs} > {rhs})"),

            Self::IntGe { lhs, rhs }
            | Self::RealGe { lhs, rhs }
            | Self::StrGe { lhs, rhs }
            | Self::BvGe { lhs, rhs, .. }
            | Self::FloatGe { lhs, rhs, .. } => write!(f, "({lhs} >= {rhs})"),

            // -----------------------------------------------------------------
            // Prefix / Function Style
            // -----------------------------------------------------------------
            // Boolean
            Self::BoolNot { val } => write!(f, "!{val}"),
            Self::BoolNand { lhs, rhs } => write!(f, "nand({lhs}, {rhs})"),
            Self::BoolNor { lhs, rhs } => write!(f, "nor({lhs}, {rhs})"),
            Self::BoolXnor { lhs, rhs } => write!(f, "xnor({lhs}, {rhs})"),

            // Integer
            Self::IntNeg { val } => write!(f, "-{val}"),
            Self::IntMod { lhs, rhs } => write!(f, "mod({lhs}, {rhs})"),
            Self::IntPow { base, exp } => write!(f, "pow({base}, {exp})"),
            Self::IntAbs { val } => write!(f, "abs({val})"),
            Self::IntDivides { lhs, rhs } => write!(f, "divides({lhs}, {rhs})"),
            // Int Conversions/Checks
            Self::IntToReal { val } => write!(f, "Int::to_real({val})"),
            Self::IntToI32 { val } => write!(f, "Int::to_i32({val})"),
            Self::IntToI64 { val } => write!(f, "Int::to_i64({val})"),
            Self::IntToU32 { val } => write!(f, "Int::to_u32({val})"),
            Self::IntToU64 { val } => write!(f, "Int::to_u64({val})"),
            Self::IntToF32 { val } => write!(f, "Int::to_f32({val})"),
            Self::IntToF64 { val } => write!(f, "Int::to_f64({val})"),
            Self::IntFromHex { val } => write!(f, "Int::from_hex({val})"),
            Self::IntFromOct { val } => write!(f, "Int::from_oct({val})"),
            Self::IntFromBin { val } => write!(f, "Int::from_bin({val})"),
            // Range checks (abbreviated)
            Self::IntIsGtI64Max { val } => write!(f, "is_gt_i64_max({val})"),
            Self::IntIsLtI64Min { val } => write!(f, "is_lt_i64_min({val})"),
            Self::IntIsGtU64Max { val } => write!(f, "is_gt_u64_max({val})"),
            Self::IntIsLtU64Min { val } => write!(f, "is_lt_u64_min({val})"),
            Self::IntIsLtI32Min { val } => write!(f, "is_lt_i32_min({val})"),
            Self::IntIsGtI32Max { val } => write!(f, "is_gt_i32_max({val})"),
            Self::IntIsLtU32Min { val } => write!(f, "is_lt_u32_min({val})"),
            Self::IntIsGtU32Max { val } => write!(f, "is_gt_u32_max({val})"),

            // Real
            Self::RealNeg { val } => write!(f, "-{val}"),
            Self::RealAbs { val } => write!(f, "abs({val})"),
            Self::RealPow { base, exp } => write!(f, "pow({base}, {exp})"),
            Self::RealRound { val } => write!(f, "round({val})"),
            Self::RealFloor { val } => write!(f, "floor({val})"),
            Self::RealCeil { val } => write!(f, "ceil({val})"),
            Self::RealIsInt { val } => write!(f, "is_int({val})"),
            Self::RealToInt { val } => write!(f, "Real::to_int({val})"),
            Self::RealToF32 { val } => write!(f, "Real::to_f32({val})"),
            Self::RealToF64 { val } => write!(f, "Real::to_f64({val})"),
            Self::RealNumer { val } => write!(f, "numer({val})"),
            Self::RealDenom { val } => write!(f, "denom({val})"),

            // String
            Self::StrLen { seq } => write!(f, "len({seq})"),
            Self::StrAt { seq, idx } => write!(f, "{seq}[{idx}]"),
            Self::StrIsEmpty { seq } => write!(f, "is_empty({seq})"),
            Self::StrContains { seq, item } => write!(f, "contains({seq}, {item})"),
            Self::StrStartsWith { seq, item } => write!(f, "starts_with({seq}, {item})"),
            Self::StrEndsWith { seq, item } => write!(f, "ends_with({seq}, {item})"),
            Self::StrIsDigit { seq } => write!(f, "is_digit({seq})"),
            Self::StrIndexOf { seq, sub, offset } => write!(f, "index_of({seq}, {sub}, {offset})"),
            Self::StrReplace { seq, src, dst } => write!(f, "replace({seq}, {src}, {dst})"),
            Self::StrReplaceAll { seq, src, dst } => write!(f, "replace_all({seq}, {src}, {dst})"),
            Self::StrToInt { val } => write!(f, "Str::to_int({val})"),
            Self::StrFromInt { val } => write!(f, "Str::from_int({val})"),
            Self::StrFromCode { val } => write!(f, "Str::from_code({val})"),
            Self::StrToCode { val } => write!(f, "Str::to_code({val})"),

            // Cloak
            Self::BoxShield { val, .. } => write!(f, "shield({val})"),
            Self::BoxReveal { val, .. } => write!(f, "reveal({val})"),

            // Sequence
            Self::SeqEmpty { .. } => write!(f, "Seq::empty"),
            Self::SeqUnit { val, .. } => write!(f, "Seq::unit({val})"),
            Self::SeqLen { seq, .. } => write!(f, "len({seq})"),
            Self::SeqNth { seq, idx, .. } => write!(f, "{seq}[{idx}]"),
            Self::SeqExtract {
                seq, offset, len, ..
            } => write!(f, "{seq}[{offset}..{len}]"),
            Self::SeqPush { seq, item, .. } => write!(f, "push({seq}, {item})"),
            Self::SeqContains { seq, item, .. } => write!(f, "contains({seq}, {item})"),
            Self::SeqPrefixOf { lhs, rhs, .. } => write!(f, "prefix_of({lhs}, {rhs})"),
            Self::SeqSuffixOf { lhs, rhs, .. } => write!(f, "suffix_of({lhs}, {rhs})"),
            Self::SeqReplace { seq, src, dst, .. } => write!(f, "replace({seq}, {src}, {dst})"),
            Self::SeqIsEmpty { seq, .. } => write!(f, "is_empty({seq})"),

            // Set
            Self::SetEmpty { .. } => write!(f, "Set::empty"),
            Self::SetLen { set, .. } => write!(f, "len({set})"),
            Self::SetInsert { set, item, .. } => write!(f, "insert({set}, {item})"),
            Self::SetRemove { set, item, .. } => write!(f, "remove({set}, {item})"),
            Self::SetContains { set, item, .. } => write!(f, "contains({set}, {item})"),
            Self::SetIsEmpty { set, .. } => write!(f, "is_empty({set})"),
            Self::SetIntersect { lhs, rhs, .. } => write!(f, "intersect({lhs}, {rhs})"),
            Self::SetUnion { lhs, rhs, .. } => write!(f, "union({lhs}, {rhs})"),
            Self::SetDiff { lhs, rhs, .. } => write!(f, "diff({lhs}, {rhs})"),
            Self::SetSymDiff { lhs, rhs, .. } => write!(f, "sym_diff({lhs}, {rhs})"),
            Self::SetIsSubset { lhs, rhs, .. } => write!(f, "subset({lhs}, {rhs})"),
            Self::SetIsProperSubset { lhs, rhs, .. } => write!(f, "proper_subset({lhs}, {rhs})"),
            Self::SetIsSuperset { lhs, rhs, .. } => write!(f, "superset({lhs}, {rhs})"),
            Self::SetIsDisjoint { lhs, rhs, .. } => write!(f, "disjoint({lhs}, {rhs})"),
            Self::SetHasSize { set, size, .. } => write!(f, "has_size({set}, {size})"),

            // Array / Map
            Self::ArrayEmpty { .. } => write!(f, "Array::empty"),
            Self::ArrayLen { arr, .. } => write!(f, "len({arr})"),
            Self::ArrayStore { arr, key, val, .. } => write!(f, "store({arr}, {key}, {val})"),
            Self::ArraySelect { arr, key, .. } => write!(f, "{arr}[{key}]"),
            Self::ArrayRemove { arr, key, .. } => write!(f, "remove({arr}, {key})"),
            Self::ArrayContainsKey { arr, key, .. } => write!(f, "contains_key({arr}, {key})"),
            Self::ArrayIsEmpty { arr, .. } => write!(f, "is_empty({arr})"),

            // Bitvector (Logic & Shift)
            Self::BvNot { val, .. } => write!(f, "!{val}"),
            Self::BvNeg { val, .. } => write!(f, "-{val}"),
            Self::BvNand { lhs, rhs, .. } => write!(f, "nand({lhs}, {rhs})"),
            Self::BvNor { lhs, rhs, .. } => write!(f, "nor({lhs}, {rhs})"),
            Self::BvXnor { lhs, rhs, .. } => write!(f, "xnor({lhs}, {rhs})"),
            Self::BvRedAnd { val, .. } => write!(f, "redand({val})"),
            Self::BvRedOr { val, .. } => write!(f, "redor({val})"),
            Self::BvMod { lhs, rhs, .. } => write!(f, "mod({lhs}, {rhs})"),
            Self::BvAddNoOverflow { lhs, rhs, .. } => write!(f, "add_no_overflow({lhs}, {rhs})"),
            Self::BvSubNoOverflow { lhs, rhs, .. } => write!(f, "sub_no_overflow({lhs}, {rhs})"),
            Self::BvNegNoOverflow { val, .. } => write!(f, "neg_no_overflow({val})"),
            Self::BvMulNoOverflow { lhs, rhs, .. } => write!(f, "mul_no_overflow({lhs}, {rhs})"),
            Self::BvDivNoOverflow { lhs, rhs, .. } => write!(f, "div_no_overflow({lhs}, {rhs})"),
            Self::BvRotLeft { lhs, rhs, .. } => write!(f, "rot_left({lhs}, {rhs})"),
            Self::BvRotRight { lhs, rhs, .. } => write!(f, "rot_right({lhs}, {rhs})"),
            Self::BvToInt { val, .. } => write!(f, "Bv::to_int({val})"),

            // Float
            Self::FloatNaN { .. } => write!(f, "NaN"),
            Self::FloatPosInf { .. } => write!(f, "+Inf"),
            Self::FloatNegInf { .. } => write!(f, "-Inf"),
            Self::FloatPosZero { .. } => write!(f, "+0.0"),
            Self::FloatNegZero { .. } => write!(f, "-0.0"),
            Self::FloatNeg { val, .. } => write!(f, "-{val}"),
            Self::FloatAbs { val, .. } => write!(f, "abs({val})"),
            Self::FloatSqrt { val, .. } => write!(f, "sqrt({val})"),
            Self::FloatMin { lhs, rhs, .. } => write!(f, "min({lhs}, {rhs})"),
            Self::FloatMax { lhs, rhs, .. } => write!(f, "max({lhs}, {rhs})"),
            Self::FloatIsNaN { val, .. } => write!(f, "is_nan({val})"),
            Self::FloatIsInf { val, .. } => write!(f, "is_inf({val})"),
            Self::FloatIsZero { val, .. } => write!(f, "is_zero({val})"),
            Self::FloatIsNormal { val, .. } => write!(f, "is_normal({val})"),
            Self::FloatIsSubnormal { val, .. } => write!(f, "is_subnormal({val})"),
            Self::FloatIsNeg { val, .. } => write!(f, "is_neg({val})"),
            Self::FloatIsPos { val, .. } => write!(f, "is_pos({val})"),
            Self::FloatToInt { val, .. } => write!(f, "Float::to_int({val})"),
            Self::FloatToReal { val, .. } => write!(f, "Float::to_real({val})"),

            // Errors
            Self::ErrFresh => write!(f, "err_fresh"),
            Self::ErrMerge { lhs, rhs } => write!(f, "err_merge({lhs}, {rhs})"),
        }
    }
}

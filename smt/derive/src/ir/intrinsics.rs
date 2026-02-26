use num_bigint::BigInt;
use num_rational::BigRational;

use crate::ir::index::ExpId;
use crate::ir::sort::Sort;

#[derive(Debug, Clone, PartialEq)]
/// Intrinsic procedure
pub enum Intrinsic {
    /// `Boolean::from`
    BoolVal(bool),
    /// `Boolean::not`
    BoolNot { val: ExpId },
    /// `Boolean::and`
    BoolAnd { lhs: ExpId, rhs: ExpId },
    /// `Boolean::or`
    BoolOr { lhs: ExpId, rhs: ExpId },
    /// `Boolean::xor`
    BoolXor { lhs: ExpId, rhs: ExpId },
    /// `Boolean::nand`
    BoolNand { lhs: ExpId, rhs: ExpId },
    /// `Boolean::nor`
    BoolNor { lhs: ExpId, rhs: ExpId },
    /// `Boolean::xnor`
    BoolXnor { lhs: ExpId, rhs: ExpId },
    /// `Boolean::implies`
    BoolImplies { lhs: ExpId, rhs: ExpId },
    /// `Boolean::iff`
    BoolIff { lhs: ExpId, rhs: ExpId },
    /// `Boolean::ite`
    BoolIte {
        cond: ExpId,
        then: ExpId,
        else_: ExpId,
    },

    /// `Integer::from`
    IntVal(BigInt),
    /// `Integer::neg`
    IntNeg { val: ExpId },
    /// `Integer::add`
    IntAdd { lhs: ExpId, rhs: ExpId },
    /// `Integer::sub`
    IntSub { lhs: ExpId, rhs: ExpId },
    /// `Integer::mul`
    IntMul { lhs: ExpId, rhs: ExpId },
    /// `Integer::div`
    IntDiv { lhs: ExpId, rhs: ExpId },
    /// `Integer::div_trunc`
    IntDivTrunc { lhs: ExpId, rhs: ExpId },
    /// `Integer::modulo`
    IntMod { lhs: ExpId, rhs: ExpId },
    /// `Integer::rem`
    IntRem { lhs: ExpId, rhs: ExpId },
    /// `Integer::pow`
    IntPow { base: ExpId, exp: ExpId },
    /// `Integer::abs`
    IntAbs { val: ExpId },
    /// `Integer::divides`
    IntDivides { lhs: ExpId, rhs: ExpId },
    /// `Integer::lt`
    IntLt { lhs: ExpId, rhs: ExpId },
    /// `Integer::le`
    IntLe { lhs: ExpId, rhs: ExpId },
    /// `Integer::gt`
    IntGt { lhs: ExpId, rhs: ExpId },
    /// `Integer::ge`
    IntGe { lhs: ExpId, rhs: ExpId },

    // Integer Conversions
    /// `Integer::to_real`
    IntToReal { val: ExpId },
    /// `Integer::to_i32`
    IntToI32 { val: ExpId },
    /// `Integer::to_i64`
    IntToI64 { val: ExpId },
    /// `Integer::to_u32`
    IntToU32 { val: ExpId },
    /// `Integer::to_u64`
    IntToU64 { val: ExpId },
    /// `Integer::to_f32`
    IntToF32 { val: ExpId },
    /// `Integer::to_f64`
    IntToF64 { val: ExpId },

    // Integer Parsing
    /// `Integer::from_hex_str`
    IntFromHex { val: ExpId },
    /// `Integer::from_oct_str`
    IntFromOct { val: ExpId },
    /// `Integer::from_bin_str`
    IntFromBin { val: ExpId },

    // Integer Range Checks
    /// `Integer::is_gt_i64_max`
    IntIsGtI64Max { val: ExpId },
    /// `Integer::is_lt_i64_min`
    IntIsLtI64Min { val: ExpId },
    /// `Integer::is_gt_u64_max`
    IntIsGtU64Max { val: ExpId },
    /// `Integer::is_lt_u64_min`
    IntIsLtU64Min { val: ExpId },
    /// `Integer::is_lt_i32_min`
    IntIsLtI32Min { val: ExpId },
    /// `Integer::is_gt_i32_max`
    IntIsGtI32Max { val: ExpId },
    /// `Integer::is_lt_u32_min`
    IntIsLtU32Min { val: ExpId },
    /// `Integer::is_gt_u32_max`
    IntIsGtU32Max { val: ExpId },

    /// `Real::from`
    RealVal(BigRational),
    /// `Real::neg`
    RealNeg { val: ExpId },
    /// `Real::add`
    RealAdd { lhs: ExpId, rhs: ExpId },
    /// `Real::sub`
    RealSub { lhs: ExpId, rhs: ExpId },
    /// `Real::mul`
    RealMul { lhs: ExpId, rhs: ExpId },
    /// `Real::div`
    RealDiv { lhs: ExpId, rhs: ExpId },
    /// `Real::pow`
    RealPow { base: ExpId, exp: ExpId },
    /// `Real::abs`
    RealAbs { val: ExpId },
    /// `Real::round`
    RealRound { val: ExpId },
    /// `Real::floor`
    RealFloor { val: ExpId },
    /// `Real::ceil`
    RealCeil { val: ExpId },
    /// `Real::is_integer`
    RealIsInt { val: ExpId },
    /// `Real::lt`
    RealLt { lhs: ExpId, rhs: ExpId },
    /// `Real::le`
    RealLe { lhs: ExpId, rhs: ExpId },
    /// `Real::gt`
    RealGt { lhs: ExpId, rhs: ExpId },
    /// `Real::ge`
    RealGe { lhs: ExpId, rhs: ExpId },
    /// `Real::to_int`
    RealToInt { val: ExpId },
    /// `Real::to_f32`
    RealToF32 { val: ExpId },
    /// `Real::to_f64`
    RealToF64 { val: ExpId },

    /// `String::from`
    StrVal(String),
    /// `String::new`
    StrNew,
    /// `String::length`
    StrLen { seq: ExpId },
    /// `String::concat`
    StrConcat { lhs: ExpId, rhs: ExpId },
    /// `String::at`
    StrAt { seq: ExpId, idx: ExpId },
    /// `String::index_of`
    StrIndexOf {
        seq: ExpId,
        sub: ExpId,
        offset: ExpId,
    },
    /// `String::index_of_default`
    StrIndexOfDefault { seq: ExpId, sub: ExpId },
    /// `String::substr`
    StrSubstr {
        seq: ExpId,
        start: ExpId,
        len: ExpId,
    },
    /// `String::is_empty`
    StrIsEmpty { seq: ExpId },
    /// `String::contains`
    StrContains { seq: ExpId, item: ExpId },
    /// `String::starts_with`
    StrStartsWith { seq: ExpId, item: ExpId },
    /// `String::ends_with`
    StrEndsWith { seq: ExpId, item: ExpId },
    /// `String::is_digit`
    StrIsDigit { seq: ExpId },
    /// `String::le`
    StrLe { lhs: ExpId, rhs: ExpId },
    /// `String::lt`
    StrLt { lhs: ExpId, rhs: ExpId },
    /// `String::ge`
    StrGe { lhs: ExpId, rhs: ExpId },
    /// `String::gt`
    StrGt { lhs: ExpId, rhs: ExpId },
    /// `String::replace`
    StrReplace { seq: ExpId, src: ExpId, dst: ExpId },
    /// `String::replace_all`
    StrReplaceAll { seq: ExpId, src: ExpId, dst: ExpId },
    /// `String::to_int`
    StrToInt { val: ExpId },
    /// `String::from_int`
    StrFromInt { val: ExpId },
    /// `String::from_code`
    StrFromCode { val: ExpId },
    /// `String::to_code`
    StrToCode { val: ExpId },

    /// `Cloak::shield`
    BoxShield { t: Sort, val: ExpId },
    /// `Cloak::reveal`
    BoxReveal { t: Sort, val: ExpId },

    /// `Seq::new` (Empty)
    SeqEmpty { t: Sort },
    /// `Seq::unit`
    SeqUnit { t: Sort, val: ExpId },
    /// `Seq::length`
    SeqLen { t: Sort, seq: ExpId },
    /// `Seq::append`
    SeqPush { t: Sort, seq: ExpId, item: ExpId },
    /// `Seq::concat`
    SeqConcat { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Seq::at` (nth)
    SeqNth { t: Sort, seq: ExpId, idx: ExpId },
    /// `Seq::at_seq`
    SeqAtSeq { t: Sort, seq: ExpId, idx: ExpId },
    /// `Seq::extract`
    SeqExtract {
        t: Sort,
        seq: ExpId,
        offset: ExpId,
        len: ExpId,
    },
    /// `Seq::index_of`
    SeqIndexOf {
        t: Sort,
        seq: ExpId,
        sub: ExpId,
        offset: ExpId,
    },
    /// `Seq::index_of_default`
    SeqIndexOfDefault { t: Sort, seq: ExpId, sub: ExpId },
    /// `Seq::contains`
    SeqContains { t: Sort, seq: ExpId, item: ExpId },
    /// `Seq::prefix_of`
    SeqPrefixOf { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Seq::suffix_of`
    SeqSuffixOf { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Seq::replace`
    SeqReplace {
        t: Sort,
        seq: ExpId,
        src: ExpId,
        dst: ExpId,
    },
    /// `Seq::is_empty`
    SeqIsEmpty { t: Sort, seq: ExpId },

    /// `Set::new`
    SetEmpty { t: Sort },
    /// `Set::length`
    SetLen { t: Sort, set: ExpId },
    /// `Set::insert`
    SetInsert { t: Sort, set: ExpId, item: ExpId },
    /// `Set::remove`
    SetRemove { t: Sort, set: ExpId, item: ExpId },
    /// `Set::contains`
    SetContains { t: Sort, set: ExpId, item: ExpId },
    /// `Set::is_empty`
    SetIsEmpty { t: Sort, set: ExpId },
    /// `Set::intersection`
    SetIntersect { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::union`
    SetUnion { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::difference`
    SetDiff { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::symmetric_difference`
    SetSymDiff { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::is_subset`
    SetIsSubset { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::is_proper_subset`
    SetIsProperSubset { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::is_disjoint`
    SetIsDisjoint { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `Set::has_size`
    SetHasSize { t: Sort, set: ExpId, size: ExpId },

    /// `Array::new`
    ArrayEmpty { k: Sort, v: Sort },
    /// `Array::length`
    ArrayLen { k: Sort, v: Sort, arr: ExpId },
    /// `Array::store`
    ArrayStore {
        k: Sort,
        v: Sort,
        arr: ExpId,
        key: ExpId,
        val: ExpId,
    },
    /// `Array::select`
    ArraySelect {
        k: Sort,
        v: Sort,
        arr: ExpId,
        key: ExpId,
    },
    /// `Array::del`
    ArrayRemove {
        k: Sort,
        v: Sort,
        arr: ExpId,
        key: ExpId,
    },
    /// `Array::contains_key`
    ArrayContainsKey {
        k: Sort,
        v: Sort,
        arr: ExpId,
        key: ExpId,
    },
    /// `Array::is_empty`
    ArrayIsEmpty { k: Sort, v: Sort, arr: ExpId },

    /// `bv_val`
    BvVal { t: Sort, val: BigInt },
    /// `bv_not`
    BvNot { t: Sort, val: ExpId },
    /// `bv_and`
    BvAnd { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_or`
    BvOr { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_xor`
    BvXor { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_nand`
    BvNand { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_nor`
    BvNor { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_xnor`
    BvXnor { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_redand`
    BvRedAnd { t: Sort, val: ExpId },
    /// `bv_redor`
    BvRedOr { t: Sort, val: ExpId },
    /// `bv_neg`
    BvNeg { t: Sort, val: ExpId },
    /// `bv_add`
    BvAdd { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_sub`
    BvSub { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_mul`
    BvMul { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_div`
    BvDiv { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_rem`
    BvRem { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_mod`
    BvMod { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_shl`
    BvShl { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_lshr`
    BvLshr { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_ashr`
    BvAshr { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_rotate_left`
    BvRotLeft { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_rotate_right`
    BvRotRight { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_lt`
    BvLt { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_le`
    BvLe { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_gt`
    BvGt { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_ge`
    BvGe { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `bv_to_int`
    BvToInt { t: Sort, val: ExpId },

    /// `Float::val`
    FloatVal { t: Sort, val: BigRational },
    /// `FloatOps::add`
    FloatAdd { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::sub`
    FloatSub { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::mul`
    FloatMul { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::div`
    FloatDiv { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::neg`
    FloatNeg { t: Sort, val: ExpId },
    /// `FloatOps::abs`
    FloatAbs { t: Sort, val: ExpId },
    /// `FloatOps::rem`
    FloatRem { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::sqrt`
    FloatSqrt { t: Sort, val: ExpId },
    /// `FloatOps::min`
    FloatMin { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::max`
    FloatMax { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::is_nan`
    FloatIsNaN { t: Sort, val: ExpId },
    /// `FloatOps::is_infinite`
    FloatIsInf { t: Sort, val: ExpId },
    /// `FloatOps::is_zero`
    FloatIsZero { t: Sort, val: ExpId },
    /// `FloatOps::is_normal`
    FloatIsNormal { t: Sort, val: ExpId },
    /// `FloatOps::is_subnormal`
    FloatIsSubnormal { t: Sort, val: ExpId },
    /// `FloatOps::is_negative`
    FloatIsNeg { t: Sort, val: ExpId },
    /// `FloatOps::is_positive`
    FloatIsPos { t: Sort, val: ExpId },
    /// `FloatOps::lt`
    FloatLt { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::le`
    FloatLe { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::gt`
    FloatGt { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::ge`
    FloatGe { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::nan`
    FloatNaN { t: Sort },
    /// `FloatOps::infinity`
    FloatPosInf { t: Sort },
    /// `FloatOps::neg_infinity`
    FloatNegInf { t: Sort },
    /// `FloatOps::pos_zero`
    FloatPosZero { t: Sort },
    /// `FloatOps::neg_zero`
    FloatNegZero { t: Sort },
    /// `FloatOps::to_integer`
    FloatToInt { t: Sort, val: ExpId },
    /// `FloatOps::to_real`
    FloatToReal { t: Sort, val: ExpId },
    /// `FloatOps::to_u32`
    FloatToU32 { t: Sort, val: ExpId },
    /// `FloatOps::to_i32`
    FloatToI32 { t: Sort, val: ExpId },
    /// `FloatOps::to_u64`
    FloatToU64 { t: Sort, val: ExpId },
    /// `FloatOps::to_i64`
    FloatToI64 { t: Sort, val: ExpId },
    /// `FloatOps::ceil`
    FloatCeil { t: Sort, val: ExpId },
    /// `FloatOps::floor`
    FloatFloor { t: Sort, val: ExpId },
    /// `FloatOps::trunc`
    FloatTrunc { t: Sort, val: ExpId },
    /// `FloatOps::nearest`
    FloatNearest { t: Sort, val: ExpId },
    /// `FloatOps::fq_eq`
    FloatFqEq { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `FloatOps::from_hex_str`
    FloatFromHexStr { t: Sort, val: ExpId },

    /// `Error::fresh`
    ErrFresh,
    /// `Error::merge`
    ErrMerge { lhs: ExpId, rhs: ExpId },
    /// `<any-smt-type>::eq`
    SmtEq { t: Sort, lhs: ExpId, rhs: ExpId },
    /// `<any-smt-type>::ne`
    SmtNe { t: Sort, lhs: ExpId, rhs: ExpId },
}

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
    BoolNot {
        val: ExpId,
    },
    /// `Boolean::and`
    BoolAnd {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::or`
    BoolOr {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::xor`
    BoolXor {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::implies`
    BoolImplies {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::iff`
    BoolIff {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::nand`
    BoolNand {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::nor`
    BoolNor {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Boolean::xnor`
    BoolXnor {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::from`
    IntVal(BigInt),
    /// `Integer::neg`
    IntNeg {
        val: ExpId,
    },
    /// `Integer::lt`
    IntLt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::le`
    IntLe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::ge`
    IntGe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::gt`
    IntGt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::add`
    IntAdd {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::sub`
    IntSub {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::mul`
    IntMul {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::div`
    IntDiv {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::mod`
    IntMod {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::rem`
    IntRem {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Integer::to_real`
    IntoToReal {
        val: ExpId,
    },
    /// `Integer::pow`
    IntPow {
        base: ExpId,
        exp: ExpId,
    },
    /// `Integer::abs`
    IntAbs {
        val: ExpId,
    },
    /// `Integer::divides`
    IntDivides {
        lhs: ExpId,
        rhs: ExpId,
    },

    // Integer Conversions
    /// `Integer::to_i32`
    IntToI32 {
        val: ExpId,
    },
    /// `Integer::to_i64`
    IntToI64 {
        val: ExpId,
    },
    /// `Integer::to_u32`
    IntToU32 {
        val: ExpId,
    },
    /// `Integer::to_u64`
    IntToU64 {
        val: ExpId,
    },
    /// `Integer::to_f32`
    IntToF32 {
        val: ExpId,
    },
    /// `Integer::to_f64`
    IntToF64 {
        val: ExpId,
    },

    // Integer Parsing
    /// `Integer::from_hex_str`
    IntFromHex {
        val: ExpId,
    },
    /// `Integer::from_oct_str`
    IntFromOct {
        val: ExpId,
    },
    /// `Integer::from_bin_str`
    IntFromBin {
        val: ExpId,
    },

    // Integer Range Checks
    /// `Integer::is_gt_i64_max`
    IntIsGtI64Max {
        val: ExpId,
    },
    /// `Integer::is_lt_i64_min`
    IntIsLtI64Min {
        val: ExpId,
    },
    /// `Integer::is_gt_u64_max`
    IntIsGtU64Max {
        val: ExpId,
    },
    /// `Integer::is_lt_u64_min`
    IntIsLtU64Min {
        val: ExpId,
    },
    /// `Integer::is_lt_i32_min`
    IntIsLtI32Min {
        val: ExpId,
    },
    /// `Integer::is_gt_i32_max`
    IntIsGtI32Max {
        val: ExpId,
    },
    /// `Integer::is_lt_u32_min`
    IntIsLtU32Min {
        val: ExpId,
    },
    /// `Integer::is_gt_u32_max`
    IntIsGtU32Max {
        val: ExpId,
    },
    /// `Rational::from`
    RealVal(BigRational),
    /// `Rational::neg`
    RealNeg {
        val: ExpId,
    },
    /// `Rational::lt`
    RealLt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::le`
    RealLe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::ge`
    RealGe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::gt`
    RealGt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::add`
    RealAdd {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::sub`
    RealSub {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::mul`
    RealMul {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Rational::div`
    RealDiv {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Real::pow`
    RealPow {
        base: ExpId,
        exp: ExpId,
    },
    /// `Real::abs`
    RealAbs {
        val: ExpId,
    },
    /// `Real::round`
    RealRound {
        val: ExpId,
    },
    /// `Real::floor`
    RealFloor {
        val: ExpId,
    },
    /// `Real::ceil`
    RealCeil {
        val: ExpId,
    },
    /// `Real::is_integer`
    RealIsInt {
        val: ExpId,
    },
    /// `Real::to_int`
    RealToInt {
        val: ExpId,
    },
    /// `Real::to_f32`
    RealToF32 {
        val: ExpId,
    },
    /// `Real::to_f64`
    RealToF64 {
        val: ExpId,
    },
    /// `Real::numerator`
    RealRealer {
        val: ExpId,
    },
    /// `Real::denominator`
    RealDenom {
        val: ExpId,
    },
    /// `String::from`
    StrVal(String),
    /// `String::lt`
    StrLt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `String::le`
    StrLe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `String::gt`
    StrGt {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `String::ge`
    StrGe {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `String::concat`
    StrConcat {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `String::at_index`
    StrAt {
        seq: ExpId,
        idx: ExpId,
    },
    /// `String::length`
    StrLength {
        seq: ExpId,
    },
    /// `String::is_empty`
    StrIsEmpty {
        seq: ExpId,
    },
    /// `String::contains`
    StrIncludes {
        seq: ExpId,
        item: ExpId,
    },
    /// `String::starts_with`
    StrStartsWith {
        seq: ExpId,
        item: ExpId,
    },
    /// `String::ends_with`
    StrEndsWith {
        seq: ExpId,
        item: ExpId,
    },
    /// `String::is_digit`
    StrIsDigit {
        seq: ExpId,
    },
    /// `String::index_of`
    StrIndexOf {
        seq: ExpId,
        sub: ExpId,
        offset: ExpId,
    },
    /// `String::replace`
    StrReplace {
        seq: ExpId,
        src: ExpId,
        dst: ExpId,
    },
    /// `String::replace_all`
    StrReplaceAll {
        seq: ExpId,
        src: ExpId,
        dst: ExpId,
    },
    /// `String::to_int`
    StrToInt {
        val: ExpId,
    },
    /// `String::from_int`
    StrFromInt {
        val: ExpId,
    },
    /// `String::from_code`
    StrFromCode {
        val: ExpId,
    },
    /// `String::to_code`
    StrToCode {
        val: ExpId,
    },
    /// `Cloak::shield`
    BoxShield {
        t: Sort,
        val: ExpId,
    },
    /// `Cloak::reveal`
    BoxReveal {
        t: Sort,
        val: ExpId,
    },
    /// `Seq::empty`
    SeqEmpty {
        t: Sort,
    },
    /// `Seq::unit`
    SeqUnit {
        t: Sort,
        val: ExpId,
    },
    /// `Seq::length`
    SeqLength {
        t: Sort,
        seq: ExpId,
    },
    /// `Seq::at` (nth)
    SeqNth {
        t: Sort,
        seq: ExpId,
        idx: ExpId,
    },
    /// `Seq::at_unchecked`
    SeqAt {
        t: Sort,
        seq: ExpId,
        idx: ExpId,
    },
    /// `Seq::extract`
    SeqExtract {
        t: Sort,
        seq: ExpId,
        offset: ExpId,
        len: ExpId,
    },
    /// `Seq::append` (Push)
    SeqAppend {
        t: Sort,
        seq: ExpId,
        item: ExpId,
    },
    /// `Seq::concat`
    SeqConcat {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Seq::includes`
    SeqIncludes {
        t: Sort,
        seq: ExpId,
        item: ExpId,
    },
    /// `Seq::prefix_of`
    SeqPrefixOf {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Seq::suffix_of`
    SeqSuffixOf {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Seq::replace`
    SeqReplace {
        t: Sort,
        seq: ExpId,
        src: ExpId,
        dst: ExpId,
    },
    /// `Seq::is_empty`
    SeqIsEmpty {
        t: Sort,
        seq: ExpId,
    },
    /// `Set::empty`
    SetEmpty {
        t: Sort,
    },
    /// `Set::length`
    SetLength {
        t: Sort,
        set: ExpId,
    },
    /// `Set::insert`
    SetInsert {
        t: Sort,
        set: ExpId,
        item: ExpId,
    },
    /// `Set::remove`
    SetRemove {
        t: Sort,
        set: ExpId,
        item: ExpId,
    },
    /// `Set::contains`
    SetContains {
        t: Sort,
        set: ExpId,
        item: ExpId,
    },
    /// `Set::is_empty`
    SetIsEmpty {
        t: Sort,
        set: ExpId,
    },
    /// `Set::intersection`
    SetIntersection {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::union`
    SetUnion {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::difference`
    SetDifference {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::symmetric_difference`
    SetSymDiff {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::is_subset`
    SetIsSubset {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::is_proper_subset`
    SetIsProperSubset {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::is_superset`
    SetIsSuperset {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::is_disjoint`
    SetIsDisjoint {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Set::has_size`
    SetHasSize {
        t: Sort,
        set: ExpId,
        size: ExpId,
    },
    /// `Map::empty`
    MapEmpty {
        k: Sort,
        v: Sort,
    },
    /// `Map::length`
    MapLength {
        k: Sort,
        v: Sort,
        map: ExpId,
    },
    /// `Map::put_unchecked` (Store)
    MapPut {
        k: Sort,
        v: Sort,
        map: ExpId,
        key: ExpId,
        val: ExpId,
    },
    /// `Map::get_unchecked` (Select)
    MapGet {
        k: Sort,
        v: Sort,
        map: ExpId,
        key: ExpId,
    },
    /// `Map::del_unchecked`
    MapDel {
        k: Sort,
        v: Sort,
        map: ExpId,
        key: ExpId,
    },
    /// `Map::contains_key`
    MapContainsKey {
        k: Sort,
        v: Sort,
        map: ExpId,
        key: ExpId,
    },
    /// `Map::is_empty`
    MapIsEmpty {
        k: Sort,
        v: Sort,
        map: ExpId,
    },
    /// `Bv::val`
    BvVal {
        t: Sort,
        val: u64,
    }, // Using u64 to match IntVal(i64) pattern
    /// `Bv::not`
    BvNot {
        t: Sort,
        val: ExpId,
    },
    /// `Bv::neg`
    BvNeg {
        t: Sort,
        val: ExpId,
    },
    /// `Bv::redand`
    BvRedAnd {
        t: Sort,
        val: ExpId,
    },
    /// `Bv::redor`
    BvRedOr {
        t: Sort,
        val: ExpId,
    },
    /// `Bv::and`
    BvAnd {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::or`
    BvOr {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::xor`
    BvXor {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::nand`
    BvNand {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::nor`
    BvNor {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::xnor`
    BvXnor {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::add`
    BvAdd {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::sub`
    BvSub {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::mul`
    BvMul {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::div`
    BvDiv {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::rem`
    BvRem {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::mod`
    BvMod {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::shl`
    BvShl {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::lshr`
    BvLshr {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::ashr`
    BvAshr {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::rotate_left`
    BvRotLeft {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::rotate_right`
    BvRotRight {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::lt`
    BvLt {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::le`
    BvLe {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::gt`
    BvGt {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::ge`
    BvGe {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Bv::to_int`
    BvToInt {
        t: Sort,
        val: ExpId,
    },
    // Bv Overflow checks
    BvAddNoOverflow {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    BvSubNoOverflow {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    BvNegNoOverflow {
        t: Sort,
        val: ExpId,
    },
    BvMulNoOverflow {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    BvDivNoOverflow {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::val`
    FloatVal {
        t: Sort,
        val: BigRational,
    },
    /// `Float::nan`
    FloatNaN {
        t: Sort,
    },
    /// `Float::infinity`
    FloatPosInf {
        t: Sort,
    },
    /// `Float::neg_infinity`
    FloatNegInf {
        t: Sort,
    },
    /// `Float::pos_zero`
    FloatPosZero {
        t: Sort,
    },
    /// `Float::neg_zero`
    FloatNegZero {
        t: Sort,
    },
    /// `Float::neg`
    FloatNeg {
        t: Sort,
        val: ExpId,
    },
    /// `Float::abs`
    FloatAbs {
        t: Sort,
        val: ExpId,
    },
    /// `Float::sqrt`
    FloatSqrt {
        t: Sort,
        val: ExpId,
    },
    /// `Float::add`
    FloatAdd {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::sub`
    FloatSub {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::mul`
    FloatMul {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::div`
    FloatDiv {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::rem`
    FloatRem {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::min`
    FloatMin {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::max`
    FloatMax {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::is_nan`
    FloatIsNaN {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_infinite`
    FloatIsInf {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_zero`
    FloatIsZero {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_normal`
    FloatIsNormal {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_subnormal`
    FloatIsSubnormal {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_negative`
    FloatIsNeg {
        t: Sort,
        val: ExpId,
    },
    /// `Float::is_positive`
    FloatIsPos {
        t: Sort,
        val: ExpId,
    },
    /// `Float::lt`
    FloatLt {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::le`
    FloatLe {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::gt`
    FloatGt {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::ge`
    FloatGe {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `Float::to_integer`
    FloatToInt {
        t: Sort,
        val: ExpId,
    },
    /// `Float::to_real`
    FloatToReal {
        t: Sort,
        val: ExpId,
    },
    /// `Error::fresh`
    ErrFresh,
    /// `Error::merge`
    ErrMerge {
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `<any-smt-type>::eq`
    SmtEq {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
    /// `<any-smt-type>::ne`
    SmtNe {
        t: Sort,
        lhs: ExpId,
        rhs: ExpId,
    },
}

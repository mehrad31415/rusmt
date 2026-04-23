//! Intermediate representation (IR) intrinsics which are the equivalent of the parser intrinsic operations.

use crate::ir::index::ExpId;
use crate::ir::sort::Sort;
use num_bigint::BigInt;
use num_rational::BigRational;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
/// Intrinsic procedure
pub enum Intrinsic {
    /// `Boolean::from`
    BoolVal(bool),
    /// `Boolean::not`
    BoolNot {
        /// The value of the boolean not
        val: ExpId,
    },
    /// `Boolean::and`
    BoolAnd {
        /// The left operand of the boolean and
        lhs: ExpId,
        /// The right operand of the boolean and
        rhs: ExpId,
    },
    /// `Boolean::or`
    BoolOr {
        /// The left operand of the boolean or
        lhs: ExpId,
        /// The right operand of the boolean or
        rhs: ExpId,
    },
    /// `Boolean::xor`
    BoolXor {
        /// The left operand of the boolean xor
        lhs: ExpId,
        /// The right operand of the boolean xor
        rhs: ExpId,
    },
    /// `Boolean::nand`
    BoolNand {
        /// The left operand of the boolean nand
        lhs: ExpId,
        /// The right operand of the boolean nand
        rhs: ExpId,
    },
    /// `Boolean::nor`
    BoolNor {
        /// The left operand of the boolean nor
        lhs: ExpId,
        /// The right operand of the boolean nor
        rhs: ExpId,
    },
    /// `Boolean::xnor`
    BoolXnor {
        /// The left operand of the boolean xnor
        lhs: ExpId,
        /// The right operand of the boolean xnor
        rhs: ExpId,
    },
    /// `Boolean::implies`
    BoolImplies {
        /// The left operand of the boolean implies
        lhs: ExpId,
        /// The right operand of the boolean implies
        rhs: ExpId,
    },
    /// `Boolean::iff`
    BoolIff {
        /// The left operand of the boolean iff
        lhs: ExpId,
        /// The right operand of the boolean iff
        rhs: ExpId,
    },
    /// `Boolean::ite`
    BoolIte {
        /// The condition of the boolean ite
        cond: ExpId,
        /// The then operand of the boolean ite
        then: ExpId,
        /// The else operand of the boolean ite
        else_: ExpId,
    },

    /// `Integer::from`
    IntVal(BigInt),
    /// `Integer::neg`
    IntNeg {
        /// The value of the integer neg
        val: ExpId,
    },
    /// `Integer::add`
    IntAdd {
        /// The left operand of the integer add
        lhs: ExpId,
        /// The right operand of the integer add
        rhs: ExpId,
    },
    /// `Integer::sub`
    IntSub {
        /// The left operand of the integer sub
        lhs: ExpId,
        /// The right operand of the integer sub
        rhs: ExpId,
    },
    /// `Integer::mul`
    IntMul {
        /// The left operand of the integer mul
        lhs: ExpId,
        /// The right operand of the integer mul
        rhs: ExpId,
    },
    /// `Integer::div`
    IntDiv {
        /// The left operand of the integer div
        lhs: ExpId,
        /// The right operand of the integer div
        rhs: ExpId,
    },
    /// `Integer::div_trunc`
    IntDivTrunc {
        /// The left operand of the integer div_trunc
        lhs: ExpId,
        /// The right operand of the integer div_trunc
        rhs: ExpId,
    },
    /// `Integer::modulo`
    IntMod {
        /// The left operand of the integer modulo
        lhs: ExpId,
        /// The right operand of the integer modulo
        rhs: ExpId,
    },
    /// `Integer::rem`
    IntRem {
        /// The left operand of the integer rem
        lhs: ExpId,
        /// The right operand of the integer rem
        rhs: ExpId,
    },
    /// `Integer::pow`
    IntPow {
        /// The base of the integer pow
        base: ExpId,
        /// The exponent of the integer pow
        exp: ExpId,
    },
    /// `Integer::abs`
    IntAbs {
        /// The value of the integer abs
        val: ExpId,
    },
    /// `Integer::divides`
    IntDivides {
        /// The left operand of the integer divides
        lhs: ExpId,
        /// The right operand of the integer divides
        rhs: ExpId,
    },
    /// `Integer::lt`
    IntLt {
        /// The left operand of the integer lt
        lhs: ExpId,
        /// The right operand of the integer lt
        rhs: ExpId,
    },
    /// `Integer::le`
    IntLe {
        /// The left operand of the integer le
        lhs: ExpId,
        /// The right operand of the integer le
        rhs: ExpId,
    },
    /// `Integer::gt`
    IntGt {
        /// The left operand of the integer gt
        lhs: ExpId,
        /// The right operand of the integer gt
        rhs: ExpId,
    },
    /// `Integer::ge`
    IntGe {
        /// The left operand of the integer ge
        lhs: ExpId,
        /// The right operand of the integer ge
        rhs: ExpId,
    },

    // Integer Conversions
    /// `Integer::to_real`
    IntToReal {
        /// The value of the integer to real
        val: ExpId,
    },
    /// `Integer::to_i32`
    IntToI32 {
        /// The value of the integer to i32
        val: ExpId,
    },
    /// `Integer::to_i64`
    IntToI64 {
        /// The value of the integer to i64
        val: ExpId,
    },
    /// `Integer::to_u32`
    IntToU32 {
        /// The value of the integer to u32
        val: ExpId,
    },
    /// `Integer::to_u64`
    IntToU64 {
        /// The value of the integer to u64
        val: ExpId,
    },
    /// `Integer::to_f32`
    IntToF32 {
        /// The value of the integer to f32
        val: ExpId,
    },
    /// `Integer::to_f64`
    IntToF64 {
        /// The value of the integer to f64
        val: ExpId,
    },

    // Integer Parsing
    /// `Integer::from_hex_str`
    IntFromHex {
        /// The value of the integer from hex str
        val: ExpId,
    },
    /// `Integer::from_oct_str`
    IntFromOct {
        /// The value of the integer from oct str
        val: ExpId,
    },
    /// `Integer::from_bin_str`
    IntFromBin {
        /// The value of the integer from bin str
        val: ExpId,
    },

    // Integer Range Checks
    /// `Integer::is_gt_i64_max`
    IntIsGtI64Max {
        /// The value of the integer is gt i64 max
        val: ExpId,
    },
    /// `Integer::is_lt_i64_min`
    IntIsLtI64Min {
        /// The value of the integer is lt i64 min
        val: ExpId,
    },
    /// `Integer::is_gt_u64_max`
    IntIsGtU64Max {
        /// The value of the integer is gt u64 max
        val: ExpId,
    },
    /// `Integer::is_lt_u64_min`
    IntIsLtU64Min {
        /// The value of the integer is lt u64 min
        val: ExpId,
    },
    /// `Integer::is_lt_i32_min`
    IntIsLtI32Min {
        /// The value of the integer is lt i32 min
        val: ExpId,
    },
    /// `Integer::is_gt_i32_max`
    IntIsGtI32Max {
        /// The value of the integer is gt i32 max
        val: ExpId,
    },
    /// `Integer::is_lt_u32_min`
    IntIsLtU32Min {
        /// The value of the integer is lt u32 min
        val: ExpId,
    },
    /// `Integer::is_gt_u32_max`
    IntIsGtU32Max {
        /// The value of the integer is gt u32 max
        val: ExpId,
    },

    /// `Real::from`
    RealVal(BigRational),
    /// `Real::neg`
    RealNeg {
        /// The value of the real neg
        val: ExpId,
    },
    /// `Real::add`
    RealAdd {
        /// The left operand of the real add
        lhs: ExpId,
        /// The right operand of the real add
        rhs: ExpId,
    },
    /// `Real::sub`
    RealSub {
        /// The left operand of the real sub
        lhs: ExpId,
        /// The right operand of the real sub
        rhs: ExpId,
    },
    /// `Real::mul`
    RealMul {
        /// The left operand of the real mul
        lhs: ExpId,
        /// The right operand of the real mul
        rhs: ExpId,
    },
    /// `Real::div`
    RealDiv {
        /// The left operand of the real div
        lhs: ExpId,
        /// The right operand of the real div
        rhs: ExpId,
    },
    /// `Real::pow`
    RealPow {
        /// The base operand of the real pow
        base: ExpId,
        /// The exponent operand of the real pow
        exp: ExpId,
    },
    /// `Real::abs`
    RealAbs {
        /// The value of the real abs
        val: ExpId,
    },
    /// `Real::round`
    RealRound {
        /// The value of the real round
        val: ExpId,
    },
    /// `Real::floor`
    RealFloor {
        /// The value of the real floor
        val: ExpId,
    },
    /// `Real::ceil`
    RealCeil {
        /// The value of the real ceil
        val: ExpId,
    },
    /// `Real::is_integer`
    RealIsInt {
        /// The value of the real is integer
        val: ExpId,
    },
    /// `Real::lt`
    RealLt {
        /// The left operand of the real lt
        lhs: ExpId,
        /// The right operand of the real lt
        rhs: ExpId,
    },
    /// `Real::le`
    RealLe {
        /// The left operand of the real le
        lhs: ExpId,
        /// The right operand of the real le
        rhs: ExpId,
    },
    /// `Real::gt`
    RealGt {
        /// The left operand of the real gt
        lhs: ExpId,
        /// The right operand of the real gt
        rhs: ExpId,
    },
    /// `Real::ge`
    RealGe {
        /// The left operand of the real ge
        lhs: ExpId,
        /// The right operand of the real ge
        rhs: ExpId,
    },
    /// `Real::to_int`
    RealToInt {
        /// The value of the real to int
        val: ExpId,
    },
    /// `Real::to_f32`
    RealToF32 {
        /// The value of the real to f32
        val: ExpId,
    },
    /// `Real::to_f64`
    RealToF64 {
        /// The value of the real to f64
        val: ExpId,
    },

    /// `String::from`
    StrVal(String),
    /// `String::new`
    StrNew,
    /// `String::length`
    StrLen {
        /// The sequence of the string length
        seq: ExpId,
    },
    /// `String::concat`
    StrConcat {
        /// The left operand of the string concat
        lhs: ExpId,
        /// The right operand of the string concat
        rhs: ExpId,
    },
    /// `String::at`
    StrAt {
        /// The sequence of the string at
        seq: ExpId,
        /// The index of the string at
        idx: ExpId,
    },
    /// `String::index_of`
    StrIndexOf {
        /// The sequence of the string index of
        seq: ExpId,
        /// The substring of the string index of
        sub: ExpId,
        /// The offset of the string index of
        offset: ExpId,
    },
    /// `String::index_of_default`
    StrIndexOfDefault {
        /// The sequence of the string index of default
        seq: ExpId,
        /// The substring of the string index of default
        sub: ExpId,
    },
    /// `String::substr`
    StrSubstr {
        /// The sequence of the string substr
        seq: ExpId,
        /// The start of the string substr
        start: ExpId,
        /// The length of the string substr
        len: ExpId,
    },
    /// `String::is_empty`
    StrIsEmpty {
        /// The sequence of the string is empty
        seq: ExpId,
    },
    /// `String::contains`
    StrContains {
        /// The sequence of the string contains
        seq: ExpId,
        /// The item of the string contains
        item: ExpId,
    },
    /// `String::starts_with`
    StrStartsWith {
        /// The sequence of the string starts with
        seq: ExpId,
        /// The item of the string starts with
        item: ExpId,
    },
    /// `String::ends_with`
    StrEndsWith {
        /// The sequence of the string ends with
        seq: ExpId,
        /// The item of the string ends with
        item: ExpId,
    },
    /// `String::is_digit`
    StrIsDigit {
        /// The sequence of the string is digit
        seq: ExpId,
    },
    /// `String::le`
    StrLe {
        /// The left operand of the string le
        lhs: ExpId,
        /// The right operand of the string le
        rhs: ExpId,
    },
    /// `String::lt`
    StrLt {
        /// The left operand of the string lt
        lhs: ExpId,
        /// The right operand of the string lt
        rhs: ExpId,
    },
    /// `String::ge`
    StrGe {
        /// The left operand of the string ge
        lhs: ExpId,
        /// The right operand of the string ge
        rhs: ExpId,
    },
    /// `String::gt`
    StrGt {
        /// The left operand of the string gt
        lhs: ExpId,
        /// The right operand of the string gt
        rhs: ExpId,
    },
    /// `String::replace`
    StrReplace {
        /// The sequence of the string replace
        seq: ExpId,
        /// The source of the string replace
        src: ExpId,
        /// The destination of the string replace
        dst: ExpId,
    },
    /// `String::replace_all`
    StrReplaceAll {
        /// The sequence of the string replace all
        seq: ExpId,
        /// The source of the string replace all
        src: ExpId,
        /// The destination of the string replace all
        dst: ExpId,
    },
    /// `String::to_int`
    StrToInt {
        /// The value of the string to int
        val: ExpId,
    },
    /// `String::from_int`
    StrFromInt {
        /// The value of the string from int
        val: ExpId,
    },
    /// `String::from_code`
    StrFromCode {
        /// The value of the string from code
        val: ExpId,
    },
    /// `String::to_code`
    StrToCode {
        /// The value of the string to code
        val: ExpId,
    },

    /// `Cloak::shield`
    BoxShield {
        /// The type of the box shield
        t: Sort,
        /// The value of the box shield
        val: ExpId,
    },
    /// `Cloak::reveal`
    BoxReveal {
        /// The type of the box reveal
        t: Sort,
        /// The value of the box reveal
        val: ExpId,
    },

    /// `Seq::new` (Empty)
    SeqEmpty {
        /// The type of the sequence empty
        t: Sort,
    },
    /// `Seq::unit`
    SeqUnit {
        /// The type of the sequence unit
        t: Sort,
        /// The value of the sequence unit
        val: ExpId,
    },
    /// `Seq::length`
    SeqLen {
        /// The type of the sequence length
        t: Sort,
        /// The sequence of the sequence length
        seq: ExpId,
    },
    /// `Seq::append`
    SeqPush {
        /// The type of the sequence push
        t: Sort,
        /// The sequence of the sequence push
        seq: ExpId,
        /// The item of the sequence push
        item: ExpId,
    },
    /// `Seq::concat`
    SeqConcat {
        /// The type of the sequence concat
        t: Sort,
        /// The left operand of the sequence concat
        lhs: ExpId,
        /// The right operand of the sequence concat
        rhs: ExpId,
    },
    /// `Seq::at` (nth)
    SeqNth {
        /// The type of the sequence nth
        t: Sort,
        /// The sequence of the sequence nth
        seq: ExpId,
        /// The index of the sequence nth
        idx: ExpId,
    },
    /// `Seq::at_seq`
    SeqAtSeq {
        /// The type of the sequence at seq
        t: Sort,
        /// The sequence of the sequence at seq
        seq: ExpId,
        /// The index of the sequence at seq
        idx: ExpId,
    },
    /// `Seq::extract`
    SeqExtract {
        /// The type of the sequence extract
        t: Sort,
        /// The sequence of the sequence extract
        seq: ExpId,
        /// The offset of the sequence extract
        offset: ExpId,
        /// The length of the sequence extract
        len: ExpId,
    },
    /// `Seq::index_of`
    SeqIndexOf {
        /// The type of the sequence index of
        t: Sort,
        /// The sequence of the sequence index of
        seq: ExpId,
        /// The subsequence of the sequence index of
        sub: ExpId,
        /// The offset of the sequence index of
        offset: ExpId,
    },
    /// `Seq::index_of_default`
    SeqIndexOfDefault {
        /// The type of the sequence index of default
        t: Sort,
        /// The sequence of the sequence index of default
        seq: ExpId,
        /// The subsequence of the sequence index of default
        sub: ExpId,
    },
    /// `Seq::contains`
    SeqContains {
        /// The type of the sequence contains
        t: Sort,
        /// The sequence of the sequence contains
        seq: ExpId,
        /// The item of the sequence contains
        item: ExpId,
    },
    /// `Seq::prefix_of`
    SeqPrefixOf {
        /// The type of the sequence prefix of
        t: Sort,
        /// The left operand of the sequence prefix of
        lhs: ExpId,
        /// The right operand of the sequence prefix of
        rhs: ExpId,
    },
    /// `Seq::suffix_of`
    SeqSuffixOf {
        /// The type of the sequence suffix of
        t: Sort,
        /// The left operand of the sequence suffix of
        lhs: ExpId,
        /// The right operand of the sequence suffix of
        rhs: ExpId,
    },
    /// `Seq::replace`
    SeqReplace {
        /// The type of the sequence replace
        t: Sort,
        /// The sequence of the sequence replace
        seq: ExpId,
        /// The source of the sequence replace
        src: ExpId,
        /// The destination of the sequence replace
        dst: ExpId,
    },
    /// `Seq::is_empty`
    SeqIsEmpty {
        /// The type of the sequence is empty
        t: Sort,
        /// The sequence of the sequence is empty
        seq: ExpId,
    },

    /// `Set::new`
    SetEmpty {
        /// The type of the set is empty
        t: Sort,
    },
    /// `Set::length`
    SetLen {
        /// The type of the set length
        t: Sort,
        /// The set of the set length
        set: ExpId,
    },
    /// `Set::insert`
    SetInsert {
        /// The type of the set insert
        t: Sort,
        /// The set of the set insert
        set: ExpId,
        /// The item of the set insert
        item: ExpId,
    },
    /// `Set::remove`
    SetRemove {
        /// The type of the set remove
        t: Sort,
        /// The set of the set remove
        set: ExpId,
        /// The item of the set remove
        item: ExpId,
    },
    /// `Set::contains`
    SetContains {
        /// The type of the set contains
        t: Sort,
        /// The set of the set contains
        set: ExpId,
        /// The item of the set contains
        item: ExpId,
    },
    /// `Set::is_empty`
    SetIsEmpty {
        /// The type of the set is empty
        t: Sort,
        /// The set of the set is empty
        set: ExpId,
    },
    /// `Set::intersection`
    SetIntersect {
        /// The type of the set intersect
        t: Sort,
        /// The left operand of the set intersect
        lhs: ExpId,
        /// The right operand of the set intersect
        rhs: ExpId,
    },
    /// `Set::union`
    SetUnion {
        /// The type of the set union
        t: Sort,
        /// The left operand of the set union
        lhs: ExpId,
        /// The right operand of the set union
        rhs: ExpId,
    },
    /// `Set::difference`
    SetDiff {
        /// The type of the set difference
        t: Sort,
        /// The left operand of the set difference
        lhs: ExpId,
        /// The right operand of the set difference
        rhs: ExpId,
    },
    /// `Set::symmetric_difference`
    SetSymDiff {
        /// The type of the set symmetric difference
        t: Sort,
        /// The left operand of the set symmetric difference
        lhs: ExpId,
        /// The right operand of the set symmetric difference
        rhs: ExpId,
    },
    /// `Set::is_subset`
    SetIsSubset {
        /// The type of the set is subset
        t: Sort,
        /// The left operand of the set is subset
        lhs: ExpId,
        /// The right operand of the set is subset
        rhs: ExpId,
    },
    /// `Set::is_proper_subset`
    SetIsProperSubset {
        /// The type of the set is proper subset
        t: Sort,
        /// The left operand of the set is proper subset
        lhs: ExpId,
        /// The right operand of the set is proper subset
        rhs: ExpId,
    },
    /// `Set::is_disjoint`
    SetIsDisjoint {
        /// The type of the set is disjoint
        t: Sort,
        /// The left operand of the set is disjoint
        lhs: ExpId,
        /// The right operand of the set is disjoint
        rhs: ExpId,
    },
    /// `Set::has_size`
    SetHasSize {
        /// The type of the set has size
        t: Sort,
        /// The set of the set has size
        set: ExpId,
        /// The size of the set has size
        size: ExpId,
    },

    /// `Array::new`
    ArrayEmpty {
        /// The type of the array empty
        k: Sort,
        /// The value type of the array empty
        v: Sort,
    },
    /// `Array::length`
    ArrayLen {
        /// The type of the array length
        k: Sort,
        /// The value type of the array length
        v: Sort,
        /// The array of the array length
        arr: ExpId,
    },
    /// `Array::store`
    ArrayStore {
        /// The type of the array store
        k: Sort,
        /// The value type of the array store
        v: Sort,
        /// The array of the array store
        arr: ExpId,
        /// The key of the array store
        key: ExpId,
        /// The value of the array store
        val: ExpId,
    },
    /// `Array::select`
    ArraySelect {
        /// The type of the array select
        k: Sort,
        /// The value type of the array select
        v: Sort,
        /// The array of the array select
        arr: ExpId,
        /// The key of the array select
        key: ExpId,
    },
    /// `Array::del`
    ArrayRemove {
        /// The type of the array remove
        k: Sort,
        /// The value type of the array remove
        v: Sort,
        /// The array of the array remove
        arr: ExpId,
        /// The key of the array remove
        key: ExpId,
    },
    /// `Array::contains_key`
    ArrayContainsKey {
        /// The type of the array contains key
        k: Sort,
        /// The value type of the array contains key
        v: Sort,
        /// The array of the array contains key
        arr: ExpId,
        /// The key of the array contains key
        key: ExpId,
    },
    /// `Array::is_empty`
    ArrayIsEmpty {
        /// The type of the array is empty
        k: Sort,
        /// The value type of the array is empty
        v: Sort,
        /// The array of the array is empty
        arr: ExpId,
    },

    /// `bv_val`
    BvVal {
        /// The type of the bv val
        t: Sort,
        /// The value of the bv val
        val: BigInt,
    },
    /// `bv_not`
    BvNot {
        /// The type of the bv not
        t: Sort,
        /// The value of the bv not
        val: ExpId,
    },
    /// `bv_and`
    BvAnd {
        /// The type of the bv and
        t: Sort,
        /// The left operand of the bv and
        lhs: ExpId,
        /// The right operand of the bv and
        rhs: ExpId,
    },
    /// `bv_or`
    BvOr {
        /// The type of the bv or
        t: Sort,
        /// The left operand of the bv or
        lhs: ExpId,
        /// The right operand of the bv or
        rhs: ExpId,
    },
    /// `bv_xor`
    BvXor {
        /// The type of the bv xor
        t: Sort,
        /// The left operand of the bv xor
        lhs: ExpId,
        /// The right operand of the bv xor
        rhs: ExpId,
    },
    /// `bv_nand`
    BvNand {
        /// The type of the bv nand
        t: Sort,
        /// The left operand of the bv nand
        lhs: ExpId,
        /// The right operand of the bv nand
        rhs: ExpId,
    },
    /// `bv_nor`
    BvNor {
        /// The type of the bv nor
        t: Sort,
        /// The left operand of the bv nor
        lhs: ExpId,
        /// The right operand of the bv nor
        rhs: ExpId,
    },
    /// `bv_xnor`
    BvXnor {
        /// The type of the bv xnor
        t: Sort,
        /// The left operand of the bv xnor
        lhs: ExpId,
        /// The right operand of the bv xnor
        rhs: ExpId,
    },
    /// `bv_redand`
    BvRedAnd {
        /// The type of the bv redand
        t: Sort,
        /// The value of the bv redand
        val: ExpId,
    },
    /// `bv_redor`
    BvRedOr {
        /// The type of the bv redor
        t: Sort,
        /// The value of the bv redor
        val: ExpId,
    },
    /// `bv_neg`
    BvNeg {
        /// The type of the bv neg
        t: Sort,
        /// The value of the bv neg
        val: ExpId,
    },
    /// `bv_add`
    BvAdd {
        /// The type of the bv add
        t: Sort,
        /// The left operand of the bv add
        lhs: ExpId,
        /// The right operand of the bv add
        rhs: ExpId,
    },
    /// `bv_sub`
    BvSub {
        /// The type of the bv sub
        t: Sort,
        /// The left operand of the bv sub
        lhs: ExpId,
        /// The right operand of the bv sub
        rhs: ExpId,
    },
    /// `bv_mul`
    BvMul {
        /// The type of the bv mul
        t: Sort,
        /// The left operand of the bv mul
        lhs: ExpId,
        /// The right operand of the bv mul
        rhs: ExpId,
    },
    /// `bv_div`
    BvDiv {
        /// The type of the bv div
        t: Sort,
        /// The left operand of the bv div
        lhs: ExpId,
        /// The right operand of the bv div
        rhs: ExpId,
    },
    /// `bv_rem`
    BvRem {
        /// The type of the bv rem
        t: Sort,
        /// The left operand of the bv rem
        lhs: ExpId,
        /// The right operand of the bv rem
        rhs: ExpId,
    },
    /// `bv_mod`
    BvMod {
        /// The type of the bv mod
        t: Sort,
        /// The left operand of the bv mod
        lhs: ExpId,
        /// The right operand of the bv mod
        rhs: ExpId,
    },
    /// `bv_shl`
    BvShl {
        /// The type of the bv shl
        t: Sort,
        /// The left operand of the bv shl
        lhs: ExpId,
        /// The right operand of the bv shl
        rhs: ExpId,
    },
    /// `bv_lshr`
    BvLshr {
        /// The type of the bv lshr
        t: Sort,
        /// The left operand of the bv lshr
        lhs: ExpId,
        /// The right operand of the bv lshr
        rhs: ExpId,
    },
    /// `bv_ashr`
    BvAshr {
        /// The type of the bv ashr
        t: Sort,
        /// The left operand of the bv ashr
        lhs: ExpId,
        /// The right operand of the bv ashr
        rhs: ExpId,
    },
    /// `bv_rotate_left`
    BvRotLeft {
        /// The type of the bv rotate left
        t: Sort,
        /// The left operand of the bv rotate left
        lhs: ExpId,
        /// The right operand of the bv rotate left
        rhs: ExpId,
    },
    /// `bv_rotate_right`
    BvRotRight {
        /// The type of the bv rotate right
        t: Sort,
        /// The left operand of the bv rotate right
        lhs: ExpId,
        /// The right operand of the bv rotate right
        rhs: ExpId,
    },
    /// `bv_lt`
    BvLt {
        /// The type of the bv lt
        t: Sort,
        /// The left operand of the bv lt
        lhs: ExpId,
        /// The right operand of the bv lt
        rhs: ExpId,
    },
    /// `bv_le`
    BvLe {
        /// The type of the bv le
        t: Sort,
        /// The left operand of the bv le
        lhs: ExpId,
        /// The right operand of the bv le
        rhs: ExpId,
    },
    /// `bv_gt`
    BvGt {
        /// The type of the bv gt
        t: Sort,
        /// The left operand of the bv gt
        lhs: ExpId,
        /// The right operand of the bv gt
        rhs: ExpId,
    },
    /// `bv_ge`
    BvGe {
        /// The type of the bv ge
        t: Sort,
        /// The left operand of the bv ge
        lhs: ExpId,
        /// The right operand of the bv ge
        rhs: ExpId,
    },
    /// `bv_to_int`
    BvToInt {
        /// The type of the bv to int
        t: Sort,
        /// The value of the bv to int
        val: ExpId,
    },

    /// `Float::val`
    FloatVal {
        /// The type of the float val
        t: Sort,
        /// The value of the float val
        val: BigRational,
    },
    /// `FloatOps::add`
    FloatAdd {
        /// The type of the float add
        t: Sort,
        /// The left operand of the float add
        lhs: ExpId,
        /// The right operand of the float add
        rhs: ExpId,
    },
    /// `FloatOps::sub`
    FloatSub {
        /// The type of the float sub
        t: Sort,
        /// The left operand of the float sub
        lhs: ExpId,
        /// The right operand of the float sub
        rhs: ExpId,
    },
    /// `FloatOps::mul`
    FloatMul {
        /// The type of the float mul
        t: Sort,
        /// The left operand of the float mul
        lhs: ExpId,
        /// The right operand of the float mul
        rhs: ExpId,
    },
    /// `FloatOps::div`
    FloatDiv {
        /// The type of the float div
        t: Sort,
        /// The left operand of the float div
        lhs: ExpId,
        /// The right operand of the float div
        rhs: ExpId,
    },
    /// `FloatOps::neg`
    FloatNeg {
        /// The type of the float neg
        t: Sort,
        /// The value of the float neg
        val: ExpId,
    },
    /// `FloatOps::abs`
    FloatAbs {
        /// The type of the float abs
        t: Sort,
        /// The value of the float abs
        val: ExpId,
    },
    /// `FloatOps::rem`
    FloatRem {
        /// The type of the float rem
        t: Sort,
        /// The left operand of the float rem
        lhs: ExpId,
        /// The right operand of the float rem
        rhs: ExpId,
    },
    /// `FloatOps::sqrt`
    FloatSqrt {
        /// The type of the float sqrt
        t: Sort,
        /// The value of the float sqrt
        val: ExpId,
    },
    /// `FloatOps::min`
    FloatMin {
        /// The type of the float min
        t: Sort,
        /// The left operand of the float min
        lhs: ExpId,
        /// The right operand of the float min
        rhs: ExpId,
    },
    /// `FloatOps::max`
    FloatMax {
        /// The type of the float max
        t: Sort,
        /// The left operand of the float max
        lhs: ExpId,
        /// The right operand of the float max
        rhs: ExpId,
    },
    /// `FloatOps::is_nan`
    FloatIsNaN {
        /// The type of the float is nan
        t: Sort,
        /// The value of the float is nan
        val: ExpId,
    },
    /// `FloatOps::is_infinite`
    FloatIsInf {
        /// The type of the float is infinite
        t: Sort,
        /// The value of the float is infinite
        val: ExpId,
    },
    /// `FloatOps::is_zero`
    FloatIsZero {
        /// The type of the float is zero
        t: Sort,
        /// The value of the float is zero
        val: ExpId,
    },
    /// `FloatOps::is_normal`
    FloatIsNormal {
        /// The type of the float is normal
        t: Sort,
        /// The value of the float is normal
        val: ExpId,
    },
    /// `FloatOps::is_subnormal`
    FloatIsSubnormal {
        /// The type of the float is subnormal
        t: Sort,
        /// The value of the float is subnormal
        val: ExpId,
    },
    /// `FloatOps::is_negative`
    FloatIsNeg {
        /// The type of the float is negative
        t: Sort,
        /// The value of the float is negative
        val: ExpId,
    },
    /// `FloatOps::is_positive`
    FloatIsPos {
        /// The type of the float is positive
        t: Sort,
        /// The value of the float is positive
        val: ExpId,
    },
    /// `FloatOps::lt`
    FloatLt {
        /// The type of the float lt
        t: Sort,
        /// The left operand of the float lt
        lhs: ExpId,
        /// The right operand of the float lt
        rhs: ExpId,
    },
    /// `FloatOps::le`
    FloatLe {
        /// The type of the float le
        t: Sort,
        /// The left operand of the float le
        lhs: ExpId,
        /// The right operand of the float le
        rhs: ExpId,
    },
    /// `FloatOps::gt`
    FloatGt {
        /// The type of the float gt
        t: Sort,
        /// The left operand of the float gt
        lhs: ExpId,
        /// The right operand of the float gt
        rhs: ExpId,
    },
    /// `FloatOps::ge`
    FloatGe {
        /// The type of the float ge
        t: Sort,
        /// The left operand of the float ge
        lhs: ExpId,
        /// The right operand of the float ge
        rhs: ExpId,
    },
    /// `FloatOps::nan`
    FloatNaN {
        /// The type of the float nan
        t: Sort,
    },
    /// `FloatOps::infinity`
    FloatPosInf {
        /// The type of the float pos infinity
        t: Sort,
    },
    /// `FloatOps::neg_infinity`
    FloatNegInf {
        /// The type of the float neg infinity
        t: Sort,
    },
    /// `FloatOps::pos_zero`
    FloatPosZero {
        /// The type of the float pos zero
        t: Sort,
    },
    /// `FloatOps::neg_zero`
    FloatNegZero {
        /// The type of the float neg zero
        t: Sort,
    },
    /// `FloatOps::to_integer`
    FloatToInt {
        /// The type of the float to integer
        t: Sort,
        /// The value of the float to integer
        val: ExpId,
    },
    /// `FloatOps::to_real`
    FloatToReal {
        /// The type of the float to real
        t: Sort,
        /// The value of the float to real
        val: ExpId,
    },
    /// `FloatOps::to_u32`
    FloatToU32 {
        /// The type of the float to u32
        t: Sort,
        /// The value of the float to u32
        val: ExpId,
    },
    /// `FloatOps::to_i32`
    FloatToI32 {
        /// The type of the float to i32
        t: Sort,
        /// The value of the float to i32
        val: ExpId,
    },
    /// `FloatOps::to_u64`
    FloatToU64 {
        /// The type of the float to u64
        t: Sort,
        /// The value of the float to u64
        val: ExpId,
    },
    /// `FloatOps::to_i64`
    FloatToI64 {
        /// The type of the float to i64
        t: Sort,
        /// The value of the float to i64
        val: ExpId,
    },
    /// `FloatOps::ceil`
    FloatCeil {
        /// The type of the float ceil
        t: Sort,
        /// The value of the float ceil
        val: ExpId,
    },
    /// `FloatOps::floor`
    FloatFloor {
        /// The type of the float floor
        t: Sort,
        /// The value of the float floor
        val: ExpId,
    },
    /// `FloatOps::trunc`
    FloatTrunc {
        /// The type of the float trunc
        t: Sort,
        /// The value of the float trunc
        val: ExpId,
    },
    /// `FloatOps::nearest`
    FloatNearest {
        /// The type of the float nearest
        t: Sort,
        /// The value of the float nearest
        val: ExpId,
    },
    /// `FloatOps::fq_eq`
    FloatFqEq {
        /// The type of the float fq eq
        t: Sort,
        /// The left operand
        lhs: ExpId,
        /// The right operand
        rhs: ExpId,
    },
    /// `Error::fresh` - carries the globally unique error ID assigned at IR-build time
    ErrFresh(usize),
    /// `Error::merge`
    ErrMerge {
        /// The left operand
        lhs: ExpId,
        /// The right operand
        rhs: ExpId,
        /// All ErrFresh IDs reachable through the merge tree
        ids: BTreeSet<usize>,
    },
    /// `<any-smt-type>::eq`
    SmtEq {
        /// The type of the smt eq
        t: Sort,
        /// The left operand
        lhs: ExpId,
        /// The right operand
        rhs: ExpId,
    },
    /// `<any-smt-type>::ne`
    SmtNe {
        /// The type of the smt ne
        t: Sort,
        /// The left operand
        lhs: ExpId,
        /// The right operand
        rhs: ExpId,
    },
}

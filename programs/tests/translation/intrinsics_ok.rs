// Test 4: Intrinsic operations
// Tests: Boolean ops, Integer ops, Real ops, String ops, comparisons

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Real, String, smt::SMT};

// Boolean operations
#[smt_fn]
pub fn bool_ops(a: Boolean, b: Boolean) -> Boolean {
    Boolean::and(Boolean::or(a, b), Boolean::not(Boolean::xor(a, b)))
}

// Integer arithmetic
#[smt_fn]
pub fn int_arithmetic(x: Integer, y: Integer) -> Integer {
    Integer::add(
        Integer::mul(x, Integer::from(2)),
        Integer::div(y, Integer::from(3)),
    )
}

// Integer comparisons
#[smt_fn]
pub fn is_in_range(x: Integer, low: Integer, high: Integer) -> Boolean {
    Boolean::and(Integer::ge(x, low), Integer::le(x, high))
}

// Real number operations
#[smt_fn]
pub fn real_ops(a: Real, b: Real) -> Real {
    Real::add(Real::mul(a, Real::from(2)), Real::div(b, Real::from(2)))
}

// String operations
#[smt_fn]
pub fn string_ops(s1: String, s2: String) -> Boolean {
    Boolean::and(
        String::starts_with(s1, String::from("hello")),
        String::contains(s2, String::from("world")),
    )
}

// Mixed operations
#[smt_fn]
pub fn quadratic_discriminant(a: Real, b: Real, c: Real) -> Real {
    Real::sub(Real::mul(b, b), Real::mul(Real::from(4), Real::mul(a, c)))
}

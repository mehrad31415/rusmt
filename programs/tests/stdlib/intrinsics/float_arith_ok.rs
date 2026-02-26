use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::float::FloatOps;
use rusmart_smt_stdlib::{Boolean, F32};

#[smt_fn]
pub fn f32_arith(a: F32, b: F32) -> F32 {
    F32::add(F32::mul(a, b), F32::div(a, b))
}

#[smt_fn]
pub fn f32_tests(a: F32, b: F32) -> Boolean {
    Boolean::and(
        F32::lt(a, b),
        Boolean::and(F32::is_nan(a), F32::is_infinite(b)),
    )
}

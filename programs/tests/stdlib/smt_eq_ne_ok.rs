use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, String, smt::SMT};

#[smt_fn]
pub fn smt_eq_ne(x: Integer, y: Integer, s1: String, s2: String) -> Boolean {
    // Generic SMT intrinsics (on any T: SMT)
    x.eq(y)
        .and(x.ne(y))
        .and(s1.eq(s2).or(s1.ne(s2)))
}


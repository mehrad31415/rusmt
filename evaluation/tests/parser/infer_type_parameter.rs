use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Integer, smt::SMT};

// $crate::parser::err::bail_on!($spanned, "no viable type");
// in infer.rs

#[smt_type]
struct A<T: SMT>(T);

#[smt_impl]
fn f1<T: SMT>(a: A<T>) -> T {
    let x = A(Integer::from(1));
    a.0
}

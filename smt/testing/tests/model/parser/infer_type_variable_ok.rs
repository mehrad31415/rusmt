use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::*;

#[smt_impl]
fn foo() -> Integer {
    let a = Integer::from(1);
    let b = a;
    let c = b;
    let d = c;
    d
}

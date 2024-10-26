use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::SMT;

#[smt_impl]
fn foo<T: SMT>(x: T) -> T {
    let y = x;
    y
}
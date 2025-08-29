use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::smt::SMT;

#[smt_impl]
fn foo<T: SMT>(x: Boolean) -> Boolean {
    let y = x;
    y
}

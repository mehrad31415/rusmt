use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::smt::SMT;

#[smt_impl]
fn foo<T: SMT>(t: T) -> T {
    t
}

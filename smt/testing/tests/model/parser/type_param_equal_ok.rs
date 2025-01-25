use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::SMT;

#[smt_impl]
fn foo<T: SMT>(x: T, y: T) -> Boolean {
    Boolean::from(x.eq(&y)).ne(&x.ne(&y).into()).into()
}

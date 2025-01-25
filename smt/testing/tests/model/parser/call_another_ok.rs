use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::{Boolean, SMT};

#[smt_impl]
fn foo<T: SMT>(x: T, y: T) -> Boolean {
    Boolean::from(x.eq(&y)).ne(&x.ne(&y).into()).into()
}

#[smt_impl]
fn bar() -> Boolean {
    foo::<Boolean>(true.into(), false.into())
}

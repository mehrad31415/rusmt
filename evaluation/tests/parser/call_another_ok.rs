use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::{Boolean, smt::SMT};

#[smt_impl]
fn foo<T: SMT>(x: T, y: T) -> Boolean {
    x.eq(y).ne(T::ne(x, y))
}

#[smt_impl]
fn bar() -> Boolean {
    foo::<Boolean>(true.into(), false.into())
}

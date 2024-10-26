use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::Boolean;

#[smt_impl]
fn foo(x: Boolean, y: Boolean) -> Boolean {
    Boolean::from(
        x.not()
            .and(false.into())
            .or(true.into())
            .xor(y)
            .eq(&Boolean::from(x.ne(&y))),
    )
}

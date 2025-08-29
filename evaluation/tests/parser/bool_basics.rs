use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::Boolean;

// this does not pass because ne and eq should be used from the SMT trait and not the partial eq
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

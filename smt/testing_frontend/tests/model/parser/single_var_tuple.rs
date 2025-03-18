use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::Boolean;

// bail_on!(ident, "expect a non-pack type"); in expr.rs
#[smt_impl]
fn f1() -> Boolean {
    let x = Boolean::from(false);
    let y = Boolean::from(true);
    let z: (Boolean, Boolean) = (x, y);
    // but this is allowed let z = (x, y); why? throw an error here
    x
}

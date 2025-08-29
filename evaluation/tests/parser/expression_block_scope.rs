use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::smt::SMT;

#[smt_impl] // if it was not marked, it would not have been analyzed and no error.
fn foo() -> Boolean {
    let x = {
        let x = Boolean::default();
        x
    };
    x
}

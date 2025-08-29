use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, smt::SMT};

#[smt_impl]
fn foo() -> Boolean {
    if true {
        Boolean::default()
    } else {
        Boolean::default()
    }
}

use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::SMT;
use rusmart_smt_stdlib::{Boolean, Integer};

#[smt_impl]
fn foo() -> Boolean {
    let x = if Integer::from(1).add(Integer::from(1)).eq(&Integer::from(2)) {
        Boolean::from(false)
    } else {
        let x = Boolean::from(true);
        x
    };
    x
}

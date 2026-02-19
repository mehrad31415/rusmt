use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Error};

#[smt_fn]
pub fn error_all() -> Boolean {
    let e1 = Error::fresh();
    let e2 = Error::fresh();
    let _m = Error::merge(e1, e2);
    Boolean::from(true)
}


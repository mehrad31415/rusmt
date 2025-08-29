use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

// not a literal type in path.rs for qualified path for from
#[smt_impl]
fn foo() -> Boolean {
    let x = IntegerWrapper(Integer::from(1));
    match x {
        IntegerWrapper(_) => Boolean::from(true),
        _ => Boolean::from(false),
    }
}

#[smt_type]
struct IntegerWrapper(Integer);

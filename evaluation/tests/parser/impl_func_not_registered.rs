use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::smt::SMT;
use rusmart_smt_stdlib::{Boolean, Integer};

// not a literal type in path.rs for qualified path for from
// rrr no such function
#[smt_impl]
fn foo() -> Boolean {
    let x = IntegerWrapper::inner(1);
    match x {
        IntegerWrapper { inner: _ } => Boolean::from(true),
        _ => Boolean::from(false),
    }
}

#[smt_type]
struct IntegerWrapper {
    inner: Integer,
}

impl IntegerWrapper {
    fn inner(value: i32) -> Self {
        IntegerWrapper {
            inner: Integer::from(value),
        }
    }
}

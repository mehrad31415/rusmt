use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::SMT;
use rusmart_smt_stdlib::{Boolean, Integer};

// not a literal type in path.rs for qualified path for from
// because of let adt = ADTPath::from_path(ctxt, path)?; in adt.rs
#[smt_impl]
fn foo() -> Boolean {
    let x = IntegerWrapper {
        inner: Integer::from(1),
    };
    let IntegerWrapper { inner: y } = x;
    // match x {
    //     IntegerWrapper { inner: _ } => Boolean::from(true),
    //     _ => Boolean::from(false),
    // }
    Boolean::from(true)
}

#[smt_type]
struct IntegerWrapper {
    inner: Integer,
}

use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, smt::SMT};

// a function that is marked as smt_impl can be used inside a non-smt function (for example bar can be used in foo as foo is not analyzed)
// but a function that is not marked as smt_impl cannot be used inside a smt function. So, the following code will not compile.
// rule of thumb: anything used inside a marked function or type should be marked as well.
fn foo() -> Boolean {
    Boolean::from(true)
}

#[smt_impl]
fn bar() -> Boolean {
    foo()
}

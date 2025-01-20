use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::SMT;

#[smt_impl]
fn foo(x: MyBoolean) -> Boolean {
    x.0 // varname
}

#[smt_type]
struct MyBoolean(Boolean);

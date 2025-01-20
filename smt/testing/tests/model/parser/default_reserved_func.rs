use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::SMT;

// FuncName::Reserved(_) => bail_on!(ident, "reserved function"), in path.rs
#[smt_impl]
fn foo() -> Boolean {
    Boolean::default() // Boolean::from(false) is the alternative which works
}

use rusmart_smt_remark_derive::{smt_impl, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Text, smt::SMT};

#[smt_impl]
fn foo(x: Boolean, y: Boolean) -> Boolean {
    x.not().and(false.into()).or(true.into()).xor(y).eq(x.ne(y))
}

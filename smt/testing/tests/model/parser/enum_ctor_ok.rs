use rusmart_smt_remark_derive::{smt_impl, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Text, SMT};

#[smt_type]
enum ADT {
    Unit,
    Bool(Boolean),
    Items { a: Integer, b: Text },
}

#[smt_impl]
fn foo(x: ADT) -> Boolean {
    x.eq(ADT::Unit)
        .xor(x.ne(ADT::Bool(false.into())))
        .xor(x.ne(ADT::Items {
            a: 0.into(),
            b: "abc".into(),
        }))
}

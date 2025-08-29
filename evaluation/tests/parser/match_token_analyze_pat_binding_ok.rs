use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

#[smt_impl]
fn foo() -> Boolean {
    match MyEnum::<Integer>::A {
        MyEnum::A => Boolean::from(true),
        MyEnum::B(_) => Boolean::from(false),
        MyEnum::C { x } => x.eq(Integer::from(0)),
    }
}

#[smt_type]
enum MyEnum<T: SMT> {
    A,
    B(T),
    C { x: T },
}

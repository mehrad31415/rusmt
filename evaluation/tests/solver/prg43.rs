use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Error, Integer, Map, Set, Text, smt::SMT};

#[smt_type]
enum MyStruct {
    Unit,
    Struct(Integer, Integer),
    Record { a: Integer, b: Integer },
}
#[smt_impl]
fn pack() -> Integer {
    // inside a match we must have an enum expression
    match (
        MyStruct::Unit,
        MyStruct::Struct(Integer::from(1), Integer::from(2)),
        MyStruct::Record {
            a: Integer::from(3),
            b: Integer::from(4),
        },
    ) {
        (MyStruct::Unit, _, _) => Integer::from(0),
        (MyStruct::Struct(_, _), _, _) => Integer::from(1),
        (MyStruct::Record { a, b }, _, _) => a.add(b),
    }
}

#[smt_spec(impls = pack)]
fn pack_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    pack_spec().eq(Integer::from(1).sub(Integer::from(1)))
}

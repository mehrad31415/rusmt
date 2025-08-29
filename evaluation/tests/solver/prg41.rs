use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Error, Integer, Map, Set, Text, smt::SMT};

#[smt_impl]
fn pack() -> Integer {
    let y = (Integer::from(1), Integer::from(1));
    let x = Integer::from(0);
    Integer::from(0)
}

#[smt_spec(impls = pack)]
fn pack_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    pack_spec().eq(Integer::from(0))
}

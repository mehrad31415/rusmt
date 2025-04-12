use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Error, Integer, Map, Set, Text, SMT};

#[smt_impl]
fn pack() -> Integer {
    let (y1, y2) = (Integer::from(1), Integer::from(1));
    let x = Integer::from(0);
    y2.mul(x)
}

#[smt_spec(impls = pack)]
fn pack_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    pack_spec().eq(Integer::from(0))
}
// for no x => x^2 < 0
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, smt::SMT,
};

#[smt_impl]
fn seq_min() -> Boolean {
    Boolean::from(false)
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec() -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom() -> Boolean {
    exists!(|x: Integer| x.mul(x).lt(Integer::from(0))).eq(seq_min_spec())
}

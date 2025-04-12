// x - x = 0
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, SMT,
};

#[smt_impl]
fn seq_min() -> Integer {
    Integer::from(0)
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom() -> Boolean {
    forall!(|x:Integer| x.sub(x).eq(seq_min_spec()))
}
// testing infer
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, smt::SMT,
};

#[smt_impl]
fn seq_min(set: Set<Integer>) -> Integer {
    let a = Integer::from(0);
    let b = a;
    let c = b;
    c
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec(set: Set<Integer>) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom(set: Set<Integer>) -> Boolean {
    seq_min_spec(set).eq(Integer::from(0))
}

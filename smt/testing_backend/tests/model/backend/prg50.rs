use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, SMT,
};

#[smt_impl]
fn seq_min(set: Set<Integer>) -> Integer {
    choose! (x in set => forall! (y in set => x.lt(y).or(x.eq(y))))
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec(set: Set<Integer>) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom(set: Set<Integer>) -> Boolean {
    exists!(x in set => seq_min_spec(set).gt(x)).not()
}
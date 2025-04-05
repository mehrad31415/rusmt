// testing Boolean
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{choose, exists, forall, Boolean, Integer, Seq, SMT};

#[smt_impl]
fn seq_min(seq: Seq<Integer>) -> Integer {
    seq.at_unchecked(Integer::from(0))
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec(seq: Seq<Integer>) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom(seq: Seq<Integer>) -> Boolean {
    forall!(x in seq => seq_min_spec(seq).le(x))
}

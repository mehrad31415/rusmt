use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{choose, exists, forall, Boolean, Integer, Seq, SMT};

#[smt_impl]
fn seq_new_length() -> Integer {
    let x: Seq<Integer> = Seq::new();
    Seq::length(x)
}

#[smt_spec(impls = seq_new_length)]
fn seq_new_length_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_new_length_axiom() -> Boolean {
    let x: Seq<Integer> = Seq::new();
    seq_new_length_spec().eq(Integer::from(0))
}
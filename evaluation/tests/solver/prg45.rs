// this program makes no sense and is just for testing purposes of forall and exists
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{choose, exists, forall, Boolean, Integer, Seq, smt::SMT};

#[smt_impl]
fn seq_min() -> Integer {
    Integer::from(1)
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom() -> Boolean {
    forall!(|x: Integer| x.mul(seq_min_spec()).eq(x))
}

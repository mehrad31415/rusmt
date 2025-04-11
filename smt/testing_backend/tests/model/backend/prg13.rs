use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    exists, forall, Boolean, Cloak, Error, Integer, Map, Seq, Set, Text, SMT,
};

#[smt_impl]
fn pack() -> Seq<Integer> {
    let a: Seq<Integer> = Seq::new();
    let b: Seq<Integer> = a.append(Integer::from(0));
    let c: Seq<Integer> = b.append(Integer::from(0));
    c
}

#[smt_spec(impls = pack)]
fn pack_spec() -> Seq<Integer> {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    forall!(|x: Integer| (pack_spec().includes(x).implies(x.eq(Integer::from(0)))))
}

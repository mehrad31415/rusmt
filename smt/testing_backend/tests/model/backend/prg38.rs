use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Set, Text, SMT};

#[smt_impl]
fn len_elem() -> Boolean {
    // cannot write let a = Seq::new(); as we will get incomplete type error
    let a: Set<Integer> = Set::new();
    // must have type annotation
    let b: Set<Integer> = a.insert(Integer::from(0));
    b.contains(Integer::from(0))
}

#[smt_spec(impls = len_elem)]
fn len_elem_spec() -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    len_elem_spec()
}
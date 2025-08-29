use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Set, Text, smt::SMT};

#[smt_impl]
fn len_elem() -> Set<Integer> {
    // cannot write let a = Seq::new(); as we will get incomplete type error
    let a: Set<Integer> = Set::new();
    // must have type annotation
    let b: Set<Integer> = a.insert(Integer::from(0));
    b
}

#[smt_spec(impls = len_elem)]
fn len_elem_spec() -> Set<Integer> {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    len_elem_spec()
        .contains(Integer::from(0))
        .and(len_elem_spec().length().eq(Integer::from(1)))
}

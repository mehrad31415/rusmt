use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Seq, Text, smt::SMT};

#[smt_impl]
fn len_elem() -> (Integer, Integer, Boolean) {
    // cannot write let a = Seq::new(); as we will get incomplete type error
    let a: Seq<Integer> = Seq::new();
    // must have type annotation
    let b: Seq<Integer> = a.append(Integer::from(0));
    let c = b.at_unchecked(0.into());
    let d = b.includes(c);
    (b.length(), c, d)
}

#[smt_spec(impls = len_elem)]
fn len_elem_spec() -> (Integer, Integer, Boolean) {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom() -> Boolean {
    len_elem_spec().eq((Integer::from(1), Integer::from(0), Boolean::from(true)))
}

use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Text, smt::SMT};

#[smt_impl]
fn grade_to_integer() -> Boolean {
    // cannot write let a = Cloak::shield(Integer::from(0)); as we will get incomplete type error
    let a: Cloak<Integer> = Cloak::shield(Integer::from(0));
    let b = a.reveal();
    let c: Cloak<Integer> = Cloak::shield(b);
    let d = c.reveal();
    b.eq(d)
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec() -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom() -> Boolean {
    grade_to_integer_spec().eq(Boolean::from(true))
}

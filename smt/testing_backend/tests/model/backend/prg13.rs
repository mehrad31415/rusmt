use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Seq, Text, SMT};

#[smt_impl]
fn grade_to_integer() -> Integer {
    // cannot write let a = Seq::new(); as we will get incomplete type error
    let a: Seq<Integer> = Seq::new();
    // must have type annotation
    let b: Seq<Integer> = a.append(Integer::from(0));
    b.length()
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom() -> Boolean {
    grade_to_integer_spec().eq(Integer::from(1))
}

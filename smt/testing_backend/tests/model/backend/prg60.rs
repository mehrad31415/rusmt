use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Seq, Text, SMT};

#[smt_impl]
fn x_impl() -> Integer {
    let o = Integer::from(0);
    o
}

#[smt_spec(impls = x_impl)]
fn x_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    let a: Seq<Integer> = Seq::new();
    let b: Seq<Integer> = a.append(Integer::from(1));
    b.includes(x_spec()).not()
}

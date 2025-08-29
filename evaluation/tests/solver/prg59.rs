use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Set, Text, smt::SMT};

#[smt_impl]
fn x_impl() -> Integer {
    Integer::from(0)
}

#[smt_spec(impls = x_impl)]
fn x_spec() -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    let a: Set<Integer> = Set::new();
    let b: Set<Integer> = a.insert(Integer::from(0));
    exists!( x in b => x.eq(x_spec()))
}

// testing Integers
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_impl(specs = some_computation_spec)]
fn some_computation(hs: Integer) -> Integer {
    hs.mul(hs).div(hs)
}

#[smt_spec(impls = some_computation)]
fn some_computation_spec(_hs: Integer) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(hs: Integer) -> Boolean {
    some_computation_spec(hs).eq(hs)
}
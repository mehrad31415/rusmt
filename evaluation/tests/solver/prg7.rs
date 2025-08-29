// testing Integers
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

#[smt_impl(specs = some_computation_spec)]
fn some_computation(lhs: Integer, rhs: Integer) -> Boolean {
    lhs.add(rhs).gt(rhs)
}

#[smt_spec(impls = some_computation)]
fn some_computation_spec(_lhs: Integer, _rhs: Integer) -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(lhs: Integer, rhs: Integer) -> Boolean {
    if *lhs.gt(Integer::from(0)) {
        some_computation_spec(lhs, rhs).eq(Boolean::from(true))
    } else {
        some_computation_spec(lhs, rhs).eq(Boolean::from(false))
    }
}

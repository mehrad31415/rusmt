// testing Rational
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, Rational, smt::SMT};

#[smt_impl(specs = some_computation_spec)]
fn some_computation(lhs: Rational, rhs: Rational) -> Boolean {
    lhs.add(rhs).gt(rhs)
}

#[smt_spec(impls = some_computation)]
fn some_computation_spec(_lhs: Rational, _rhs: Rational) -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(lhs: Rational, rhs: Rational) -> Boolean {
    if *lhs.gt(Rational::from(0)) {
        some_computation_spec(lhs, rhs).eq(Boolean::from(true))
    } else {
        some_computation_spec(lhs, rhs).eq(Boolean::from(false))
    }
}

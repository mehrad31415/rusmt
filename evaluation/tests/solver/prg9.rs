// testing Rational
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, Rational, smt::SMT};

#[smt_impl(specs = some_computation_spec)]
fn some_computation(hs: Rational) -> Rational {
    let x = Rational::from(-1);
    if *hs.ge(Rational::from(0)) {
        hs
    } else {
        hs.mul(x)
    }
}

#[smt_spec(impls = some_computation)]
fn some_computation_spec(hs: Rational) -> Rational {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(hs: Rational) -> Boolean {
    some_computation_spec(hs).ge(Rational::from(0))
}

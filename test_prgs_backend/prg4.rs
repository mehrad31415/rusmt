// testing Rational commutative property
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Rational, SMT};

#[smt_impl] // method cannot be defined because Rational is a system type
fn _add(lhs: Rational, rhs: Rational) -> Rational {
    lhs.add(rhs)
}

#[smt_spec(impls = _add)]
fn _add_spec(_lhs: Rational, _rhs: Rational) -> Rational {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(lhs: Rational, rhs: Rational) -> Boolean {
    _add_spec(lhs, rhs).eq(_add(rhs, lhs))
}

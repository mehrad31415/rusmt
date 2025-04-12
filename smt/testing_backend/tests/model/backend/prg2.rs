// testing Boolean
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, SMT};

#[smt_impl(specs = _and_spec)]
fn _and(lhs: Boolean, rhs: Boolean) -> Boolean {
    lhs.and(rhs)
}

#[smt_spec(impls = _and)]
fn _and_spec(_lhs: Boolean, _rhs: Boolean) -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn _and_axiom(lhs: Boolean, rhs: Boolean) -> Boolean {
    _and_spec(lhs, rhs).eq(if *lhs { Boolean::from(false) } else { rhs })
}
// testing Integers mutually dependent functions
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_impl]
pub fn _add(lhs: Integer, rhs: Integer) -> Integer {
    lhs.add(rhs)
}

#[smt_impl]
pub fn _another_add(one: Integer, two: Integer, three: Integer) -> Integer {
    _add(_add(one, two), three)
}

#[smt_impl]
pub fn _addtwo(one: Integer, two: Integer) -> Integer {
    _another_add(one, two, Integer::from(0))
}

#[smt_spec(impls = _addtwo)]
pub fn _spec_add(_lhs: Integer, _rhs: Integer) -> Integer {
    unimplemented!()
}

#[smt_axiom]
pub fn _axiom1(lhs: Integer, rhs: Integer) -> Boolean {
    _spec_add(lhs, rhs).eq(lhs.add(rhs))
}

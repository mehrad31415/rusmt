// testing Integers mutually dependent functions
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

// these are the impl and specs
#[smt_impl(specs = _spec_add)]
pub fn _add(lhs: Integer, rhs: Integer) -> Integer {
    lhs.add(rhs)
}

#[smt_spec(impls = _add)]
pub fn _spec_add(_lhs: Integer, _rhs: Integer) -> Integer {
    unimplemented!()
}

// dummy implementations of the addition functions
#[smt_impl]
pub fn _another_add(one: Integer, two: Integer, three: Integer) -> Integer {
    one.add(two).add(three)
}

#[smt_impl]
pub fn _addtwo(one: Integer, two: Integer) -> Integer {
    _another_add(one, two, Integer::from(0))
}

#[smt_axiom]
pub fn _axiom1(lhs: Integer, rhs: Integer) -> Boolean {
    _spec_add(lhs, rhs).eq(_addtwo(lhs, rhs))
}

// testing Integers
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_impl(specs = _spec_add)]
pub fn _add(one: Integer, two: Integer, three: Integer) -> Integer {
    one.add(two).add(three)
}

#[smt_spec]
pub fn _spec_add(one: Integer, two: Integer, three: Integer) -> Integer {
    unimplemented!()
}

#[smt_impl]
pub fn addtwo(one: Integer, two: Integer) -> Integer {
    another_add(one, two, Integer::from(0))
}

#[smt_impl]
pub fn another_add(one: Integer, two: Integer, three: Integer) -> Integer {
    one.add(two).add(three)
}

#[smt_axiom]
pub fn _axiom1(lhs: Integer, rhs: Integer) -> Boolean {
    _spec_add(Integer::from(1), lhs, rhs).eq(Integer::from(1).add(addtwo(lhs, rhs)))
}

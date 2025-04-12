// testing struct access expression and method and generic axiom
use rusmart_smt_remark_derive::{smt_type, smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_type]
struct MyInteger {
    value: Integer,
}

#[smt_impl (method = myadd)]
fn _add(lhs: MyInteger, rhs: MyInteger) -> Integer {
    lhs.value.add(rhs.value)
}

#[smt_spec(impls = _add)]
fn _spec_add(_lhs: MyInteger, _rhs: MyInteger) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn _axiom1<T : SMT>(lhs: MyInteger, rhs: MyInteger) -> Boolean {
    _spec_add(lhs, rhs).eq(lhs.myadd(rhs))
}
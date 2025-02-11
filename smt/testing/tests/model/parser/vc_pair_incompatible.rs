use rusmart_smt_remark_derive::{smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, Rational, SMT};

#[smt_impl(specs = spec_foo)]
fn impl_foo(a: Integer, b: Integer) -> Boolean {
    a.eq(b)
}

#[smt_spec(impls = impl_foo)]
fn spec_foo(a: Rational, b: Rational) -> Boolean {
    a.eq(b)
}

/*
#[smt_impl(spec=spec_add_one)]
fn add_one(a:Integer) -> Integer {
    a+1
}

#[smt_spec]
fn spec_add_one(a: Integer) {
    unimplemented!()
}

#[smt_axiom]
fn spec_add_one_axiom(a: Integer) -> Boolean {
  spec_add_one(a) == a + 1
}
*/

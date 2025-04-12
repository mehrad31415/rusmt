use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, SMT,
};

#[smt_type]
struct MyInteger(Integer);

#[smt_impl(method = my_add)]
fn my_add(x: MyInteger, y: MyInteger) -> Integer {
    x.0.add(y.0)
}

// we can add as many methods we want for a type (like inside an impl block)
#[smt_impl(method = my_add2)]
fn my_add2(x: MyInteger, y: MyInteger) -> Integer {
    x.0.add(y.0)
}

// adding the following will cause an error because the method is already defined
// #[smt_impl(method = my_add)]
// fn my_add3(x: MyInteger, y: MyInteger) -> Integer {
//     x.0.add(y.0)
// }

#[smt_spec(impls = my_add)]
fn my_add_spec(x: MyInteger, y: MyInteger) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn my_add_axiom(x: MyInteger, y: MyInteger) -> Boolean {
    my_add_spec(x, y).eq(x.my_add(y))
}

use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Integer, Set, Text, smt::SMT};

#[smt_type]
struct MyType<T: SMT>(Integer, Boolean, T);

#[smt_impl]
fn my_type_impl<T: SMT>(x: MyType<T>) -> Boolean {
    if *x.0.eq(Integer::from(0)) {
        x.1
    } else {
        x.1.not()
    }
}

#[smt_spec(impls = my_type_impl)]
fn my_type_spec<T: SMT>(x: MyType<T>) -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn my_type_axiom<T: SMT>(x: MyType<T>) -> Boolean {
    my_type_spec(x).eq(Boolean::from(true))
}

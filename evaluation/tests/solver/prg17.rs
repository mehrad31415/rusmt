use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

#[smt_type]
struct MyInteger(Integer, Boolean);

#[smt_impl]
fn some_impl(hs: MyInteger) -> Integer {
    if *hs.1 {
        hs.0.add(Integer::from(1))
    } else {
        hs.0
    }
}

#[smt_spec(impls = some_impl)]
fn some_spec(hs: MyInteger) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn axiom<T: SMT>(hs: MyInteger) -> Boolean {
    (some_spec(hs).eq(hs.0).and(hs.1.not()))
        .or(some_spec(hs).eq(hs.0.add(Integer::from(1))).and(hs.1))
}

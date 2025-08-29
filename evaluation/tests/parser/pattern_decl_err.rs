use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_stdlib::{Integer, smt::SMT};

// why not use wildcard? this is because in expr.rs in parse_decl we have unrecognized pattern for declaration - expect an identifier or a tuple for Pat:wild
#[smt_impl]
fn f1<T: SMT>() -> Integer {
    let _ = Integer::from(1); // gives an error
    let (x, _) = (Integer::from(1), Integer::from(2)); // gives an error
    let (x, _) = f2(); // gives an error
    Integer::from(2)
}

#[smt_impl]
fn f2() -> (Integer, Integer) {
    (Integer::from(1), Integer::from(2))
}

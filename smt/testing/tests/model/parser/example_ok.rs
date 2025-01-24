use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, Text, SMT};

#[smt_impl]
fn add_two_nums(x: Integer, y: Integer) -> Integer {
    let a = x.add(y);
    let b = a.add(Integer::from(1));
    let c = b;
    let d = c;
    c
}



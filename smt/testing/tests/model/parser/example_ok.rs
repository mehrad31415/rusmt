use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec};
use rusmart_smt_stdlib::{Boolean, Integer, Text, SMT};

#[smt_spec]
fn add_two_nums(x: Integer, y: Integer) -> Integer {
    unimplemented!()
}
/*
let a = MyStruct {
    x: 1,
    y: 2,
    z: 3,
};

let b = a.x;


*/
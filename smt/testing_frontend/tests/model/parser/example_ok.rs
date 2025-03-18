use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Seq, Text, SMT};

#[smt_spec]
fn add_two_nums() -> Integer {
    let x : Integer = 1.into();
    Integer::from(2)
}


#[smt_type]
enum MyStruct {
    Integer1,
    Integer2,
    Integer3,
}

impl MyStruct {
    #[smt_impl]
    fn new() -> Self {
        MyStruct::Integer1
    }
}
/*
let a = MyStruct {
    x: 1,
    y: 2,
    z: 3,
};

let b = a.x;


*/

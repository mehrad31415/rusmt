use rusmart_smt_remark_derive::{smt_impl, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Text, smt::SMT};

#[smt_impl(method = my_fn)]
fn another_add(f: MyStruct, y: Integer) -> Integer {
    f.x.add(y)
}

#[smt_type]
struct MyStruct {
    x: Integer,
    y: Integer,
}

/*
impl MyStruct {
    fn my_fn(&self, y: Integer) -> Integer {
        self.x.add(y)
    }
 */

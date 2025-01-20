use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_spec;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::*;

// A type must have a #[smt_type] annotation when being used in a function with a #[smt_impl] annotation or #[smt_spec] annotation.
// if the function is not annotated with #[smt_impl] or #[smt_spec], the type can have but does not need to have a #[smt_type] annotation.
// rule of thumb: anything used inside a marked function or type should be marked as well.

// let generics = match ctxt.get_type_def(&ty_name) {
//     None => bail_on!(ident, "no such type"),
// in path.rs is triggered

#[smt_impl]
fn f1<T: SMT>() -> Integer {
    let x = Point {
        a: Integer::from(1),
        b: Integer::from(2),
    };
    x.a
}

struct Point {
    a: Integer,
    b: Integer,
}

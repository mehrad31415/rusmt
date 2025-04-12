use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, SMT};

#[smt_type]
struct MyIntBool {
    a: Integer,
    b: Boolean,
}

#[smt_impl]
fn grade_to_integer(x: Integer) -> Integer {
    // we cannot destructure the tuple here (error: unrecognized pattern for declaration - expect an identifier or a tuple)
    // let MyIntBool{
    //     a,
    //     b,
    // } = MyIntBool{
    //     a: x,
    //     b: Boolean::from(true),
    // };
    let var = MyIntBool {
        a: x,
        b: Boolean::from(true),
    };
    var.a
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(x: Integer) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Integer) -> Boolean {
    grade_to_integer_spec(x).le(Integer::from(3))
}
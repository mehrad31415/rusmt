use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, SMT};

#[smt_type]
enum IntBool {
    Int(Integer),
    Bool(Boolean),
}

#[smt_impl]
fn grade_to_integer(x1: Integer, x2: Boolean) -> IntBool {
    let var1 = IntBool::Int(x1);
    let var2 = IntBool::Bool(x2);
    if *x2 {
        var1
    } else {
        var2
    }
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(x1: Integer, x2: Boolean) -> IntBool {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x1: Integer, x2: Boolean) -> Boolean {
    (grade_to_integer_spec(x1, x2).eq(IntBool::Int(x1)).and(x2)).or(grade_to_integer_spec(x1, x2)
        .eq(IntBool::Bool(x2))
        .and(x2.not()))
}
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, smt::SMT};

#[smt_type]
enum Grades {
    A,
    B(Integer),
    C { x: Integer, y: Integer },
    D(Integer),
    E { x: Integer, y: Integer },
}

#[smt_impl]
fn grade_to_integer(x1: Grades) -> Grades {
    let x = Grades::B(Integer::from(0));
    x
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(_x: Grades) -> Grades {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Grades) -> Boolean {
    grade_to_integer_spec(x).eq(Grades::B(Integer::from(0)))
}

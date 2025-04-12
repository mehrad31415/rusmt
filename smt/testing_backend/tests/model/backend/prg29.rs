use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, SMT};

#[smt_type]
enum Grades {
    A,
    B(Integer),
    C { x: Integer, y: Integer },
}

#[smt_impl]
fn grade_to_integer(x1: Grades) -> Integer {
    match x1 {
        Grades::A => Integer::from(1),
        Grades::B(i) => i,
        Grades::C { x, y } => x.add(y),
    }
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(_x: Grades) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Grades) -> Boolean {
    match x {
        Grades::A => grade_to_integer_spec(x).eq(Integer::from(0)),
        Grades::B(i) => grade_to_integer_spec(x).eq(i),
        Grades::C { x: x1, y: y1 } => grade_to_integer_spec(x).eq(x1.add(y1)),
    }
}
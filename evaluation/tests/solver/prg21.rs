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
fn grade_to_integer(x1: Grades, x2: Integer) -> Grades {
    if *x2.rem(Integer::from(2)).eq(Integer::from(0)) {
        x1 // cannot write return here
    } else {
        Grades::B(x2) // cannot write return here
    }
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(x1: Grades, x2: Integer) -> Grades {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x1: Grades, x2: Integer) -> Boolean {
    (grade_to_integer_spec(x1, x2)
        .eq(x1)
        .and(x2.rem(Integer::from(2)).eq(Integer::from(0))))
    .or(grade_to_integer_spec(x1, x2)
        .eq(Grades::B(x2))
        .and(x2.rem(Integer::from(2)).eq(Integer::from(1))))
}

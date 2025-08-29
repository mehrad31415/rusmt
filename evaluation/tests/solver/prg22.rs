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
fn grade_to_integer() -> Grades {
    let x = Integer::from(0);
    let y = Integer::from(1);
    Grades::C { x, y }
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec() -> Grades {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x1: Integer, x2: Integer) -> Boolean {
    x1.eq(Integer::from(0))
        .and(x2.eq(Integer::from(1)))
        .implies(grade_to_integer_spec().eq(Grades::C { x: x1, y: x2 }))
}

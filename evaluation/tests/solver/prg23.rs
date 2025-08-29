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
    Grades::D(x)
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec() -> Grades {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Integer) -> Boolean {
    x.eq(Integer::from(0))
        .implies(grade_to_integer_spec().eq(Grades::D(x)))
}

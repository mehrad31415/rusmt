// testing Boolean
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, SMT};

#[smt_type]
enum Grades {
    A,
    B(Integer),
    C { x: Integer, y: Integer },
    D(Integer),
}

#[smt_impl]
fn grade_to_integer(x1: Grades) -> Grades {
    let x = Grades::C {
        x: Integer::from(1),
        y: Integer::from(2),
    };
    x
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(_x: Grades) -> Grades {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Grades) -> Boolean {
    grade_to_integer_spec(x).eq(Grades::C { x: Integer::from(1), y: Integer::from(2) })
}

// #[smt_impl]
// fn sort(s : Seq<Integer>) -> Seq<Integer> {
//     let tail
// }

// #[smt_spec(impls = sort)]
// fn sort_spec(s : Seq<Integer>) -> Seq<Integer> {
//     let x = iterforall
// }

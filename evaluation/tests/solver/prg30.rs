use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Text, smt::SMT};

#[smt_impl]
fn grade_to_integer(x1: Text) -> Integer {
    let a: Text = Text::from("A");
    let b: Text = Text::from("B");
    let c: Text = Text::from("C");

    // do not write as this because the else if will be ignored by the parser
    // if *x1.eq(A) {
    //     Integer::from(0)
    // } else if *x1.eq(B) {
    //     Integer::from(1)
    // } else if *x1.eq(C) {
    //     Integer::from(2)
    // } else {
    //     Integer::from(-1)
    // }
    if *x1.eq(a) {
        Integer::from(0)
    } else {
        if *x1.eq(b) {
            Integer::from(1)
        } else {
            if *x1.eq(c) {
                Integer::from(2)
            } else {
                Integer::from(-1)
            }
        }
    }
}

#[smt_spec(impls = grade_to_integer)]
fn grade_to_integer_spec(_x: Text) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn grade_to_integer_axiom(x: Text) -> Boolean {
    x.eq(Text::from("A"))
        .implies(grade_to_integer_spec(x).eq(Integer::from(0)))
        .and(
            x.eq(Text::from("B"))
                .implies(grade_to_integer_spec(x).eq(Integer::from(1))),
        )
        .and(
            x.eq(Text::from("C"))
                .implies(grade_to_integer_spec(x).eq(Integer::from(2))),
        )
        .and(
            (x.gt(Text::from("C")).or(x.lt(Text::from("A"))))
                .implies(grade_to_integer_spec(x).eq(Integer::from(-1))),
        )
}

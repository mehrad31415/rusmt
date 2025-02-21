use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_type]
struct Point {
    x: Integer,
    y: Integer,
}

#[smt_impl]
fn add(lhs: Point, rhs: Point) -> Point {
    Point {
        x: lhs.x.add(rhs.x),
        y: lhs.y.add(rhs.y),
    }
}

#[smt_spec(impls = add)]
fn add_spec(lhs: Point, rhs: Point) -> Point {
    unimplemented!()
}

#[smt_axiom(relations = {(add, add_spec)})]
fn add_axiom(lhs: Point, rhs: Point) -> Boolean {
    add_spec(lhs, rhs).eq(Point {
        x: lhs.x.add(rhs.x),
        y: lhs.y.add(rhs.y),
    })
}

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

// T cannot be used in the axiom without monomorphisms
#[smt_axiom(relations = {(add, add_spec)})]
fn add_axiom(lhs: Point, rhs: Point) -> Boolean {
    add_spec(lhs, rhs).eq(Point {
        x: lhs.x.add(rhs.x),
        y: lhs.y.add(rhs.y),
    })
}

/*
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};

#[smt_type]
struct Point {
    x: Integer,
    y: Integer,
}

#[smt_impl(method=)]
fn add<T:SMT>(lhs: Point, rhs: Point) -> Point {
    Point {
        x: lhs.x.add(rhs.x),
        y: lhs.y.add(rhs.y),
    }
}

#[smt_spec(impls = add)]
fn add_spec<T:SMT>(lhs: Point, rhs: Point) -> Point {
    unimplemented!()
}

// T cannot be used in the axiom without monomorphisms
#[smt_axiom(relations = {(add, add_spec)})]
fn add_axiom(lhs: Point, rhs: Point) -> Boolean {
lhs.add(r)
    add_spec::<Boolean>(lhs, rhs).eq(Point {
        x: lhs.x.add(rhs.x),
        y: lhs.y.add(rhs.y),
    })
}

DOES NOT WORK!
*/




/*
DOES NOT WORK!
#[smt_type]
enum Dummy {
    A,
    B(Integer),
}

#[smt_impl]
fn a_dummy (x:Dummy) -> Dummy {
    Dummy::A
}

#[smt_spec]
fn a_dummy_spec (x:Dummy) -> Dummy {
    unimplemented!()
}

#[smt_axiom(relations = {(a_dummy, a_dummy_spec)})]
fn a_dummy_axiom (x:Dummy) -> Boolean {
    a_dummy_spec(x).eq(Dummy::A)

}
*/
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, SMT};




#[smt_type]
enum Students {
    // tuple struct
    Mehrad(Integer),
    // record struct
    Mehrad2 { age: Integer },
    // unit struct
    Mehrad3,
}


#[smt_type]
struct Age {
    age: Integer,
}

#[smt_impl(specs = get_age_if_mehrad_spec)]
fn get_age_if_mehrad(x: Students) -> Age {
    match x {
        Students::Mehrad(age) => Age { age },
        Students::Mehrad2 { age } => Age { age },
        Students::Mehrad3 => Age { age: Integer::from(0) },
    }
}

#[smt_spec(impls = get_age_if_mehrad)]
fn get_age_if_mehrad_spec(x: Students) -> Age {
    unimplemented!()
}

// #[smt_axiom(relations = {(a_dummy, a_dummy_spec)})]
#[smt_axiom]
fn get_age_if_mehrad_axiom(x: Students) -> Boolean {
    match x {
        Students::Mehrad(age) => get_age_if_mehrad_spec(x).eq(Age { age }),
        Students::Mehrad2 { age } => get_age_if_mehrad_spec(x).eq(Age { age }),
        Students::Mehrad3 => get_age_if_mehrad_spec(x).eq(Age { age: Integer::from(0) }),
    }
}























// #[smt_type]
// struct Wrap(Seq<Integer>);



// // tuple struct
// #[smt_type]
// struct Start(Integer);

// // record struct
// #[smt_type]
// struct End {
//     x: Integer,
// }


// #[smt_impl(specs = start_spec)]
// fn start(x: Start, y : (Integer, Boolean)) -> End {
//     End { x: x.0 }
// }

// #[smt_spec(impls = start)]
// fn start_spec(x: Start, y : (Integer, Boolean)) -> End {
//     unimplemented!()
// }

// // #[smt_axiom(relations = {(start, start_spec)})]
// #[smt_axiom]
// fn start_axiom(x: Start) -> Boolean {
//     start_spec(x, (0.into(), false.into())).eq(End { x: x.0 })
// }





































// // #[smt_type]
// // struct Point {
// //     x: Integer,
// //     y: Integer,
// // }

// // #[smt_impl]
// // fn add(lhs: Point, rhs: Point) -> Point {
// //     Point {
// //         x: lhs.x.add(rhs.x),
// //         y: lhs.y.add(rhs.y),
// //     }
// // }

// // #[smt_spec(impls = add)]
// // fn add_spec(lhs: Point, rhs: Point) -> Point {
// //     unimplemented!()
// // }

// // // T cannot be used in the axiom without monomorphisms
// // #[smt_axiom(relations = {(add, add_spec)})]
// // fn add_axiom(lhs: Point, rhs: Point) -> Boolean {
// //     add_spec(lhs, rhs).eq(Point {
// //         x: lhs.x.add(rhs.x),
// //         y: lhs.y.add(rhs.y),
// //     })
// // }

// /*
// use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
// use rusmart_smt_stdlib::{Boolean, Integer, SMT};

// #[smt_type]
// struct Point {
//     x: Integer,
//     y: Integer,
// }

// #[smt_impl(method=)]
// fn add<T:SMT>(lhs: Point, rhs: Point) -> Point {
//     Point {
//         x: lhs.x.add(rhs.x),
//         y: lhs.y.add(rhs.y),
//     }
// }

// #[smt_spec(impls = add)]
// fn add_spec<T:SMT>(lhs: Point, rhs: Point) -> Point {
//     unimplemented!()
// }

// // T cannot be used in the axiom without monomorphisms
// #[smt_axiom(relations = {(add, add_spec)})]
// fn add_axiom(lhs: Point, rhs: Point) -> Boolean {
// lhs.add(r)
//     add_spec::<Boolean>(lhs, rhs).eq(Point {
//         x: lhs.x.add(rhs.x),
//         y: lhs.y.add(rhs.y),
//     })
// }

// DOES NOT WORK!
// */




// /*
// DOES NOT WORK!
// #[smt_type]
// enum Dummy {
//     A,
//     B(Integer),
// }

// #[smt_impl]
// fn a_dummy (x:Dummy) -> Dummy {
//     Dummy::A
// }

// #[smt_spec]
// fn a_dummy_spec (x:Dummy) -> Dummy {
//     unimplemented!()
// }

// #[smt_axiom(relations = {(a_dummy, a_dummy_spec)})]
// fn a_dummy_axiom (x:Dummy) -> Boolean {
//     a_dummy_spec(x).eq(Dummy::A)

// }
// */
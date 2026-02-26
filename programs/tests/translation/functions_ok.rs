// Test 3: Function declarations and definitions
// Tests: Simple functions, recursive functions, match expressions, if-else

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Cloak, Integer, Real, smt::SMT};

#[smt_type]
pub enum Listx<T: SMT> {
    Nil,
    Cons(T, Cloak<Listx<T>>),
}

// Simple non-recursive function
#[smt_fn]
pub fn add_two(x: Integer) -> Integer {
    Integer::add(x, Integer::from(2))
}

// Function with conditionals
#[smt_fn]
pub fn abs_val(x: Integer) -> Integer {
    let is_neg = Integer::lt(x, Integer::from(0));
    if *is_neg { Integer::neg(x) } else { x }
}

// Recursive function
#[smt_fn]
pub fn list_length<T: SMT>(lst: Listx<T>) -> Integer {
    match lst {
        Listx::Nil => Integer::from(0),
        Listx::Cons(_, tail) => Integer::add(Integer::from(1), list_length::<T>(tail.reveal())),
    }
}

// Function with multiple parameters
#[smt_fn]
pub fn max_of_three(a: Integer, b: Integer, c: Integer) -> Integer {
    let a_gt_b = Integer::gt(a, b);
    if *a_gt_b {
        let a_gt_c = Integer::gt(a, c);
        if *a_gt_c { a } else { c }
    } else {
        let b_gt_c = Integer::gt(b, c);
        if *b_gt_c { b } else { c }
    }
}

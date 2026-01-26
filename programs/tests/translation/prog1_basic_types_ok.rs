// Test 1: Basic type conversions
// Tests: Tuple, Record, Simple Enum, Type parameters

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Integer, Real, String, smt::SMT};

// Simple tuple type
#[smt_type]
pub struct Point {
    x: Integer,
    y: Integer,
}

// Record type
#[smt_type]
pub struct Person {
    name: String,
    age: Integer,
    height: Real,
}

// Simple enum
#[smt_type]
pub enum Color {
    Red,
    Green,
    Blue,
    Custom(Integer, Integer, Integer),
}

// Enum with named fields
#[smt_type]
pub enum Shape {
    Circle { radius: Real },
    Rectangle { width: Real, height: Real },
    Point,
}

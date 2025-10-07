//! Rusmart standard library DSL that contains constructs that cannot be expressed in Rust as they have special semantics in SMT.
//!
//! Module Tree:
//! * dt - SMT-related data types
//! * exp - SMT-related expressions

#![warn(missing_docs)]

mod dt;
mod exp;
pub use dt::*;
pub use exp::*;
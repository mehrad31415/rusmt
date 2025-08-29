//! Rusmart standard library (stdlib) that contains language constructs that
//! cannot be expressed readily in Rust as they have special semantics in SMT.
//!
//! Module Tree:
//! * dt - SMT-related data types
//! * exp - SMT-related expressions

mod dt;
mod exp;

/// Re-export SMT-related data types and expressions
/// This allows users to call `rusmart_stdlib::Boolean` instead of `rusmart_stdlib::dt::Boolean` or call `rusmart_stdlib::forall` instead of `rusmart_stdlib::exp::forall`
pub use dt::*;
pub use exp::*;

//! # Remark library crate for Rusmart.
//!
//! The `rusmart_smt_remark` package contains one library crate.
//! - module tree: attr, generics, err, func, ty
mod attr;
mod err;
mod generics;
// func and ty are public modules and can be accessed from outside the crate.
// They handle annotations for the functions and types and are used for defining the procedural macros in the `rusmart_smt_remark_derive` package.
pub mod func;
pub mod ty;

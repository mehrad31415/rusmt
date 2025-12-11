//! # Rusmart SMT Remark
//!
//! The `rusmart_smt_remark` package contains one library crate.
//! 
//! ## Module Tree
//!
//! - [`func`]: Function annotation
//! - [`ty`]: Type annotation
//! - [`err`]: Error types
//! - `attr`: Attribute parsing (internal)
//! - `generics`: Generic parameter processing (internal)
//!
//! ## Usage
//!
//! This library is primarily intended for use by derive macros in the
//! `rusmart_smt_remark_derive` crate.

#![warn(missing_docs)]

mod attr;
mod generics;
pub mod err;
pub mod func;
pub mod ty;

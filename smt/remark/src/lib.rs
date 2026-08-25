//! # RuSmt SMT Remark
//!
//! The `rusmt_smt_remark` package contains one library crate.
//!
//! ## Module Tree
//!
//! - [`func`]: Function annotation
//! - [`ty`]: Type annotation
//! - [`err`]: Error types (internal)
//! - `generics`: Generic parameter processing (internal)
//!
//! ## Usage
//!
//! This library is primarily intended for use by derive macros in the
//! `rusmt_smt_remark_derive` crate.

#![warn(missing_docs)]

mod err;
pub mod func;
mod generics;
mod marker;
pub mod ty;

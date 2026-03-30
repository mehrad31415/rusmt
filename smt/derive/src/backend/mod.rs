//! Module tree for the SMT backend

/// Module for the common backend code generator
pub mod codegen;
/// Module for the error types
mod error;
/// Module for the response enum
mod response;
/// Module for the Z3 theorem prover backend
pub mod z3;

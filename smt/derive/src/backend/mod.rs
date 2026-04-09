//! Module tree for the SMT backend

/// Module for the common backend code generator
pub mod codegen;
/// Module for the error types
pub(crate) mod error;
/// Module for the response enum
pub(crate) mod response;
/// Module for the Z3 theorem prover backend
pub(crate) mod z3;
// /// Module for the Z3 API backend
// pub(crate) mod z3_api;

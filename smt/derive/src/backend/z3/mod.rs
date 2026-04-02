//! Z3 SMT-LIB backend: modules for generating SMT-LIB2 code consumed by Z3.                                                                                     

// top-level code generation: orchestrates type, function, and query emission
pub mod ctxt;
/// Converts IR expressions to SMT-LIB S-expressions (function bodies)
mod exp;
/// Emits function declarations (`declare-fun`) and definitions (`define-fun`/`define-funs-rec`)
pub(crate) mod fun;
/// Maps each IR intrinsic operation to its Z3 SMT-LIB formula
pub(crate) mod intrinsics;
/// Converts IR type definitions to Z3 `declare-datatypes` blocks
pub(crate) mod sort;

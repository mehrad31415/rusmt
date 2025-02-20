//! The module tree for the intermediate representation (IR) of the SMT-LIB input.

pub mod ctxt;

pub mod index;
pub mod name;

pub mod sort;

mod mono;

mod axiom;
mod exp;
pub mod fun;
mod intrinsics;

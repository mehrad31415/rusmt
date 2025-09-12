//! Generic `CodeGen` trait for backend solvers

use crate::backend::error::BackendResult;
use crate::backend::z3::common::CodeGenZ3;
use crate::ir::ctxt::IRContext;
use std::{
    fmt::{Display, Formatter},
    path::Path,
};
use z3::Model;

#[derive(Debug, Clone)]
/// The response returned by the backend solver.
pub enum Response {
    /// solver does not return anything in the desired time
    Timeout,
    /// SMT: unknown
    Unknown,
    /// SMT: sat
    Sat,
    /// SMT: unsat
    Unsat,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Timeout => "timeout",
            Self::Unknown => "unknown",
            Self::Sat => "sat",
            Self::Unsat => "unsat",
        };
        f.write_str(text)
    }
}

/// A generic trait for backend code generators.
pub trait CodeGen {
    /// Constructs a new `CodeGen` wrapper
    fn new() -> Self
    where
        Self: Sized;

    /// Returns the name of this code generator.
    fn name(&self) -> String;

    /// Returns the file extension (or flavor) of the source code.
    fn flavor(&self) -> &'static str;

    /// Given an IRContext, give the response from the backend solver.
    fn process(&self, ir: &IRContext, workspace: &Path)
    -> BackendResult<(Response, Option<Model>)>;
}

/// Available list of backend solvers
pub fn solvers() -> Vec<Box<dyn CodeGen>> {
    vec![Box::new(CodeGenZ3::new())]
}

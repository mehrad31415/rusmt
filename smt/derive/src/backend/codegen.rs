//! This module defines the `CodeGen` trait and its implementation for Z3.
//! It provides utility functions for generating SMT-LIB code from an IRContext.

use crate::backend::error::BackendResult;
use crate::backend::z3::common::CodeGenZ3;
use crate::backend::z3::engine_chc::BackendZ3CHC;
use crate::ir::ctxt::IRContext;
use z3::Model;

/// The result of the API call to Z3.
pub enum ApiResult<'a> {
    Sat(Model<'a>),
    Unsat,
    Unknown,
}

/// A generic trait for backend code generators (CodeGenZ3 implements this trait).
pub trait CodeGen {
    /// Returns the name of this code generator (e.g., "z3_chc").
    fn name(&self) -> String;

    /// Returns the file extension (or flavor) of the source code (e.g., "smt2").
    fn flavor(&self) -> &'static str {
        "smt2"
    }

    /// Given an IRContext, generate the backend source code.
    /// Returns a `BackendResult<String>` containing either the full source code or a BackendError::NotSupported error.
    fn process(&self, ir: &IRContext) -> BackendResult<String>;

    /// Returns the result of the process using the Z3 API.
    fn call_z3_api(&self, ir: &IRContext) -> BackendResult<ApiResult<'_>>;
}

/// A utility for source code builder
pub struct ContentBuilder {
    /// Internal buffer holding all lines of code so far.
    buffer: String,
    /// Current indentation level (counted as number of tabs).
    indent: usize,
}

impl ContentBuilder {
    /// Creates a new, empty content builder: let mut builder = ContentBuilder::new();
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: 0,
        }
    }

    /// Appends a new line of code to the buffer. Automatically inserts indentation based on `indent`.
    pub fn line<S: AsRef<str>>(&mut self, code: S) {
        for _ in 0..self.indent {
            self.buffer.push('\t');
        }
        self.buffer.push_str(code.as_ref());
        self.buffer.push('\n');
    }

    /// Consumes this builder, returning the final accumulated string of code.
    pub fn build(self) -> String {
        self.buffer
    }
}

/// A helper macro to simplify adding lines to a `ContentBuilder`.
///
/// # Variants
///
/// - `l!(builder)` appends an empty line (just a newline).
/// - `l!(builder, $item:expr)` appends `$item` as a line.
/// - `l!(builder, $fmt:expr, $($args:tt)*)` performs a `format!` call first.
macro_rules! l {
    // Appends an empty line.
    ($builder:expr) => {
        $builder.line("")
    };
    // Appends a single line from a string expression.
    ($builder:expr, $item:expr) => {
        $builder.line($item)
    };
    // Appends a line using a format string and additional arguments.
    ($builder:expr, $fmt:expr, $($args:tt)*) => {
        $builder.line(format!($fmt, $($args)*))
    };
}
pub(crate) use l;

/// Available list of backend solvers
pub fn solvers() -> Vec<Box<dyn CodeGen>> {
    vec![Box::new(CodeGenZ3::new(BackendZ3CHC::new()))]
}

//! Generic `CodeGen` trait for backend solvers

use crate::backend::error::BackendResult;
use crate::backend::z3::common::CodeGenZ3;
use crate::ir::ctxt::IRContext;
use std::fmt::{Display, Formatter};

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

    /// Given an IRContext, generate the backend source code.
    ///
    /// Returns a `BackendResult<String>` containing either the full source code or a BackendError::NotSupported error.
    fn process(&self, ir: &IRContext) -> BackendResult<String>;
}

/// A utility for source code builder
/// This struct collects lines of code in a buffer.
pub struct ContentBuilder {
    /// Internal buffer holding all lines of code so far.
    buffer: String,
    /// Current indentation level (counted as number of tabs).
    indent: usize,
}

impl ContentBuilder {
    /// Creates a new, empty content builder.
    ///
    /// let mut builder = ContentBuilder::new();
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
    vec![Box::new(CodeGenZ3::new())]
}

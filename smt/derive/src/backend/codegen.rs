//! Generic `CodeGen` trait for backend solvers

use crate::backend::error::BackendResult;
use crate::backend::response::Response;
use crate::backend::z3::common::CodeGenZ3;
use crate::ir::ctxt::IRContext;
use std::path::Path;

/// A generic trait for backend code generators.
pub(crate) trait CodeGen {
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

    /// Invokes the backend solver with the given source code file.
    fn invoke_backend(&self, path_src: &Path) -> BackendResult<Response>;

    /// For a given `error_id`, generate per-function SMT-LIB query strings.
    ///
    /// Returns a list of `(function_name, query_code)` pairs.  Each pair targets
    /// a function that **directly** contains `Error::fresh()` calls for `error_id`.
    /// The `query_code` extends `base_code` with fresh input constants, an error
    /// assertion, `(check-sat)`, and `(get-model)`.
    fn process_error_queries(
        &self,
        base_code: &str,
        ir: &IRContext,
        error_id: usize,
    ) -> Vec<(String, String)>;
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
pub(crate) fn solvers() -> Vec<Box<dyn CodeGen>> {
    vec![Box::new(CodeGenZ3::new())]
}

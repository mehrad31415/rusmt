//! Generic `CodeGen` trait for backend solvers

use crate::backend::error::BackendResult;
use crate::ir::ctxt::IRContext;
use std::collections::BTreeSet;

/// A generic trait for backend code generators.
pub(crate) trait CodeGen {
    /// Constructs a new `CodeGen` wrapper
    fn new() -> Self
    where
        Self: Sized;

    /// Returns the name of this code generator.
    fn name(&self) -> &'static str;

    /// Returns the file extension (or flavor) of the source code.
    fn flavor(&self) -> &'static str;

    /// Given an IRContext, generate the backend source code.
    /// Returns a `BackendResult<String>` containing either the full source code or a BackendError.
    fn process(&self, ir: &IRContext, unroll_depth: usize) -> BackendResult<String>;

    /// For a given path-marker target (set of path IDs), generate an SMT-LIB query that asks Z3
    /// to find inputs to `top_level_fn` such that ALL IDs in the target are in the result's
    /// path-marker set. A single-element set queries one path site; a multi-element set queries
    /// a merge point where all those sites were reached simultaneously.
    ///
    /// # Panics
    ///
    /// - `top_level_fn` is not found in the function registry
    /// - `top_level_fn` is polymorphic (more than one instantiation)
    /// - The return type of `top_level_fn` does not contain a `Sort::Path` field
    fn process_path_queries(
        &self,
        base_code: &str,
        ir: &IRContext,
        top_level_fn: &str,
        target_ids: &BTreeSet<usize>,
    ) -> String;
}

/// A utility for source code builder
/// This struct collects lines of code in a buffer.
pub(crate) struct ContentBuilder {
    /// Internal buffer holding all lines of code so far.
    buffer: String,
    /// Current indentation level (counted as number of tabs).
    indent: usize,
}

impl ContentBuilder {
    /// Create a new, empty content builder.
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

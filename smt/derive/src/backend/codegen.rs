use crate::backend::error::BackendResult;
use crate::ir::ctxt::IRContext;
// use crate::backend::cvc5::common::CodeGenCVC5;
// use crate::backend::cvc5::engine_smt::BackendCVC5SMT;
use crate::backend::z3::common::CodeGenZ3;
use crate::backend::z3::engine_chc::BackendZ3CHC;

/// A generic trait for backend code generators. 
///
/// CodeGenCVC5 and CodeGenZ3 implement this trait.
pub trait CodeGen {
    /// Returns the name of this code generator (e.g., "z3_chc").
    fn name(&self) -> String;

    /// Returns the file extension (or flavor) of the source code (e.g., "smt2").
    fn flavor(&self) -> &'static str {
        "smt2"
    }

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

    /// Runs a closure in an incremented indentation scope. 
    ///
    /// Anything inserted via `.line()` inside the closure will have +1 indentation level 
    /// compared to the outer scope. After the closure finishes, indentation reverts.
    ///
    /// let mut builder = ContentBuilder::new();
    /// builder.line("fn main() {");
    /// builder.scope(|b| {
    ///     b.line("println!(\"Inside scope\");");
    /// });
    /// builder.line("}");
    pub fn scope<F: Fn(&mut Self)>(&mut self, f: F) {
        self.indent += 1;
        f(self);
        self.indent -= 1;
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
    vec![
        Box::new(CodeGenZ3::new(BackendZ3CHC::new())),
        // Box::new(CodeGenCVC5::new(BackendCVC5SMT::new())), //? uncomment this!
    ]
}

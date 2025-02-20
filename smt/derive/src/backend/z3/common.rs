use std::path::PathBuf;

use lazy_static::lazy_static;

use rusmart_cli::cli::expect_z3;

use crate::backend::codegen::CodeGen;
use crate::backend::error::BackendResult;
use crate::ir::ctxt::IRContext;

// `lazy_static!` ensures that a static, thread-safe value is initialized only once
// and reused across the program. 
// This is ideal for configurations or artifacts that are expensive to compute or fetch.
//
// `ARTIFACT` holds the path to the Z3 binary or artifact (located via `expect_z3`).
// This is time consuming and should be done only once because it may need to build.
lazy_static! {
    static ref ARTIFACT: PathBuf = expect_z3(); // Locate Z3 and store its path.
}

/// A generic backend for Z3-related
pub trait BackendZ3 {
    /// Returns the name of the backend (e.g., "Z3").
    fn name(&self) -> String;

    /// Given an IRContext, produce Z3-compatible SMT code.
    ///
    /// # Arguments
    /// - `ir`: The Intermediate Representation (IR) context to be translated.
    ///
    /// # Returns
    /// A `BackendResult` wrapping the generated code as a `String` or an error.
    fn process(&self, ir: &IRContext) -> BackendResult<String>;
}

/// A wrapper for Z3 backends that implements the `CodeGen` trait.
pub struct CodeGenZ3<T: BackendZ3> {
    backend: T, // The specific Z3 backend being wrapped.
}

impl<T: BackendZ3> CodeGenZ3<T> {
    /// Constructs a new `CodeGenZ3` wrapper around a specific `BackendZ3` implementation.
    pub fn new(backend: T) -> Self {
        Self { backend }
    }
}

/// Implement the `CodeGen` trait for `CodeGenZ3`.
impl<T: BackendZ3> CodeGen for CodeGenZ3<T> {
    /// Returns the name of the backend (delegated to the wrapped `BackendZ3` implementation).
    fn name(&self) -> String {
        self.backend.name()
    }

    /// Generates the backend-specific code by delegating to the wrapped `BackendZ3` implementation.
    ///
    /// # Arguments
    /// - `ir`: The Intermediate Representation (IR) context to be translated.
    ///
    /// # Returns
    /// A `BackendResult` wrapping the generated code as a `String` or an error.
    fn process(&self, ir: &IRContext) -> BackendResult<String> {
        self.backend.process(ir)
    }
}

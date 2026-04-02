//! Z3 API backend: builds Z3 objects directly in memory using z3-sys.
//!
//! Unlike the text backend (which generates SMT-LIB2 strings and spawns a Z3 subprocess),
//! this backend constructs Z3 sorts, datatypes, functions, and expressions as in-memory
//! objects via the Z3 C API, then uses the Z3 solver API for satisfiability checking.

pub mod context;
pub mod intrinsics;
pub mod solver;
pub mod translate;

use crate::backend::response::Response;
use std::ffi::CStr;
use std::marker::PhantomData;

/// Result of solving a single error target.
pub struct SolveResult {
    pub target_idx: usize,
    pub response: Response,
    pub elapsed_ms: u128,
}

/// A ref-counted wrapper around a raw Z3_ast.
///
/// Calls Z3_inc_ref on creation and Z3_dec_ref on drop.
/// All Z3 expression construction goes through this type.
#[derive(Debug)]
pub struct Z3Ast<'ctx> {
    ctx: z3_sys::Z3_context,
    ast: z3_sys::Z3_ast,
    _phantom: PhantomData<&'ctx ()>,
}

impl<'ctx> Z3Ast<'ctx> {
    /// Wrap a raw Z3_ast with reference counting.
    ///
    /// # Safety
    /// `ast` must be a valid Z3_ast created from the given context.
    pub unsafe fn new(ctx: z3_sys::Z3_context, ast: z3_sys::Z3_ast) -> Self {
        z3_sys::Z3_inc_ref(ctx, ast);
        Self {
            ctx,
            ast,
            _phantom: PhantomData,
        }
    }

    /// Get the raw Z3_ast pointer.
    pub fn raw(&self) -> z3_sys::Z3_ast {
        self.ast
    }
}

impl Clone for Z3Ast<'_> {
    fn clone(&self) -> Self {
        unsafe {
            z3_sys::Z3_inc_ref(self.ctx, self.ast);
        }
        Self {
            ctx: self.ctx,
            ast: self.ast,
            _phantom: PhantomData,
        }
    }
}

impl Drop for Z3Ast<'_> {
    fn drop(&mut self) {
        unsafe {
            z3_sys::Z3_dec_ref(self.ctx, self.ast);
        }
    }
}

/// Helper to create a Z3 string symbol from a Rust &str.
pub(crate) unsafe fn mk_string_symbol(ctx: z3_sys::Z3_context, name: &str) -> z3_sys::Z3_symbol {
    let c_str = std::ffi::CString::new(name).unwrap();
    z3_sys::Z3_mk_string_symbol(ctx, c_str.as_ptr()).expect("Z3_mk_string_symbol failed")
}

/// Convert a Z3 model to a string representation.
pub(crate) unsafe fn model_to_string(
    ctx: z3_sys::Z3_context,
    model: z3_sys::Z3_model,
) -> String {
    let raw = z3_sys::Z3_model_to_string(ctx, model);
    CStr::from_ptr(raw).to_string_lossy().into_owned()
}

/// RAII wrapper for a z3-sys context.
pub struct Z3Context {
    pub ctx: z3_sys::Z3_context,
}

impl Z3Context {
    pub fn new() -> Self {
        unsafe {
            let cfg = z3_sys::Z3_mk_config().expect("Z3_mk_config failed");
            // Enable model generation
            let k = std::ffi::CString::new("model").unwrap();
            let v = std::ffi::CString::new("true").unwrap();
            z3_sys::Z3_set_param_value(cfg, k.as_ptr(), v.as_ptr());
            // Disable proof generation
            let k2 = std::ffi::CString::new("proof").unwrap();
            let v2 = std::ffi::CString::new("false").unwrap();
            z3_sys::Z3_set_param_value(cfg, k2.as_ptr(), v2.as_ptr());

            let ctx = z3_sys::Z3_mk_context_rc(cfg).expect("Z3_mk_context_rc failed");
            z3_sys::Z3_set_error_handler(ctx, None);
            z3_sys::Z3_del_config(cfg);

            Self { ctx }
        }
    }
}

impl Drop for Z3Context {
    fn drop(&mut self) {
        unsafe {
            z3_sys::Z3_del_context(self.ctx);
        }
    }
}

// z3-sys 0.8 is missing some newer Z3 C API functions that exist in the installed Z3 library.
// We declare them here via extern "C".
unsafe extern "C" {
    pub fn Z3_mk_string_to_code(c: z3_sys::Z3_context, s: z3_sys::Z3_ast) -> z3_sys::Z3_ast;
    pub fn Z3_mk_string_from_code(c: z3_sys::Z3_context, s: z3_sys::Z3_ast) -> z3_sys::Z3_ast;
    pub fn Z3_mk_str_le(c: z3_sys::Z3_context, s1: z3_sys::Z3_ast, s2: z3_sys::Z3_ast) -> z3_sys::Z3_ast;
    pub fn Z3_mk_str_lt(c: z3_sys::Z3_context, s1: z3_sys::Z3_ast, s2: z3_sys::Z3_ast) -> z3_sys::Z3_ast;
    pub fn Z3_mk_seq_nth(
        c: z3_sys::Z3_context,
        s: z3_sys::Z3_ast,
        index: z3_sys::Z3_ast,
    ) -> z3_sys::Z3_ast;
}

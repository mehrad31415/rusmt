//! # Executable Semantics of Programming Languages

// This macro checks the missing documentation in all the `public` modules in the module tree of the library crate.
#![warn(missing_docs)]

/// TOML module for parsing TOML files.
/// This module can be accessed from the binary crate (and other external crates).
pub mod toml;

/// IMP/WHILE module: the canonical small imperative language from
/// Winskel, *The Formal Semantics of Programming Languages* (MIT Press, 1993),
pub mod imp;

/// Render synthesized Z3 models of `input_0 : Com` back into `.imp` source.
pub mod imp_render;

/// Module for certifying the correctness of programs.
pub mod certify;

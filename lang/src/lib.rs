//! # Executable Semantics of Programming Languages

// This macro checks the missing documentation in all the `public` modules in the module tree of the library crate.
#![warn(missing_docs)]

/// TOML module for parsing TOML files.
/// This module can be accessed from the binary crate (and other external crates).
pub mod toml;

/// Demo parser module for testing error discovery.
pub mod demo_parser;

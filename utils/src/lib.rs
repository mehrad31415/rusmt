//! # Utils
//!
//! `utils` is a collection of utilities for the other crates in the rustmart project.
//!
//! It provides the following modules:
//!
//! * config - a configuration module
//! * lib - a library module

// this macro checks the missing documentation in all the `public` modules in the mdoule tree of the library crate. 
// In this case, the only public module in the module tree is `config`.
#![deny(missing_docs)]

/// This module contains all the configuration settings for the application
pub mod config;

//! Error handling utilities
//!
//! This module provides five <macros by example> to handle errors:
//! - `fail_on!` to return a compiler error
//! - `fail_if_error!` to return a compiler error if an error is found
//! - `bail_on!` to return an Error instance
//! - `bail_if_exists!` to return an Error instance if a token exists
//! - `bail_if_missing!` to return an Error instance if a token is missing

/// Emit a compile error at the span of `$item` with a formatted message.
/// patterns:
/// - `fail_on!(item (value), "message")` to return a compiler error with a literal message
/// - `fail_on!(item (value), "message {}", arg)` to return a compiler error with a formatted message
#[macro_export]
macro_rules! fail_on {
    // The proc_macro::TokenStream is the input and return type of the top level procedural macros.
    // We know that "proc-macro is used at API level and proc-macro2 everywhere else" so the fail_on! macro will be used as the return type of procedural macros, defined in the lib.rs file of the rusmart-smt-remark-derive crate.
    // The into_compile_error method converts a syn::Error to proc_macro2::TokenStream.
    // The proc_macro::TokenStream::from is used to convert the proc_macro2::TokenStream to proc_macro::TokenStream.
    // Error has got a method named into_compile_error() which generates a compilation error from the error object.
    // Syn uses spans to represent the location (line and column number) of the expression in the source where it was initially located.
    // This is used mainly for error reporting. All structs (AST elements) implement Spanned.
    ($item:expr, $msg:literal $(,)?) => {
        return ::proc_macro::TokenStream::from(
            ::syn::Error::new_spanned($item, $msg).into_compile_error()
        )
    };
    ($item:expr, $fmt:expr, $($arg:tt)+) => {
        return ::proc_macro::TokenStream::from(
            ::syn::Error::new_spanned($item, format!($fmt, $($arg)+)).into_compile_error()
        )
    };
}

/// Convert a `Result<proc_macro2::TokenStream, ::syn::Error>` into
/// a `proc_macro::TokenStream`, emitting a compile error on `Err`.
/// patterns:
/// - `fail_if_error!(Ok(proc_macro2::TokenStream::from(value)))` to return the value wrapped as token stream if it is Ok
/// - `fail_if_error!(Err(error))` to return the error as a compiler error
#[macro_export]
macro_rules! fail_if_error {
    ($item:expr) => {
        match $item {
            Ok(__v) => return ::proc_macro::TokenStream::from(__v),
            Err(__e) => return ::proc_macro::TokenStream::from(__e.into_compile_error()),
        }
    };
}

/// Return a `Result<T, syn::Error>` with an error at `$item`’s span.
/// patterns:
/// - `bail_on!(item (value), "message")` to return an Error instance with a literal message
/// - `bail_on!(item (value), "message {}", arg)` to return an Error instance with a formatted message
#[macro_export]
macro_rules! bail_on {
    ($item:expr, $msg:literal $(,)?) => {
        return Err(::syn::Error::new_spanned($item, $msg))
    };
    ($item:expr, $fmt:expr, $($arg:tt)+) => {
        return Err(::syn::Error::new_spanned($item, format!($fmt, $($arg)+)))
    };
}

/// Ensure `$opt` is `None`, otherwise bail with a message.
/// pattern: `ensure_none!(item, "msg")` to return an Error instance if the item exists (is Some)
#[macro_export]
macro_rules! ensure_none {
    ($item:expr, $msg:literal $(,)?) => {
        if let Some(spaned) = $item {
            $crate::bail_on!(spaned, $msg);
        }
    };
}

/// Ensure `$opt` is `Some(value)`, otherwise bail with a message.
/// pattern: `ensure_some!(item, parent, "note")` to return an Error instance if the item is missing (none)
#[macro_export]
macro_rules! ensure_some {
    ($item:expr, $par:expr, $msg:literal $(,)?) => {
        match $item {
            None => $crate::bail_on!($par, "expect {}", $msg),
            Some(__v) => __v,
        }
    };
}

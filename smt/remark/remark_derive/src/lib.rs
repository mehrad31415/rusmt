//! Procedural macros for the SMT remarking framework.

use proc_macro::TokenStream as Syntax;
use proc_macro2::TokenStream;
use rusmart_smt_remark::fail_if_error;
use rusmart_smt_remark::{func, ty};

/// Annotation over a Rust type (can only be applied on a Rust type definition i.e., struct and enum)
/// Attributes will not be accepted. i.e., #[smt_type(<...attrs...>)] will cause an error
/// This will basically generate the code for deriving Clone, Debug, Hash, Default, Copy for the struct/enum and also implement the SMT trait for the struct/enum
/// if ty::derive_for_type(attr, item) gives an Err, then a compile error will be raised. Otherwise if it gives an Ok, then the output will be produced
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[inline]
pub fn smt_type(attr: Syntax, item: Syntax) -> Syntax {
    let attr: TokenStream = TokenStream::from(attr);
    let item: TokenStream = TokenStream::from(item);
    fail_if_error!(ty::derive_for_type(attr, item))
}

/// Annotation over a Rust function
/// This annotation can only be applied on a Rust function i.e., fn as top-level module item (in contrast to fn in an impl block)
#[proc_macro_attribute]
#[cfg(not(tarpaulin_include))]
#[inline]
pub fn smt_impl(attr: Syntax, item: Syntax) -> Syntax {
    let attr: TokenStream = TokenStream::from(attr);
    let item: TokenStream = TokenStream::from(item);
    fail_if_error!(func::derive_for_impl(attr, item))
}

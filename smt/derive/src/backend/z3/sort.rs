//! This module contains the conversion functions for converting Rusmart types to SMT-LIB types

use crate::backend::z3::ty::tyuse_in_smt;
use crate::ir::sort::Sort;
use crate::IRContext;

/// Converts a Rust `Sort` into the corresponding SMT-LIB sort as a `String`
pub fn sort_to_smt(s: &Sort, ir: &IRContext) -> String {
    match s {
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Rational => "Real".to_string(),
        Sort::Text => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", sort_to_smt(inner, ir)),
        Sort::Set(inner) => format!("(Set {})", sort_to_smt(inner, ir)),
        Sort::Map(key, value) => {
            format!(
                "(Array {} {})",
                sort_to_smt(key, ir),
                sort_to_smt(value, ir)
            )
        }
        Sort::Error => "undefined_function".to_string(), // triggers an undefined function which leads to a crash assuming that `undefined_function` is not defined!
        Sort::User(usr_sort_id) => tyuse_in_smt(*usr_sort_id, ir),
        Sort::Uninterpreted(name) => format!("{}", name),
    }
}

/// This function gives the value no present for sorts
pub fn sort_not_present(s: &Sort, ir: &IRContext) -> String {
    format!("(declare-const not_present_{} {})", s, sort_to_smt(s, ir))
}
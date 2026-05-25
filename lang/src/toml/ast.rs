//! The Abstract Syntax Tree (AST) for a parsed TOML document.

use rusmt_smt_remark_derive::smt_type;
use rusmt_smt_stdlib::smt::SMT;
use rusmt_smt_stdlib::{Array, Boolean, Cloak, F64, I64, Seq, String};
use std::hash::Hash;

/// A term *in its valid state* is defined by the following ADT
///
/// This enum is the target data structure for our parser. The goal is to
/// transform a TOML text file into a `Value::Table`.
#[smt_type]
pub enum Value {
    /// A TOML string.
    String(String),
    /// A TOML integer (always 64-bit per the spec).
    Integer(I64),
    /// A TOML float (always 64-bit per the spec).
    Float(F64),
    /// A TOML boolean.
    Boolean(Boolean),
    /// A TOML datetime.
    DateTime(DateTime),
    /// A TOML array of values.
    Array(Cloak<Seq<Value>>),
    /// A TOML table (map from string keys to values).
    Table(Cloak<Array<String, Value>>),
}

/// A TOML date-time value.
#[smt_type]
pub enum DateTime {
    /// A TOML offset-date-time.
    OffsetDateTime(String),
    /// A TOML local-date-time.
    LocalDateTime(String),
    /// A TOML local-date.
    LocalDate(String),
    /// A TOML local-time.
    LocalTime(String),
}

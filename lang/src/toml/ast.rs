//! The Abstract Syntax Tree (AST) for a parsed TOML document.

use rusmart_smt_stdlib::smt::SMT;
use rusmart_smt_stdlib::{Array, Boolean, Cloak, F64, I64, Seq, String};
use std::cmp::Ordering;
use std::hash::Hash;

/// A term *in its valid state* is defined by the following ADT
///
/// This enum is the target data structure for our parser. The goal is to
/// transform a TOML text file into a `Value::Table`.
#[derive(Debug, Clone, Copy, Hash)] // TODO: remove at give smt_type
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

// TODO: remove
impl Default for Value {
    fn default() -> Self {
        Value::Table(Cloak::shield(Array::new()))
    }
}

/// A TOML date-time value.
#[derive(Debug, Clone, Copy, Hash)] // TODO: remove at give smt_type
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

// TODO: remove
/// Implement the Default trait for DateTime
impl Default for DateTime {
    fn default() -> Self {
        DateTime::LocalDate(String::default())
    }
}

// TODO: remove
/// Implement the SMT trait for Value
impl SMT for Value {
    fn _cmp(self, other: Self) -> Ordering {
        if core::mem::discriminant(&self) != core::mem::discriminant(&other) {
            // Define a simple ordering based on the variant type and then the inner value.
            use Value::*;
            let order_self = match self {
                String(_) => 0,
                Integer(_) => 1,
                Float(_) => 2,
                Boolean(_) => 3,
                DateTime(_) => 4,
                Array(_) => 5,
                Table(_) => 6,
            };
            let order_rhs = match other {
                String(_) => 0,
                Integer(_) => 1,
                Float(_) => 2,
                Boolean(_) => 3,
                DateTime(_) => 4,
                Array(_) => 5,
                Table(_) => 6,
            };
            return order_self.cmp(&order_rhs);
        } else {
            // If variants are the same, compare the inner values.
            match (self, other) {
                (Self::String(l), Self::String(r)) => l._cmp(r),
                (Self::Integer(l), Self::Integer(r)) => l._cmp(r),
                (Self::Float(l), Self::Float(r)) => l._cmp(r),
                (Self::Boolean(l), Self::Boolean(r)) => l._cmp(r),
                (Self::DateTime(l), Self::DateTime(r)) => l._cmp(r),
                (Self::Array(l), Self::Array(r)) => l._cmp(r),
                (Self::Table(l), Self::Table(r)) => l._cmp(r),
                // This case is unreachable due to the discriminant check, but required for exhaustiveness.
                _ => Ordering::Equal,
            }
        }
    }
}

// TODO: remove
/// Implement the SMT trait for DateTime
impl SMT for DateTime {
    fn _cmp(self, other: Self) -> Ordering {
        use DateTime::*;
        match (self, other) {
            (OffsetDateTime(l), OffsetDateTime(r)) => l._cmp(r),
            (LocalDateTime(l), LocalDateTime(r)) => l._cmp(r),
            (LocalDate(l), LocalDate(r)) => l._cmp(r),
            (LocalTime(l), LocalTime(r)) => l._cmp(r),
            // Define an arbitrary ordering between different DateTime variants.
            (OffsetDateTime(_), _) => Ordering::Greater,
            (LocalDateTime(_), OffsetDateTime(_)) => Ordering::Less,
            (LocalDateTime(_), _) => Ordering::Greater,
            (LocalDate(_), LocalTime(_)) => Ordering::Greater,
            (LocalDate(_), _) => Ordering::Less,
            (LocalTime(_), _) => Ordering::Less,
        }
    }
}

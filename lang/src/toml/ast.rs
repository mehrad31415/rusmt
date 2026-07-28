//! The Abstract Syntax Tree (AST) for a parsed TOML document.

use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::smt::SMT;
use rusmt_smt_stdlib::{Boolean, Cloak, F64, I64, Seq, String};
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
    Table(Cloak<Table>),
}

/// A TOML table represented as a recursive association list.
///
/// This replaces the previous `Array<String, Value>` encoding. The motivation
/// is that Z3's array theory becomes incomplete on `(Array String Value)` when
/// `Value` is itself recursive (it contains a `Table`); Z3 returns
/// `unknown ("incomplete (theory array)")` even for fully concrete inputs.
/// A recursive datatype keeps the encoding inside the datatype theory only,
/// with no array axioms and no quantifiers.
#[smt_type]
pub enum Table {
    /// The empty table.
    Empty,
    /// `Bind(key, value, rest)` — one binding prepended to the rest of the table.
    Bind(String, Cloak<Value>, Cloak<Table>),
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

/// `Table::Empty` — the empty table (replaces `Array::new()`).
#[smt_fn]
pub(crate) fn table_new() -> Table {
    Table::Empty
}

/// Whether a table has no bindings (replaces `array.is_empty()`).
#[smt_fn]
pub(crate) fn table_is_empty(t: Table) -> Boolean {
    match t {
        Table::Empty => Boolean::from(true),
        Table::Bind(_k, _v, _rest) => Boolean::from(false),
    }
}

/// Whether a table contains a binding for `k` (replaces `array.contains_key(k)`).
#[smt_fn]
pub(crate) fn table_contains_key(t: Table, k: String) -> Boolean {
    match t {
        Table::Empty => Boolean::from(false),
        Table::Bind(key, _v, rest) => {
            if *key.eq(k) {
                Boolean::from(true)
            } else {
                table_contains_key(rest.reveal(), k)
            }
        }
    }
}

/// Look up the value bound to `k` (replaces `array.select(k)`).
///
/// Callers always guard with `table_contains_key` first, so the missing-key
/// branch is unreachable in practice.
#[smt_fn]
pub(crate) fn table_get(t: Table, k: String) -> Value {
    match t {
        Table::Empty => Value::Boolean(Boolean::from(false)),
        Table::Bind(key, v, rest) => {
            if *key.eq(k) {
                v.reveal()
            } else {
                table_get(rest.reveal(), k)
            }
        }
    }
}

/// Insert/overwrite the binding for `k` with `v` (replaces `array.store(k, v)`).
///
/// If `k` already exists, its value is replaced in place (rebuilding the spine);
/// otherwise the new binding is prepended. This matches `(store arr k v)`.
#[smt_fn]
pub(crate) fn table_store(t: Table, k: String, v: Value) -> Table {
    match t {
        Table::Empty => Table::Bind(k, Cloak::shield(v), Cloak::shield(Table::Empty)),
        Table::Bind(key, val, rest) => {
            if *key.eq(k) {
                // overwrite this binding
                Table::Bind(key, Cloak::shield(v), rest)
            } else {
                Table::Bind(key, val, Cloak::shield(table_store(rest.reveal(), k, v)))
            }
        }
    }
}

/// Remove the binding for `k` if present (replaces `array.del(k)`).
#[smt_fn]
pub(crate) fn table_del(t: Table, k: String) -> Table {
    match t {
        Table::Empty => Table::Empty,
        Table::Bind(key, val, rest) => {
            if *key.eq(k) {
                rest.reveal()
            } else {
                Table::Bind(key, val, Cloak::shield(table_del(rest.reveal(), k)))
            }
        }
    }
}

/// The minimum key (by `String::lt`) among the bindings of a non-empty table.
///
/// On an empty table this returns the empty string (callers guard with `table_is_empty` before calling).
#[smt_fn]
pub(crate) fn table_key_min(t: Table) -> String {
    match t {
        Table::Empty => String::from(""),
        Table::Bind(key, _v, rest) => {
            let rest_t = rest.reveal();
            match rest_t {
                Table::Empty => key,
                Table::Bind(_k2, _v2, _r2) => {
                    let rest_min = table_key_min(rest_t);
                    if *key.lt(rest_min) { key } else { rest_min }
                }
            }
        }
    }
}

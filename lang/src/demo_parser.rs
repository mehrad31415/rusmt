//! Simple demo parser to test error discovery system

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Error, Integer, Seq, String, smt::SMT};

/// Parser state
#[smt_type]
pub struct State {
    pub stream: Seq<String>,
    pub cursor: Integer,
}

/// Optional value (since stdlib doesn't have Optional)
#[smt_type]
pub enum Optional<T: SMT> {
    None,
    Some(T),
}

/// Parse result
#[smt_type]
pub enum ParseResult<T: SMT> {
    NoMatch,
    Ok(T, State),
    Err(Error),
}

/// Simple value type
#[smt_type]
pub enum Value {
    Number(Integer),
    Letter(String),
}

/// Get current character
#[smt_fn]
fn current_char(state: State) -> Optional<String> {
    if *state.cursor.lt(state.stream.length()) {
        Optional::Some(state.stream.at(state.cursor))
    } else {
        Optional::None
    }
}

/// Advance cursor by 1
#[smt_fn]
fn advance(state: State) -> State {
    State {
        stream: state.stream,
        cursor: state.cursor.add(Integer::from(1)),
    }
}

/// Check if character is a digit (0-9)
#[smt_fn]
fn is_digit(c: String) -> Boolean {
    Boolean::from(
        *c.eq(String::from("0"))
            || *c.eq(String::from("1"))
            || *c.eq(String::from("2"))
            || *c.eq(String::from("3"))
            || *c.eq(String::from("4"))
            || *c.eq(String::from("5"))
            || *c.eq(String::from("6"))
            || *c.eq(String::from("7"))
            || *c.eq(String::from("8"))
            || *c.eq(String::from("9")),
    )
}

/// Check if character is a letter (a-z, A-Z) - simplified
#[smt_fn]
fn is_letter(c: String) -> Boolean {
    Boolean::from(
        *c.eq(String::from("a"))
            || *c.eq(String::from("b"))
            || *c.eq(String::from("z"))
            || *c.eq(String::from("A"))
            || *c.eq(String::from("B"))
            || *c.eq(String::from("Z")),
    )
}

/// Parse a number
#[smt_fn]
fn parse_number(state: State) -> ParseResult<Value> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_digit(c) {
                ParseResult::Ok(Value::Number(Integer::from(0)), advance(state))
            } else {
                // ERROR 0: Expected digit but got non-digit
                ParseResult::Err(Error::fresh())
            }
        }
        Optional::None => {
            // ERROR 1: Unexpected end of input when parsing number
            ParseResult::Err(Error::fresh())
        }
    }
}

/// Parse a letter
#[smt_fn]
fn parse_letter(state: State) -> ParseResult<Value> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_letter(c) {
                ParseResult::Ok(Value::Letter(c), advance(state))
            } else {
                // ERROR 2: Expected letter but got non-letter
                ParseResult::Err(Error::fresh())
            }
        }
        Optional::None => {
            // ERROR 3: Unexpected end of input when parsing letter
            ParseResult::Err(Error::fresh())
        }
    }
}

/// Top-level parser: try number first, then letter
#[smt_fn]
pub fn parse_value(state: State) -> ParseResult<Value> {
    match parse_number(state) {
        ParseResult::Ok(val, st) => ParseResult::Ok(val, st),
        ParseResult::Err(_) => parse_letter(state),
        ParseResult::NoMatch => ParseResult::NoMatch,
    }
}

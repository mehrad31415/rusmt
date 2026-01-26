//! Simple demo parser to test error discovery system

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Error, Integer, Seq, String, smt::SMT};

/// Parser state
#[smt_type]
pub struct State {
    pub stream: Seq<String>,
    pub cursor: Integer,
}

/// Get current character
#[smt_fn]
fn current_char(state: State) -> Boolean {
    let a = state.stream.append(String::from("a"));
    Boolean::from(true)
    // if *state.cursor.lt(state.stream.length()) {
    //    state.stream.at(state.cursor)
    // } else {
    //     String::from("")
    // }
}
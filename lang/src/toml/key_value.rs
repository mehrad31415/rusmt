//! Parsing TOML key-value pairs.

use crate::toml::{
    Optional, ParseResult, State, advance,
    array::parse_array,
    ast::Value,
    boolean::parse_boolean,
    current_char,
    datetime::parse_datetime,
    float::{Number, parse_float},
    is_alpha, is_apostrophe, is_dec_digit, is_quotation_mark, parse_ws,
    string::{parse_basic_string, parse_literal_string, parse_string},
    table::parse_inline_table,
};
use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Cloak, Error, Seq, String, smt::SMT};

/// A key-value pair returned by `parse_key_value`.
///
/// We use a record (not a tuple/pack) because Rusmart's expression IR intentionally restricts
/// tuple destructuring and does not support projecting pack slots from a variable.
#[smt_type]
pub struct KeyVal {
    pub key: Seq<String>,
    pub val: Value,
}

/// `keyval = key keyval-sep val`
#[smt_fn]
pub fn parse_key_value(state: State) -> ParseResult<KeyVal> {
    match parse_key(state) {
        ParseResult::Ok(key, state_after_key) => {
            match parse_keyval_sep(state_after_key) {
                ParseResult::Ok(_content, _state_after_sep) => {
                    match current_char(_state_after_sep) {
                        Optional::Some(_c) => {
                            match parse_value(key, _state_after_sep) {
                                ParseResult::Ok(val, state_after_val) => {
                                    return ParseResult::Ok(KeyVal { key, val }, state_after_val);
                                }
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    // println!("expected a value but none matched");
                                    return ParseResult::Err(Error::fresh());
                                } // expected a value but none matched
                            }
                        }
                        Optional::None => {
                            // println!("expected a value but found end of input");
                            return ParseResult::Err(Error::fresh());
                        } // expected a value but found end of input
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// `key = simple-key / dotted-key`
/// `dotted-key = simple-key 1*( dot-sep simple-key )`
/// `key = simple-key *( dot-sep simple-key )`
#[smt_fn]
pub(crate) fn parse_key(state: State) -> ParseResult<Seq<String>> {
    match parse_simple_key(state) {
        ParseResult::Ok(first_key, state_after_first) => {
            let after_ws = parse_ws(state_after_first);
            match current_char(after_ws) {
                Optional::Some(_c) => {
                    let acc = Seq::new().append(first_key);
                    if *_c.eq(String::from(".")) {
                        // it's a dotted key
                        parse_dotted_key_loop(acc, after_ws)
                    } else {
                        // it's a simple key
                        ParseResult::Ok(acc, after_ws)
                    }
                }
                Optional::None => ParseResult::Ok(Seq::new().append(first_key), after_ws),
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// `keyval-sep = ws %x3D ws ; =`
#[smt_fn]
fn parse_keyval_sep(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    let c = current_char(state_after_ws);
    match c {
        Optional::Some(ch) => {
            if *ch.eq(String::from("=")) {
                ParseResult::Ok(ch, parse_ws(advance(state_after_ws)))
            } else {
                // expected '=' but found something else
                // println!("expected '=' but found another character {:?}", ch);
                ParseResult::Err(Error::fresh())
            }
        }
        // expected '=' but found end of input
        Optional::None => {
            // println!("expected '=' but found end of input");
            ParseResult::Err(Error::fresh())
        }
    }
}

/// Helper function to parse the rest of a dotted key.
#[smt_fn]
fn parse_dotted_key_loop(acc: Seq<String>, state: State) -> ParseResult<Seq<String>> {
    match parse_dot_sep(state) {
        ParseResult::Ok(_dot, state_after_dot) => match parse_simple_key(state_after_dot) {
            ParseResult::Ok(next_key, state_after_key) => {
                let new_acc = acc.append(next_key);
                parse_dotted_key_loop(new_acc, state_after_key)
            }
            ParseResult::Err(e) => return ParseResult::Err(e),
            // there were no more keys after dot
            ParseResult::NoMatch => {
                // println!("expected a key after '.' but none found");
                return ParseResult::Err(Error::fresh())
            }
        },
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => ParseResult::Ok(acc, state), // end of dotted key
    }
}

/// `dot-sep   = ws %x2E ws  ; . Period`
#[smt_fn]
fn parse_dot_sep(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    let c = current_char(state_after_ws);
    match c {
        Optional::Some(ch) => {
            if *ch.eq(String::from(".")) {
                return ParseResult::Ok(ch, parse_ws(advance(state_after_ws)))
            } else {
                return ParseResult::NoMatch // cannot happen
            }
        }
        Optional::None => return ParseResult::NoMatch, // cannot happen
    }
}

/// `simple-key = quoted-key / unquoted-key`
#[smt_fn]
fn parse_simple_key(state: State) -> ParseResult<String> {
    match parse_quoted_key(state) {
        ParseResult::Ok(quoted_key, state_after_quoted) => {
            return ParseResult::Ok(quoted_key, state_after_quoted)
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => parse_unquoted_key(state),
    }
}

/// `unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _`
#[smt_fn]
fn parse_unquoted_key(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(first_char) => {
            if *is_alpha(first_char) {
                parse_rest_of_unquoted_key(advance(state), first_char)
            } else {
                if *is_dec_digit(first_char) {
                    parse_rest_of_unquoted_key(advance(state), first_char)
                } else {
                    if *first_char.eq(String::from("-")) {
                        parse_rest_of_unquoted_key(advance(state), first_char)
                    } else {
                        if *first_char.eq(String::from("_")) {
                            parse_rest_of_unquoted_key(advance(state), first_char)
                        } else {
                            // println!("invalid first character for unquoted key {:?}", first_char);
                            ParseResult::Err(Error::fresh()) // invalid first character for unquoted key
                        }
                    }
                }
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// Helper function to parse the rest of an unquoted key.
#[smt_fn]
fn parse_rest_of_unquoted_key(state: State, acc: String) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_alpha(c) {
                let new_acc = acc.concat(c);
                parse_rest_of_unquoted_key(advance(state), new_acc)
            } else {
                if *is_dec_digit(c) {
                    let new_acc = acc.concat(c);
                    parse_rest_of_unquoted_key(advance(state), new_acc)
                } else {
                    if *c.eq(String::from("-")) {
                        let new_acc = acc.concat(c);
                        parse_rest_of_unquoted_key(advance(state), new_acc)
                    } else {
                        if *c.eq(String::from("_")) {
                            let new_acc = acc.concat(c);
                            parse_rest_of_unquoted_key(advance(state), new_acc)
                        } else {
                            ParseResult::Ok(acc, state) // end of unquoted key
                        }
                    }
                }
            }
        }
        Optional::None => ParseResult::Ok(acc, state),
    }
}

/// `quoted-key = basic-string / literal-string`
#[smt_fn]
fn parse_quoted_key(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_quotation_mark(c) {
                parse_basic_string(state)
            } else {
                if *is_apostrophe(c) {
                    parse_literal_string(state)
                } else {
                    ParseResult::NoMatch
                }
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// The main dispatcher for parsing any TOML value.
/// `val = string / boolean / array / inline-table / date-time / float / integer`
#[smt_fn]
pub(crate) fn parse_value(key: Seq<String>, input: State) -> ParseResult<Value> {
    // first we match boolean because it is more restrictive than string
    match parse_boolean(input) {
        ParseResult::Ok(bool_val, remaining_input) => {
            ParseResult::Ok(Value::Boolean(bool_val), remaining_input)
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // then parse array
            match parse_array(key, input) {
                ParseResult::Ok(array_val, remaining_input) => {
                    ParseResult::Ok(Value::Array(Cloak::shield(array_val)), remaining_input)
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    // then parse inline table
                    match parse_inline_table(key, input) {
                        ParseResult::Ok(table_val, remaining_input) => {
                            ParseResult::Ok(Value::Table(Cloak::shield(table_val)), remaining_input)
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            // the parse string
                            match parse_string(input) {
                                ParseResult::Ok(str_val, remaining_input) => {
                                    ParseResult::Ok(Value::String(str_val), remaining_input)
                                }
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    // then parse date-time
                                    match parse_datetime(input) {
                                        ParseResult::Ok(dt_val, remaining_input) => {
                                            ParseResult::Ok(
                                                Value::DateTime(dt_val),
                                                remaining_input,
                                            )
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::NoMatch => {
                                            // then parse float
                                            match parse_float(input) {
                                                ParseResult::Ok(float_val, remaining_input) => {
                                                    match float_val {
                                                        Number::F64(f) => ParseResult::Ok(
                                                            Value::Float(f),
                                                            remaining_input,
                                                        ),
                                                        Number::Integer(i) => ParseResult::Ok(
                                                            Value::Integer(i),
                                                            remaining_input,
                                                        ),
                                                    }
                                                }
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                ParseResult::NoMatch => return ParseResult::NoMatch,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

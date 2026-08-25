//! Array parsing functions.
use crate::toml::{
    Optional, ParseResult, ParserContext, State, advance, ast::Value, current_char,
    key_value::parse_value, parse_comment, parse_newline, parse_wschar,
};
use rusmt_smt_remark_derive::smt_fn;
use rusmt_smt_stdlib::{Boolean, Path, Seq, String, U32, smt::SMT};

/// Remember `key` -- an absolute path -- as a statically defined array.
#[smt_fn]
fn remember_inline_array(state: State, key: Seq<String>) -> State {
    let state_temp = state;
    let stream = state_temp.stream;
    let cursor = state_temp.cursor;
    let context = state_temp.context;
    let context_temp = context;
    let new_context = ParserContext {
        current_table_path: context_temp.current_table_path,
        explicit_tables: context_temp.explicit_tables,
        closed_tables: context_temp.closed_tables,
        inline_tables: context_temp.inline_tables,
        inline_arrays: context_temp.inline_arrays.append(key),
        array_of_tables: context_temp.array_of_tables,
    };
    State {
        stream,
        cursor,
        context: new_context,
    }
}

/// array = array-open [ array-values ] ws-comment-newline array-close
#[smt_fn]
pub(crate) fn parse_array(key: Seq<String>, input: State) -> ParseResult<Seq<Value>> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(_c) => {
            if *is_array_open(_c) {
                let after_open = advance(input);
                match current_char(after_open) {
                    Optional::None => {
                        // println!("expect array-close after array-open found nothing");
                        ParseResult::Err(Path::named(String::from("array_open_eof")))
                    } // expect array-close
                    Optional::Some(_next_c) => {
                        match parse_ws_comment_newline(after_open) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                            ParseResult::Ok(_ws, after_ws) => {
                                // the array's absolute path: the enclosing table path plus its key
                                let after_ws_temp = after_ws;
                                let context = after_ws_temp.context;
                                let context_temp = context;
                                let current_table_path = context_temp.current_table_path;
                                let array_of_tables = context_temp.array_of_tables;
                                let new_key = current_table_path.concat(key);
                                if *array_of_tables.contains(new_key) {
                                    // println!("arrays of tables cannot contain inline arrays");
                                    return ParseResult::Err(Path::named(String::from(
                                        "array_of_tables_inline_array",
                                    ))); // arrays of tables cannot contain inline arrays
                                } else {
                                    // a statically defined array closes its path, empty or not
                                    let state_with_array = remember_inline_array(after_ws, new_key);
                                    match current_char(state_with_array) {
                                        Optional::None => {
                                            // println!("expect array-close after array-open found nothing");
                                            ParseResult::Err(Path::named(String::from(
                                                "array_open_after_ws_eof",
                                            )))
                                        }
                                        Optional::Some(x) => {
                                            if *is_array_close(x) {
                                                return ParseResult::Ok(
                                                    Seq::<Value>::new(),
                                                    advance(state_with_array),
                                                );
                                            } else {
                                                parse_array_values(key, state_with_array)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                ParseResult::NoMatch
            }
        }
    }
}

/// array-open =  %x5B ; [
#[smt_fn]
fn is_array_open(c: U32) -> Boolean {
    c.eq(U32::from(0x5B))
}

/// array-close = %x5D ; ]
#[smt_fn]
fn is_array_close(c: U32) -> Boolean {
    c.eq(U32::from(0x5D))
}

/// array-sep = %x2C  ; , Comma
#[smt_fn]
fn is_array_sep(c: U32) -> Boolean {
    c.eq(U32::from(0x2C))
}

/// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
/// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
#[smt_fn]
pub(crate) fn parse_array_values(key: Seq<String>, input: State) -> ParseResult<Seq<Value>> {
    match parse_value(key, input) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // println!("expected a value but none matched");
            ParseResult::Err(Path::named(String::from("array_values_expected_value")))
        } // expected a value but none matched
        ParseResult::Ok(val, after_val) => {
            match parse_ws_comment_newline(after_val) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                ParseResult::Ok(_ws2, after_ws2) => {
                    match current_char(after_ws2) {
                        Optional::None => {
                            // println!(
                            //     "expect array-close or separator after array value found nothing"
                            // );
                            ParseResult::Err(Path::named(String::from(
                                "array_value_eof_after_value",
                            ))) // expect array-close or separator
                        }
                        Optional::Some(next_c) => {
                            if *is_array_sep(next_c) {
                                let after_sep = advance(after_ws2);
                                match parse_ws_comment_newline(after_sep) {
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                    ParseResult::Ok(_ws3, after_ws3) => {
                                        match current_char(after_ws3) {
                                            Optional::None => {
                                                // println!(
                                                //     "expect array value after separator found nothing"
                                                // );
                                                ParseResult::Err(Path::named(String::from(
                                                    "array_sep_eof_after_comma",
                                                )))
                                            } // expect array value
                                            Optional::Some(newnew) => {
                                                if *is_array_close(newnew) {
                                                    // trailing comma case is okay
                                                    return ParseResult::Ok(
                                                        Seq::<Value>::new().append(val),
                                                        advance(after_ws3),
                                                    );
                                                } else {
                                                    match parse_array_values(key, after_ws3) {
                                                        ParseResult::Err(e) => {
                                                            return ParseResult::Err(e);
                                                        }
                                                        ParseResult::NoMatch => {
                                                            ParseResult::NoMatch
                                                        } // cannot happen
                                                        ParseResult::Ok(res, final_state) => {
                                                            let new_seq: Seq<Value> =
                                                                Seq::<Value>::new().append(val);
                                                            let rest_vals: Seq<Value> =
                                                                new_seq.concat(res);
                                                            ParseResult::Ok(rest_vals, final_state)
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                if *is_array_close(next_c) {
                                    return ParseResult::Ok(
                                        Seq::<Value>::new().append(val),
                                        advance(after_ws2),
                                    );
                                } else {
                                    // println!(
                                    //     "expect array-close or separator after array value"
                                    // );
                                    return ParseResult::Err(Path::named(String::from(
                                        "array_value_invalid_separator",
                                    ))); // expect array-close or separator
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// ws-comment-newline = *( wschar / [ comment ] newline )
#[smt_fn]
pub(crate) fn parse_ws_comment_newline(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::Ok(String::from(""), input),
        Optional::Some(_first_char) => match parse_wschar(input) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => match parse_comment(input) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => match parse_newline(input) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => ParseResult::Ok(String::from(""), input),
                    ParseResult::Ok(newline, after_newline) => {
                        match parse_ws_comment_newline(after_newline) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch,
                            ParseResult::Ok(rest_ws, final_state) => {
                                let result = newline.concat(rest_ws);
                                ParseResult::Ok(result, final_state)
                            }
                        }
                    }
                },
                ParseResult::Ok(comment, after_comment) => match parse_newline(after_comment) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // println!("expected newline after comment in ws-comment-newline");
                        ParseResult::Err(Path::named(String::from("array_comment_missing_newline")))
                    } // need newline after comment
                    ParseResult::Ok(newline, after_newline) => {
                        match parse_ws_comment_newline(after_newline) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch,
                            ParseResult::Ok(rest_ws, final_state) => {
                                let result = comment.concat(newline).concat(rest_ws);
                                ParseResult::Ok(result, final_state)
                            }
                        }
                    }
                },
            },
            ParseResult::Ok(_wschar, after_wschar) => {
                match parse_ws_comment_newline(after_wschar) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(rest_ws, final_state) => {
                        let result = _wschar.concat(rest_ws);
                        ParseResult::Ok(result, final_state)
                    }
                }
            }
        },
    }
}

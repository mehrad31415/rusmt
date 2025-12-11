//! Array parsing functions.

use crate::toml::{
    Optional, ParseResult, ParserContext, State, advance, ast::Value, current_char,
    key_value::parse_value, parse_comment, parse_newline, parse_wschar,
};
use rusmart_smt_stdlib::{Boolean, Error, Seq, String, smt::SMT};

/// array = array-open [ array-values ] ws-comment-newline array-close
pub(crate) fn parse_array(key: Seq<String>, input: State) -> ParseResult<Seq<Value>> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(_c) => {
            if *is_array_open(_c) {
                let after_open = advance(input);
                match current_char(after_open) {
                    Optional::None => {
                        // println!("expect array-close after array-open found nothing");
                        ParseResult::Err(Error::fresh())
                    } // expect array-close
                    Optional::Some(_next_c) => {
                        match parse_ws_comment_newline(after_open) {
                            ParseResult::Err(e) => ParseResult::Err(e),
                            ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                            ParseResult::Ok(_ws, after_ws) => {
                                match current_char(after_ws) {
                                    Optional::None => {
                                        // println!("expect array-close after array-open found nothing");
                                        ParseResult::Err(Error::fresh())
                                    }
                                    Optional::Some(x) => {
                                        if *is_array_close(x) {
                                            let after_close = advance(after_ws);
                                            // check if the array is in the array of tables and throw error if so
                                            let State {
                                                stream,
                                                cursor,
                                                context,
                                            } = after_close;
                                            let ParserContext {
                                                current_table_path,
                                                explicit_tables,
                                                closed_tables,
                                                inline_tables,
                                                inline_arrays,
                                                array_of_tables,
                                            } = context;
                                            if *array_of_tables.contains(key) {
                                                // println!("arrays of tables cannot contain inline arrays");
                                                return ParseResult::Err(Error::fresh()); // arrays of tables cannot contain inline arrays
                                            } else {
                                                // update the inline_arrays in the context
                                                let new_inline_arrays = inline_arrays.append(key);
                                                let new_context = ParserContext {
                                                    current_table_path,
                                                    explicit_tables,
                                                    closed_tables,
                                                    inline_tables,
                                                    inline_arrays: new_inline_arrays,
                                                    array_of_tables,
                                                };
                                                let new_state = State {
                                                    stream,
                                                    cursor,
                                                    context: new_context,
                                                };
                                                return ParseResult::Ok(Seq::new(), new_state);
                                            }
                                        } else {
                                            parse_array_values(key, after_ws)
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
fn is_array_open(c: String) -> Boolean {
    c.eq("[".into())
}

/// array-close = %x5D ; ]
fn is_array_close(c: String) -> Boolean {
    c.eq("]".into())
}

/// array-sep = %x2C  ; , Comma
fn is_array_sep(c: String) -> Boolean {
    c.eq(",".into())
}

/// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
/// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
fn parse_array_values(key: Seq<String>, input: State) -> ParseResult<Seq<Value>> {
    match parse_value(key, input) {
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::NoMatch => {
            // println!("expected a value but none matched");
            ParseResult::Err(Error::fresh())
        } // expected a value but none matched
        ParseResult::Ok(val, after_val) => {
            match parse_ws_comment_newline(after_val) {
                ParseResult::Err(e) => ParseResult::Err(e),
                ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                ParseResult::Ok(_ws2, after_ws2) => {
                    match current_char(after_ws2) {
                        Optional::None => {
                            // println!(
                            //     "expect array-close or separator after array value found nothing"
                            // );
                            ParseResult::Err(Error::fresh()) // expect array-close or separator
                        }
                        Optional::Some(next_c) => {
                            if *is_array_sep(next_c) {
                                let after_sep = advance(after_ws2);
                                match parse_ws_comment_newline(after_sep) {
                                    ParseResult::Err(e) => ParseResult::Err(e),
                                    ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                                    ParseResult::Ok(_ws3, after_ws3) => {
                                        match current_char(after_ws3) {
                                            Optional::None => {
                                                // println!(
                                                //     "expect array value after separator found nothing"
                                                // );
                                                ParseResult::Err(Error::fresh())
                                            } // expect array value
                                            Optional::Some(newnew) => {
                                                if *is_array_close(newnew) {
                                                    // trailing comma case is okay
                                                    return ParseResult::Ok(Seq::new().append(val), advance(after_ws3));
                                                } else {
                                                    match parse_array_values(key, after_ws3) {
                                                        ParseResult::Err(e) => ParseResult::Err(e),
                                                        ParseResult::NoMatch => {
                                                            ParseResult::NoMatch
                                                        } // cannot happen
                                                        ParseResult::Ok(res, final_state) => {
                                                            let new_seq = Seq::new().append(val);
                                                            let rest_vals = new_seq.concat(res);
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
                                    return ParseResult::Ok(Seq::new().append(val), advance(after_ws2));
                                } else {
                                    // println!(
                                    //     "expect array-close or separator after array value"
                                    // );
                                    return ParseResult::Err(Error::fresh()); // expect array-close or separator
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
fn parse_ws_comment_newline(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::Ok(String::from(""), input),
        Optional::Some(_first_char) => match parse_wschar(input) {
            ParseResult::Err(e) => ParseResult::Err(e),
            ParseResult::NoMatch => match parse_comment(input) {
                ParseResult::Err(e) => ParseResult::Err(e),
                ParseResult::NoMatch => match parse_newline(input) {
                    ParseResult::Err(e) => ParseResult::Err(e),
                    ParseResult::NoMatch => ParseResult::Ok(String::from(""), input),
                    ParseResult::Ok(newline, after_newline) => {
                        match parse_ws_comment_newline(after_newline) {
                            ParseResult::Err(e) => ParseResult::Err(e),
                            ParseResult::NoMatch => ParseResult::NoMatch,
                            ParseResult::Ok(rest_ws, final_state) => {
                                let result = newline.concat(rest_ws);
                                ParseResult::Ok(result, final_state)
                            }
                        }
                    }
                },
                ParseResult::Ok(comment, after_comment) => match parse_newline(after_comment) {
                    ParseResult::Err(e) => ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // println!("expected newline after comment in ws-comment-newline");
                        ParseResult::Err(Error::fresh())
                    } // need newline after comment
                    ParseResult::Ok(newline, after_newline) => {
                        match parse_ws_comment_newline(after_newline) {
                            ParseResult::Err(e) => ParseResult::Err(e),
                            ParseResult::NoMatch => ParseResult::NoMatch,
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
                    ParseResult::Err(e) => ParseResult::Err(e),
                    ParseResult::NoMatch => ParseResult::NoMatch,
                    ParseResult::Ok(rest_ws, final_state) => {
                        let result = _wschar.concat(rest_ws);
                        ParseResult::Ok(result, final_state)
                    }
                }
            }
        },
    }
}

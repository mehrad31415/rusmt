//! TOML string parsing functions.
//!
//! ;; String
//! string = ml-basic-string / basic-string / ml-literal-string / literal-string

use crate::toml::{
    Optional, ParseResult, State, advance, current_char, integer::is_hex_digit, is_apostrophe,
    is_exclamation, is_htab, is_newline, is_non_ascii, is_quotation_mark, is_wschar, parse_newline,
    parse_ws, parse_wschar,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{
    Boolean, Error, Integer, String, U32,
    bitvector::BitvectorOps,
    smt::SMT,
};

/// `string = ml-basic-string / basic-string / ml-literal-string / literal-string`
#[smt_fn]
pub(crate) fn parse_string(state: State) -> ParseResult<String> {
    match parse_ml_basic_string(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
        ParseResult::NoMatch => match parse_ml_literal_string(state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
            ParseResult::NoMatch => match parse_basic_string(state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
                ParseResult::NoMatch => {
                    return parse_literal_string(state);
                }
            },
        },
    }
}

/// ml-basic-string = ml-basic-string-delim [ newline ] ml-basic-body ml-basic-string-delim
#[smt_fn]
fn parse_ml_basic_string(state: State) -> ParseResult<String> {
    if *is_ml_basic_string_delim(state) {
        let after_opening = advance(advance(advance(state))); // Advance past the 3 quotation marks
        match current_char(after_opening) {
            Optional::Some(_c) => {
                match parse_newline(after_opening) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // no newline after opening delimiter
                        match parse_ml_body(after_opening) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch, // not happening
                            ParseResult::Ok(s, new_state) => {
                                if *is_ml_basic_string_delim(new_state) {
                                    return ParseResult::Ok(s, advance(advance(advance(new_state))));
                                } else {
                                    return {
                                        // println!(
                                        //    "missing closing delimiter for multi-line basic string"
                                        // );
                                        ParseResult::Err(Error::fresh())
                                    } // missing closing delimiter
                                }
                            }
                        }
                    }
                    ParseResult::Ok(_newline_str, state_after_newline) => {
                        // newline after opening delimiter
                        match parse_ml_body(state_after_newline) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch, // not happening
                            ParseResult::Ok(s, new_state) => {
                                if *is_ml_basic_string_delim(new_state) {
                                    return ParseResult::Ok(s, advance(advance(advance(new_state))));
                                } else {
                                    // println!(
                                    //    "missing closing delimiter for multi-line basic string where newline was present"
                                    // );
                                    return ParseResult::Err(Error::fresh()); // missing closing delimiter
                                }
                            }
                        }
                    }
                }
            }
            Optional::None => {
                // println!("No input after opening delimiter for multi-line basic string");
                return ParseResult::Err(Error::fresh());
            } // No input after opening delimiter
        }
    } else {
        return ParseResult::NoMatch;
    }
}

/// ml-basic-string-delim = 3quotation-mark
#[smt_fn]
fn is_ml_basic_string_delim(state: State) -> Boolean {
    match current_char(state) {
        Optional::Some(c1) => {
            if *is_quotation_mark(c1) {
                let second_state = advance(state);
                match current_char(second_state) {
                    Optional::Some(c2) => {
                        if *is_quotation_mark(c2) {
                            let third_state = advance(second_state);
                            match current_char(third_state) {
                                Optional::Some(c3) => {
                                    if *is_quotation_mark(c3) {
                                        return Boolean::from(true);
                                    } else {
                                        return Boolean::from(false);
                                    }
                                }
                                Optional::None => return Boolean::from(false),
                            }
                        } else {
                            return Boolean::from(false);
                        }
                    }
                    Optional::None => return Boolean::from(false),
                }
            } else {
                return Boolean::from(false);
            }
        }
        Optional::None => return Boolean::from(false),
    }
}

/// ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
#[smt_fn]
fn parse_ml_body(state: State) -> ParseResult<String> {
    match parse_mlb_content(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            match parse_mlb_quote_content(state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::Ok(String::from(""), state), // empty ml-basic-body
                ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
            }
        }
        ParseResult::Ok(s1, new_state) => match parse_mlb_quote_content(new_state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => return ParseResult::Ok(s1, new_state), // only mlb-content
            ParseResult::Ok(s2, final_state) => {
                let combined = s1.concat(s2);
                return ParseResult::Ok(combined, final_state);
            }
        },
    }
}

/// *mlb-content
/// mlb-content = mlb-char / newline / mlb-escaped-nl
#[smt_fn]
fn parse_mlb_content(state: State) -> ParseResult<String> {
    match parse_mlb_char(state) {
        ParseResult::Ok(s, new_state) => {
            // recurse
            let rest_parse = parse_mlb_content(new_state);
            match rest_parse {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    return ParseResult::Ok(s, new_state);
                }
                ParseResult::Ok(s2, final_state) => {
                    let combined = s.concat(s2);
                    return ParseResult::Ok(combined, final_state);
                }
            }
        }
        ParseResult::NoMatch => {
            match parse_newline(state) {
                ParseResult::Ok(s, new_state) => {
                    // recurse
                    let rest_parse = parse_mlb_content(new_state);
                    match rest_parse {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            return ParseResult::Ok(s, new_state);
                        }
                        ParseResult::Ok(s2, final_state) => {
                            let combined = s.concat(s2);
                            return ParseResult::Ok(combined, final_state);
                        }
                    }
                }
                ParseResult::NoMatch => {
                    match parse_mlb_escaped_nl(state) {
                        ParseResult::Ok(s, new_state) => {
                            // recurse
                            let rest_parse = parse_mlb_content(new_state);
                            match rest_parse {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    return ParseResult::Ok(s, new_state);
                                }
                                ParseResult::Ok(s2, final_state) => {
                                    let combined = s.concat(s2);
                                    return ParseResult::Ok(combined, final_state);
                                }
                            }
                        }
                        ParseResult::NoMatch => {
                            return ParseResult::NoMatch;
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
    }
}

/// mlb-char = mlb-unescaped / escaped
#[smt_fn]
fn parse_mlb_char(state: State) -> ParseResult<String> {
    match parse_mlb_unescaped(state) {
        ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return parse_escaped(state, Integer::from(0)),
    }
}

/// mlb-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
#[smt_fn]
fn parse_mlb_unescaped(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            // Check for %x21
            if *is_exclamation(c) {
                return ParseResult::Ok(c, advance(state));
            } else {
                // Check for %x23-5B
                if *is_range_23_5b(c) {
                    return ParseResult::Ok(c, advance(state));
                } else {
                    // Check for %x5D-7E
                    if *is_range_5d_7e(c) {
                        return ParseResult::Ok(c, advance(state));
                    } else {
                        // Check for non-ascii
                        if *is_non_ascii(c) {
                            return ParseResult::Ok(c, advance(state));
                        } else {
                            // Check for wschar
                            if *is_wschar(c) {
                                return ParseResult::Ok(c, advance(state));
                            } else {
                                return ParseResult::NoMatch;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// escaped = escape escape-seq-char
#[smt_fn]
fn parse_escaped(state: State, val: Integer) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_escape_char(c) {
                // escape-seq-char
                match parse_escape_seq_char(advance(state), val) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(s, new_state) => {
                        return ParseResult::Ok(s, new_state);
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// escape = %x5C ; \
#[smt_fn]
fn is_escape_char(c: String) -> Boolean {
    c.eq(String::from("\\"))
}

/// Helper to parse exactly `count` hex digits and return them as a String
#[smt_fn]
fn parse_n_hex_digits(state: State, count: Integer, acc: String) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_hex_digit(c) {
                let new_acc = acc.concat(c);
                if *count.eq(Integer::from(1)) {
                    return ParseResult::Ok(new_acc, advance(state));
                } else {
                    return parse_n_hex_digits(
                        advance(state),
                        count.sub(Integer::from(1)),
                        new_acc,
                    );
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
        Optional::None => return ParseResult::NoMatch,
    }
}

/// escape-seq-char =  %x22 / %x5C / %x62 / %x66 / %x6E / %x72 / %x74 ; (", \, b, f, n, r, t)
#[smt_fn]
fn parse_escape_seq_char(state: State, val: Integer) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from("\""))
            // %x22 - "
            {
                return ParseResult::Ok(String::from("\""), advance(state));
            } else {
                if *c.eq(String::from("\\"))
                // %x5C - \
                {
                    return ParseResult::Ok(String::from("\\"), advance(state));
                } else {
                    if *c.eq(String::from("b"))
                    // %x62 - b
                    {
                        return ParseResult::Ok(String::from("\x08"), advance(state));
                    } else {
                        if *c.eq(String::from("f"))
                        // %x66 - f
                        {
                            return ParseResult::Ok(String::from("\x0C"), advance(state));
                        } else {
                            if *c.eq(String::from("n"))
                            // %x6E - n
                            {
                                return ParseResult::Ok(String::from("\n"), advance(state));
                            } else {
                                if *c.eq(String::from("r"))
                                // %x72 - r
                                {
                                    return ParseResult::Ok(String::from("\r"), advance(state));
                                } else {
                                    if *c.eq(String::from("t"))
                                    // %x74 - t
                                    {
                                        return ParseResult::Ok(String::from("\t"), advance(state));
                                    } else {
                                        let needed = if *c.eq(String::from("u")) {
                                            Integer::from(4)
                                        } else {
                                            if *c.eq(String::from("U")) {
                                                Integer::from(8)
                                            } else {
                                                if *val.eq(Integer::from(0)) {
                                                    return ParseResult::NoMatch; // for multi-line string try other escape sequences
                                                } else {
                                                    // println!(
                                                    //    "invalid escape sequence in basic string: {:?}",
                                                    //     c
                                                    // );
                                                    return ParseResult::Err(Error::fresh()); // for basic strings, other escape sequences are invalid
                                                }
                                            }
                                        };
                                        let start_state = advance(state);
                                        match parse_n_hex_digits(
                                            start_state,
                                            needed,
                                            String::from(""),
                                        ) {
                                            ParseResult::NoMatch => {
                                                // println!(
                                                //    "invalid hex digits in unicode escape sequence"
                                                // );
                                                return ParseResult::Err(Error::fresh());
                                            } // Invalid Hex
                                            ParseResult::Err(e) => ParseResult::Err(e),
                                            ParseResult::Ok(hex_str, hex_state) => {
                                                let code_point = Integer::from_hex_str(hex_str); // mathematically cannot panic!
                                                let code_point_u32 = code_point.to_u32(); // cannot panic
                                                if *is_valid_unicode_scalar(code_point_u32) {
                                                    let char_str =
                                                        String::from_code(code_point_u32);
                                                    return ParseResult::Ok(char_str, hex_state);
                                                } else {
                                                    // println!(
                                                    //    "invalid unicode scalar value in escape sequence: {:?}",
                                                    //    code_point
                                                    // );
                                                    return ParseResult::Err(Error::fresh());
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
}

#[smt_fn]
fn is_valid_unicode_scalar(value: U32) -> Boolean {
    // Range 1: 0x0000 to 0xD7FF (Standard Unicode)
    let is_low_range = value.bv_le(U32::from(0xD7FF));

    // Range 2: 0xE000 to 0x10FFFF (Post-Surrogate to Max Unicode)
    let is_high_range = value
        .bv_ge(U32::from(0xE000))
        .and(value.bv_le(U32::from(0x10FFFF)));

    // Valid it falls into either bucket
    is_low_range.or(is_high_range)
}

/// mlb-escaped-nl = escape ws newline *( wschar / newline )
#[smt_fn]
fn parse_mlb_escaped_nl(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_escape_char(c) {
                // escape
                let after_ws_parse = parse_ws(advance(state));
                match parse_newline(after_ws_parse) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // println!(
                        //   "newline expected after escaped newline in multi-line basic string"
                        //);
                        return ParseResult::Err(Error::fresh());
                    } // newline expected
                    ParseResult::Ok(_newline_str, after_newline_state) => {
                        match parse_wschar_newline_sequence(after_newline_state) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch,
                            ParseResult::Ok(_s2, final_state) => {
                                return ParseResult::Ok(String::from(""), final_state);
                            }
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// *( wschar / newline )
#[smt_fn]
fn parse_wschar_newline_sequence(state: State) -> ParseResult<String> {
    match parse_wschar(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => match parse_newline(state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => return ParseResult::Ok(String::from(""), state),
            ParseResult::Ok(_s, new_state) => {
                // Recurse to get the rest of the sequence.
                match parse_wschar_newline_sequence(new_state) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(_s2, final_state) => {
                        return ParseResult::Ok(_s.concat(_s2), final_state);
                    }
                }
            }
        },
        ParseResult::Ok(_s1, new_state) => {
            // Recurse to get the rest of the sequence.
            match parse_wschar_newline_sequence(new_state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
                ParseResult::Ok(_s2, final_state) => {
                    return ParseResult::Ok(_s1.concat(_s2), final_state);
                }
            }
        }
    }
}

/// *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
#[smt_fn]
fn parse_mlb_quote_content(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(_c) => {
            match parse_mlb_quotes(state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch, // no mlb-quotes found
                ParseResult::Ok(quote_str, after_quotes_state) => {
                    match parse_mlb_content(after_quotes_state) {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            if *is_ml_basic_string_delim(after_quotes_state) {
                                return ParseResult::Ok(quote_str, after_quotes_state);
                            } else {
                                // println!(
                                //    "at least one mlb-content is required after mlb-quotes in multi-line basic string"
                                // );
                                return ParseResult::Err(Error::fresh());
                            }
                        } // at least one mlb-content is required
                        ParseResult::Ok(content_str, final_state) => {
                            let combined = quote_str.concat(content_str);
                            // Recurse to get more quote-content pairs
                            match parse_mlb_quote_content(final_state) {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    return ParseResult::Ok(combined, final_state);
                                }
                                ParseResult::Ok(rest_str, end_state) => {
                                    let total_combined = combined.concat(rest_str);
                                    return ParseResult::Ok(total_combined, end_state);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// mlb-quotes = 1*2quotation-mark
#[smt_fn]
fn parse_mlb_quotes(state: State) -> ParseResult<String> {
    let first_char = current_char(state);
    match first_char {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *is_quotation_mark(c1) {
                let second_state = advance(state);
                match current_char(second_state) {
                    Optional::None => {
                        // Only one quotation mark found
                        return ParseResult::Ok(String::from("\""), second_state);
                    }
                    Optional::Some(c2) => {
                        if *is_quotation_mark(c2) {
                            let third_state = advance(second_state);
                            match current_char(third_state) {
                                Optional::None => {
                                    // Two quotation marks found
                                    return ParseResult::Ok(String::from("\"\""), third_state);
                                }
                                Optional::Some(c3) => {
                                    if *is_quotation_mark(c3) {
                                        // we need to check the fourth character to avoid consuming 3 quotation marks here
                                        let fourth_state = advance(third_state);
                                        match current_char(fourth_state) {
                                            Optional::None => {
                                                // Three quotation marks found
                                                return ParseResult::NoMatch;
                                            }
                                            Optional::Some(c4) => {
                                                if *is_quotation_mark(c4) {
                                                    // check for the fifth character
                                                    let fifth_state = advance(fourth_state);
                                                    match current_char(fifth_state) {
                                                        Optional::None => {
                                                            // Four quotation marks found, consume only the first one
                                                            return ParseResult::Ok(
                                                                String::from("\""),
                                                                second_state,
                                                            );
                                                        }
                                                        Optional::Some(_c5) => {
                                                            if *is_quotation_mark(_c5) {
                                                                // Five quotation marks found, consume only the first two
                                                                return ParseResult::Ok(
                                                                    String::from("\"\""),
                                                                    third_state,
                                                                );
                                                            } else {
                                                                // only four quotation marks found, consume the first one
                                                                return ParseResult::Ok(
                                                                    String::from("\""),
                                                                    second_state,
                                                                );
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    // Only three quotation marks found
                                                    return ParseResult::NoMatch;
                                                }
                                            }
                                        }
                                    } else {
                                        // Only two quotation marks found
                                        return ParseResult::Ok(String::from("\"\""), third_state);
                                    }
                                }
                            }
                        } else {
                            // Only one quotation mark found
                            return ParseResult::Ok(String::from("\""), second_state);
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// ml-literal-string = ml-literal-string-delim [ newline ] ml-literal-body ml-literal-string-delim
#[smt_fn]
fn parse_ml_literal_string(state: State) -> ParseResult<String> {
    if *is_ml_literal_string_delim(state) {
        let after_opening = advance(advance(advance(state))); // Advance past the 3 apostrophes
        match current_char(after_opening) {
            Optional::Some(_c) => {
                match parse_newline(after_opening) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // no newline after opening delimiter
                        match parse_ml_literal_body(after_opening) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch, // not happening
                            ParseResult::Ok(s, new_state) => {
                                if *is_ml_literal_string_delim(new_state) {
                                    return ParseResult::Ok(s, advance(advance(advance(new_state))));
                                } else {
                                    // println!(
                                    //     "missing closing delimiter for multi-line literal string"
                                    // );
                                    return ParseResult::Err(Error::fresh()); // missing closing delimiter
                                }
                            }
                        }
                    }
                    ParseResult::Ok(_newline_str, state_after_newline) => {
                        // newline after opening delimiter
                        match parse_ml_literal_body(state_after_newline) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => return ParseResult::NoMatch,
                            ParseResult::Ok(s, new_state) => {
                                if *is_ml_literal_string_delim(new_state) {
                                    return ParseResult::Ok(s, advance(advance(advance(new_state))));
                                } else {
                                    // println!(
                                    //     "missing closing delimiter for multi-line literal string where newline was present"
                                    // );
                                    return ParseResult::Err(Error::fresh()); // missing closing delimiter
                                }
                            }
                        }
                    }
                }
            }
            Optional::None => {
                // println!("no input after opening delimiter for multi-line literal string");
                return ParseResult::Err(Error::fresh());
            } // No input after opening delimiter
        }
    } else {
        return ParseResult::NoMatch;
    }
}

/// ml-literal-string-delim = 3apostrophe
#[smt_fn]
fn is_ml_literal_string_delim(state: State) -> Boolean {
    match current_char(state) {
        Optional::Some(c1) => {
            if *is_apostrophe(c1) {
                let second_state = advance(state);
                match current_char(second_state) {
                    Optional::Some(c2) => {
                        if *is_apostrophe(c2) {
                            let third_state = advance(second_state);
                            match current_char(third_state) {
                                Optional::Some(c3) => {
                                    if *is_apostrophe(c3) {
                                        return Boolean::from(true);
                                    } else {
                                        return Boolean::from(false);
                                    }
                                }
                                Optional::None => return Boolean::from(false),
                            }
                        } else {
                            return Boolean::from(false);
                        }
                    }
                    Optional::None => return Boolean::from(false),
                }
            } else {
                return Boolean::from(false);
            }
        }
        Optional::None => return Boolean::from(false),
    }
}

/// ml-literal-body = *mll-content *( mll-quotes 1*mll-content ) [ mll-quotes ]
#[smt_fn]
fn parse_ml_literal_body(state: State) -> ParseResult<String> {
    match parse_ml_literal_content(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => match parse_ml_literal_quote_content(state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => return ParseResult::Ok(String::from(""), state), // empty ml-literal-body
            ParseResult::Ok(s2, final_state) => return ParseResult::Ok(s2, final_state),
        },
        ParseResult::Ok(s1, new_state) => match parse_ml_literal_quote_content(new_state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => return ParseResult::Ok(s1, new_state),
            ParseResult::Ok(s2, final_state) => {
                let combined = s1.concat(s2);
                return ParseResult::Ok(combined, final_state);
            }
        },
    }
}

/// *mll-content
/// mll-content = mll-char / newline
#[smt_fn]
fn parse_ml_literal_content(state: State) -> ParseResult<String> {
    match parse_ml_literal_char(state) {
        ParseResult::Ok(s, new_state) => {
            // recurse
            let rest_parse = parse_ml_literal_content(new_state);
            match rest_parse {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    return ParseResult::Ok(s, new_state);
                }
                ParseResult::Ok(s2, final_state) => {
                    let combined = s.concat(s2);
                    return ParseResult::Ok(combined, final_state);
                }
            }
        }
        ParseResult::NoMatch => {
            match parse_newline(state) {
                ParseResult::Ok(s, new_state) => {
                    // recurse
                    let rest_parse = parse_ml_literal_content(new_state);
                    match rest_parse {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => return ParseResult::Ok(s, new_state),
                        ParseResult::Ok(s2, final_state) => {
                            let combined = s.concat(s2);
                            return ParseResult::Ok(combined, final_state);
                        }
                    }
                }
                ParseResult::NoMatch => {
                    return ParseResult::NoMatch;
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
    }
}

/// mll-char = %x09 / %x20-26 / %x28-7E / non-ascii
#[smt_fn]
fn parse_ml_literal_char(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_htab(c) {
                return ParseResult::Ok(c, advance(state));
            } else {
                if *is_range_20_26(c) {
                    return ParseResult::Ok(c, advance(state));
                } else {
                    if *is_range_28_7e(c) {
                        return ParseResult::Ok(c, advance(state));
                    } else {
                        if *is_non_ascii(c) {
                            return ParseResult::Ok(c, advance(state));
                        } else {
                            return ParseResult::NoMatch;
                        }
                    }
                }
            }
        }
    }
}

/// *( mll-quotes 1*mll-content ) [ mll-quotes ]
#[smt_fn]
fn parse_ml_literal_quote_content(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(_c) => {
            match parse_ml_literal_quotes(state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch, // no mll-quotes found
                ParseResult::Ok(quote_str, after_quotes_state) => {
                    match parse_ml_literal_content(after_quotes_state) {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            if *is_ml_literal_string_delim(after_quotes_state) {
                                return ParseResult::Ok(quote_str, after_quotes_state);
                            } else {
                                // println!(
                                //    "at least one mll-content is required after mll-quotes in multi-line literal string"
                                // );
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                        ParseResult::Ok(content_str, final_state) => {
                            let combined = quote_str.concat(content_str);
                            // Recurse to get more quote-content pairs
                            match parse_ml_literal_quote_content(final_state) {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    return ParseResult::Ok(combined, final_state);
                                }
                                ParseResult::Ok(rest_str, end_state) => {
                                    let total_combined = combined.concat(rest_str);
                                    return ParseResult::Ok(total_combined, end_state);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// mll-quotes = 1*2apostrophe
#[smt_fn]
fn parse_ml_literal_quotes(state: State) -> ParseResult<String> {
    let first_char = current_char(state);
    match first_char {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *is_apostrophe(c1) {
                let second_state = advance(state);
                match current_char(second_state) {
                    Optional::None => {
                        // Only one apostrophe found
                        return ParseResult::Ok(String::from("'"), second_state);
                    }
                    Optional::Some(c2) => {
                        if *is_apostrophe(c2) {
                            let third_state = advance(second_state);
                            match current_char(third_state) {
                                Optional::None => {
                                    // Two apostrophes found
                                    return ParseResult::Ok(String::from("''"), third_state);
                                }
                                Optional::Some(c3) => {
                                    if *is_apostrophe(c3) {
                                        // we need to check the fourth character to avoid consuming 3 apostrophes here
                                        let fourth_state = advance(third_state);
                                        match current_char(fourth_state) {
                                            Optional::None => {
                                                // Three apostrophes found
                                                return ParseResult::NoMatch;
                                            }
                                            Optional::Some(c4) => {
                                                if *is_apostrophe(c4) {
                                                    // check for the fifth character
                                                    let fifth_state = advance(fourth_state);
                                                    match current_char(fifth_state) {
                                                        Optional::None => {
                                                            // Four apostrophes found, consume only the first one
                                                            return ParseResult::Ok(
                                                                String::from("'"),
                                                                second_state,
                                                            );
                                                        }
                                                        Optional::Some(_c5) => {
                                                            if *is_apostrophe(_c5) {
                                                                // Five apostrophes found, consume only the first two
                                                                return ParseResult::Ok(
                                                                    String::from("''"),
                                                                    third_state,
                                                                );
                                                            } else {
                                                                // only four apostrophes found, consume the first one
                                                                return ParseResult::Ok(
                                                                    String::from("'"),
                                                                    second_state,
                                                                );
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    // Only three apostrophes found
                                                    return ParseResult::NoMatch;
                                                }
                                            }
                                        }
                                    } else {
                                        // Only two apostrophes found
                                        return ParseResult::Ok(String::from("''"), third_state);
                                    }
                                }
                            }
                        } else {
                            // Only one apostrophe found
                            return ParseResult::Ok(String::from("'"), second_state);
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// `basic-string = quotation-mark *basic-char quotation-mark``
#[smt_fn]
pub(crate) fn parse_basic_string(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_quotation_mark(c) {
                let after_quote_state = advance(state);
                match parse_basic_string_content(after_quote_state, String::from("")) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(content_str, state_after_content) => {
                        match current_char(state_after_content) {
                            Optional::None => {
                                // println!("missing closing quotation mark for basic string");
                                return ParseResult::Err(Error::fresh());
                            } // missing closing quotation mark
                            Optional::Some(closing_char) => {
                                if *is_quotation_mark(closing_char) {
                                    return ParseResult::Ok(content_str, advance(state_after_content));
                                } else {
                                    // println!("missing closing quotation mark for basic string");
                                    return ParseResult::Err(Error::fresh());
                                } // missing closing quotation mark
                            }
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// The content inside a basic string, after the opening quotation mark.
/// *basic-char
#[smt_fn]
fn parse_basic_string_content(state: State, acc: String) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => ParseResult::Ok(acc, state), // empty string content
        Optional::Some(_c) => {
            if *is_newline(state) {
                {
                    // println!("newlines are not allowed in basic strings");
                    return ParseResult::Err(Error::fresh());
                } // newlines are not allowed in basic strings
            } else {
                if *is_quotation_mark(_c) {
                    return ParseResult::Ok(acc, state); // end of string content
                } else {
                    match parse_basic_char(state) {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            // println!("invalid character in basic string: {:?}", _c);
                            return ParseResult::Err(Error::fresh());
                        } // invalid character
                        ParseResult::Ok(s1, new_state) => {
                            // Recurse to get the rest of the string content.
                            let new_acc = acc.concat(s1);
                            match parse_basic_string_content(new_state, new_acc) {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => return ParseResult::NoMatch,
                                ParseResult::Ok(_s2, final_state) => {
                                    return ParseResult::Ok(_s2, final_state);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// basic-char = basic-unescaped / escaped
#[smt_fn]
fn parse_basic_char(state: State) -> ParseResult<String> {
    match parse_basic_unescaped(state) {
        ParseResult::Ok(s, new_state) => return ParseResult::Ok(s, new_state),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => parse_escaped(state, Integer::from(1)), // pass 1 to indicate basic string
    }
}

/// basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
#[smt_fn]
fn parse_basic_unescaped(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            // Check for %x21
            if *is_exclamation(c) {
                return ParseResult::Ok(c, advance(state));
            } else {
                // Check for %x23-5B
                if *is_range_23_5b(c) {
                    return ParseResult::Ok(c, advance(state));
                } else {
                    // Check for %x5D-7E
                    if *is_range_5d_7e(c) {
                        return ParseResult::Ok(c, advance(state));
                    } else {
                        if *is_non_ascii(c) {
                            return ParseResult::Ok(c, advance(state));
                        } else {
                            // Check for wschar
                            if *is_wschar(c) {
                                return ParseResult::Ok(c, advance(state));
                            } else {
                                return ParseResult::NoMatch;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// %x23-5B range: # through [
#[smt_fn]
fn is_range_23_5b(c: String) -> Boolean {
    c.ge(String::from("#")).and(c.le(String::from("[")))
}

/// %x5D-7E range: ] through ~
#[smt_fn]
fn is_range_5d_7e(c: String) -> Boolean {
    c.ge(String::from("]")).and(c.le(String::from("~")))
}

/// literal-string = apostrophe *literal-char apostrophe
#[smt_fn]
pub(crate) fn parse_literal_string(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_apostrophe(c) {
                let after_apostrophe_state = advance(state);
                match parse_literal_string_content(after_apostrophe_state, String::from("")) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(content_str, state_after_content) => {
                        match current_char(state_after_content) {
                            Optional::None => {
                                // println!("missing closing apostrophe for literal string");
                                return ParseResult::Err(Error::fresh());
                            }
                            Optional::Some(closing_char) => {
                                if *is_apostrophe(closing_char) {
                                    return ParseResult::Ok(content_str, advance(state_after_content));
                                } else {
                                    // println!("missing closing apostrophe for literal string");
                                    return ParseResult::Err(Error::fresh());
                                } // missing closing apostrophe
                            }
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// *literal-char
#[smt_fn]
fn parse_literal_string_content(state: State, acc: String) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => ParseResult::Ok(acc, state), // empty string content
        Optional::Some(_c) => {
            if *is_newline(state) {
                {
                    // println!("newlines are not allowed in literal strings");
                    return ParseResult::Err(Error::fresh());
                } // newlines are not allowed in literal strings
            } else {
                if *is_apostrophe(_c) {
                    return ParseResult::Ok(acc, state); // end of string content
                } else {
                    match parse_literal_char(state) {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            // println!("invalid character in literal string: {:?}", _c);
                            return ParseResult::Err(Error::fresh());
                        } // invalid character
                        ParseResult::Ok(s1, new_state) => {
                            // Recurse to get the rest of the string content.
                            match parse_literal_string_content(new_state, acc.concat(s1)) {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => return ParseResult::NoMatch,
                                ParseResult::Ok(s2, final_state) => {
                                    return ParseResult::Ok(s2, final_state);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// literal-char = %x09 / %x20-26 / %x28-7E / non-ascii
#[smt_fn]
fn parse_literal_char(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_htab(c) {
                return ParseResult::Ok(c, advance(state));
            } else {
                if *is_range_20_26(c) {
                    return ParseResult::Ok(c, advance(state));
                } else {
                    if *is_range_28_7e(c) {
                        return ParseResult::Ok(c, advance(state));
                    } else {
                        if *is_non_ascii(c) {
                            return ParseResult::Ok(c, advance(state));
                        } else {
                            return ParseResult::NoMatch;
                        }
                    }
                }
            }
        }
    }
}

/// %x20-26 range: space through &
#[smt_fn]
fn is_range_20_26(c: String) -> Boolean {
    c.ge(String::from(" ")).and(c.le(String::from("&")))
}

/// %x28-7E range: ( through ~
#[smt_fn]
fn is_range_28_7e(c: String) -> Boolean {
    c.ge(String::from("(")).and(c.le(String::from("~")))
}

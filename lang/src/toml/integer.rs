//! Parses integer literals in TOML.

use crate::toml::{
    Optional, ParseResult, State, advance, current_char, is_alpha, is_dec_digit, peek,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Error, I64, Integer, String, boolean::FALSE, smt::SMT};

/// `hex prefix = "0x"`
#[smt_fn]
fn is_hex_prefix(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("x".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// fake hex prefix: `0X`
#[smt_fn]
fn is_hex_prefix_upper(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("X".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// `oct prefix = "0o"`
#[smt_fn]
fn is_oct_prefix(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("o".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// fake oct prefix: `0O`
#[smt_fn]
fn is_oct_prefix_upper(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("O".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// `bin prefix = "0b"`
#[smt_fn]
fn is_bin_prefix(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("b".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// fake bin prefix: `0B`
#[smt_fn]
fn is_bin_prefix_upper(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c1) => {
            let peek = peek(input, 1.into());
            match peek {
                Optional::Some(c2) => c1.eq("0".into()).and(c2.eq("B".into())),
                Optional::None => FALSE,
            }
        }
        Optional::None => FALSE,
    }
}

/// Checks if a character is a hexadecimal digit (0-9, a-f, A-F).
#[smt_fn]
pub(crate) fn is_hex_digit(c: String) -> Boolean {
    is_dec_digit(c)
        .or(c.ge("a".into()).and(c.le("f".into())))
        .or(c.ge("A".into()).and(c.le("F".into())))
}

/// Checks if a character is an octal digit (0-7).
#[smt_fn]
fn is_oct_digit(c: String) -> Boolean {
    c.ge("0".into()).and(c.le("7".into()))
}

/// Checks if a character is a binary digit (0 or 1).
#[smt_fn]
fn is_bin_digit(c: String) -> Boolean {
    c.eq("0".into()).or(c.eq("1".into()))
}

/// Checks if a character is +
#[smt_fn]
pub(crate) fn is_plus(c: String) -> Boolean {
    c.eq("+".into())
}

/// Checks if a character is -
#[smt_fn]
pub(crate) fn is_minus(c: String) -> Boolean {
    c.eq("-".into())
}

/// Checks if a character is an underscore (_)
#[smt_fn]
pub(crate) fn is_underscore(c: String) -> Boolean {
    c.eq("_".into())
}

/// The main dispatcher for parsing any integer type.
/// I64 Parsing (ABNF: `integer = dec-int / hex-int / oct-int / bin-int`)
#[smt_fn]
pub(crate) fn parse_integer(input: State) -> ParseResult<I64> {
    match current_char(input) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *is_plus(c).or(is_minus(c)) {
                let new_state = advance(input);
                if *is_bin_prefix(new_state) {
                    // println!("bin prefix");
                    return ParseResult::Err(Error::fresh());
                } else {
                    if *is_oct_prefix(new_state) {
                        // println!("oct prefix");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_hex_prefix(new_state) {
                            // println!("hex prefix");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return parse_dec_int(Optional::Some(c), new_state);
                        }
                    }
                }
            } else {
                if *is_bin_prefix(input) {
                    return parse_bin_int(input);
                } else {
                    if *is_oct_prefix(input) {
                        return parse_oct_int(input);
                    } else {
                        if *is_hex_prefix(input) {
                            return parse_hex_int(input);
                        } else {
                            if *is_bin_prefix_upper(input) {
                                // println!("0B is invalid");
                                return ParseResult::Err(Error::fresh());
                            } else {
                                if *is_oct_prefix_upper(input) {
                                    // println!("0O is invalid");
                                    return ParseResult::Err(Error::fresh());
                                } else {
                                    if *is_hex_prefix_upper(input) {
                                        // println!("0X is invalid");
                                        return ParseResult::Err(Error::fresh());
                                    } else {
                                        return parse_dec_int(Optional::None, input);
                                    }
                                }
                            }
                        }
                    }
                }
            };
        }
    }
}

/// `dec-int = [ minus / plus ] unsigned-dec-int`
#[smt_fn]
pub(crate) fn parse_dec_int(sign: Optional<String>, input: State) -> ParseResult<I64> {
    match sign {
        Optional::None => {
            match parse_unsigned_dec_int(input) {
                ParseResult::Ok(integer, remaining_input) => {
                    // check for overflow?
                    if *integer.is_gt_i64_max() {
                        // println!("overflow in dec int parsing absence of sign and above i64 max");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        return ParseResult::Ok(integer.to_i64(), remaining_input);
                    }
                }
                ParseResult::NoMatch => ParseResult::NoMatch,
                // Propagate any other errors found.
                ParseResult::Err(e) => ParseResult::Err(e),
            }
        }
        Optional::Some(c) => {
            if *is_plus(c) {
                match parse_unsigned_dec_int(input) {
                    ParseResult::Ok(integer, remaining_input) => {
                        // check for overflow?
                        if *integer.is_gt_i64_max() {
                            // println!("overflow in dec int parsing presence of plus sign and above i64 max");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(integer.to_i64(), remaining_input);
                        }
                    }
                    ParseResult::NoMatch => {
                        match current_char(input) {
                            Optional::None => {
                                // println!("sign without digits and no more input");
                                return ParseResult::Err(Error::fresh());
                            }
                            Optional::Some(_) => {
                                // println!("sign without digits and something else found");
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                };
            } else {
                match parse_unsigned_dec_int(input) {
                    ParseResult::Ok(integer, remaining_input) => {
                        // check for overflow?
                        if *integer.neg().is_lt_i64_min() {
                            // println!("overflow in dec int parsing presence of minus sign and below i64 min");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(integer.neg().to_i64(), remaining_input);
                        }
                    }
                    ParseResult::NoMatch => {
                        match current_char(input) {
                            Optional::None => {
                                // println!("minus sign without digits and no more input");
                                return ParseResult::Err(Error::fresh());
                            }
                            Optional::Some(_) => {
                                // println!("minus sign without digits and something else found");
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                };
            }
        }
    }
}

/// `unsigned-dec-int = DIGIT / digit1-9 1*( DIGIT / underscore DIGIT )`
#[smt_fn]
fn parse_unsigned_dec_int(input: State) -> ParseResult<Integer> {
    // Get the first character.
    let first_char = match current_char(input) {
        Optional::Some(c) => c,
        Optional::None => return ParseResult::NoMatch,
    };

    // This must be a digit to match this rule at all.
    if *is_dec_digit(first_char).not() {
        return ParseResult::NoMatch;
    } else {
        // Handle single-digit '0'
        if *first_char.eq("0".into()) {
            let next_input = advance(input);
            // Check for the "leading zero" error. A '0' cannot be followed by another digit or underscore.
            match current_char(next_input) {
                Optional::None => {
                    // If it's just '0' by itself, return 0.
                    return ParseResult::Ok(Integer::from(0), next_input);
                }
                Optional::Some(next_char) => {
                    if *is_dec_digit(next_char) {
                        // println!("leading zero error due to digit after zero");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_underscore(next_char) {
                            // println!("leading zero error due to underscore after zero");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            if *is_alpha(next_char) {
                                // println!("invalid character after zero");
                                return ParseResult::Err(Error::fresh());
                            } else {
                                return ParseResult::Ok(Integer::from(0), next_input);
                            }
                        }
                    }
                }
            }
        } else {
            // Handle numbers starting with '1'-'9'
            match parse_unsigned_dec_rest_int(advance(input), first_char) {
                ParseResult::Ok(s, i) => return ParseResult::Ok(s.to_int(), i),
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
            };
        }
    }
}

/// A recursive helper to parse the rest of an unsigned decimal integer.
/// *( DIGIT / underscore DIGIT )
#[smt_fn]
fn parse_unsigned_dec_rest_int(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => {
            // Base Case: End of input reached.
            return ParseResult::Ok(acc, input);
        }
        Optional::Some(c) => {
            if *c.eq(".".into()) {
                // Decimal point indicates end of integer part.
                return ParseResult::Ok(acc, input);
            } else {
                // e or E indicates end of integer part.
                if *c.eq("e".into()).or(c.eq("E".into())) {
                    return ParseResult::Ok(acc, input);
                } else {
                    if *is_dec_digit(c) {
                        // Case 1: The next character is a digit. Append it and continue.
                        let new_acc = acc.concat(c);
                        return parse_unsigned_dec_rest_int(advance(input), new_acc);
                    } else {
                        if *c.eq("_".into()) {
                            // Case 2: The next character is an underscore.
                            let after_underscore = advance(input);
                            // An underscore MUST be followed by a digit.
                            match current_char(after_underscore) {
                                Optional::None => {
                                    // println!("underscore at end of input");
                                    return ParseResult::Err(Error::fresh());
                                }
                                Optional::Some(next_c) => {
                                    if *is_dec_digit(next_c) {
                                        // This is a valid `_DIGIT`. Append the digit and continue.
                                        let new_acc = acc.concat(next_c);
                                        return parse_unsigned_dec_rest_int(
                                            advance(after_underscore),
                                            new_acc,
                                        );
                                    } else {
                                        if *is_underscore(next_c) {
                                            // double underscore error
                                            // println!("double underscore error");
                                            return ParseResult::Err(Error::fresh());
                                        } else {
                                            if *is_hex_digit(next_c) {
                                                // println!("invalid hex character after underscore");
                                                return ParseResult::Err(Error::fresh());
                                            } else {
                                                // println!("invalid character after underscore");
                                                return ParseResult::Err(Error::fresh());
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            if *is_hex_digit(c) {
                                // println!("invalid hex character in decimal integer");
                                return ParseResult::Err(Error::fresh());
                            } else {
                                return ParseResult::Ok(acc, input);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `hex-int = hex-prefix HEXDIG *( HEXDIG / underscore HEXDIG )`
#[smt_fn]
fn parse_hex_int(input: State) -> ParseResult<I64> {
    let after_prefix = advance(advance(input));
    // The first character after the prefix MUST be a hex digit.
    match current_char(after_prefix) {
        Optional::Some(c) => {
            if *is_hex_digit(c) {
                // Recursively parse the rest of the hex digits and underscores.
                match parse_hex_rest(advance(after_prefix), c) {
                    ParseResult::Ok(s, i) => {
                        // check for overflow?
                        if *Integer::from_hex_str(s).is_gt_i64_max() {
                            // println!("overflow in hex int parsing above i64 max");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(Integer::from_hex_str(s).to_i64(), i);
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                };
            } else {
                if *is_underscore(c) {
                    // println!("underscore immediately after hex prefix");
                    return ParseResult::Err(Error::fresh());
                } else {
                    // println!("invalid character after hex prefix");
                    return ParseResult::Err(Error::fresh());
                }
            }
        }
        Optional::None => {
            // println!("no digits after hex prefix");
            return ParseResult::Err(Error::fresh());
        } // no digits after prefix
    };
}

/// A recursive helper to parse the rest of a hexadecimal integer.
#[smt_fn]
fn parse_hex_rest(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_hex_digit(c) {
                let new_acc = acc.concat(c);
                return parse_hex_rest(advance(input), new_acc);
            } else {
                if *is_underscore(c) {
                    let after_underscore = advance(input);
                    match current_char(after_underscore) {
                        Optional::Some(next_c) => {
                            if *is_hex_digit(next_c) {
                                let new_acc = acc.concat(next_c);
                                return parse_hex_rest(advance(after_underscore), new_acc);
                            } else {
                                if *is_underscore(next_c) {
                                    // println!("double underscore error in hex int");
                                    return ParseResult::Err(Error::fresh());
                                } else {
                                    // println!("invalid character after underscore in hex int");
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                        }
                        Optional::None => {
                            // println!("underscore at end of input in hex int");
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                } else {
                    return ParseResult::Ok(acc, input);
                }
            }
        }
        Optional::None => {
            // End of input reached.
            return ParseResult::Ok(acc, input);
        }
    }
}

/// `oct-int = oct-prefix digit0-7 *( digit0-7 / underscore digit0-7 )`
#[smt_fn]
fn parse_oct_int(input: State) -> ParseResult<I64> {
    let after_prefix = advance(advance(input));
    // The first character after the prefix MUST be a oct digit.
    match current_char(after_prefix) {
        Optional::Some(c) => {
            if *is_oct_digit(c) {
                // Recursively parse the rest of the oct digits and underscores.
                match parse_oct_rest(advance(after_prefix), c) {
                    ParseResult::Ok(s, i) => {
                        // check for overflow?
                        if *Integer::from_oct_str(s).is_gt_i64_max() {
                            // println!("overflow in octal int parsing above i64 max");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(Integer::from_oct_str(s).to_i64(), i);
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                };
            } else {
                if *is_underscore(c) {
                    // println!("underscore immediately after octal prefix");
                    return ParseResult::Err(Error::fresh());
                } else {
                    if *is_dec_digit(c) {
                        // println!("invalid decimal digit after octal prefix");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_hex_digit(c) {
                            // println!("invalid hex digit after octal prefix");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            // println!("invalid character after octal prefix");
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                }
            }
        }
        Optional::None => {
            // println!("no digits after octal prefix");
            return ParseResult::Err(Error::fresh());
        } // no digits after prefix
    };
}

/// A recursive helper to parse the rest of a octal integer.
#[smt_fn]
fn parse_oct_rest(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_oct_digit(c) {
                let new_acc = acc.concat(c);
                return parse_oct_rest(advance(input), new_acc);
            } else {
                if *is_underscore(c) {
                    let after_underscore = advance(input);
                    match current_char(after_underscore) {
                        Optional::Some(next_c) => {
                            if *is_oct_digit(next_c) {
                                let new_acc = acc.concat(next_c);
                                return parse_oct_rest(advance(after_underscore), new_acc);
                            } else {
                                if *is_underscore(next_c) {
                                    // println!("double underscore error in octal int");
                                    return ParseResult::Err(Error::fresh());
                                } else {
                                    if *is_dec_digit(next_c) {
                                        // println!("invalid decimal digit after underscore in octal int");
                                        return ParseResult::Err(Error::fresh());
                                    } else {
                                        if *is_hex_digit(next_c) {
                                            // println!("invalid hex digit after underscore in octal int");
                                            return ParseResult::Err(Error::fresh());
                                        } else {
                                            // println!("invalid character after underscore in octal int");
                                            return ParseResult::Err(Error::fresh());
                                        }
                                    }
                                }
                            }
                        }
                        Optional::None => {
                            // println!("underscore at end of input in octal int");
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                } else {
                    if *is_dec_digit(c) {
                        // println!("invalid decimal digit in octal int");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_hex_digit(c) {
                            // println!("invalid hex digit in octal int");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(acc, input);
                        }
                    }
                }
            }
        }
        Optional::None => {
            // End of input reached.
            return ParseResult::Ok(acc, input);
        }
    }
}

/// `bin-int = bin-prefix digit0-1 *( digit0-1 / underscore digit0-1 )`
#[smt_fn]
fn parse_bin_int(input: State) -> ParseResult<I64> {
    let after_prefix = advance(advance(input));
    // The first character after the prefix MUST be a bin digit.
    match current_char(after_prefix) {
        Optional::Some(c) => {
            if *is_bin_digit(c) {
                // Recursively parse the rest of the bin digits and underscores.
                match parse_bin_rest(advance(after_prefix), c) {
                    ParseResult::Ok(s, i) => {
                        // check for overflow?
                        if *Integer::from_bin_str(s).is_gt_i64_max() {
                            // println!("overflow in binary int parsing above i64 max");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(Integer::from_bin_str(s).to_i64(), i);
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                };
            } else {
                if *is_underscore(c) {
                    // println!("underscore immediately after binary prefix");
                    return ParseResult::Err(Error::fresh());
                } else {
                    if *is_oct_digit(c) {
                        // println!("invalid octal digit after prefix");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_dec_digit(c) {
                            // println!("invalid decimal digit after binary prefix");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            if *is_hex_digit(c) {
                                // println!("invalid hex digit after binary prefix");
                                return ParseResult::Err(Error::fresh());
                            } else {
                                // println!("invalid character after binary prefix");
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                }
            }
        }
        Optional::None => {
            // println!("no digits after binary prefix");
            return ParseResult::Err(Error::fresh());
        } // no digits after prefix
    };
}

/// A recursive helper to parse the rest of a binary integer.
#[smt_fn]
fn parse_bin_rest(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_bin_digit(c) {
                let new_acc = acc.concat(c);
                return parse_bin_rest(advance(input), new_acc);
            } else {
                if *is_underscore(c) {
                    let after_underscore = advance(input);
                    match current_char(after_underscore) {
                        Optional::Some(next_c) => {
                            if *is_bin_digit(next_c) {
                                let new_acc = acc.concat(next_c);
                                return parse_bin_rest(advance(after_underscore), new_acc);
                            } else {
                                if *is_underscore(next_c) {
                                    // println!("double underscore error in binary int");
                                    return ParseResult::Err(Error::fresh());
                                } else {
                                    if *is_oct_digit(next_c) {
                                        // println!("invalid octal digit after underscore in binary int");
                                        return ParseResult::Err(Error::fresh());
                                    } else {
                                        if *is_dec_digit(next_c) {
                                            // println!("invalid decimal digit after underscore in binary int");
                                            return ParseResult::Err(Error::fresh());
                                        } else {
                                            if *is_hex_digit(next_c) {
                                                // println!("invalid hex digit after underscore in binary int");
                                                return ParseResult::Err(Error::fresh());
                                            } else {
                                                // println!("invalid character after underscore in binary int");
                                                return ParseResult::Err(Error::fresh());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Optional::None => {
                            // println!("underscore at end of input in binary int");
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                } else {
                    if *is_oct_digit(c) {
                        // println!("invalid octal digit in binary int");
                        return ParseResult::Err(Error::fresh());
                    } else {
                        if *is_dec_digit(c) {
                            // println!("invalid decimal digit in binary int");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            if *is_hex_digit(c) {
                                // println!("invalid hex digit in binary int");
                                return ParseResult::Err(Error::fresh());
                            } else {
                                return ParseResult::Ok(acc, input);
                            }
                        }
                    }
                }
            }
        }
        Optional::None => {
            // End of input reached.
            return ParseResult::Ok(acc, input);
        }
    }
}

//! Parses floating-point literals in TOML.

use crate::toml::{
    Optional, ParseResult, State, advance, current_char,
    integer::{is_hex_digit, is_minus, is_plus, is_underscore, parse_integer},
    is_dec_digit, parse_literal,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{
    Error, F64, I64, Integer, Real, String, bitvector::BitvectorOps, float::FloatOps, smt::SMT,
};

#[smt_type]
pub enum Number {
    F64(F64),
    Integer(I64),
}

/// `float = float-int-part ( exp / frac [ exp ] ) / special-float`
#[smt_fn]
pub(crate) fn parse_float(input: State) -> ParseResult<Number> {
    // try special float
    let res = parse_special_float(input);
    match res {
        ParseResult::Ok(f, i) => return ParseResult::Ok(Number::F64(f), i),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            let decimal_part = parse_integer(input);
            match decimal_part {
                ParseResult::Ok(_d, _i) => {
                    match current_char(_i) {
                        Optional::Some(nc) => {
                            if *nc.eq(".".into()) {
                                match parse_unsigned_dec_rest(advance(_i), Integer::from(0)) {
                                    ParseResult::NoMatch => return ParseResult::NoMatch,
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::Ok(val, after_val) => {
                                        let first_part = _d.to_int();
                                        match current_char(after_val) {
                                            Optional::Some(ec) => {
                                                if *ec.eq("e".into()).or(ec.eq("E".into())) {
                                                    match parse_float_exp_part(advance(after_val)) {
                                                        ParseResult::Ok(exp_val, after_exp) => {
                                                            let combined = first_part
                                                                .mul(
                                                                    Integer::from(10)
                                                                        .pow(number_of_digits(val)),
                                                                )
                                                                .add(val);
                                                            // check for to_f64 overflow
                                                            // we do allow literal inf, nan separately but we don't allow overflow to become them
                                                            // also disallowing option in the DSL means that every condition must be explicitly tested in the parser here
                                                            if *combined.to_f64().is_infinite() {
                                                                // println!(
                                                                //     "the combination of integer part and fractional part overflows in float parsing"
                                                                // );
                                                                ParseResult::Err(Error::fresh())
                                                            } else {
                                                                let combined_f64 =
                                                                    combined.to_f64();
                                                                let exp = exp_val
                                                                    .sub(number_of_digits(val));
                                                                // check if exp is less than i32
                                                                if *exp.is_gt_i32_max() {
                                                                    // println!(
                                                                    //     "exponent after e overflows in float parsing"
                                                                    // );
                                                                    return ParseResult::Err(
                                                                        Error::fresh(),
                                                                    );
                                                                }
                                                                if *exp.is_lt_i32_min() {
                                                                    // println!(
                                                                    //     "exponent after e underflows in float parsing"
                                                                    // );
                                                                    return ParseResult::Err(
                                                                        Error::fresh(),
                                                                    );
                                                                } else {
                                                                    let after_e = Real::from(10)
                                                                        .pow(exp.to_real());
                                                                    let after_e_f64 =
                                                                        after_e.to_f64();
                                                                    if *after_e_f64.is_infinite() {
                                                                        // println!(
                                                                        //     "the number after e is okay(within i32) but the 10^exp overflows in float parsing"
                                                                        // );
                                                                        ParseResult::Err(
                                                                            Error::fresh(),
                                                                        )
                                                                    } else {
                                                                        let f = combined_f64
                                                                            .mul(after_e_f64);
                                                                        if *f.is_infinite() {
                                                                            // println!(
                                                                            //     "the number before the decimal point and after the decimal point combined with the exponent overflows in float parsing"
                                                                            // );
                                                                            ParseResult::Err(
                                                                                Error::fresh(),
                                                                            )
                                                                        } else {
                                                                            ParseResult::Ok(
                                                                                Number::F64(f),
                                                                                after_exp,
                                                                            )
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                                        ParseResult::NoMatch => {
                                                            ParseResult::NoMatch // should not happen
                                                        }
                                                    }
                                                } else {
                                                    // no exponent part
                                                    let combined = first_part
                                                        .mul(
                                                            Integer::from(10)
                                                                .pow(number_of_digits(val)),
                                                        )
                                                        .add(val);
                                                    // check for to_f64 overflow
                                                    if *combined.to_f64().is_infinite() {
                                                        // println!(
                                                        //     "the combination of integer part and fractional part overflows in float parsing where no exponent part"
                                                        // );
                                                        ParseResult::Err(Error::fresh())
                                                    } else {
                                                        let f = combined.to_f64().mul(
                                                            Real::from(10)
                                                                .pow(
                                                                    number_of_digits(val)
                                                                        .neg()
                                                                        .to_real(),
                                                                )
                                                                .to_f64(),
                                                        );
                                                        ParseResult::Ok(Number::F64(f), after_val)
                                                    }
                                                }
                                            }
                                            Optional::None => {
                                                // no exponent part
                                                let combined = first_part
                                                    .mul(
                                                        Integer::from(10)
                                                            .pow(number_of_digits(val)),
                                                    )
                                                    .add(val);
                                                // check for to_f64 overflow
                                                if *combined.to_f64().is_infinite() {
                                                    // println!(
                                                    //     "the combination of integer part and fractional part overflows in float parsing where nothing after decimal part"
                                                    // );
                                                    ParseResult::Err(Error::fresh())
                                                } else {
                                                    let f = combined.to_f64().mul(
                                                        Real::from(10)
                                                            .pow(
                                                                number_of_digits(val)
                                                                    .neg()
                                                                    .to_real(),
                                                            )
                                                            .to_f64(),
                                                    );
                                                    ParseResult::Ok(Number::F64(f), after_val)
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // check if there is an exponent part
                                if *nc.eq("e".into()).or(nc.eq("E".into())) {
                                    match parse_float_exp_part(advance(_i)) {
                                        ParseResult::Ok(exp_val, after_exp) => {
                                            let combined = _d.to_int();
                                            // check for to_f64 overflow
                                            if *combined.to_f64().is_infinite() {
                                                // println!(
                                                //     "the integer part overflows in float parsing with only exponent part"
                                                // );
                                                ParseResult::Err(Error::fresh())
                                            } else {
                                                let combined_f64 = combined.to_f64();
                                                let exp = exp_val;
                                                // check if exp is less than i32
                                                if *exp.is_gt_i32_max() {
                                                    // println!(
                                                    //     "exponent after e overflows in float parsing with only exponent part"
                                                    // );
                                                    return ParseResult::Err(Error::fresh());
                                                } else {
                                                    if *exp.is_lt_i32_min() {
                                                        // println!(
                                                        //     "exponent after e underflows in float parsing with only exponent part"
                                                        // );
                                                        return ParseResult::Err(Error::fresh());
                                                    } else {
                                                        let after_e =
                                                            Real::from(10).pow(exp.to_real());
                                                        let after_e_f64 = after_e.to_f64();
                                                        if *after_e_f64.is_infinite() {
                                                            // println!(
                                                            //     "the number after e is okay(within i32) but the 10^exp overflows in float parsing with only exponent part"
                                                            // );
                                                            ParseResult::Err(Error::fresh())
                                                        } else {
                                                            let f = combined_f64.mul(after_e_f64);
                                                            if *f.is_infinite() {
                                                                // println!(
                                                                //     "the number before the decimal point combined with the exponent overflows in float parsing with only exponent part"
                                                                // );
                                                                ParseResult::Err(Error::fresh())
                                                            } else {
                                                                ParseResult::Ok(
                                                                    Number::F64(f),
                                                                    after_exp,
                                                                )
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::NoMatch => {
                                            ParseResult::NoMatch // should not happen
                                        }
                                    }
                                } else {
                                    // it is an integer float
                                    ParseResult::Ok(Number::Integer(_d), _i)
                                }
                            }
                        }
                        Optional::None => {
                            // it is an integer float
                            ParseResult::Ok(Number::Integer(_d), _i)
                        }
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
            }
        }
    }
}

/// Parses a normal (non-special) float value.
/// float-int-part ( exp / frac [ exp ] )
/// float-int-part = dec-int
/// `frac = decimal-point zero-prefixable-int`
/// `zero-prefixable-int = DIGIT *( DIGIT / underscore DIGIT )`
#[smt_fn]
fn parse_unsigned_dec_rest(input: State, number: Integer) -> ParseResult<Integer> {
    // Get the first character.
    let first_char = match current_char(input) {
        Optional::Some(c) => c,
        Optional::None => {
            // println!("must have at least one digit after decimal point");
            return ParseResult::Err(Error::fresh());
        } // must have at least one digit after decimal point
    };

    if *is_dec_digit(first_char).not() {
        // println!("must have at least one digit after decimal point; found {:?}", first_char);
        return ParseResult::Err(Error::fresh()); // must start with a digit
    } else {
        match parse_float_rest(advance(input), first_char, number) {
            ParseResult::Ok(s, i) => {
                return ParseResult::Ok(s.to_int(), i);
            }
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => return ParseResult::NoMatch,
        };
    }
}

/// *( DIGIT / underscore DIGIT )
#[smt_fn]
fn parse_float_rest(input: State, acc: String, number: Integer) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => {
            // Base Case: End of input reached.
            return ParseResult::Ok(acc, input);
        }
        Optional::Some(c) => {
            if *is_dec_digit(c) {
                // Case 1: The next character is a digit. Append it and continue.
                let new_acc = acc.concat(c);
                return parse_float_rest(advance(input), new_acc, number);
            } else {
                if *c.eq("_".into()) {
                    // Case 2: The next character is an underscore.
                    let after_underscore = advance(input);
                    // An underscore MUST be followed by a digit.
                    match current_char(after_underscore) {
                        Optional::None => {
                            // println!("underscore at the end of float part");
                            return ParseResult::Err(Error::fresh());
                        }
                        Optional::Some(next_c) => {
                            if *is_dec_digit(next_c) {
                                // This is a valid `_DIGIT`. Append the digit and continue.
                                let new_acc = acc.concat(next_c);
                                return parse_float_rest(
                                    advance(after_underscore),
                                    new_acc,
                                    number,
                                );
                            } else {
                                if *is_underscore(next_c) {
                                    // println!("multiple underscores in float part");
                                    return ParseResult::Err(Error::fresh());
                                } else {
                                    if *is_hex_digit(next_c) {
                                        // println!("invalid hex character after underscore");
                                        return ParseResult::Err(Error::fresh());
                                    } else {
                                        // println!(
                                        //     "invalid character after underscore in float part"
                                        // );
                                        return ParseResult::Err(Error::fresh());
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if *c.eq("e".into()).or(c.eq("E".into())) {
                        if *number.eq(Integer::from(0)) {
                            // end of float part; exponent part follows.
                            ParseResult::Ok(acc, input) // start of exponent part
                        } else {
                            // duplicate e in float part
                            // println!("duplicate e in float part");
                            return ParseResult::Err(Error::fresh());
                        }
                    } else {
                        if *is_hex_digit(c) {
                            // println!("invalid hex character in float part");
                            return ParseResult::Err(Error::fresh());
                        } else {
                            return ParseResult::Ok(acc, input); // end of float part
                        }
                    }
                }
            }
        }
    }
}

/// `special-float = [ minus / plus ] ( inf / nan )`
#[smt_fn]
fn parse_special_float(input: State) -> ParseResult<F64> {
    match current_char(input) {
        Optional::Some(c) => {
            let new_state = if *is_plus(c).or(is_minus(c)) {
                let input_after_sign = advance(input);
                input_after_sign
            } else {
                input
            };

            match is_inf(new_state) {
                ParseResult::Ok(v, i) => {
                    if *is_minus(c) {
                        ParseResult::Ok(v.neg(), i)
                    } else {
                        ParseResult::Ok(v, i)
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => match is_nan(new_state) {
                    ParseResult::Ok(v, i) => {
                        if *is_minus(c) {
                            ParseResult::Ok(v.neg(), i)
                        } else {
                            ParseResult::Ok(v, i)
                        }
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        match parse_literal(new_state, "Nan".into()) {
                            ParseResult::Ok(_x, _remaining_input) => {
                                // println!("invalid Nan casing");
                                ParseResult::Err(Error::fresh())
                            }
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => {
                                match parse_literal(new_state, "Inf".into()) {
                                    ParseResult::Ok(_x, _remaining_input) => {
                                        // println!("invalid Inf casing");
                                        ParseResult::Err(Error::fresh())
                                    }
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::NoMatch => {
                                        match parse_literal(new_state, "NAN".into()) {
                                            ParseResult::Ok(_x, _remaining_input) => {
                                                // println!("invalid NAN casing");
                                                ParseResult::Err(Error::fresh())
                                            }
                                            ParseResult::Err(e) => return ParseResult::Err(e),
                                            ParseResult::NoMatch => {
                                                match parse_literal(new_state, "INF".into()) {
                                                    ParseResult::Ok(_x, _remaining_input) => {
                                                        // println!(
                                                        //     "invalid INF casing"
                                                        // );
                                                        ParseResult::Err(Error::fresh())
                                                    }
                                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                                    ParseResult::NoMatch => {
                                                        match parse_literal(new_state, "NaN".into())
                                                        {
                                                            ParseResult::Ok(
                                                                _x,
                                                                _remaining_input,
                                                            ) => {
                                                                // println!(
                                                                //     "invalid NaN casing"
                                                                // );
                                                                ParseResult::Err(Error::fresh())
                                                            }
                                                            ParseResult::Err(e) => {
                                                                ParseResult::Err(e)
                                                            }
                                                            ParseResult::NoMatch => {
                                                                ParseResult::NoMatch
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
                },
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `inf = %x69.6e.66  ; inf`
#[smt_fn]
fn is_inf(input: State) -> ParseResult<F64> {
    match parse_literal(input, "inf".into()) {
        ParseResult::Ok(_x, remaining_input) => {
            // If successful, return the infinite value.
            ParseResult::Ok(F64::infinity(), remaining_input)
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// `nan = %x6e.61.6e  ; nan`
#[smt_fn]
fn is_nan(input: State) -> ParseResult<F64> {
    match parse_literal(input, "nan".into()) {
        ParseResult::Ok(_x, remaining_input) => ParseResult::Ok(F64::nan(), remaining_input),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// exp = "e" float-exp-part
/// float-exp-part = [ minus / plus ] zero-prefixable-int
#[smt_fn]
fn parse_float_exp_part(input: State) -> ParseResult<Integer> {
    match current_char(input) {
        Optional::Some(c) => {
            let new_state = if *is_plus(c).or(is_minus(c)) {
                let input_after_sign = advance(input);
                input_after_sign
            } else {
                input
            };
            match parse_unsigned_dec_rest(new_state, Integer::from(1)) {
                ParseResult::Ok(digits, after_digits) => {
                    if *is_minus(c) {
                        return ParseResult::Ok(digits.neg(), after_digits);
                    } else {
                        return ParseResult::Ok(digits, after_digits);
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
            }
        }
        // after e there must be at least one digit
        Optional::None => {
            // println!("must have at least one digit after e in float exponent part");
            ParseResult::Err(Error::fresh())
        }
    }
}

/// Returns the number of digits in the given integer.
#[smt_fn]
fn number_of_digits(i: Integer) -> Integer {
    return recursive_count_digits(i, Integer::from(10), Integer::from(0));
}

/// Helper function to recursively count digits.
#[smt_fn]
fn recursive_count_digits(i: Integer, divisor: Integer, acc: Integer) -> Integer {
    if *i.lt(divisor) {
        return acc.add(Integer::from(1));
    } else {
        return recursive_count_digits(
            i,
            divisor.mul(Integer::from(10)),
            acc.add(Integer::from(1)),
        );
    }
}

//! Parses floating-point literals in TOML.

use crate::toml::{
    Optional, ParseResult, State, advance, cp_to_str, current_char,
    integer::{
        is_bin_prefix, is_hex_digit, is_hex_prefix, is_minus, is_oct_prefix, is_plus,
        is_underscore, parse_integer,
    },
    is_dec_digit, parse_literal,
};
use rusmt_smt_remark_derive::smt_fn;
use rusmt_smt_remark_derive::smt_type;
use rusmt_smt_stdlib::{
    Boolean, F64, I64, Integer, Path, Real, String, U32, bitvector::BitvectorOps, float::FloatOps,
    smt::SMT,
};

#[smt_type]
pub enum Number {
    F64(F64),
    Integer(I64),
}

/// Whether the input's first character is a minus sign (`-`). The float's sign is
/// taken from here so that magnitude-based fraction assembly re-signs values whose
/// integer part is `0` (e.g. `-0.5`) correctly.
#[smt_fn]
fn starts_with_minus(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c) => is_minus(c),
        Optional::None => Boolean::from(false),
    }
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
                            if *nc.eq(U32::from(0x2E)).and(
                                is_hex_prefix(input)
                                    .or(is_oct_prefix(input))
                                    .or(is_bin_prefix(input))
                                    .not(),
                            ) {
                                match parse_unsigned_dec_rest(advance(_i), Integer::from(0)) {
                                    ParseResult::NoMatch => return ParseResult::NoMatch,
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::Ok(frac_str, after_val) => {
                                        let neg = starts_with_minus(input);
                                        let mag =
                                            if *neg { _d.to_int().neg() } else { _d.to_int() };
                                        let l = frac_str.length();
                                        let v = frac_str.to_int();
                                        match current_char(after_val) {
                                            Optional::Some(ec) => {
                                                if *ec
                                                    .eq(U32::from(0x65))
                                                    .or(ec.eq(U32::from(0x45)))
                                                {
                                                    match parse_float_exp_part(advance(after_val)) {
                                                        ParseResult::Ok(exp_val, after_exp) => {
                                                            let combined = mag
                                                                .mul(Integer::from(10).pow(l))
                                                                .add(v);
                                                            // check for to_f64 overflow
                                                            // we do allow literal inf, nan separately but we don't allow overflow to become them
                                                            // also disallowing option in the DSL means that every condition must be explicitly tested in the parser here
                                                            if *combined.to_f64().is_infinite() {
                                                                // println!(
                                                                //     "the combination of integer part and fractional part overflows in float parsing"
                                                                // );
                                                                return ParseResult::Err(
                                                                    Path::named(String::from(
                                                                        "float_frac_combined_overflow",
                                                                    )),
                                                                );
                                                            } else {
                                                                let combined_f64 =
                                                                    combined.to_f64();
                                                                let exp = exp_val.sub(l);
                                                                // check if exp is less than i32
                                                                if *exp.is_gt_i32_max() {
                                                                    // println!(
                                                                    //     "exponent after e overflows in float parsing"
                                                                    // );
                                                                    return ParseResult::Err(
                                                                        Path::named(String::from(
                                                                            "float_frac_exp_overflow_i32",
                                                                        )),
                                                                    );
                                                                } else {
                                                                    if *exp.is_lt_i32_min() {
                                                                        // println!(
                                                                        //     "exponent after e underflows in float parsing"
                                                                        // );
                                                                        return ParseResult::Err(
                                                                            Path::named(
                                                                                String::from(
                                                                                    "float_frac_exp_underflow_i32",
                                                                                ),
                                                                            ),
                                                                        );
                                                                    } else {
                                                                        let after_e =
                                                                            Real::from(10)
                                                                                .pow(exp.to_real());
                                                                        let after_e_f64 =
                                                                            after_e.to_f64();
                                                                        if *after_e_f64
                                                                            .is_infinite()
                                                                        {
                                                                            // println!(
                                                                            //     "the number after e is okay(within i32) but the 10^exp overflows in float parsing"
                                                                            // );
                                                                            return ParseResult::Err(
                                                                                Path::named(String::from("float_frac_pow10_overflow")),
                                                                            );
                                                                        } else {
                                                                            let f = combined_f64
                                                                                .mul(after_e_f64);
                                                                            if *f.is_infinite() {
                                                                                // println!(
                                                                                //     "the number before the decimal point and after the decimal point combined with the exponent overflows in float parsing"
                                                                                // );
                                                                                return ParseResult::Err(
                                                                                    Path::named(String::from("float_frac_final_overflow")),
                                                                                );
                                                                            } else {
                                                                                let signed = if *neg
                                                                                {
                                                                                    f.neg()
                                                                                } else {
                                                                                    f
                                                                                };
                                                                                return ParseResult::Ok(
                                                                                    Number::F64(signed),
                                                                                    after_exp,
                                                                                );
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        ParseResult::Err(e) => {
                                                            return ParseResult::Err(e);
                                                        }
                                                        ParseResult::NoMatch => {
                                                            return ParseResult::NoMatch; // should not happen
                                                        }
                                                    }
                                                } else {
                                                    // no exponent part
                                                    let combined =
                                                        mag.mul(Integer::from(10).pow(l)).add(v);
                                                    // check for to_f64 overflow
                                                    if *combined.to_f64().is_infinite() {
                                                        // println!(
                                                        //     "the combination of integer part and fractional part overflows in float parsing where no exponent part"
                                                        // );
                                                        return ParseResult::Err(Path::named(
                                                            String::from(
                                                                "float_frac_noexp_combined_overflow",
                                                            ),
                                                        ));
                                                    } else {
                                                        let f = combined.to_f64().mul(
                                                            Real::from(10)
                                                                .pow(l.neg().to_real())
                                                                .to_f64(),
                                                        );
                                                        let signed = if *neg { f.neg() } else { f };
                                                        return ParseResult::Ok(
                                                            Number::F64(signed),
                                                            after_val,
                                                        );
                                                    }
                                                }
                                            }
                                            Optional::None => {
                                                // no exponent part
                                                let combined =
                                                    mag.mul(Integer::from(10).pow(l)).add(v);
                                                // check for to_f64 overflow
                                                if *combined.to_f64().is_infinite() {
                                                    // println!(
                                                    //     "the combination of integer part and fractional part overflows in float parsing where nothing after decimal part"
                                                    // );
                                                    return ParseResult::Err(Path::named(
                                                        String::from(
                                                            "float_frac_eof_combined_overflow",
                                                        ),
                                                    ));
                                                } else {
                                                    let f = combined.to_f64().mul(
                                                        Real::from(10)
                                                            .pow(l.neg().to_real())
                                                            .to_f64(),
                                                    );
                                                    let signed = if *neg { f.neg() } else { f };
                                                    return ParseResult::Ok(
                                                        Number::F64(signed),
                                                        after_val,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // check if there is an exponent part
                                if *nc.eq(U32::from(0x65)).or(nc.eq(U32::from(0x45))) {
                                    match parse_float_exp_part(advance(_i)) {
                                        ParseResult::Ok(exp_val, after_exp) => {
                                            let combined = _d.to_int();
                                            let combined_f64 = combined.to_f64();
                                            let exp = exp_val;
                                            // check if exp is less than i32
                                            if *exp.is_gt_i32_max() {
                                                // println!(
                                                //     "exponent after e overflows in float parsing with only exponent part"
                                                // );
                                                return ParseResult::Err(Path::named(
                                                    String::from("float_exp_only_exp_overflow_i32"),
                                                ));
                                            } else {
                                                if *exp.is_lt_i32_min() {
                                                    // println!(
                                                    //     "exponent after e underflows in float parsing with only exponent part"
                                                    // );
                                                    return ParseResult::Err(Path::named(
                                                        String::from(
                                                            "float_exp_only_exp_underflow_i32",
                                                        ),
                                                    ));
                                                } else {
                                                    let after_e = Real::from(10).pow(exp.to_real());
                                                    let after_e_f64 = after_e.to_f64();
                                                    if *after_e_f64.is_infinite() {
                                                        // println!(
                                                        //     "the number after e is okay(within i32) but the 10^exp overflows in float parsing with only exponent part"
                                                        // );
                                                        return ParseResult::Err(Path::named(
                                                            String::from(
                                                                "float_exp_only_pow10_overflow",
                                                            ),
                                                        ));
                                                    } else {
                                                        let f = combined_f64.mul(after_e_f64);
                                                        if *f.is_infinite() {
                                                            // println!(
                                                            //     "the number before the decimal point combined with the exponent overflows in float parsing with only exponent part"
                                                            // );
                                                            return ParseResult::Err(Path::named(
                                                                String::from(
                                                                    "float_exp_only_final_overflow",
                                                                ),
                                                            ));
                                                        } else {
                                                            return ParseResult::Ok(
                                                                Number::F64(f),
                                                                after_exp,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::NoMatch => {
                                            return ParseResult::NoMatch; // should not happen
                                        }
                                    }
                                } else {
                                    // it is an integer float
                                    return ParseResult::Ok(Number::Integer(_d), _i);
                                }
                            }
                        }
                        Optional::None => {
                            // it is an integer float
                            return ParseResult::Ok(Number::Integer(_d), _i);
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
fn parse_unsigned_dec_rest(input: State, number: Integer) -> ParseResult<String> {
    // Avoid `return ...` inside a match-as-expression; RuSmt does not treat `return`
    // as a diverging expression for type-checking purposes.
    match current_char(input) {
        Optional::None => {
            // println!("must have at least one digit after decimal point");
            return ParseResult::Err(Path::named(String::from("float_no_digit_after_dot_eof")));
        } // must have at least one digit after decimal point
        Optional::Some(first_char) => {
            if *is_dec_digit(first_char).not() {
                // println!("must have at least one digit after decimal point; found {:?}", first_char);
                return ParseResult::Err(Path::named(String::from("float_no_digit_after_dot"))); // must have a digit after the decimal point
            } else {
                match parse_float_rest(advance(input), cp_to_str(first_char), number) {
                    ParseResult::Ok(s, i) => {
                        // Return the raw digit string; the fraction's DIGIT COUNT
                        // (including leading zeros) is needed by `parse_float` for
                        // correct scaling, which the integer value alone loses.
                        return ParseResult::Ok(s, i);
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                }
            }
        }
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
                let new_acc = acc.concat(cp_to_str(c));
                return parse_float_rest(advance(input), new_acc, number);
            } else {
                if *c.eq(U32::from(0x5F)) {
                    // Case 2: The next character is an underscore.
                    let after_underscore = advance(input);
                    // An underscore MUST be followed by a digit.
                    match current_char(after_underscore) {
                        Optional::None => {
                            // println!("underscore at the end of float part");
                            return ParseResult::Err(Path::named(String::from(
                                "float_underscore_at_end",
                            )));
                        }
                        Optional::Some(next_c) => {
                            if *is_dec_digit(next_c) {
                                // This is a valid `_DIGIT`. Append the digit and continue.
                                let new_acc = acc.concat(cp_to_str(next_c));
                                return parse_float_rest(
                                    advance(after_underscore),
                                    new_acc,
                                    number,
                                );
                            } else {
                                if *is_underscore(next_c) {
                                    // println!("multiple underscores in float part");
                                    return ParseResult::Err(Path::named(String::from(
                                        "float_multiple_underscores",
                                    )));
                                } else {
                                    if *is_hex_digit(next_c) {
                                        // println!("invalid hex character after underscore");
                                        return ParseResult::Err(Path::named(String::from(
                                            "float_hex_char_after_underscore",
                                        )));
                                    } else {
                                        // println!(
                                        //     "invalid character after underscore in float part"
                                        // );
                                        return ParseResult::Err(Path::named(String::from(
                                            "float_invalid_char_after_underscore",
                                        )));
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if *c.eq(U32::from(0x65)).or(c.eq(U32::from(0x45))) {
                        if *number.eq(Integer::from(0)) {
                            // end of float part; exponent part follows.
                            ParseResult::Ok(acc, input) // start of exponent part
                        } else {
                            // duplicate e in float part
                            // println!("duplicate e in float part");
                            return ParseResult::Err(Path::named(String::from(
                                "float_duplicate_exponent",
                            )));
                        }
                    } else {
                        if *is_hex_digit(c) {
                            // println!("invalid hex character in float part");
                            return ParseResult::Err(Path::named(String::from(
                                "float_hex_char_in_part",
                            )));
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
                                ParseResult::Err(Path::named(String::from(
                                    "float_invalid_nan_casing_titlecase",
                                )))
                            }
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => {
                                match parse_literal(new_state, "Inf".into()) {
                                    ParseResult::Ok(_x, _remaining_input) => {
                                        // println!("invalid Inf casing");
                                        ParseResult::Err(Path::named(String::from(
                                            "float_invalid_inf_casing_titlecase",
                                        )))
                                    }
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::NoMatch => {
                                        match parse_literal(new_state, "NAN".into()) {
                                            ParseResult::Ok(_x, _remaining_input) => {
                                                // println!("invalid NAN casing");
                                                ParseResult::Err(Path::named(String::from(
                                                    "float_invalid_nan_casing_allcaps",
                                                )))
                                            }
                                            ParseResult::Err(e) => return ParseResult::Err(e),
                                            ParseResult::NoMatch => {
                                                match parse_literal(new_state, "INF".into()) {
                                                    ParseResult::Ok(_x, _remaining_input) => {
                                                        // println!(
                                                        //     "invalid INF casing"
                                                        // );
                                                        ParseResult::Err(Path::named(String::from(
                                                            "float_invalid_inf_casing_allcaps",
                                                        )))
                                                    }
                                                    ParseResult::Err(e) => {
                                                        return ParseResult::Err(e);
                                                    }
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
                                                                ParseResult::Err(Path::named(
                                                                    String::from(
                                                                        "float_invalid_nan_casing_camelcase",
                                                                    ),
                                                                ))
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
                    let dval = digits.to_int();
                    if *is_minus(c) {
                        return ParseResult::Ok(dval.neg(), after_digits);
                    } else {
                        return ParseResult::Ok(dval, after_digits);
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => return ParseResult::NoMatch,
            }
        }
        // after e there must be at least one digit
        Optional::None => {
            // println!("must have at least one digit after e in float exponent part");
            return ParseResult::Err(Path::named(String::from("float_exp_no_digit_after_e")));
        }
    }
}

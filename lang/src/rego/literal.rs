//! Scalar literal parsing for the Rego subset: `null`, `true`, `false`,
//! number, string.
//!
//! Spec (scalars): <https://www.openpolicyagent.org/docs/policy-language/#scalar-values>

use crate::rego::{
    Optional, ParseResult, State, advance, ast::Term, current_char, is_dec_digit, is_ident_cont,
    parse_literal,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Error, Integer, Real, String, smt::SMT};

/// Top-level scalar literal dispatcher.
///
/// Returns `NoMatch` if the lookahead does not start a scalar literal so the
/// caller can try a non-scalar term (array, object, ref, ...).
#[smt_fn]
pub(crate) fn parse_scalar(state: State) -> ParseResult<Term> {
    match parse_null(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(t, ns) => return ParseResult::Ok(t, ns),
        ParseResult::NoMatch => match parse_boolean(state) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::Ok(t, ns) => return ParseResult::Ok(t, ns),
            ParseResult::NoMatch => match parse_string(state) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::Ok(t, ns) => return ParseResult::Ok(t, ns),
                ParseResult::NoMatch => parse_number(state),
            },
        },
    }
}

/// `null` literal.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#scalar-values>
/// Rego is case-sensitive: only the lowercase form is valid.
#[smt_fn]
pub(crate) fn parse_null(state: State) -> ParseResult<Term> {
    match parse_literal(state, String::from("null")) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(_kw, after) => {
            // Reject `nullish` etc. — must not be followed by an identifier
            // character, otherwise this is a different identifier that just
            // happens to start with "null".
            if *next_is_ident_cont(after) {
                ParseResult::NoMatch
            } else {
                ParseResult::Ok(Term::Null, after)
            }
        }
    }
}

/// `true` / `false` literal.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#scalar-values>
#[smt_fn]
pub(crate) fn parse_boolean(state: State) -> ParseResult<Term> {
    // Reject case-violating spellings explicitly so they become Error::fresh
    // synthesis targets — not silent NoMatch (which would let a downstream
    // rule mis-classify them).
    match parse_literal(state, String::from("True")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                // ERROR: case-violating boolean
                return ParseResult::Err(Error::fresh());
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("TRUE")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                // ERROR: case-violating boolean
                return ParseResult::Err(Error::fresh());
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("False")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                // ERROR: case-violating boolean
                return ParseResult::Err(Error::fresh());
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("FALSE")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                // ERROR: case-violating boolean
                return ParseResult::Err(Error::fresh());
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    // Now try the canonical lowercase forms.
    match parse_literal(state, String::from("true")) {
        ParseResult::Ok(_, after) => {
            if *next_is_ident_cont(after) {
                return ParseResult::NoMatch;
            } else {
                return ParseResult::Ok(Term::Boolean(Boolean::from(true)), after);
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("false")) {
        ParseResult::Ok(_, after) => {
            if *next_is_ident_cont(after) {
                return ParseResult::NoMatch;
            } else {
                return ParseResult::Ok(Term::Boolean(Boolean::from(false)), after);
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// True if the next character (if any) is a valid identifier-continuation
/// character — used to disambiguate keyword vs. identifier prefixes.
#[smt_fn]
pub(crate) fn next_is_ident_cont(state: State) -> Boolean {
    match current_char(state) {
        Optional::Some(c) => is_ident_cont(c),
        Optional::None => Boolean::from(false),
    }
}

/// Parse a Rego number: optional `-`, then `digit+`, then optional fractional
/// part `. digit+`. Numbers are JSON numbers (Rego does not have a separate
/// integer type) — represented as `Real` for SMT decidability.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-reference/#numbers>
#[smt_fn]
pub(crate) fn parse_number(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from("-")) {
                let after_sign = advance(state);
                match parse_unsigned_number(after_sign) {
                    ParseResult::Ok(val, after) => {
                        ParseResult::Ok(Term::Number(val.neg()), after)
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // ERROR: `-` without following digit
                        return ParseResult::Err(Error::fresh());
                    }
                }
            } else {
                if *is_dec_digit(c) {
                    match parse_unsigned_number(state) {
                        ParseResult::Ok(val, after) => ParseResult::Ok(Term::Number(val), after),
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                    }
                } else {
                    return ParseResult::NoMatch;
                }
            }
        }
    }
}

/// Parse `digit+ ("." digit+)?` and return the resulting Real.
#[smt_fn]
fn parse_unsigned_number(state: State) -> ParseResult<Real> {
    match parse_digits(state, Integer::from(0), Integer::from(0)) {
        ParseResult::Ok(int_part, after_int) => {
            // int_part is held as Integer for exactness; lift to Real.
            let int_real = int_part.to_real();
            match current_char(after_int) {
                Optional::Some(c) => {
                    if *c.eq(String::from(".")) {
                        let after_dot = advance(after_int);
                        match current_char(after_dot) {
                            Optional::None => {
                                // ERROR: trailing `.` with no fractional digits
                                return ParseResult::Err(Error::fresh());
                            }
                            Optional::Some(d) => {
                                if *is_dec_digit(d) {
                                    match parse_digits(
                                        after_dot,
                                        Integer::from(0),
                                        Integer::from(0),
                                    ) {
                                        ParseResult::Ok(_frac_unused, after_frac) => {
                                            // Re-parse the digits as a Real fractional part by
                                            // accumulating with division by 10 each step.
                                            match parse_frac_part(
                                                after_dot,
                                                Real::from(0),
                                                Real::from(1),
                                            ) {
                                                ParseResult::Ok(frac_real, ns) => {
                                                    let _unused_state = after_frac;
                                                    ParseResult::Ok(int_real.add(frac_real), ns)
                                                }
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                ParseResult::NoMatch => return ParseResult::NoMatch,
                                            }
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::NoMatch => return ParseResult::NoMatch,
                                    }
                                } else {
                                    // ERROR: `.` followed by non-digit
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                        }
                    } else {
                        ParseResult::Ok(int_real, after_int)
                    }
                }
                Optional::None => ParseResult::Ok(int_real, after_int),
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// Recursively read decimal digits, accumulating into `acc` (base-10).
/// Returns the count of digits read in `count` so the caller can detect the
/// "no digits at all" case as `NoMatch`.
#[smt_fn]
fn parse_digits(state: State, acc: Integer, count: Integer) -> ParseResult<Integer> {
    match current_char(state) {
        Optional::None => {
            if *count.eq(Integer::from(0)) {
                ParseResult::NoMatch
            } else {
                ParseResult::Ok(acc, state)
            }
        }
        Optional::Some(c) => {
            if *is_dec_digit(c) {
                let digit = digit_value(c);
                let new_acc = acc.mul(Integer::from(10)).add(digit);
                parse_digits(advance(state), new_acc, count.add(Integer::from(1)))
            } else {
                if *count.eq(Integer::from(0)) {
                    ParseResult::NoMatch
                } else {
                    ParseResult::Ok(acc, state)
                }
            }
        }
    }
}

/// Decimal digit character to its integer value (`'0'..'9' -> 0..9`).
#[smt_fn]
fn digit_value(c: String) -> Integer {
    // Linear ladder: avoid str.to_int (Z3's parses only decimal strings of
    // length ≥ 1, which is fine, but the ladder keeps the encoding closer to
    // the spec wording where each '0'..'9' is a separate ABNF character).
    if *c.eq(String::from("0")) {
        Integer::from(0)
    } else {
        if *c.eq(String::from("1")) {
            Integer::from(1)
        } else {
            if *c.eq(String::from("2")) {
                Integer::from(2)
            } else {
                if *c.eq(String::from("3")) {
                    Integer::from(3)
                } else {
                    if *c.eq(String::from("4")) {
                        Integer::from(4)
                    } else {
                        if *c.eq(String::from("5")) {
                            Integer::from(5)
                        } else {
                            if *c.eq(String::from("6")) {
                                Integer::from(6)
                            } else {
                                if *c.eq(String::from("7")) {
                                    Integer::from(7)
                                } else {
                                    if *c.eq(String::from("8")) {
                                        Integer::from(8)
                                    } else {
                                        Integer::from(9)
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

/// Parse the fractional part (digits after the `.`) by accumulating
/// `acc + d * scale` where `scale` halves at each step.
///
/// `scale` starts at 1 and gets divided by 10 *before* it is multiplied with
/// the digit; that way digit at position 0 contributes `d/10`, digit at
/// position 1 contributes `d/100`, etc., as the spec dictates.
#[smt_fn]
fn parse_frac_part(state: State, acc: Real, scale: Real) -> ParseResult<Real> {
    match current_char(state) {
        Optional::None => return ParseResult::Ok(acc, state),
        Optional::Some(c) => {
            if *is_dec_digit(c) {
                let new_scale = scale.div(Real::from(10));
                let digit_real = digit_value(c).to_real();
                let contribution = digit_real.mul(new_scale);
                parse_frac_part(advance(state), acc.add(contribution), new_scale)
            } else {
                return ParseResult::Ok(acc, state);
            }
        }
    }
}

/// Parse a Rego string literal: `"..."` with backslash escapes for `"`, `\`,
/// `n`, `t`, `r`. Single-line strings only (no embedded newlines).
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-reference/#strings>
#[smt_fn]
pub(crate) fn parse_string(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from("\"")) {
                parse_string_body(advance(state), String::from(""))
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// Continuation worker for [`parse_string`].
#[smt_fn]
fn parse_string_body(state: State, acc: String) -> ParseResult<Term> {
    match current_char(state) {
        Optional::None => {
            // ERROR: end of input before closing `"`
            return ParseResult::Err(Error::fresh());
        }
        Optional::Some(c) => {
            if *c.eq(String::from("\"")) {
                ParseResult::Ok(Term::String(acc), advance(state))
            } else {
                if *c.eq(String::from("\n")) {
                    // ERROR: literal newline inside a single-line string
                    return ParseResult::Err(Error::fresh());
                } else {
                    if *c.eq(String::from("\\")) {
                        let after_bs = advance(state);
                        match current_char(after_bs) {
                            Optional::None => {
                                // ERROR: trailing backslash
                                return ParseResult::Err(Error::fresh());
                            }
                            Optional::Some(esc) => {
                                if *esc.eq(String::from("\"")) {
                                    parse_string_body(advance(after_bs), acc.concat(String::from("\"")))
                                } else {
                                    if *esc.eq(String::from("\\")) {
                                        parse_string_body(
                                            advance(after_bs),
                                            acc.concat(String::from("\\")),
                                        )
                                    } else {
                                        if *esc.eq(String::from("n")) {
                                            parse_string_body(
                                                advance(after_bs),
                                                acc.concat(String::from("\n")),
                                            )
                                        } else {
                                            if *esc.eq(String::from("t")) {
                                                parse_string_body(
                                                    advance(after_bs),
                                                    acc.concat(String::from("\t")),
                                                )
                                            } else {
                                                if *esc.eq(String::from("r")) {
                                                    parse_string_body(
                                                        advance(after_bs),
                                                        acc.concat(String::from("\r")),
                                                    )
                                                } else {
                                                    // ERROR: unrecognized escape sequence
                                                    return ParseResult::Err(Error::fresh());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        parse_string_body(advance(state), acc.concat(c))
                    }
                }
            }
        }
    }
}

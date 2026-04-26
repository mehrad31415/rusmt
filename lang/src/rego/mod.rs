//! Runnable specification for a Rego subset.
//!
//! Spec: <https://www.openpolicyagent.org/docs/policy-language/>
//! Reference (used only to disambiguate spec wording): <https://github.com/open-policy-agent/opa>
//!
//! This is a *spec* implementation, not a port of OPA: every encoded rule traces
//! to a published spec section and is annotated with the citation. See
//! `README.md` for the explicit subset / out-of-scope boundary, and
//! `ERRORS.md` for the index of `Error::fresh()` synthesis targets.
//!
//! The shape mirrors the TOML case study (`lang/src/toml/`) — the parser is a
//! collection of `#[smt_fn]` functions that consume `State` and return
//! `ParseResult<T>`; the AST lives in `ast.rs`.

use crate::rego::{
    ast::{Module, Term},
    module::parse_module,
    rule::eval_module,
};
use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Array, Boolean, Integer, Seq, String, smt::SMT};

/// AST types for the Rego subset.
pub mod ast;
/// Expression parsing and evaluation.
pub mod expr;
/// Scalar literal parsing (null, bool, number, string).
pub mod literal;
/// Module / package parsing.
pub mod module;
/// Rule parsing and evaluation.
pub mod rule;
/// Composite term parsing (arrays, objects, sets, refs).
pub mod term;

/// The parser's input state.
///
/// We thread input as `Seq<String>` (one Unicode code point per element) +
/// a cursor, exactly like the TOML parser, so cursor manipulation does not
/// rely on any byte-level operation that would diverge between Rust and Z3.
#[smt_type]
pub struct State {
    /// The full sequence of characters being parsed (one Unicode code point per element).
    pub stream: Seq<String>,
    /// The current position (character index) in the stream.
    pub cursor: Integer,
}

/// Result of a parsing function.
///
/// The three variants encode the standard top-down-parser disposition:
/// `NoMatch` for "this rule does not apply, the caller should try another",
/// `Ok` for a successful match (with the leftover state), and `Err` for a
/// hard parse failure (the caller propagates).
#[smt_type]
pub enum ParseResult<T: SMT> {
    /// The parser did not find a match for its rule, but no error occurred.
    NoMatch,
    /// The parser successfully matched and produced a value of type `T`,
    /// along with the remaining input stream.
    Ok(T, State),
    /// The parser encountered a hard error.
    Err(rusmart_smt_stdlib::Error),
}

/// An optional value type (used because `core::Option` is not part of the DSL surface).
#[smt_type]
pub enum Optional<T: SMT> {
    /// Absence.
    None,
    /// Presence.
    Some(T),
}

/// Returns the character at the current cursor position, or `None` at EOF.
#[smt_fn]
pub(crate) fn current_char(input: State) -> Optional<String> {
    if *input.cursor.lt(input.stream.length()) {
        return Optional::Some(input.stream.at(input.cursor));
    } else {
        return Optional::None;
    }
}

/// Returns a new `State` advanced by one character.
#[smt_fn]
pub(crate) fn advance(input: State) -> State {
    return State {
        stream: input.stream,
        cursor: input.cursor.add(1.into()),
    };
}

/// Peek N characters ahead.
#[smt_fn]
pub(crate) fn peek(state: State, n: Integer) -> Optional<String> {
    let new_state = State {
        stream: state.stream,
        cursor: state.cursor.add(n),
    };
    current_char(new_state)
}

/// Space (`%x20`).
#[smt_fn]
pub(crate) fn is_space(c: String) -> Boolean {
    c.eq(String::from(" "))
}

/// Horizontal tab (`%x09`).
#[smt_fn]
pub(crate) fn is_htab(c: String) -> Boolean {
    c.eq(String::from("\t"))
}

/// LF newline (`%x0A`).
#[smt_fn]
pub(crate) fn is_lf(c: String) -> Boolean {
    c.eq(String::from("\n"))
}

/// CR (`%x0D`).
#[smt_fn]
pub(crate) fn is_cr(c: String) -> Boolean {
    c.eq(String::from("\r"))
}

/// `wschar = SP / HT` (whitespace within a line — newlines handled separately).
#[smt_fn]
pub(crate) fn is_wschar(c: String) -> Boolean {
    is_space(c).or(is_htab(c))
}

/// LF or CRLF newline. Returns true if the current position is at a newline.
///
/// Currently unused by the rule-level grammar (which folds newlines into the
/// `parse_ws_nl` shared helper) but kept on the public surface so individual
/// modules can opt into newline-aware parsing without re-deriving the rule.
#[smt_fn]
#[allow(dead_code)]
pub(crate) fn is_newline(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_lf(c) {
                Boolean::from(true)
            } else {
                if *is_cr(c) {
                    match peek(input, 1.into()) {
                        Optional::Some(c2) => is_lf(c2),
                        Optional::None => Boolean::from(false),
                    }
                } else {
                    Boolean::from(false)
                }
            }
        }
        Optional::None => Boolean::from(false),
    }
}

/// Consume one newline (LF or CRLF). Returns the consumed text and the new state.
///
/// See [`is_newline`] for the rationale behind keeping this on the public
/// surface even though the current grammar handles separators via
/// `parse_ws_nl`.
#[smt_fn]
#[allow(dead_code)]
pub(crate) fn parse_newline(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_lf(c) {
                ParseResult::Ok(String::from("\n"), advance(input))
            } else {
                if *is_cr(c) {
                    match peek(input, 1.into()) {
                        Optional::Some(c2) => {
                            if *is_lf(c2) {
                                ParseResult::Ok(String::from("\r\n"), advance(advance(input)))
                            } else {
                                ParseResult::NoMatch
                            }
                        }
                        Optional::None => ParseResult::NoMatch,
                    }
                } else {
                    ParseResult::NoMatch
                }
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `ws = *wschar` — consume zero or more in-line whitespace characters.
#[smt_fn]
pub(crate) fn parse_ws(input: State) -> State {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_wschar(c) {
                parse_ws(advance(input))
            } else {
                input
            }
        }
        Optional::None => input,
    }
}

/// Consume zero or more whitespace characters AND newlines (used between rules).
#[smt_fn]
pub(crate) fn parse_ws_nl(input: State) -> State {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_wschar(c) {
                parse_ws_nl(advance(input))
            } else {
                if *is_lf(c) {
                    parse_ws_nl(advance(input))
                } else {
                    if *is_cr(c) {
                        match peek(input, 1.into()) {
                            Optional::Some(c2) => {
                                if *is_lf(c2) {
                                    parse_ws_nl(advance(advance(input)))
                                } else {
                                    input
                                }
                            }
                            Optional::None => input,
                        }
                    } else {
                        if *c.eq(String::from("#")) {
                            // Rego line comments: `#` to end of line.
                            // Spec: <https://www.openpolicyagent.org/docs/policy-language/#comments>
                            parse_ws_nl(skip_to_newline(advance(input)))
                        } else {
                            input
                        }
                    }
                }
            }
        }
        Optional::None => input,
    }
}

/// Skip every character up to (but not including) the next newline / EOF.
#[smt_fn]
pub(crate) fn skip_to_newline(input: State) -> State {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_lf(c) {
                input
            } else {
                if *is_cr(c) {
                    match peek(input, 1.into()) {
                        Optional::Some(c2) => {
                            if *is_lf(c2) {
                                input
                            } else {
                                skip_to_newline(advance(input))
                            }
                        }
                        Optional::None => skip_to_newline(advance(input)),
                    }
                } else {
                    skip_to_newline(advance(input))
                }
            }
        }
        Optional::None => input,
    }
}

/// `ALPHA = A-Z / a-z`.
#[smt_fn]
pub(crate) fn is_alpha(c: String) -> Boolean {
    c.ge(String::from("A"))
        .and(c.le(String::from("Z")))
        .or(c.ge(String::from("a")).and(c.le(String::from("z"))))
}

/// `DIGIT = 0-9`.
#[smt_fn]
pub(crate) fn is_dec_digit(c: String) -> Boolean {
    c.ge(String::from("0")).and(c.le(String::from("9")))
}

/// First character of a Rego identifier: letter or underscore.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#variables>
#[smt_fn]
pub(crate) fn is_ident_start(c: String) -> Boolean {
    is_alpha(c).or(c.eq(String::from("_")))
}

/// Continuation character of a Rego identifier: letter, digit, or underscore.
#[smt_fn]
pub(crate) fn is_ident_cont(c: String) -> Boolean {
    is_alpha(c).or(is_dec_digit(c)).or(c.eq(String::from("_")))
}

/// Match a literal multi-character keyword.
///
/// Returns `NoMatch` if the keyword does not match starting at the cursor.
#[smt_fn]
pub(crate) fn parse_literal(input: State, literal: String) -> ParseResult<String> {
    parse_literal_recursive(input, literal, 0.into())
}

/// Recursive worker for [`parse_literal`].
#[smt_fn]
fn parse_literal_recursive(
    input: State,
    literal: String,
    literal_cursor: Integer,
) -> ParseResult<String> {
    if *literal_cursor.eq(literal.length()) {
        return ParseResult::Ok(literal, input);
    } else {
        let expected = literal.at(literal_cursor);
        match current_char(input) {
            Optional::None => return ParseResult::NoMatch,
            Optional::Some(actual) => {
                if *actual.eq(expected) {
                    return parse_literal_recursive(
                        advance(input),
                        literal,
                        literal_cursor.add(1.into()),
                    );
                } else {
                    return ParseResult::NoMatch;
                }
            }
        }
    }
}

/// Parse an identifier: `(alpha / "_") (alpha / digit / "_")*`.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#variables>
#[smt_fn]
pub(crate) fn parse_ident(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(first) => {
            if *is_ident_start(first) {
                parse_ident_rest(advance(input), first)
            } else {
                return ParseResult::NoMatch;
            }
        }
    }
}

/// Continuation worker for [`parse_ident`].
#[smt_fn]
fn parse_ident_rest(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => return ParseResult::Ok(acc, input),
        Optional::Some(c) => {
            if *is_ident_cont(c) {
                parse_ident_rest(advance(input), acc.concat(c))
            } else {
                return ParseResult::Ok(acc, input);
            }
        }
    }
}

/// Top-level entry point: parse a complete Rego module from `state`.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#modules>
#[smt_fn]
pub fn parse_policy(state: State) -> ParseResult<Module> {
    parse_module(state)
}

/// Top-level evaluation entry point: parse a module, then evaluate it against
/// `input`, returning the per-rule output object.
///
/// The result maps each rule head name to the value produced by the rule (or
/// the default value if the rule had no successful body).
///
/// Spec (evaluation overview): <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_fn]
pub fn evaluate_policy(state: State, input: Term) -> ParseResult<Array<String, Term>> {
    match parse_module(state) {
        ParseResult::Ok(module, leftover) => {
            let result = eval_module(module, input);
            ParseResult::Ok(result, leftover)
        }
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::NoMatch => ParseResult::NoMatch,
    }
}

/// Convenience constructor for an empty initial state — used by callers that
/// don't have a stream yet (e.g. the binary's CLI before reading the file).
#[smt_fn]
pub fn empty_state() -> State {
    State {
        stream: Seq::new(),
        cursor: Integer::from(0),
    }
}

/// `Cloak` is re-exported here so callers can build evaluation inputs without
/// also importing the stdlib path manually.
pub use rusmart_smt_stdlib::Cloak as RegoCloak;

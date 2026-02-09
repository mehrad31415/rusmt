//! A parser for the TOML v1.0.0 specification.

use crate::toml::{ast::Value, expr::parse_expression, table::recursive_merge_tables};
use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{
    Array, Boolean, Cloak, Error, Integer, Seq, String, smt::SMT,
};

/// array
mod array;
/// The AST for representing a parsed TOML document.
mod ast;
/// boolean
mod boolean;
/// datetime
mod datetime;
/// expression
mod expr;
/// float
mod float;
/// integer
mod integer;
/// key-value
mod key_value;
/// string
mod string;
/// table
mod table;

/// Represents the parser's input state: a sequence of characters, a cursor, and context.
#[smt_type]
pub struct State {
    /// The full sequence of characters being parsed.
    pub stream: Seq<String>,
    /// The current position (character index) in the stream.
    pub cursor: Integer,
    /// Context (semantics) of the parser
    pub context: ParserContext,
}

/// The result of a single parsing function.
#[smt_type]
pub enum ParseResult<T: SMT> {
    /// The parser did not find a match for its rule, but no error occurred.
    /// This allows the calling function to try a different parsing rule.
    NoMatch,
    /// The parser successfully matched and produced a value of type `T`,
    /// along with the remaining input stream to be passed to the next parser.
    Ok(T, State),
    /// The parser encountered an error.
    Err(Error),
}

/// An optional value type for the parser.
#[smt_type]
enum Optional<T: SMT> {
    None,
    Some(T),
}

/// is LF `newline = %x0A`
#[smt_fn]
fn is_lf_newline(input: State) -> Boolean {
    match current_char(input) {
        Optional::Some(c) => c.eq(String::from("\n")),
        Optional::None => Boolean::from(false),
    }
}

/// is CRLF `newline = %x0D.0A`
#[smt_fn]
fn is_crlf_newline(input: State) -> Boolean {
    let first_char = current_char(input);
    let second_char = peek(input, 1.into());
    match first_char {
        Optional::Some(c1) => match second_char {
            Optional::Some(c2) => c1.eq(String::from("\r")).and(c2.eq(String::from("\n"))),
            Optional::None => Boolean::from(false),
        },
        Optional::None => Boolean::from(false),
    }
}

/// `newline = %x0A / %x0D.0A` (LF / CRLF)
#[smt_fn]
fn is_newline(input: State) -> Boolean {
    is_lf_newline(input).or(is_crlf_newline(input))
}

/// parses a newline
#[smt_fn]
fn parse_newline(input: State) -> ParseResult<String> {
    if *is_newline(input) {
        if *is_lf_newline(input) {
            return ParseResult::Ok(String::from("\n"), advance(input));
        } else {
            // must be CRLF
            return ParseResult::Ok(String::from("\r\n"), advance(advance(input)));
        }
    } else {
        ParseResult::NoMatch
    }
}

/// Returns the character at the current cursor position.
#[smt_fn]
fn current_char(input: State) -> Optional<String> {
    if *input.cursor.lt(input.stream.length()) {
        return Optional::Some(input.stream.at(input.cursor));
    } else {
        // only indicates eof
        return Optional::None;
    }
}

/// Returns a new `State` state advanced by one character.
#[smt_fn]
fn advance(input: State) -> State {
    return State {
        stream: input.stream,
        cursor: input.cursor.add(1.into()),
        context: input.context,
    };
}

/// Peek ahead N characters
#[smt_fn]
fn peek(state: State, n: Integer) -> Optional<String> {
    let new_state = State {
        stream: state.stream,
        cursor: state.cursor.add(n),
        context: state.context,
    };
    current_char(new_state)
}

/// is Horizontal Tab (%x09)
#[smt_fn]
fn is_htab(c: String) -> Boolean {
    c.eq(String::from("\t"))
}

/// is Space (%x20)
#[smt_fn]
fn is_space(c: String) -> Boolean {
    c.eq(String::from(" "))
}

/// wschar = %x20 / %x09  (Space / Horizontal Tab)
#[smt_fn]
fn is_wschar(c: String) -> Boolean {
    is_space(c).or(is_htab(c))
}

/// parse wschar
/// wschar =  %x20  / %x09  ; Space / Horizontal tab
#[smt_fn]
fn parse_wschar(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_wschar(c) {
                ParseResult::Ok(c, advance(input))
            } else {
                // we expect a whitespace character but something else found
                ParseResult::NoMatch
            }
        }
        // we expect a whitespace character but no more input
        Optional::None => ParseResult::NoMatch,
    }
}

/// Parse zero or more whitespace characters: ws = *wschar
/// give the state after consuming all wschars
#[smt_fn]
fn parse_ws(input: State) -> State {
    match current_char(input) {
        Optional::Some(c) => {
            if *is_wschar(c) {
                let next_state = advance(input);
                parse_ws(next_state)
            } else {
                // no more wschar, return current state
                input
            }
        }
        // end of input reached, return current state
        Optional::None => input,
    }
}

/// `comment-start-symbol = %x23 ; #`
#[smt_fn]
fn is_comment_start_symbol(c: String) -> Boolean {
    c.eq(String::from("#"))
}

/// non-ascii = %x80-D7FF / %xE000-10FFFF
#[smt_fn]
fn is_non_ascii(c: String) -> Boolean {
    // Check character is >= U+0080 (first non-ASCII)
    let is_above_ascii = c.ge(String::from("\u{0080}"));

    // Check NOT in surrogate range (U+D800..U+DFFF)
    let not_surrogate = c
        .le(String::from("\u{D7FF}"))
        .or(c.ge(String::from("\u{E000}")));

    // Check <= U+10FFFF (max valid Unicode)
    let is_valid_unicode = c.le(String::from("\u{10FFFF}"));

    is_above_ascii.and(not_surrogate).and(is_valid_unicode)
}

/// `non-eol = %x09 / %x20-7F / non-ascii`
#[smt_fn]
fn is_non_eol(c: String) -> Boolean {
    c.eq(String::from("\t")) // %x09
        .or(c
            .ge(String::from("\u{0020}"))
            .and(c.le(String::from("\u{007E}")))) // %x20-7F
        .or(is_non_ascii(c)) // non-ascii
}

/// `comment = comment-start-symbol *non-eol`
#[smt_fn]
fn parse_comment(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_comment_start_symbol(c) {
                parse_comment_rest(String::from(""), advance(state))
            } else {
                // not a comment start (another symbol)
                ParseResult::NoMatch
            }
        }
        // we expect a '#' to start a comment (# missing)
        Optional::None => ParseResult::NoMatch,
    }
}

/// Helper function to parse the rest of a comment after the starting `#`.
#[smt_fn]
fn parse_comment_rest(acc: String, state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(c) => {
            if *is_non_eol(c) {
                parse_comment_rest(acc.concat(c), advance(state))
            } else {
                // reached eol
                if *is_newline(state) {
                    ParseResult::Ok(acc, state)
                } else {
                    // invalid character in comment
                    // println!("Invalid character in comment");
                    ParseResult::Err(Error::fresh())
                }
            }
        }
        // zero occurrences case
        Optional::None => ParseResult::Ok(acc, state),
    }
}

/// is alpha `A-Z / a-z`
#[smt_fn]
fn is_alpha(c: String) -> Boolean {
    let a_upper = String::from("A");
    let z_upper = String::from("Z");
    let a_lower = String::from("a");
    let z_lower = String::from("z");

    c.ge(a_upper)
        .and(c.le(z_upper))
        .or(c.ge(a_lower).and(c.le(z_lower)))
}

/// A character is a decimal digit (0-9).
#[smt_fn]
fn is_dec_digit(c: String) -> Boolean {
    c.ge("0".into()).and(c.le("9".into()))
}

/// is quotation-mark = %x22  (")
#[smt_fn]
fn is_quotation_mark(c: String) -> Boolean {
    c.eq(String::from("\""))
}

/// %x21 = "!"
#[smt_fn]
fn is_exclamation(c: String) -> Boolean {
    c.eq(String::from("!"))
}

/// %x27 = apostrophe (')
#[smt_fn]
pub(crate) fn is_apostrophe(c: String) -> Boolean {
    c.eq(String::from("'"))
}

/// Building block for keyword parsing.
#[smt_fn]
fn parse_literal(input: State, literal: String) -> ParseResult<String> {
    // We need a recursive helper to check each character of the literal.
    return parse_literal_recursive(input, literal, 0.into());
}

/// The recursive worker for `parse_literal`.
#[smt_fn]
fn parse_literal_recursive(
    input: State,
    literal: String,
    literal_cursor: Integer,
) -> ParseResult<String> {
    if *literal_cursor.eq(literal.length()) {
        return ParseResult::Ok(literal, input);
    } else {
        // Get the character from the literal we need to match.
        let expected_char = literal.at(literal_cursor);

        // Get the character from the input stream.
        match current_char(input) {
            Optional::None => {
                // the input stream ends early and this rule doesn't match.
                return ParseResult::NoMatch;
            }
            Optional::Some(actual_char) => {
                // Check the characters match.
                if *actual_char.eq(expected_char) {
                    // If they match, recurse on the next character of both the input and the literal.
                    return parse_literal_recursive(
                        advance(input),
                        literal,
                        literal_cursor.add(1.into()),
                    );
                } else {
                    // If they don't match, this rule doesn't match.
                    return ParseResult::NoMatch;
                }
            }
        }
    }
}

/// Context for the TOML parser to track current table and defined tables.
#[smt_type]
pub struct ParserContext {
    /// The current table path being parsed.
    pub current_table_path: Seq<String>,
    /// The set of explicitly defined tables like `[table]`.
    pub explicit_tables: Seq<Seq<String>>,
    /// The set of implicitly defined tables like `x` in x.y = 1.
    /// We need to track these because `[x.y]` followed by `[x]` is OK but `x.y=1` followed by `[x]` is an error.
    pub closed_tables: Seq<Seq<String>>,
    /// Inline tables like `{ key = "value" }`.
    pub inline_tables: Seq<Seq<String>>,
    /// Inline arrays like `[ value1, value2 ]`.
    pub inline_arrays: Seq<Seq<String>>,
    /// The set of array of tables like `[[table]]`.
    pub array_of_tables: Seq<Seq<String>>,
}

/// Creates a default parser context.
#[smt_fn]
pub fn default_parser_context() -> ParserContext {
    ParserContext {
        current_table_path: Seq::new(),
        explicit_tables: Seq::new(),
        closed_tables: Seq::new(),
        inline_tables: Seq::new(),
        inline_arrays: Seq::new(),
        array_of_tables: Seq::new(),
    }
}

/// Main entry point for parsing a TOML document.
///
/// `toml = expression *( newline expression )`
#[smt_fn]
pub fn parse_toml(state: State) -> ParseResult<Value> {
    let exp = parse_expression(state);
    match exp {
        ParseResult::Ok(first_value, new_state) => {
            match current_char(new_state) {
                // covering the zero occurrences case
                Optional::None => {
                    ParseResult::Ok(Value::Table(Cloak::shield(first_value)), new_state)
                }
                // if there is a character, we expect it to be a newline
                Optional::Some(_c) => parse_toml_loop(first_value, new_state),
            }
        }
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::NoMatch => ParseResult::NoMatch, // this should never happen
    }
}

/// `*( newline expression )` - helper function for `parse_toml`
#[smt_fn]
fn parse_toml_loop(acc: Array<String, Value>, state: State) -> ParseResult<Value> {
    match current_char(state) {
        // recursion base case: end of input
        Optional::None => ParseResult::Ok(Value::Table(Cloak::shield(acc)), state),
        // if there is a character, we expect it to be a newline
        Optional::Some(_c) => {
            match parse_newline(state) {
                ParseResult::Ok(_x, state_after_newline) => {
                    match parse_expression(state_after_newline) {
                        ParseResult::Ok(next_value, next_state) => {
                            let combined = recursive_merge_tables(acc, next_value);
                            match combined {
                                Optional::Some(merged_table) => {
                                    // Recurse to parse the next (newline expression)
                                    parse_toml_loop(merged_table, next_state)
                                }
                                Optional::None => {
                                    // println!("Error merging tables; type mismatch?");
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                        }
                        ParseResult::Err(e) => ParseResult::Err(e),
                        ParseResult::NoMatch => ParseResult::NoMatch, // an expression can be anything so no match cannot happen
                    }
                }
                ParseResult::NoMatch => {
                    // println!("Expected newline but found something else");
                    ParseResult::Err(Error::fresh()) // expected newline but not found (for this to be invoked the expression parsing needs to terminate complete)
                }
                ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
            }
        }
    }
}

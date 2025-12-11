//! Parsing boolean literals in TOML.

use crate::toml::{ParseResult, State, parse_literal};
use rusmart_smt_stdlib::{
    Boolean, Error,
    boolean::{FALSE, TRUE},
};

/// Parses a boolean literal (`true` or `false`).
/// ABNF: `boolean = "true" / "false"`
pub(crate) fn parse_boolean(input: State) -> ParseResult<Boolean> {
    // toml is case-sensitive
    match parse_literal(input, "True".into()) {
        ParseResult::Ok(_s, _new_state) => {
            // println!("invalid boolean True");
            ParseResult::Err(Error::fresh())
        } // invalid boolean
        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
        ParseResult::NoMatch => {
            match parse_literal(input, "TRUE".into()) {
                ParseResult::Ok(_s, _new_state) => {
                    // println!("invalid boolean TRUE");
                    ParseResult::Err(Error::fresh())
                } // invalid boolean
                ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                ParseResult::NoMatch => {
                    match parse_literal(input, "true".into()) {
                        ParseResult::Ok(_true, remaining_input) => {
                            // If successful, return the boolean value.
                            ParseResult::Ok(TRUE, remaining_input)
                        }
                        ParseResult::Err(e) => ParseResult::Err(e),
                        // If it didn't match "true", try to parse "false".
                        ParseResult::NoMatch => {
                            match parse_literal(input, "False".into()) {
                                ParseResult::Ok(_s, _new_state) => {
                                    // println!("invalid boolean False");
                                    ParseResult::Err(Error::fresh())
                                }
                                ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                ParseResult::NoMatch => {
                                    match parse_literal(input, "FALSE".into()) {
                                        ParseResult::Ok(_s, _new_state) => {
                                            // println!("invalid boolean FALSE");
                                            ParseResult::Err(Error::fresh()) // invalid boolean
                                        }
                                        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                        ParseResult::NoMatch => {
                                            match parse_literal(input, "false".into()) {
                                                ParseResult::Ok(_false, remaining_input) => {
                                                    // If successful, return the boolean value.
                                                    ParseResult::Ok(FALSE, remaining_input)
                                                }
                                                ParseResult::Err(e) => ParseResult::Err(e),
                                                // If it didn't match "false", return NoMatch.
                                                ParseResult::NoMatch => ParseResult::NoMatch,
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

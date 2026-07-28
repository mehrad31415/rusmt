//! Parsing boolean literals in TOML.

use crate::toml::{ParseResult, State, parse_literal};
use rusmt_smt_remark_derive::smt_fn;
use rusmt_smt_stdlib::{Boolean, Path, String};

/// Parses a boolean literal (`true` or `false`).
/// ABNF: `boolean = "true" / "false"`
#[smt_fn]
pub(crate) fn parse_boolean(input: State) -> ParseResult<Boolean> {
    // toml is case-sensitive
    match parse_literal(input, "True".into()) {
        ParseResult::Ok(_s, _new_state) => {
            // println!("invalid boolean True");
            ParseResult::Err(Path::named(String::from("boolean_invalid_capital_true")))
        } // invalid boolean
        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
        ParseResult::NoMatch => {
            match parse_literal(input, "TRUE".into()) {
                ParseResult::Ok(_s, _new_state) => {
                    // println!("invalid boolean TRUE");
                    ParseResult::Err(Path::named(String::from("boolean_invalid_allcaps_true")))
                } // invalid boolean
                ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                ParseResult::NoMatch => {
                    match parse_literal(input, "true".into()) {
                        ParseResult::Ok(_true, remaining_input) => {
                            // If successful, return the boolean value.
                            ParseResult::Ok(Boolean::from(true), remaining_input)
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        // If it didn't match "true", try to parse "false".
                        ParseResult::NoMatch => {
                            match parse_literal(input, "False".into()) {
                                ParseResult::Ok(_s, _new_state) => {
                                    // println!("invalid boolean False");
                                    ParseResult::Err(Path::named(String::from(
                                        "boolean_invalid_capital_false",
                                    )))
                                }
                                ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                ParseResult::NoMatch => {
                                    match parse_literal(input, "FALSE".into()) {
                                        ParseResult::Ok(_s, _new_state) => {
                                            // println!("invalid boolean FALSE");
                                            ParseResult::Err(Path::named(String::from(
                                                "boolean_invalid",
                                            ))) // invalid boolean
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                        ParseResult::NoMatch => {
                                            match parse_literal(input, "false".into()) {
                                                ParseResult::Ok(_false, remaining_input) => {
                                                    // If successful, return the boolean value.
                                                    ParseResult::Ok(
                                                        Boolean::from(false),
                                                        remaining_input,
                                                    )
                                                }
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                // If it didn't match "false", return NoMatch.
                                                ParseResult::NoMatch => {
                                                    return ParseResult::NoMatch;
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

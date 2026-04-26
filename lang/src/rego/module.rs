//! Module / package parsing for the Rego subset.
//!
//! Spec (modules): <https://www.openpolicyagent.org/docs/policy-language/#modules>
//! Spec (package clause): <https://www.openpolicyagent.org/docs/policy-language/#packages>
//!
//! The subset only accepts a single module per file with one `package` clause
//! at the top, optionally preceded by whitespace and comments. `import`
//! statements are out of scope and rejected at parse time.

use crate::rego::{
    Optional, ParseResult, State, advance,
    ast::{Module, Rule},
    current_char,
    literal::next_is_ident_cont,
    parse_ident, parse_literal, parse_ws, parse_ws_nl,
    rule::parse_rule,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Error, Seq, String, smt::SMT};

/// Parse a complete Rego module: package clause followed by zero or more rules.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#modules>
#[smt_fn]
pub(crate) fn parse_module(state: State) -> ParseResult<Module> {
    let after_ws = parse_ws_nl(state);
    match parse_package_clause(after_ws) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // ERROR: module without a package clause
            return ParseResult::Err(Error::fresh());
        }
        ParseResult::Ok(pkg, after_pkg) => {
            // Reject `import` statements explicitly (out of scope).
            let after_pkg_ws = parse_ws_nl(after_pkg);
            match parse_literal(after_pkg_ws, String::from("import")) {
                ParseResult::Ok(_, after_imp) => {
                    if !*next_is_ident_cont(after_imp) {
                        // ERROR: `import` is out of scope; the subset only
                        // supports a single self-contained module.
                        return ParseResult::Err(Error::fresh());
                    }
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {}
            }
            parse_rules_loop(after_pkg_ws, Seq::<Rule>::new(), pkg)
        }
    }
}

/// `package <ident> ("." <ident>)*`
#[smt_fn]
fn parse_package_clause(state: State) -> ParseResult<Seq<String>> {
    match parse_literal(state, String::from("package")) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(_kw, after_kw) => {
            if *next_is_ident_cont(after_kw) {
                // The lookahead character is an identifier-continuation, so
                // the prefix "package" was actually the start of an
                // identifier (e.g. "packagename") — not the keyword.
                return ParseResult::NoMatch;
            }
            let after_ws = parse_ws(after_kw);
            match parse_ident(after_ws) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    // ERROR: `package` keyword without an identifier
                    return ParseResult::Err(Error::fresh());
                }
                ParseResult::Ok(first, after_first) => {
                    let acc = Seq::<String>::new().append(first);
                    parse_package_path_tail(after_first, acc)
                }
            }
        }
    }
}

/// Continuation of the package path (`. ident . ident ...`).
#[smt_fn]
fn parse_package_path_tail(state: State, acc: Seq<String>) -> ParseResult<Seq<String>> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from(".")) {
                let after_dot = advance(state);
                match parse_ident(after_dot) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // ERROR: trailing `.` in package path
                        return ParseResult::Err(Error::fresh());
                    }
                    ParseResult::Ok(seg, after_seg) => {
                        parse_package_path_tail(after_seg, acc.append(seg))
                    }
                }
            } else {
                ParseResult::Ok(acc, state)
            }
        }
        Optional::None => ParseResult::Ok(acc, state),
    }
}

/// Loop over rules at the top of the module.
#[smt_fn]
fn parse_rules_loop(
    state: State,
    acc: Seq<Rule>,
    pkg: Seq<String>,
) -> ParseResult<Module> {
    let after_ws = parse_ws_nl(state);
    match current_char(after_ws) {
        Optional::None => ParseResult::Ok(
            Module {
                package: pkg,
                rules: acc,
            },
            after_ws,
        ),
        Optional::Some(_c) => match parse_rule(after_ws) {
            ParseResult::Err(e) => return ParseResult::Err(e),
            ParseResult::NoMatch => {
                // ERROR: junk between rules — neither end-of-input nor a rule head
                return ParseResult::Err(Error::fresh());
            }
            ParseResult::Ok(r, after_rule) => {
                parse_rules_loop(after_rule, acc.append(r), pkg)
            }
        },
    }
}

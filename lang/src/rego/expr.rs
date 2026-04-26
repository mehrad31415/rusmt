//! Expression parsing for the Rego subset.
//!
//! Spec (rule bodies / expressions): <https://www.openpolicyagent.org/docs/policy-language/#rules>
//! Spec (comparisons): <https://www.openpolicyagent.org/docs/policy-reference/#comparison>
//! Spec (assignment vs unification): <https://www.openpolicyagent.org/docs/policy-language/#assignment-and-equality>

use crate::rego::{
    Optional, ParseResult, State, advance,
    ast::{Body, CompareOp, Expr},
    current_char,
    literal::next_is_ident_cont,
    parse_ident, parse_literal, parse_ws, parse_ws_nl,
    term::parse_term,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Cloak, Error, Seq, String, smt::SMT};

/// Parse a single expression from the rule body.
///
/// Grammar (subset):
/// ```text
/// expr = "not" wschar+ expr
///      / ident ws ":=" ws expr        (single-assignment binding)
///      / term ws cmp-op ws term
///      / term                          (truth test / fact)
/// cmp-op = "==" / "!=" / "<=" / ">=" / "<" / ">"
/// ```
#[smt_fn]
pub(crate) fn parse_expr(state: State) -> ParseResult<Expr> {
    // Reject out-of-scope `every` / `some` / `with` immediately.
    match parse_literal(state, String::from("every")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                // Out-of-scope: `every` is not in the subset.
                return ParseResult::NoMatch;
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("some")) {
        ParseResult::Ok(_, after) => {
            if !*next_is_ident_cont(after) {
                return ParseResult::NoMatch;
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    // `not <expr>`
    match parse_literal(state, String::from("not")) {
        ParseResult::Ok(_, after_kw) => {
            if !*next_is_ident_cont(after_kw) {
                let after_ws = parse_ws(after_kw);
                match parse_expr(after_ws) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // ERROR: `not` keyword without a following expression
                        return ParseResult::Err(Error::fresh());
                    }
                    ParseResult::Ok(inner, ns) => {
                        return ParseResult::Ok(Expr::Not(Cloak::shield(inner)), ns);
                    }
                }
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    // Try `ident := expr` (assignment) vs comparison vs bare term.
    // We attempt assignment by snapshotting the state, parsing an ident, and
    // looking ahead for `:=`. If anything fails we fall through to the
    // term/comparison path.
    match parse_assignment(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(e, ns) => return ParseResult::Ok(e, ns),
        ParseResult::NoMatch => parse_compare_or_term(state),
    }
}

/// Try to parse `ident ws ":=" ws expr` as an assignment expression.
///
/// Returns `NoMatch` if the prefix doesn't form an assignment, allowing
/// the caller to try a comparison or bare term.
#[smt_fn]
fn parse_assignment(state: State) -> ParseResult<Expr> {
    match parse_ident(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(name, after_name) => {
            let after_ws = parse_ws(after_name);
            // Look ahead for `:=`.
            match current_char(after_ws) {
                Optional::Some(c1) => {
                    if *c1.eq(String::from(":")) {
                        match current_char(advance(after_ws)) {
                            Optional::Some(c2) => {
                                if *c2.eq(String::from("=")) {
                                    let after_op = parse_ws(advance(advance(after_ws)));
                                    match parse_term(after_op) {
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::NoMatch => {
                                            // ERROR: `:=` not followed by a term
                                            return ParseResult::Err(Error::fresh());
                                        }
                                        ParseResult::Ok(t, ns) => {
                                            return ParseResult::Ok(Expr::Assign(name, t), ns);
                                        }
                                    }
                                } else {
                                    return ParseResult::NoMatch;
                                }
                            }
                            Optional::None => return ParseResult::NoMatch,
                        }
                    } else {
                        return ParseResult::NoMatch;
                    }
                }
                Optional::None => return ParseResult::NoMatch,
            }
        }
    }
}

/// Parse `term (ws cmp-op ws term)?` — a comparison or a bare term used as
/// a truth test.
#[smt_fn]
fn parse_compare_or_term(state: State) -> ParseResult<Expr> {
    match parse_term(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(lhs, after_lhs) => {
            let after_ws = parse_ws(after_lhs);
            match parse_compare_op(after_ws) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    // No comparison — bare term used as truth test / fact.
                    ParseResult::Ok(Expr::Term(lhs), after_lhs)
                }
                ParseResult::Ok(op, after_op) => {
                    let after_op_ws = parse_ws(after_op);
                    match parse_term(after_op_ws) {
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::NoMatch => {
                            // ERROR: comparison operator with no RHS term
                            return ParseResult::Err(Error::fresh());
                        }
                        ParseResult::Ok(rhs, after_rhs) => {
                            ParseResult::Ok(Expr::Compare(op, lhs, rhs), after_rhs)
                        }
                    }
                }
            }
        }
    }
}

/// Parse a comparison operator. Try the two-character forms first so that
/// `<=` does not get mis-classified as `<` followed by `=`.
#[smt_fn]
fn parse_compare_op(state: State) -> ParseResult<CompareOp> {
    match parse_literal(state, String::from("==")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Eq, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("!=")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Ne, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("<=")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Le, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from(">=")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Ge, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from("<")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Lt, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {}
    }
    match parse_literal(state, String::from(">")) {
        ParseResult::Ok(_, ns) => return ParseResult::Ok(CompareOp::Gt, ns),
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
    }
}

/// Parse a rule body inside `{ ... }`: zero or more expressions separated by
/// newline or `;`. The opening `{` and closing `}` are NOT consumed here —
/// the caller handles them so this function can be reused for shorthand.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_fn]
pub(crate) fn parse_body(state: State) -> ParseResult<Body> {
    let after_ws = parse_ws_nl(state);
    match current_char(after_ws) {
        Optional::None => {
            // ERROR: expected at least one expression in body
            return ParseResult::Err(Error::fresh());
        }
        Optional::Some(c) => {
            if *c.eq(String::from("}")) {
                // ERROR: empty body `{ }` is forbidden by the spec —
                // a rule body must have at least one expression.
                return ParseResult::Err(Error::fresh());
            } else {
                parse_body_loop(after_ws, Seq::<Expr>::new())
            }
        }
    }
}

/// Loop body for [`parse_body`]: parse one expression, then either `;`,
/// newline, or `}` (delegated to the caller).
#[smt_fn]
fn parse_body_loop(state: State, acc: Seq<Expr>) -> ParseResult<Body> {
    match parse_expr(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // ERROR: expected an expression but none matched
            return ParseResult::Err(Error::fresh());
        }
        ParseResult::Ok(e, after_expr) => {
            let new_acc = acc.append(e);
            let after_ws = parse_ws(after_expr);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: end of input mid-body (no `}` ever found)
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(c) => {
                    if *c.eq(String::from("}")) {
                        ParseResult::Ok(Body { exprs: new_acc }, after_ws)
                    } else {
                        if *c.eq(String::from(";")) {
                            let after_sep = parse_ws_nl(advance(after_ws));
                            match current_char(after_sep) {
                                Optional::Some(cc) => {
                                    if *cc.eq(String::from("}")) {
                                        ParseResult::Ok(Body { exprs: new_acc }, after_sep)
                                    } else {
                                        parse_body_loop(after_sep, new_acc)
                                    }
                                }
                                Optional::None => {
                                    // ERROR: end of input after `;`
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                        } else {
                            // Could be a newline. parse_ws didn't consume newlines.
                            if *c.eq(String::from("\n")) || *c.eq(String::from("\r")) {
                                let after_sep = parse_ws_nl(after_ws);
                                match current_char(after_sep) {
                                    Optional::Some(cc) => {
                                        if *cc.eq(String::from("}")) {
                                            ParseResult::Ok(Body { exprs: new_acc }, after_sep)
                                        } else {
                                            parse_body_loop(after_sep, new_acc)
                                        }
                                    }
                                    Optional::None => {
                                        // ERROR: end of input after newline
                                        return ParseResult::Err(Error::fresh());
                                    }
                                }
                            } else {
                                // ERROR: missing separator (`;` or newline) between expressions
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                }
            }
        }
    }
}


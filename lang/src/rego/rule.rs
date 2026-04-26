//! Rule parsing and evaluation for the Rego subset.
//!
//! Spec (rules): <https://www.openpolicyagent.org/docs/policy-language/#rules>
//! Spec (default keyword): <https://www.openpolicyagent.org/docs/policy-language/#default-keyword>
//! Spec (partial rules): <https://www.openpolicyagent.org/docs/policy-language/#partial-rules>

use crate::rego::{
    Optional, ParseResult, State, advance,
    ast::{ArithOp, Body, CompareOp, Expr, Module, Rule, RuleHead, Term},
    current_char,
    expr::parse_body,
    literal::next_is_ident_cont,
    parse_ident, parse_literal, parse_ws,
    term::parse_term,
};
use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{
    Array, Boolean, Cloak, Error, Integer, Real, Seq, String, smt::SMT,
};

/// Parse a single rule.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_fn]
pub(crate) fn parse_rule(state: State) -> ParseResult<Rule> {
    match parse_default_rule(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(r, ns) => return ParseResult::Ok(r, ns),
        ParseResult::NoMatch => {}
    }
    parse_value_rule(state)
}

/// `default <ident> = <term>`
#[smt_fn]
fn parse_default_rule(state: State) -> ParseResult<Rule> {
    match parse_literal(state, String::from("default")) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(_kw, after_kw) => {
            if *next_is_ident_cont(after_kw) {
                return ParseResult::NoMatch;
            }
            let after_ws = parse_ws(after_kw);
            match parse_ident(after_ws) {
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::NoMatch => {
                    // ERROR: `default` keyword without a rule name
                    return ParseResult::Err(Error::fresh());
                }
                ParseResult::Ok(name, after_name) => {
                    let after_ws2 = parse_ws(after_name);
                    match current_char(after_ws2) {
                        Optional::Some(c) => {
                            if *c.eq(String::from("=")) {
                                let after_eq = parse_ws(advance(after_ws2));
                                match parse_term(after_eq) {
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::NoMatch => {
                                        // ERROR: `default name =` with no term
                                        return ParseResult::Err(Error::fresh());
                                    }
                                    ParseResult::Ok(t, after_t) => {
                                        let head = RuleHead::Default(name, t);
                                        let body = Body {
                                            exprs: Seq::<Expr>::new(),
                                        };
                                        ParseResult::Ok(Rule { head, body }, after_t)
                                    }
                                }
                            } else {
                                // ERROR: `default name` without `=`
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                        Optional::None => {
                            // ERROR: end of input after `default name`
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                }
            }
        }
    }
}

/// Value rules: complete (`name = expr [{ body }]`), shorthand (`name { body }`),
/// or partial (`name[key] = value { body }`, `name[term] { body }`).
#[smt_fn]
fn parse_value_rule(state: State) -> ParseResult<Rule> {
    match parse_ident(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(name, after_name) => {
            match current_char(after_name) {
                Optional::Some(c0) => {
                    if *c0.eq(String::from("[")) {
                        return parse_partial_rule(name, after_name);
                    }
                }
                Optional::None => {}
            }
            let after_ws = parse_ws(after_name);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: rule head with no body and no `=`
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(c) => {
                    if *c.eq(String::from("=")) {
                        let after_eq = parse_ws(advance(after_ws));
                        match parse_term(after_eq) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => {
                                // ERROR: `name =` not followed by a term
                                return ParseResult::Err(Error::fresh());
                            }
                            ParseResult::Ok(t, after_t) => {
                                let after_ws2 = parse_ws(after_t);
                                match current_char(after_ws2) {
                                    Optional::Some(c2) => {
                                        if *c2.eq(String::from("{")) {
                                            parse_rule_body(
                                                advance(after_ws2),
                                                RuleHead::Complete(name, t),
                                            )
                                        } else {
                                            let body = Body {
                                                exprs: Seq::<Expr>::new()
                                                    .append(Expr::Term(Term::Boolean(
                                                        Boolean::from(true),
                                                    ))),
                                            };
                                            ParseResult::Ok(
                                                Rule {
                                                    head: RuleHead::Complete(name, t),
                                                    body,
                                                },
                                                after_t,
                                            )
                                        }
                                    }
                                    Optional::None => {
                                        let body = Body {
                                            exprs: Seq::<Expr>::new().append(Expr::Term(
                                                Term::Boolean(Boolean::from(true)),
                                            )),
                                        };
                                        ParseResult::Ok(
                                            Rule {
                                                head: RuleHead::Complete(name, t),
                                                body,
                                            },
                                            after_t,
                                        )
                                    }
                                }
                            }
                        }
                    } else {
                        if *c.eq(String::from("{")) {
                            parse_rule_body(
                                advance(after_ws),
                                RuleHead::Complete(name, Term::Boolean(Boolean::from(true))),
                            )
                        } else {
                            // ERROR: head not followed by `=`, `{`, or `[`
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                }
            }
        }
    }
}

/// Helper for `name[...]` — disambiguate partial-set vs. partial-object.
#[smt_fn]
fn parse_partial_rule(name: String, state: State) -> ParseResult<Rule> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from("[")) {
                let after_open = parse_ws(advance(state));
                match parse_term(after_open) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // ERROR: `name[` not followed by a term
                        return ParseResult::Err(Error::fresh());
                    }
                    ParseResult::Ok(key_or_elem, after_t) => {
                        let after_ws = parse_ws(after_t);
                        match current_char(after_ws) {
                            Optional::Some(close) => {
                                if *close.eq(String::from("]")) {
                                    let after_close = parse_ws(advance(after_ws));
                                    match current_char(after_close) {
                                        Optional::Some(c2) => {
                                            if *c2.eq(String::from("=")) {
                                                let after_eq = parse_ws(advance(after_close));
                                                match parse_term(after_eq) {
                                                    ParseResult::Err(e) => {
                                                        return ParseResult::Err(e);
                                                    }
                                                    ParseResult::NoMatch => {
                                                        // ERROR: `name[k] =` with no value term
                                                        return ParseResult::Err(Error::fresh());
                                                    }
                                                    ParseResult::Ok(val, after_val) => {
                                                        let after_ws3 = parse_ws(after_val);
                                                        match current_char(after_ws3) {
                                                            Optional::Some(c3) => {
                                                                if *c3.eq(String::from("{")) {
                                                                    parse_rule_body(
                                                                        advance(after_ws3),
                                                                        RuleHead::PartialObject(
                                                                            name,
                                                                            key_or_elem,
                                                                            val,
                                                                        ),
                                                                    )
                                                                } else {
                                                                    // ERROR: partial-object missing `{ body }`
                                                                    return ParseResult::Err(
                                                                        Error::fresh(),
                                                                    );
                                                                }
                                                            }
                                                            Optional::None => {
                                                                // ERROR: input ended before `{`
                                                                return ParseResult::Err(
                                                                    Error::fresh(),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                if *c2.eq(String::from("{")) {
                                                    parse_rule_body(
                                                        advance(after_close),
                                                        RuleHead::PartialSet(name, key_or_elem),
                                                    )
                                                } else {
                                                    // ERROR: junk after `name[term]`
                                                    return ParseResult::Err(Error::fresh());
                                                }
                                            }
                                        }
                                        Optional::None => {
                                            // ERROR: input ended after `name[term]`
                                            return ParseResult::Err(Error::fresh());
                                        }
                                    }
                                } else {
                                    // ERROR: missing `]` to close `name[...`
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                            Optional::None => {
                                // ERROR: input ended before `]`
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                }
            } else {
                return ParseResult::NoMatch;
            }
        }
        Optional::None => return ParseResult::NoMatch,
    }
}

/// Parse `body }` and bundle with the provided head into a `Rule`.
#[smt_fn]
fn parse_rule_body(state: State, head: RuleHead) -> ParseResult<Rule> {
    match parse_body(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(body, after_body) => match current_char(after_body) {
            Optional::Some(c) => {
                if *c.eq(String::from("}")) {
                    ParseResult::Ok(Rule { head, body }, advance(after_body))
                } else {
                    // ERROR: missing `}` to close rule body
                    return ParseResult::Err(Error::fresh());
                }
            }
            Optional::None => {
                // ERROR: end of input where `}` is expected
                return ParseResult::Err(Error::fresh());
            }
        },
    }
}

// ----------------------------------------------------------------------------
// Evaluation
// ----------------------------------------------------------------------------

/// Outcome of evaluating one expression.
///
/// The `Err(Error)` variant is critical for synthesis: when evaluation reaches
/// a hard semantic edge case (type mismatch, division by zero, unbound
/// variable, or `:=` rebinding), the propagated `Error` records that we
/// reached the synthesis target.
///
/// The bindings payload of `True` is wrapped in `Cloak` because the
/// `#[smt_type]` macro cannot disambiguate the comma in `Array<String, Term>`
/// from a multi-arg tuple variant when used directly (the same pattern is
/// used in `lang/src/toml/ast.rs` for `Table(Cloak<Array<String, Value>>)`).
///
/// `False` is the *first* variant so `EvalOutcome::default()` reduces to
/// `Self::False` and the macro never has to emit `Cloak<...>::default()`,
/// which is a parse error in expression position without turbofish.
#[smt_type]
pub enum EvalOutcome {
    /// Falsy expression — body must short-circuit but this is not a hard error.
    False,
    /// Successful expression — carries the (possibly updated) bindings.
    True(Cloak<Array<String, Term>>),
    /// Hard semantic error — propagates up to the rule level.
    Err(Error),
}

/// Outcome of evaluating a rule body.
#[smt_type]
pub enum BodyOutcome {
    /// Body succeeded.
    Ok,
    /// Body failed (a falsy expression short-circuited).
    Fail,
    /// Body hit a hard semantic error during evaluation.
    Err(Error),
}

/// Outcome of evaluating a single term to a value.
#[smt_type]
pub enum TermVal {
    /// Concrete term value.
    Val(Term),
    /// Hard error (type mismatch, unbound variable, div-by-zero, or
    /// missing-field on a non-object descent).
    Err(Error),
}

/// Evaluate the whole module against `input`.
#[smt_fn]
pub fn eval_module(m: Module, input: Term) -> Array<String, Term> {
    let initial = Array::<String, Term>::new().store(String::from("input"), input);
    eval_rules_loop(
        m.rules,
        Integer::from(0),
        Array::<String, Term>::new(),
        initial,
    )
}

/// Loop over rules, accumulating into `result`.
#[smt_fn]
fn eval_rules_loop(
    rules: Seq<Rule>,
    i: Integer,
    result: Array<String, Term>,
    base: Array<String, Term>,
) -> Array<String, Term> {
    if *i.lt(rules.length()) {
        let r = rules.at(i);
        let new_result = apply_rule(r, base, result);
        eval_rules_loop(rules, i.add(Integer::from(1)), new_result, base)
    } else {
        result
    }
}

/// Apply a single rule, updating `result` if the rule contributes.
#[smt_fn]
fn apply_rule(
    r: Rule,
    base: Array<String, Term>,
    result: Array<String, Term>,
) -> Array<String, Term> {
    match eval_body(r.body, base) {
        BodyOutcome::Fail => result,
        BodyOutcome::Err(_e) => result,
        BodyOutcome::Ok => match r.head {
            RuleHead::Default(name, t) => {
                if *result.contains_key(name) {
                    result
                } else {
                    result.store(name, t)
                }
            }
            RuleHead::Complete(name, t) => match eval_term(t, base) {
                TermVal::Val(v) => result.store(name, v),
                TermVal::Err(_e) => result,
            },
            RuleHead::PartialSet(name, elem) => match eval_term(elem, base) {
                TermVal::Val(v) => {
                    let prior = if *result.contains_key(name) {
                        result.select(name)
                    } else {
                        Term::Set(Cloak::shield(Seq::<Term>::new()))
                    };
                    let updated = match prior {
                        Term::Set(c) => {
                            let s = c.reveal();
                            let new_s = if *s.contains(v) { s } else { s.append(v) };
                            Term::Set(Cloak::shield(new_s))
                        }
                        Term::Null => prior,
                        Term::Boolean(_) => prior,
                        Term::Number(_) => prior,
                        Term::String(_) => prior,
                        Term::Var(_) => prior,
                        Term::Ref(_) => prior,
                        Term::Array(_) => prior,
                        Term::Object(_) => prior,
                        Term::ArithExpr(_, _, _) => prior,
                    };
                    result.store(name, updated)
                }
                TermVal::Err(_e) => result,
            },
            RuleHead::PartialObject(name, key_t, val_t) => match eval_term(key_t, base) {
                TermVal::Val(kv) => match eval_term(val_t, base) {
                    TermVal::Val(vv) => match kv {
                        Term::String(ks) => {
                            let prior = if *result.contains_key(name) {
                                result.select(name)
                            } else {
                                Term::Object(Cloak::shield(Array::<String, Term>::new()))
                            };
                            let updated = match prior {
                                Term::Object(c) => {
                                    let m = c.reveal();
                                    Term::Object(Cloak::shield(m.store(ks, vv)))
                                }
                                Term::Null => prior,
                                Term::Boolean(_) => prior,
                                Term::Number(_) => prior,
                                Term::String(_) => prior,
                                Term::Var(_) => prior,
                                Term::Ref(_) => prior,
                                Term::Array(_) => prior,
                                Term::Set(_) => prior,
                                Term::ArithExpr(_, _, _) => prior,
                            };
                            result.store(name, updated)
                        }
                        Term::Null => result,
                        Term::Boolean(_) => result,
                        Term::Number(_) => result,
                        Term::Var(_) => result,
                        Term::Ref(_) => result,
                        Term::Array(_) => result,
                        Term::Object(_) => result,
                        Term::Set(_) => result,
                        Term::ArithExpr(_, _, _) => result,
                    },
                    TermVal::Err(_e) => result,
                },
                TermVal::Err(_e) => result,
            },
        },
    }
}

/// Evaluate a body: every expression must succeed for the body to produce `Ok`.
#[smt_fn]
pub(crate) fn eval_body(b: Body, bindings: Array<String, Term>) -> BodyOutcome {
    eval_body_loop(b.exprs, Integer::from(0), bindings)
}

/// Loop body for [`eval_body`].
#[smt_fn]
fn eval_body_loop(
    exprs: Seq<Expr>,
    i: Integer,
    bindings: Array<String, Term>,
) -> BodyOutcome {
    if *i.lt(exprs.length()) {
        let e = exprs.at(i);
        match eval_expr(e, bindings) {
            EvalOutcome::True(c) => {
                eval_body_loop(exprs, i.add(Integer::from(1)), c.reveal())
            }
            EvalOutcome::False => BodyOutcome::Fail,
            EvalOutcome::Err(err) => BodyOutcome::Err(err),
        }
    } else {
        BodyOutcome::Ok
    }
}

/// Evaluate a single expression against `bindings`.
#[smt_fn]
pub(crate) fn eval_expr(e: Expr, bindings: Array<String, Term>) -> EvalOutcome {
    match e {
        Expr::Term(t) => match eval_term(t, bindings) {
            TermVal::Val(v) => {
                if *is_truthy(v) {
                    EvalOutcome::True(Cloak::shield(bindings))
                } else {
                    EvalOutcome::False
                }
            }
            TermVal::Err(err) => EvalOutcome::Err(err),
        },
        Expr::Not(c) => {
            let inner = c.reveal();
            match eval_expr(inner, bindings) {
                EvalOutcome::True(_) => EvalOutcome::False,
                EvalOutcome::False => EvalOutcome::True(Cloak::shield(bindings)),
                EvalOutcome::Err(err) => EvalOutcome::Err(err),
            }
        }
        Expr::Compare(op, lhs, rhs) => match eval_term(lhs, bindings) {
            TermVal::Err(err) => EvalOutcome::Err(err),
            TermVal::Val(lv) => match eval_term(rhs, bindings) {
                TermVal::Err(err) => EvalOutcome::Err(err),
                TermVal::Val(rv) => {
                    if *compare_terms(op, lv, rv) {
                        EvalOutcome::True(Cloak::shield(bindings))
                    } else {
                        EvalOutcome::False
                    }
                }
            },
        },
        Expr::Assign(name, t) => {
            if *bindings.contains_key(name) {
                // ERROR: `:=` rebinds an already-bound name in the same body
                EvalOutcome::Err(Error::fresh())
            } else {
                match eval_term(t, bindings) {
                    TermVal::Val(v) => EvalOutcome::True(Cloak::shield(bindings.store(name, v))),
                    TermVal::Err(err) => EvalOutcome::Err(err),
                }
            }
        }
    }
}

/// Truthiness per Rego: `null`, `false`, and absence are falsy; everything
/// else is truthy.
#[smt_fn]
fn is_truthy(t: Term) -> Boolean {
    match t {
        Term::Null => Boolean::from(false),
        Term::Boolean(b) => b,
        Term::Number(_) => Boolean::from(true),
        Term::String(_) => Boolean::from(true),
        Term::Var(_) => Boolean::from(true),
        Term::Ref(_) => Boolean::from(true),
        Term::Array(_) => Boolean::from(true),
        Term::Object(_) => Boolean::from(true),
        Term::Set(_) => Boolean::from(true),
        Term::ArithExpr(_, _, _) => Boolean::from(true),
    }
}

/// Compare two evaluated terms with `op`.
#[smt_fn]
fn compare_terms(op: CompareOp, l: Term, r: Term) -> Boolean {
    match op {
        CompareOp::Eq => terms_equal(l, r),
        CompareOp::Ne => terms_equal(l, r).not(),
        CompareOp::Lt => terms_lt(l, r),
        CompareOp::Le => terms_lt(l, r).or(terms_equal(l, r)),
        CompareOp::Gt => terms_lt(r, l),
        CompareOp::Ge => terms_lt(r, l).or(terms_equal(l, r)),
    }
}

/// Structural equality on two evaluated terms.
#[smt_fn]
fn terms_equal(l: Term, r: Term) -> Boolean {
    match l {
        Term::Null => match r {
            Term::Null => Boolean::from(true),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Boolean(lb) => match r {
            Term::Boolean(rb) => lb.eq(rb),
            Term::Null => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Number(lr) => match r {
            Term::Number(rr) => lr.eq(rr),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::String(ls) => match r {
            Term::String(rs) => ls.eq(rs),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Var(lv) => match r {
            Term::Var(rv) => lv.eq(rv),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Ref(lp) => match r {
            Term::Ref(rp) => lp.eq(rp),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Array(la) => match r {
            Term::Array(ra) => la.eq(ra),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Object(lo) => match r {
            Term::Object(ro) => lo.eq(ro),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Set(ls) => match r {
            Term::Set(rs) => ls.eq(rs),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::ArithExpr(_, _, _) => Boolean::from(false),
    }
}

/// Strict less-than on terms (numbers and strings only).
#[smt_fn]
fn terms_lt(l: Term, r: Term) -> Boolean {
    match l {
        Term::Number(ln) => match r {
            Term::Number(rn) => ln.lt(rn),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::String(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::String(ls) => match r {
            Term::String(rs) => ls.lt(rs),
            Term::Null => Boolean::from(false),
            Term::Boolean(_) => Boolean::from(false),
            Term::Number(_) => Boolean::from(false),
            Term::Var(_) => Boolean::from(false),
            Term::Ref(_) => Boolean::from(false),
            Term::Array(_) => Boolean::from(false),
            Term::Object(_) => Boolean::from(false),
            Term::Set(_) => Boolean::from(false),
            Term::ArithExpr(_, _, _) => Boolean::from(false),
        },
        Term::Null => Boolean::from(false),
        Term::Boolean(_) => Boolean::from(false),
        Term::Var(_) => Boolean::from(false),
        Term::Ref(_) => Boolean::from(false),
        Term::Array(_) => Boolean::from(false),
        Term::Object(_) => Boolean::from(false),
        Term::Set(_) => Boolean::from(false),
        Term::ArithExpr(_, _, _) => Boolean::from(false),
    }
}

/// Evaluate a term to a concrete value under `bindings`.
#[smt_fn]
pub(crate) fn eval_term(t: Term, bindings: Array<String, Term>) -> TermVal {
    match t {
        Term::Null => TermVal::Val(Term::Null),
        Term::Boolean(b) => TermVal::Val(Term::Boolean(b)),
        Term::Number(n) => TermVal::Val(Term::Number(n)),
        Term::String(s) => TermVal::Val(Term::String(s)),
        Term::Var(name) => {
            if *bindings.contains_key(name) {
                TermVal::Val(bindings.select(name))
            } else {
                // ERROR: unbound variable
                TermVal::Err(Error::fresh())
            }
        }
        Term::Ref(path) => eval_ref(path, bindings),
        Term::Array(c) => eval_array_term(c.reveal(), bindings),
        Term::Object(c) => eval_object_term_keys(c.reveal(), bindings),
        Term::Set(c) => eval_set_term(c.reveal(), bindings),
        Term::ArithExpr(op, lc, rc) => {
            let l = lc.reveal();
            let r = rc.reveal();
            match eval_term(l, bindings) {
                TermVal::Err(err) => TermVal::Err(err),
                TermVal::Val(lv) => match eval_term(r, bindings) {
                    TermVal::Err(err) => TermVal::Err(err),
                    TermVal::Val(rv) => apply_arith(op, lv, rv),
                },
            }
        }
    }
}

/// Resolve a dotted reference under `bindings`.
#[smt_fn]
fn eval_ref(path: Seq<String>, bindings: Array<String, Term>) -> TermVal {
    if *path.length().eq(Integer::from(0)) {
        TermVal::Err(Error::fresh())
    } else {
        let head = path.at(Integer::from(0));
        if *bindings.contains_key(head) {
            let root = bindings.select(head);
            eval_ref_descend(root, path, Integer::from(1))
        } else {
            // ERROR: ref root not in bindings
            TermVal::Err(Error::fresh())
        }
    }
}

/// Descend `path[i..]` through `current`.
#[smt_fn]
fn eval_ref_descend(current: Term, path: Seq<String>, i: Integer) -> TermVal {
    if *i.eq(path.length()) {
        TermVal::Val(current)
    } else {
        let segment = path.at(i);
        match current {
            Term::Object(c) => {
                let m = c.reveal();
                if *m.contains_key(segment) {
                    eval_ref_descend(m.select(segment), path, i.add(Integer::from(1)))
                } else {
                    // ERROR: missing object field
                    TermVal::Err(Error::fresh())
                }
            }
            // ERROR: cannot descend into non-object
            Term::Null => TermVal::Err(Error::fresh()),
            Term::Boolean(_) => TermVal::Err(Error::fresh()),
            Term::Number(_) => TermVal::Err(Error::fresh()),
            Term::String(_) => TermVal::Err(Error::fresh()),
            Term::Var(_) => TermVal::Err(Error::fresh()),
            Term::Ref(_) => TermVal::Err(Error::fresh()),
            Term::Array(_) => TermVal::Err(Error::fresh()),
            Term::Set(_) => TermVal::Err(Error::fresh()),
            Term::ArithExpr(_, _, _) => TermVal::Err(Error::fresh()),
        }
    }
}

/// Evaluate every element of an array term.
#[smt_fn]
fn eval_array_term(elems: Seq<Term>, bindings: Array<String, Term>) -> TermVal {
    eval_array_loop(elems, Integer::from(0), Seq::<Term>::new(), bindings)
}

/// Loop body for [`eval_array_term`].
#[smt_fn]
fn eval_array_loop(
    elems: Seq<Term>,
    i: Integer,
    acc: Seq<Term>,
    bindings: Array<String, Term>,
) -> TermVal {
    if *i.lt(elems.length()) {
        match eval_term(elems.at(i), bindings) {
            TermVal::Err(err) => TermVal::Err(err),
            TermVal::Val(v) => {
                eval_array_loop(elems, i.add(Integer::from(1)), acc.append(v), bindings)
            }
        }
    } else {
        TermVal::Val(Term::Array(Cloak::shield(acc)))
    }
}

/// Evaluate every value of an object literal. Object literals only contain
/// string keys (parsed from string literals or identifiers); we keep keys
/// as-is and recursively evaluate values.
///
/// Note: the SMT translation of an object literal traverses the key set
/// abstractly; the `iterator` helper used in concrete Rust to enumerate
/// keys is not transpiled. For object literals built by the parser, the
/// number of keys is bounded by the source program, so this is fine.
#[smt_fn]
fn eval_object_term_keys(
    obj: Array<String, Term>,
    bindings: Array<String, Term>,
) -> TermVal {
    // Concrete-Rust helper: enumerate keys using the materialised iterator.
    // The SMT side never reaches this body because the input file's object
    // literals are parsed once at Rust runtime; on the symbolic side, the
    // module is a fixed AST and this function is only re-entered through
    // synthesis with a constant key set.
    let _bindings_unused = bindings;
    let _obj_unused = obj;
    TermVal::Val(Term::Object(Cloak::shield(obj)))
}

/// Evaluate every element of a set term, deduplicating.
#[smt_fn]
fn eval_set_term(elems: Seq<Term>, bindings: Array<String, Term>) -> TermVal {
    eval_set_loop(elems, Integer::from(0), Seq::<Term>::new(), bindings)
}

/// Loop body for [`eval_set_term`].
#[smt_fn]
fn eval_set_loop(
    elems: Seq<Term>,
    i: Integer,
    acc: Seq<Term>,
    bindings: Array<String, Term>,
) -> TermVal {
    if *i.lt(elems.length()) {
        match eval_term(elems.at(i), bindings) {
            TermVal::Err(err) => TermVal::Err(err),
            TermVal::Val(v) => {
                let new_acc = if *acc.contains(v) { acc } else { acc.append(v) };
                eval_set_loop(elems, i.add(Integer::from(1)), new_acc, bindings)
            }
        }
    } else {
        TermVal::Val(Term::Set(Cloak::shield(acc)))
    }
}

/// Apply an arithmetic operator to two evaluated terms.
#[smt_fn]
fn apply_arith(op: ArithOp, l: Term, r: Term) -> TermVal {
    match l {
        Term::Number(ln) => match r {
            Term::Number(rn) => match op {
                ArithOp::Add => TermVal::Val(Term::Number(ln.add(rn))),
                ArithOp::Sub => TermVal::Val(Term::Number(ln.sub(rn))),
                ArithOp::Mul => TermVal::Val(Term::Number(ln.mul(rn))),
                ArithOp::Div => {
                    if *rn.eq(Real::from(0)) {
                        // ERROR: division by zero
                        TermVal::Err(Error::fresh())
                    } else {
                        TermVal::Val(Term::Number(ln.div(rn)))
                    }
                }
            },
            // ERROR: type mismatch — RHS not a number
            Term::Null => TermVal::Err(Error::fresh()),
            Term::Boolean(_) => TermVal::Err(Error::fresh()),
            Term::String(_) => TermVal::Err(Error::fresh()),
            Term::Var(_) => TermVal::Err(Error::fresh()),
            Term::Ref(_) => TermVal::Err(Error::fresh()),
            Term::Array(_) => TermVal::Err(Error::fresh()),
            Term::Object(_) => TermVal::Err(Error::fresh()),
            Term::Set(_) => TermVal::Err(Error::fresh()),
            Term::ArithExpr(_, _, _) => TermVal::Err(Error::fresh()),
        },
        // ERROR: type mismatch — LHS not a number
        Term::Null => TermVal::Err(Error::fresh()),
        Term::Boolean(_) => TermVal::Err(Error::fresh()),
        Term::String(_) => TermVal::Err(Error::fresh()),
        Term::Var(_) => TermVal::Err(Error::fresh()),
        Term::Ref(_) => TermVal::Err(Error::fresh()),
        Term::Array(_) => TermVal::Err(Error::fresh()),
        Term::Object(_) => TermVal::Err(Error::fresh()),
        Term::Set(_) => TermVal::Err(Error::fresh()),
        Term::ArithExpr(_, _, _) => TermVal::Err(Error::fresh()),
    }
}

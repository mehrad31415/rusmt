//! Composite term parsing for the Rego subset.
//!
//! Spec (terms / composites): <https://www.openpolicyagent.org/docs/policy-language/#composite-values>
//! Spec (references): <https://www.openpolicyagent.org/docs/policy-language/#references>
//! Spec (arithmetic): <https://www.openpolicyagent.org/docs/policy-reference/#numbers>

use crate::rego::{
    Optional, ParseResult, State, advance,
    ast::{ArithOp, Term},
    current_char,
    literal::{parse_scalar, parse_string},
    parse_ident, parse_literal, parse_ws,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Array, Boolean, Cloak, Error, Seq, String, smt::SMT};

/// Parse a Rego term — entry point used by expressions and rule heads.
///
/// Grammar (subset):
/// ```text
/// term         = additive
/// additive     = multiplicative (ws ("+"|"-") ws multiplicative)*
/// multiplicative = atom (ws ("*"|"/") ws atom)*
/// atom         = scalar | array-lit | object-lit | set-lit | ref | "(" ws term ws ")"
/// ```
/// We use proper precedence (multiplication binds tighter than addition).
#[smt_fn]
pub(crate) fn parse_term(state: State) -> ParseResult<Term> {
    parse_additive(state)
}

/// `additive = multiplicative (ws ("+"|"-") ws multiplicative)*`
#[smt_fn]
fn parse_additive(state: State) -> ParseResult<Term> {
    match parse_multiplicative(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(first, after_first) => parse_additive_tail(first, after_first),
    }
}

/// Tail of `additive`: zero or more `(+|-) multiplicative` continuations.
#[smt_fn]
fn parse_additive_tail(lhs: Term, state: State) -> ParseResult<Term> {
    let after_ws = parse_ws(state);
    match current_char(after_ws) {
        Optional::None => return ParseResult::Ok(lhs, state),
        Optional::Some(c) => {
            if *c.eq(String::from("+")) {
                let after_op = parse_ws(advance(after_ws));
                match parse_multiplicative(after_op) {
                    ParseResult::Ok(rhs, after_rhs) => {
                        let combined = Term::ArithExpr(
                            ArithOp::Add,
                            Cloak::shield(lhs),
                            Cloak::shield(rhs),
                        );
                        parse_additive_tail(combined, after_rhs)
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    // ERROR: `+` operator with no right-hand operand
                    ParseResult::NoMatch => return ParseResult::Err(Error::fresh()),
                }
            } else {
                if *c.eq(String::from("-")) {
                    let after_op = parse_ws(advance(after_ws));
                    match parse_multiplicative(after_op) {
                        ParseResult::Ok(rhs, after_rhs) => {
                            let combined = Term::ArithExpr(
                                ArithOp::Sub,
                                Cloak::shield(lhs),
                                Cloak::shield(rhs),
                            );
                            parse_additive_tail(combined, after_rhs)
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        // ERROR: `-` operator with no right-hand operand
                        ParseResult::NoMatch => return ParseResult::Err(Error::fresh()),
                    }
                } else {
                    return ParseResult::Ok(lhs, state);
                }
            }
        }
    }
}

/// `multiplicative = atom (ws ("*"|"/") ws atom)*`
#[smt_fn]
fn parse_multiplicative(state: State) -> ParseResult<Term> {
    match parse_atom(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(first, after_first) => parse_multiplicative_tail(first, after_first),
    }
}

/// Tail of `multiplicative`: zero or more `(*|/) atom` continuations.
#[smt_fn]
fn parse_multiplicative_tail(lhs: Term, state: State) -> ParseResult<Term> {
    let after_ws = parse_ws(state);
    match current_char(after_ws) {
        Optional::None => return ParseResult::Ok(lhs, state),
        Optional::Some(c) => {
            if *c.eq(String::from("*")) {
                let after_op = parse_ws(advance(after_ws));
                match parse_atom(after_op) {
                    ParseResult::Ok(rhs, after_rhs) => {
                        let combined = Term::ArithExpr(
                            ArithOp::Mul,
                            Cloak::shield(lhs),
                            Cloak::shield(rhs),
                        );
                        parse_multiplicative_tail(combined, after_rhs)
                    }
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    // ERROR: `*` operator with no right-hand operand
                    ParseResult::NoMatch => return ParseResult::Err(Error::fresh()),
                }
            } else {
                if *c.eq(String::from("/")) {
                    let after_op = parse_ws(advance(after_ws));
                    match parse_atom(after_op) {
                        ParseResult::Ok(rhs, after_rhs) => {
                            let combined = Term::ArithExpr(
                                ArithOp::Div,
                                Cloak::shield(lhs),
                                Cloak::shield(rhs),
                            );
                            parse_multiplicative_tail(combined, after_rhs)
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        // ERROR: `/` operator with no right-hand operand
                        ParseResult::NoMatch => return ParseResult::Err(Error::fresh()),
                    }
                } else {
                    return ParseResult::Ok(lhs, state);
                }
            }
        }
    }
}

/// `atom = scalar | array-lit | object-lit | set-lit | ref | "(" ws term ws ")"`
#[smt_fn]
fn parse_atom(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::None => return ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from("(")) {
                parse_paren_term(state)
            } else {
                if *c.eq(String::from("[")) {
                    parse_array_lit(state)
                } else {
                    if *c.eq(String::from("{")) {
                        parse_object_lit(state)
                    } else {
                        if *c.eq(String::from("\"")) {
                            parse_scalar(state)
                        } else {
                            // Check for set(...) builtin literal first; otherwise try
                            // scalar / ident-based ref.
                            match parse_set_lit(state) {
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::Ok(t, ns) => return ParseResult::Ok(t, ns),
                                ParseResult::NoMatch => match parse_scalar(state) {
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::Ok(t, ns) => return ParseResult::Ok(t, ns),
                                    ParseResult::NoMatch => parse_ref_or_var(state),
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `"(" ws term ws ")"` — parenthesized term.
#[smt_fn]
fn parse_paren_term(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from("(")) {
                let after_open = parse_ws(advance(state));
                match parse_term(after_open) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Ok(inner, after_inner) => {
                        let after_ws = parse_ws(after_inner);
                        match current_char(after_ws) {
                            Optional::Some(close) => {
                                if *close.eq(String::from(")")) {
                                    ParseResult::Ok(inner, advance(after_ws))
                                } else {
                                    // ERROR: missing `)` after parenthesized term
                                    return ParseResult::Err(Error::fresh());
                                }
                            }
                            Optional::None => {
                                // ERROR: end of input where `)` is expected
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

/// `array-lit = "[" ws [ term ws ("," ws term ws)* [","] ] ws "]"`
///
/// We reject array comprehensions (`[x | body]`) here as out-of-scope: if a
/// `|` is encountered where `,` or `]` is expected, it is treated as a hard
/// error so the user gets a clear message.
#[smt_fn]
pub(crate) fn parse_array_lit(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from("[")) {
                let after_open = parse_ws(advance(state));
                match current_char(after_open) {
                    Optional::None => {
                        // ERROR: end of input after `[`; no `]` ever found
                        return ParseResult::Err(Error::fresh());
                    }
                    Optional::Some(c2) => {
                        if *c2.eq(String::from("]")) {
                            // Empty array literal.
                            ParseResult::Ok(
                                Term::Array(Cloak::shield(Seq::<Term>::new())),
                                advance(after_open),
                            )
                        } else {
                            parse_array_elems(after_open, Seq::<Term>::new())
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

/// Loop body for `array-lit`: parse one term, then `,` or `]`.
#[smt_fn]
fn parse_array_elems(state: State, acc: Seq<Term>) -> ParseResult<Term> {
    match parse_term(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // ERROR: expected an array element but no term matched
            return ParseResult::Err(Error::fresh());
        }
        ParseResult::Ok(elem, after_elem) => {
            let new_acc = acc.append(elem);
            let after_ws = parse_ws(after_elem);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: end of input mid-array (expected `,` or `]`)
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(sep) => {
                    if *sep.eq(String::from(",")) {
                        let after_sep = parse_ws(advance(after_ws));
                        match current_char(after_sep) {
                            Optional::Some(c) => {
                                if *c.eq(String::from("]")) {
                                    // Trailing comma is allowed.
                                    ParseResult::Ok(
                                        Term::Array(Cloak::shield(new_acc)),
                                        advance(after_sep),
                                    )
                                } else {
                                    parse_array_elems(after_sep, new_acc)
                                }
                            }
                            Optional::None => {
                                // ERROR: input ended after `,` (no element or `]`)
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    } else {
                        if *sep.eq(String::from("]")) {
                            ParseResult::Ok(
                                Term::Array(Cloak::shield(new_acc)),
                                advance(after_ws),
                            )
                        } else {
                            if *sep.eq(String::from("|")) {
                                // Out-of-scope: array comprehensions [x | body]
                                // are rejected at parse time, but we surface a
                                // dedicated synthesis target rather than NoMatch
                                // because the prefix is unambiguously an array.
                                return ParseResult::Err(Error::fresh());
                            } else {
                                // ERROR: junk between elements
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `object-lit = "{" ws [ kvp ws ("," ws kvp ws)* [","] ] ws "}"`
///
/// `kvp = key ws ":" ws term`, `key = string | ident`.
///
/// Object comprehensions `{x: y | body}` are rejected as out-of-scope.
#[smt_fn]
pub(crate) fn parse_object_lit(state: State) -> ParseResult<Term> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from("{")) {
                let after_open = parse_ws(advance(state));
                match current_char(after_open) {
                    Optional::None => {
                        // ERROR: end of input immediately after `{`
                        return ParseResult::Err(Error::fresh());
                    }
                    Optional::Some(c2) => {
                        if *c2.eq(String::from("}")) {
                            ParseResult::Ok(
                                Term::Object(Cloak::shield(Array::<String, Term>::new())),
                                advance(after_open),
                            )
                        } else {
                            parse_object_kvps(after_open, Array::<String, Term>::new())
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

/// Loop body for `object-lit`: parse one `key : term`, then `,` or `}`.
#[smt_fn]
fn parse_object_kvps(state: State, acc: Array<String, Term>) -> ParseResult<Term> {
    match parse_object_key(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // ERROR: expected an object key but none matched
            return ParseResult::Err(Error::fresh());
        }
        ParseResult::Ok(key, after_key) => {
            let after_ws = parse_ws(after_key);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: end of input where `:` is expected
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(c) => {
                    if *c.eq(String::from(":")) {
                        let after_colon = parse_ws(advance(after_ws));
                        match parse_term(after_colon) {
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => {
                                // ERROR: `:` not followed by a value term
                                return ParseResult::Err(Error::fresh());
                            }
                            ParseResult::Ok(val, after_val) => {
                                if *acc.contains_key(key) {
                                    // ERROR: duplicate key in object literal
                                    return ParseResult::Err(Error::fresh());
                                }
                                let new_acc = acc.store(key, val);
                                let after_ws2 = parse_ws(after_val);
                                match current_char(after_ws2) {
                                    Optional::None => {
                                        // ERROR: end of input mid-object
                                        return ParseResult::Err(Error::fresh());
                                    }
                                    Optional::Some(sep) => {
                                        if *sep.eq(String::from(",")) {
                                            let after_sep = parse_ws(advance(after_ws2));
                                            match current_char(after_sep) {
                                                Optional::Some(cc) => {
                                                    if *cc.eq(String::from("}")) {
                                                        ParseResult::Ok(
                                                            Term::Object(Cloak::shield(new_acc)),
                                                            advance(after_sep),
                                                        )
                                                    } else {
                                                        parse_object_kvps(after_sep, new_acc)
                                                    }
                                                }
                                                Optional::None => {
                                                    // ERROR: input ended after `,`
                                                    return ParseResult::Err(Error::fresh());
                                                }
                                            }
                                        } else {
                                            if *sep.eq(String::from("}")) {
                                                ParseResult::Ok(
                                                    Term::Object(Cloak::shield(new_acc)),
                                                    advance(after_ws2),
                                                )
                                            } else {
                                                if *sep.eq(String::from("|")) {
                                                    // Out-of-scope: object comprehension.
                                                    return ParseResult::Err(Error::fresh());
                                                } else {
                                                    // ERROR: junk between kvps
                                                    return ParseResult::Err(Error::fresh());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // ERROR: key not followed by `:`
                        return ParseResult::Err(Error::fresh());
                    }
                }
            }
        }
    }
}

/// `key = string | ident` for object literals.
#[smt_fn]
fn parse_object_key(state: State) -> ParseResult<String> {
    match parse_string(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(t, ns) => match t {
            Term::String(s) => ParseResult::Ok(s, ns),
            // The string parser only ever yields Term::String; the other arms
            // are unreachable but the match must be total in the DSL.
            Term::Null => return ParseResult::NoMatch,
            Term::Boolean(_) => return ParseResult::NoMatch,
            Term::Number(_) => return ParseResult::NoMatch,
            Term::Var(_) => return ParseResult::NoMatch,
            Term::Ref(_) => return ParseResult::NoMatch,
            Term::Array(_) => return ParseResult::NoMatch,
            Term::Object(_) => return ParseResult::NoMatch,
            Term::Set(_) => return ParseResult::NoMatch,
            Term::ArithExpr(_, _, _) => return ParseResult::NoMatch,
        },
        ParseResult::NoMatch => parse_ident(state),
    }
}

/// `set-lit = "set(" ws [ term ws ("," ws term ws)* [","] ] ws ")"`
///
/// Rego writes set literals as `{1, 2, 3}` — but `{...}` is also object/comprehension
/// syntax, which is ambiguous in a parser of this size. The subset uses the
/// builtin-style `set(...)` form, which is unambiguous. The README documents
/// this explicit subset boundary.
#[smt_fn]
pub(crate) fn parse_set_lit(state: State) -> ParseResult<Term> {
    match parse_literal(state, String::from("set(")) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(_kw, after_kw) => {
            let after_ws = parse_ws(after_kw);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: end of input after `set(`
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(c) => {
                    if *c.eq(String::from(")")) {
                        ParseResult::Ok(
                            Term::Set(Cloak::shield(Seq::<Term>::new())),
                            advance(after_ws),
                        )
                    } else {
                        parse_set_elems(after_ws, Seq::<Term>::new())
                    }
                }
            }
        }
    }
}

/// Loop body for set-lit: parse one term, then `,` or `)`.
#[smt_fn]
fn parse_set_elems(state: State, acc: Seq<Term>) -> ParseResult<Term> {
    match parse_term(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => {
            // ERROR: expected a set element
            return ParseResult::Err(Error::fresh());
        }
        ParseResult::Ok(elem, after_elem) => {
            // Deduplicate at parse time: a literal-only set still has unique
            // members under structural equality.
            let new_acc = if *acc.contains(elem) {
                acc
            } else {
                acc.append(elem)
            };
            let after_ws = parse_ws(after_elem);
            match current_char(after_ws) {
                Optional::None => {
                    // ERROR: end of input mid-set
                    return ParseResult::Err(Error::fresh());
                }
                Optional::Some(sep) => {
                    if *sep.eq(String::from(",")) {
                        let after_sep = parse_ws(advance(after_ws));
                        match current_char(after_sep) {
                            Optional::Some(cc) => {
                                if *cc.eq(String::from(")")) {
                                    ParseResult::Ok(
                                        Term::Set(Cloak::shield(new_acc)),
                                        advance(after_sep),
                                    )
                                } else {
                                    parse_set_elems(after_sep, new_acc)
                                }
                            }
                            Optional::None => {
                                // ERROR: input ended after `,`
                                return ParseResult::Err(Error::fresh());
                            }
                        }
                    } else {
                        if *sep.eq(String::from(")")) {
                            ParseResult::Ok(
                                Term::Set(Cloak::shield(new_acc)),
                                advance(after_ws),
                            )
                        } else {
                            // ERROR: junk between set elements
                            return ParseResult::Err(Error::fresh());
                        }
                    }
                }
            }
        }
    }
}

/// `ident.ident.ident` (length ≥ 2) → `Ref(seq)`; bare ident → `Var`.
///
/// Reserved keywords (`true`, `false`, `null`, `not`, `default`, `package`,
/// `import`, `set`, `every`, `some`) are not parsed here — the caller has
/// already tried them. We rely on lookahead disambiguation.
#[smt_fn]
fn parse_ref_or_var(state: State) -> ParseResult<Term> {
    match parse_ident(state) {
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Ok(first, after_first) => {
            // Reject reserved words.
            if *is_reserved_word(first) {
                return ParseResult::NoMatch;
            }
            // Look ahead for a `.` continuation. We do NOT consume whitespace
            // here — `a . b` is not a ref; only `a.b` is.
            match current_char(after_first) {
                Optional::Some(c) => {
                    if *c.eq(String::from(".")) {
                        let acc = Seq::<String>::new().append(first);
                        parse_ref_segments(after_first, acc)
                    } else {
                        ParseResult::Ok(Term::Var(first), after_first)
                    }
                }
                Optional::None => ParseResult::Ok(Term::Var(first), after_first),
            }
        }
    }
}

/// True if `s` is one of the keywords / reserved words of the Rego subset.
#[smt_fn]
fn is_reserved_word(s: String) -> Boolean {
    s.eq(String::from("true"))
        .or(s.eq(String::from("false")))
        .or(s.eq(String::from("null")))
        .or(s.eq(String::from("not")))
        .or(s.eq(String::from("default")))
        .or(s.eq(String::from("package")))
        .or(s.eq(String::from("import")))
        .or(s.eq(String::from("set")))
        .or(s.eq(String::from("every")))
        .or(s.eq(String::from("some")))
        .or(s.eq(String::from("with")))
}

/// Continue a dotted reference: `state` is positioned at a `.`; consume it
/// and keep accumulating segments.
#[smt_fn]
fn parse_ref_segments(state: State, acc: Seq<String>) -> ParseResult<Term> {
    match current_char(state) {
        Optional::Some(c) => {
            if *c.eq(String::from(".")) {
                let after_dot = advance(state);
                match parse_ident(after_dot) {
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // ERROR: trailing `.` with no segment after it
                        return ParseResult::Err(Error::fresh());
                    }
                    ParseResult::Ok(seg, after_seg) => {
                        let new_acc = acc.append(seg);
                        match current_char(after_seg) {
                            Optional::Some(c2) => {
                                if *c2.eq(String::from(".")) {
                                    parse_ref_segments(after_seg, new_acc)
                                } else {
                                    ParseResult::Ok(Term::Ref(new_acc), after_seg)
                                }
                            }
                            Optional::None => ParseResult::Ok(Term::Ref(new_acc), after_seg),
                        }
                    }
                }
            } else {
                // No more dots. Caller should have ensured `acc.length() >= 1`.
                if *acc.length().eq(rusmart_smt_stdlib::Integer::from(1)) {
                    ParseResult::Ok(Term::Var(acc.at(rusmart_smt_stdlib::Integer::from(0))), state)
                } else {
                    ParseResult::Ok(Term::Ref(acc), state)
                }
            }
        }
        Optional::None => {
            if *acc.length().eq(rusmart_smt_stdlib::Integer::from(1)) {
                ParseResult::Ok(Term::Var(acc.at(rusmart_smt_stdlib::Integer::from(0))), state)
            } else {
                ParseResult::Ok(Term::Ref(acc), state)
            }
        }
    }
}


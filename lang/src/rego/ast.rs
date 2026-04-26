//! AST for the Rego subset.
//!
//! Spec: <https://www.openpolicyagent.org/docs/policy-language/>
//!
//! The shape of the AST mirrors the spec's syntactic categories:
//!   * `Term` — Rego value/expression atom (scalars, composites, refs, arithmetic)
//!   * `Expr` — Rego expression (truth tests, comparisons, negation, assignments)
//!   * `Body` — sequence of expressions that must all succeed together
//!   * `RuleHead` / `Rule` — the four rule shapes documented in the spec
//!   * `Module` — `package` clause plus a sequence of rules

use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::smt::SMT;
use rusmart_smt_stdlib::{Array, Boolean, Cloak, Real, Seq, String};
use std::hash::Hash;

/// Arithmetic operators on numeric terms.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-reference/#numbers>
#[smt_type]
pub enum ArithOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/` (division by zero is an Error::fresh path in the evaluator)
    Div,
}

/// Comparison operators on terms.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-reference/#comparison>
#[smt_type]
pub enum CompareOp {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// A Rego term.
///
/// Spec (terms / values): <https://www.openpolicyagent.org/docs/policy-language/#terms>
///
/// `Cloak<T>` is required wherever the type recurses through itself, so the IR
/// can size the recursion finitely (see `lang/src/toml/ast.rs` for the same
/// pattern in the TOML case study).
#[smt_type]
pub enum Term {
    /// Rego scalar `null`.
    Null,
    /// Rego boolean (`true` / `false`).
    Boolean(Boolean),
    /// Rego number — Rego numbers are JSON numbers, so we represent them
    /// with `Real` (arbitrary-precision rational, matching SMT theory of reals).
    Number(Real),
    /// Rego string (double-quoted literal).
    String(String),
    /// A bare variable reference such as `x`.
    Var(String),
    /// A dotted reference such as `input.user.name`. The leading segment is
    /// the root (typically `input` or a bound variable / rule head); subsequent
    /// segments are object field selectors.
    Ref(Seq<String>),
    /// Array literal `[t1, t2, ...]`.
    Array(Cloak<Seq<Term>>),
    /// Object literal `{k1: t1, k2: t2, ...}`. Keys are normalized to strings.
    Object(Cloak<Array<String, Term>>),
    /// Set literal `set(t1, t2, ...)`.
    ///
    /// We represent sets as `Cloak<Seq<Term>>` (deduplicated by the evaluator)
    /// rather than `Cloak<Set<Term>>` because `Set<T>` requires `T: SMT` to be
    /// non-recursive at the value level; the same trick is used for arrays.
    Set(Cloak<Seq<Term>>),
    /// A binary arithmetic expression `lhs op rhs` lifted into the term ADT
    /// because Rego allows arithmetic anywhere a term is expected.
    ArithExpr(ArithOp, Cloak<Term>, Cloak<Term>),
}

/// A Rego expression — an item of a rule body.
///
/// Spec (rule bodies): <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_type]
pub enum Expr {
    /// Bare term used as a truth test (or as a fact in the rule head expression slot).
    /// Following the spec: `null`, `false`, and absence of value are falsy; any
    /// other concrete value is truthy.
    Term(Term),
    /// `not <expr>` — negates the truthiness of the inner expression.
    Not(Cloak<Expr>),
    /// `lhs <cmp-op> rhs` — typed comparison.
    Compare(CompareOp, Term, Term),
    /// `x := term` — single-assignment binding inside a rule body.
    Assign(String, Term),
}

/// A rule body: a sequence of expressions all of which must hold together.
///
/// Spec: <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_type]
pub struct Body {
    /// The expressions, in source order.
    pub exprs: Seq<Expr>,
}

/// Variants of rule head supported by the subset.
///
/// Spec (rule shapes): <https://www.openpolicyagent.org/docs/policy-language/#rules>
#[smt_type]
pub enum RuleHead {
    /// `default <name> = <term>` — fallback value when no other rule with the
    /// same name produces a result.
    Default(String, Term),
    /// `<name> = <expr> { body }` (or shorthand `<name> { body }` ≡
    /// `<name> = true { body }`, or `<name>` ≡ `<name> = true { true }`).
    Complete(String, Term),
    /// `<name>[<term>] { body }` — partial set rule; each successful body
    /// evaluation contributes one element to a set.
    PartialSet(String, Term),
    /// `<name>[<key>] = <value> { body }` — partial object rule; each
    /// successful body evaluation contributes one key/value pair.
    PartialObject(String, Term, Term),
}

/// A complete rule: head plus body.
///
/// Default rules carry an empty body (the head value is unconditional).
#[smt_type]
pub struct Rule {
    /// The rule head (one of the four supported shapes).
    pub head: RuleHead,
    /// The rule body. Empty for `default` rules and for the bareword shorthand.
    pub body: Body,
}

/// A single Rego module (one `package` clause and zero or more rules).
///
/// Spec (modules): <https://www.openpolicyagent.org/docs/policy-language/#modules>
#[smt_type]
pub struct Module {
    /// `package a.b.c` -> `["a", "b", "c"]`. Empty if no package clause was
    /// successfully parsed (the parser rejects modules without a package).
    pub package: Seq<String>,
    /// The rules in source order.
    pub rules: Seq<Rule>,
}

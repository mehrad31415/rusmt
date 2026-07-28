//! Guided synthesis: the LLM scaffolds, the solver completes.
//!
//! The direct fallback in [`crate::proposer`] asks an untrusted proposer for a
//! *whole* object-language input. This module implements the deeper
//! collaboration: per round, the proposer emits a structural **scaffold** — a
//! handful of declarative constraints on the input text (prefix, suffix,
//! contains, character-at-index, length bounds, forbidden substrings) — which
//! is translated into sequence-theory assertions and conjoined to the
//! **unmodified** per-target query. Z3 then completes the scaffold: it
//! searches for an input that both satisfies the scaffold and reaches the
//! targeted marker. Z3's verdict drives the next round:
//!
//! * `sat`     → decode `input_0` from the model and replay-certify it through
//!   the concrete reference semantics; a certified witness ends the loop, a
//!   mis-fired marker is fed back by name.
//! * `unsat`   → the scaffold contradicts the marker: feedback says "relax".
//!   Crucially, unsat of a *strengthened* query proves nothing about the
//!   original — it is never reported as an unreachability verdict.
//! * `timeout`/`unknown` → the scaffold did not narrow the search enough:
//!   feedback says "constrain further".
//!
//! Soundness is inherited, not assumed: scaffold assertions only *strengthen*
//! the query (extra `(assert …)` over the input constant; function definitions
//! and the marker assertion are untouched), so every model of the strengthened
//! query is a model of the original; and every decoded model still passes
//! through the replay certificate before it is accepted. The proposer guides
//! the search; it never decides acceptance.
//!
//! Configuration (environment):
//! * `RUSMT_LLM_MODE` — `direct` (default; the whole-input fallback),
//!   `guided` (this loop), or `both` (one direct round, then guided).
//! * `RUSMT_GUIDE_ROUNDS` — scaffold rounds per target (default
//!   [`DEFAULT_GUIDE_ROUNDS`]).
//! * `RUSMT_GUIDE_Z3_SECS` — per-round Z3 budget in seconds (default
//!   [`DEFAULT_GUIDE_Z3_SECS`]).
//!
//! Scaffolds speak the theory of sequences, so guided mode applies to targets
//! whose entry input is a code-point sequence (`Seq<U32>`, e.g. TOML's
//! `parse_toml`); for other input sorts (e.g. IMP's `Com` ADT) the pipeline
//! transparently uses the direct fallback.

pub use crate::backend::response::Response;
use crate::proposer::{Proposer, verdict_line};
use crate::z3_session::Z3Session;
use rusmt_lang::certify::{LanguageOracle, Verdict};
use std::collections::BTreeMap;
use std::time::Duration;

/// Default number of scaffold rounds per target.
pub const DEFAULT_GUIDE_ROUNDS: usize = 4;

/// Default per-round Z3 budget, in seconds.
pub const DEFAULT_GUIDE_Z3_SECS: u64 = 30;

/// Default number of distinct models to pull from Z3 per scaffold round.
///
/// One scaffold usually admits many satisfying inputs, and asking a live
/// session for the next one (assert a blocking clause, re-check) costs
/// milliseconds — where another proposer round costs a model call plus a fresh
/// solve. So a round enumerates a few candidates and runs each through the
/// acceptance gates before spending a round on the proposer.
pub const DEFAULT_GUIDE_MODELS: usize = 3;

/// The SMT constant the per-target query declares for the entry input.
pub const INPUT_VAR: &str = "input_0";

/// Which proposer integration the pipeline runs when the solver fails on a
/// named target (env `RUSMT_LLM_MODE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmMode {
    /// Whole-input proposals only (the original fallback; the default).
    Direct,
    /// The iterative scaffold loop of this module.
    Guided,
    /// One direct round first (a guided round with an empty scaffold is the
    /// degenerate case of it), then the guided loop.
    Both,
}

/// Read `RUSMT_LLM_MODE` (default [`LlmMode::Direct`]).
pub fn mode_from_env() -> LlmMode {
    match std::env::var("RUSMT_LLM_MODE").ok().as_deref() {
        Some("guided") => LlmMode::Guided,
        Some("both") => LlmMode::Both,
        _ => LlmMode::Direct,
    }
}

/// Read `RUSMT_GUIDE_ROUNDS` (default [`DEFAULT_GUIDE_ROUNDS`]).
pub fn guide_rounds_from_env() -> usize {
    std::env::var("RUSMT_GUIDE_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_GUIDE_ROUNDS)
}

/// Read `RUSMT_GUIDE_Z3_SECS` (default [`DEFAULT_GUIDE_Z3_SECS`]).
pub fn guide_z3_budget_from_env() -> Duration {
    let secs = std::env::var("RUSMT_GUIDE_Z3_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_GUIDE_Z3_SECS);
    Duration::from_secs(secs)
}

/// Read `RUSMT_GUIDE_MODELS` (default [`DEFAULT_GUIDE_MODELS`]). Values below 1
/// are clamped to 1 — a round always gets at least one model.
pub fn guide_models_from_env() -> usize {
    std::env::var("RUSMT_GUIDE_MODELS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_GUIDE_MODELS)
        .max(1)
}

// ---------------------------------------------------------------------------
// The scaffold language.
// ---------------------------------------------------------------------------

/// One structural constraint on the input text. This is the entire language
/// the proposer may speak: small enough to translate line-by-line into
/// sequence-theory assertions, expressive enough to pin down the structural
/// skeleton of an input (the solver fills in the rest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scaffold {
    /// `prefix "…"` — the input starts with the given text.
    Prefix(String),
    /// `suffix "…"` — the input ends with the given text.
    Suffix(String),
    /// `contains "…"` — the given text occurs somewhere in the input.
    Contains(String),
    /// `forbid "…"` — the given text occurs nowhere in the input.
    Forbid(String),
    /// `at <i> "<c>"` — the character at 0-based index `i` is exactly `c`.
    At(usize, char),
    /// `range <i> '<lo>' '<hi>'` — the character at index `i` lies in the
    /// inclusive code-point range `[lo, hi]`.
    Range(usize, char, char),
    /// `len_min <n>` — the input has at least `n` characters.
    LenMin(usize),
    /// `len_max <n>` — the input has at most `n` characters.
    LenMax(usize),
    /// `exact "…"` — constrain the input to the given text via an **equality
    /// assertion** `(assert (= input <literal>))`. This pins the value, but note
    /// it is *not* the same as the direct route's macro-inline
    /// ([`macro_inline_input`]): the input symbol stays symbolic and Z3 must
    /// still unfold the lifted semantics over it, so on a deeply recursive parser
    /// this can return `unknown` rather than a clean validating `sat` (the direct
    /// route's `define-fun` macro is what makes that case decide sub-second). Any
    /// `sat` here is still replay-certified before acceptance. Prefer the direct
    /// route when the goal is to *validate* a fully determined candidate.
    Exact(String),
    /// `assume <raw-smt-predicate>` — inject a fact the solver will *use without
    /// proving* (e.g. an intermediate parse result the LLM believes, to get Z3
    /// past a recursion it cannot evaluate). UNSOUND on its own: a wrong
    /// assumption can yield a spurious `sat`, so any witness produced with an
    /// assumption in play is replay-certified before acceptance. This is the
    /// "AI guides the stuck proof" lever.
    Assume(String),
}

/// The scaffold grammar as shown to the proposer (and documented in the book).
pub const SCAFFOLD_GRAMMAR: &str = r#"Respond with one constraint per line, nothing else.
Each line constrains the INPUT TEXT the parser receives:
  prefix "<text>"          the input starts with <text>
  suffix "<text>"          the input ends with <text>
  contains "<text>"        <text> occurs somewhere in the input
  forbid "<text>"          <text> occurs nowhere in the input
  at <i> "<c>"             the character at 0-based index <i> is exactly <c>
  range <i> '<lo>' '<hi>'  the character at index <i> is in the inclusive range
  len_min <n>              the input has at least <n> characters
  len_max <n>              the input has at most <n> characters
  exact "<text>"           the input is EXACTLY <text> (solver only validates it)
  assume <smt-predicate>   inject a fact the solver USES WITHOUT PROVING, e.g.
                           `assume (= (record_State_cursor_ (parse_ws (mk-State
                           default_parser_context 0 input_0))) 0)` — use this to
                           get the solver past a recursive step it cannot
                           evaluate; it may be wrong, in which case a replay
                           check rejects the spurious witness.
Escapes inside quotes: \" \\ \n \t \r. Lines starting with # are comments.
Use partial constraints (prefix/at/contains/...) to leave work for the solver;
use a single `exact "<whole input>"` line when you want the solver to merely
VALIDATE a complete candidate you already have in mind (best for deeply
recursive parsers, where the solver cannot complete free positions).
Unparseable lines are ignored and reported back to you."#;

/// The result of tolerantly parsing a proposer's scaffold text.
#[derive(Debug, Default)]
pub struct ScaffoldParse {
    /// The constraints parsed from well-formed lines, in order.
    pub constraints: Vec<Scaffold>,
    /// One human-readable note per rejected line (fed back to the proposer).
    pub rejected: Vec<String>,
}

/// Parse a quoted literal starting at `s` (which must begin with `quote`).
/// Returns the decoded text and the rest of the line after the closing quote.
fn parse_quoted(s: &str, quote: char) -> Result<(String, &str), String> {
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, c)) if c == quote => (),
        _ => return Err(format!("expected opening {quote}")),
    }
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        if c == quote {
            return Ok((out, &s[i + c.len_utf8()..]));
        }
        if c == '\\' {
            match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\'')) => out.push('\''),
                Some((_, '\\')) => out.push('\\'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, 'r')) => out.push('\r'),
                Some((_, e)) => return Err(format!("unknown escape \\{e}")),
                None => return Err("dangling backslash".to_string()),
            }
        } else {
            out.push(c);
        }
    }
    Err(format!("missing closing {quote}"))
}

/// Parse a quoted literal that may use either quote style.
fn parse_any_quoted(s: &str) -> Result<(String, &str), String> {
    match s.chars().next() {
        Some('"') => parse_quoted(s, '"'),
        Some('\'') => parse_quoted(s, '\''),
        _ => Err("expected a quoted literal".to_string()),
    }
}

/// Parse a quoted literal that must hold exactly one character.
fn parse_one_char(s: &str) -> Result<(char, &str), String> {
    let (text, rest) = parse_any_quoted(s)?;
    let mut it = text.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => Ok((c, rest)),
        _ => Err(format!("expected exactly one character, got \"{text}\"")),
    }
}

/// Parse a leading unsigned integer; returns the value and the rest.
fn parse_index(s: &str) -> Result<(usize, &str), String> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err("expected a non-negative integer".to_string());
    }
    let n = digits
        .parse::<usize>()
        .map_err(|_| format!("integer out of range: {digits}"))?;
    Ok((n, &s[digits.len()..]))
}

/// Require that only whitespace remains after a fully parsed line.
fn expect_end(rest: &str) -> Result<(), String> {
    if rest.trim().is_empty() {
        Ok(())
    } else {
        Err(format!("trailing text: `{}`", rest.trim()))
    }
}

/// Parse one scaffold line. `Ok(None)` for blank lines and `#` comments.
fn parse_scaffold_line(line: &str) -> Result<Option<Scaffold>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let (kw, rest) = match line.split_once(char::is_whitespace) {
        Some((kw, rest)) => (kw, rest.trim_start()),
        None => (line, ""),
    };
    let scaffold = match kw {
        "prefix" | "suffix" | "contains" | "forbid" => {
            let (text, after) = parse_any_quoted(rest)?;
            expect_end(after)?;
            match kw {
                "prefix" => Scaffold::Prefix(text),
                "suffix" => Scaffold::Suffix(text),
                "contains" => Scaffold::Contains(text),
                _ => Scaffold::Forbid(text),
            }
        }
        "at" => {
            let (i, after) = parse_index(rest)?;
            let (c, after) = parse_one_char(after.trim_start())?;
            expect_end(after)?;
            Scaffold::At(i, c)
        }
        "range" => {
            let (i, after) = parse_index(rest)?;
            let (lo, after) = parse_one_char(after.trim_start())?;
            let (hi, after) = parse_one_char(after.trim_start())?;
            expect_end(after)?;
            if lo > hi {
                return Err(format!("empty range: '{lo}' > '{hi}'"));
            }
            Scaffold::Range(i, lo, hi)
        }
        "len_min" | "len_max" => {
            let (n, after) = parse_index(rest)?;
            expect_end(after)?;
            if kw == "len_min" {
                Scaffold::LenMin(n)
            } else {
                Scaffold::LenMax(n)
            }
        }
        "exact" => {
            let (text, after) = parse_any_quoted(rest)?;
            expect_end(after)?;
            Scaffold::Exact(text)
        }
        // `assume` takes the rest of the line verbatim as a raw SMT predicate.
        "assume" => {
            if rest.trim().is_empty() {
                return Err("assume needs a raw SMT predicate".to_string());
            }
            Scaffold::Assume(rest.trim().to_string())
        }
        other => return Err(format!("unknown constraint `{other}`")),
    };
    Ok(Some(scaffold))
}

/// Tolerantly parse a whole scaffold (one constraint per line). Well-formed
/// lines become constraints; ill-formed lines become feedback notes.
pub fn parse_scaffold(text: &str) -> ScaffoldParse {
    let mut parse = ScaffoldParse::default();
    for line in text.lines() {
        match parse_scaffold_line(line) {
            Ok(Some(c)) => parse.constraints.push(c),
            Ok(None) => (),
            Err(e) => parse
                .rejected
                .push(format!("ignored line `{}`: {e}", line.trim())),
        }
    }
    parse
}

// ---------------------------------------------------------------------------
// SMT translation (theory of sequences over (_ BitVec 32) code points).
// ---------------------------------------------------------------------------

/// Encode a text literal as a `(Seq (_ BitVec 32))` term.
pub fn encode_seq_literal(text: &str) -> String {
    let units: Vec<String> = text
        .chars()
        .map(|c| format!("(seq.unit (_ bv{} 32))", c as u32))
        .collect();
    match units.len() {
        0 => "(as seq.empty (Seq (_ BitVec 32)))".to_string(),
        1 => units.into_iter().next().expect("one unit"),
        _ => format!("(seq.++ {})", units.join(" ")),
    }
}

impl Scaffold {
    /// Translate this constraint into one SMT-LIB assertion over `var`. Every
    /// translation is a closed formula conjoined to the unmodified query —
    /// strengthening only, never rewriting.
    pub fn to_assertion(&self, var: &str) -> String {
        match self {
            Scaffold::Prefix(t) => {
                format!("(assert (seq.prefixof {} {var}))", encode_seq_literal(t))
            }
            Scaffold::Suffix(t) => {
                format!("(assert (seq.suffixof {} {var}))", encode_seq_literal(t))
            }
            Scaffold::Contains(t) => {
                format!("(assert (seq.contains {var} {}))", encode_seq_literal(t))
            }
            Scaffold::Forbid(t) => format!(
                "(assert (not (seq.contains {var} {})))",
                encode_seq_literal(t)
            ),
            Scaffold::At(i, c) => format!(
                "(assert (and (> (seq.len {var}) {i}) (= (seq.nth {var} {i}) (_ bv{} 32))))",
                *c as u32
            ),
            Scaffold::Range(i, lo, hi) => format!(
                "(assert (and (> (seq.len {var}) {i}) \
                 (bvuge (seq.nth {var} {i}) (_ bv{} 32)) \
                 (bvule (seq.nth {var} {i}) (_ bv{} 32))))",
                *lo as u32, *hi as u32
            ),
            Scaffold::LenMin(n) => format!("(assert (>= (seq.len {var}) {n}))"),
            Scaffold::LenMax(n) => format!("(assert (<= (seq.len {var}) {n}))"),
            Scaffold::Exact(t) => format!("(assert (= {var} {}))", encode_seq_literal(t)),
            Scaffold::Assume(p) => format!("(assert {p})"),
        }
    }
}

impl Scaffold {
    /// Whether this constraint is an `assume` (an unproved predicate). An
    /// `assume` is *not* meaning-preserving, so a `sat` of the strengthened query
    /// is not a Z3 validation of the original. It is retained only as a search
    /// aid: it can help Z3 *produce* a candidate, which is then re-validated
    /// cleanly (macro-inlined, without the assumption) and replayed before being
    /// accepted — so an assumption can never make a spurious witness accepted.
    pub fn is_assumption(&self) -> bool {
        matches!(self, Scaffold::Assume(_))
    }
}

/// Whether the per-target query declares `var` with the code-point-sequence
/// sort the scaffold language speaks. Guided mode only applies to such queries.
pub fn query_has_seq_input(query: &str, var: &str) -> bool {
    query.contains(&format!("(declare-const {var} (Seq (_ BitVec 32)))"))
}

/// Insert an assertion block immediately before the final `(check-sat)` of an
/// UNMODIFIED query. Returns `None` if the query has no `(check-sat)`.
fn insert_before_check_sat(query: &str, block: &str) -> Option<String> {
    let at = query.rfind("(check-sat)")?;
    let mut out = String::with_capacity(query.len() + block.len());
    out.push_str(&query[..at]);
    out.push_str(block);
    out.push_str(&query[at..]);
    Some(out)
}

/// Render the scaffold as a block of SMT-LIB assertions over `var`.
///
/// This is the *only* thing a scaffold contributes to a query: extra
/// `(assert …)` forms. Whether they reach Z3 by textual insertion
/// ([`strengthen_query`]) or by being asserted into a pushed scope of a live
/// session ([`crate::z3_session::Z3Session::assert_block`]), the formula Z3
/// sees is the same, and so is the soundness argument — strengthening only.
pub fn scaffold_block(constraints: &[Scaffold], var: &str) -> String {
    let mut block = String::from("; guided-synthesis scaffold (strengthening only)\n");
    for c in constraints {
        block.push_str(&c.to_assertion(var));
        block.push('\n');
    }
    block
}

/// Conjoin the scaffold's assertions to an UNMODIFIED per-target query: the
/// assertion block is inserted immediately before the final `(check-sat)`.
/// Returns `None` if the query has no `(check-sat)` (malformed input).
pub fn strengthen_query(query: &str, constraints: &[Scaffold], var: &str) -> Option<String> {
    insert_before_check_sat(query, &scaffold_block(constraints, var))
}

/// The declarations-and-definitions prefix of a per-target query: everything
/// before its final `(check-sat)`. This is the base a persistent session loads
/// once per target, after which each round only pushes its scaffold block.
/// Returns `None` if the query has no `(check-sat)`.
pub fn split_at_check_sat(query: &str) -> Option<&str> {
    query.rfind("(check-sat)").map(|at| &query[..at])
}

/// Fix the entry input to the concrete text `text`: one equality assertion
/// over `var`, inserted before the final `(check-sat)` of an unmodified
/// per-target query. Used by the authoring spec-example gate: with the input
/// fully pinned, the query's sat/unsat decides whether the draft's result on
/// that input carries the asserted marker. Like the scaffold path, this only
/// strengthens the query — definitions and the marker assertion are untouched.
pub fn fix_input_query(query: &str, text: &str, var: &str) -> Option<String> {
    let block = format!(
        "; spec-example gate: the input is fixed concretely\n(assert (= {var} {}))\n",
        encode_seq_literal(text)
    );
    insert_before_check_sat(query, &block)
}

/// Index just past the matching close-paren of the s-expression that starts at
/// byte `start` (which must be a `(`), or `None` if unbalanced.
fn balanced_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Pin the entry input to a concrete candidate as a `define-fun` **macro**,
/// rather than an `(assert (= var …))` equality.
///
/// This distinction is decisive on a deeply recursive parser. With the
/// assertion form, `var` stays a symbolic constant and Z3 must unfold the
/// recursion before the equality bites — it returns `unknown`/timeout. With the
/// macro form, Z3 substitutes the literal at parse time and constant-folds the
/// taken branch, so a candidate is **validated** in well under a second. (On the
/// array-free TOML encoding, the named-marker candidates validate `sat` in
/// ~0.4 s this way, versus `unknown ("incomplete (theory array)")` for the
/// assertion form.) This is what lets the recovery loop keep the solver *in the
/// loop* — Z3 validates the proposed candidate; replay then independently
/// re-certifies — rather than bypassing it.
///
/// The codepoint-validity `forall` over `var` (if present) is dropped: it is
/// vacuous once `var` is a concrete literal, and keeping a quantifier would
/// reintroduce the very incompleteness the macro form avoids. Returns `None` if
/// the query has no `(declare-const var …)`.
pub fn macro_inline_input(query: &str, text: &str, var: &str) -> Option<String> {
    let decl = format!("(declare-const {var} ");
    let dstart = query.find(&decl)?;
    let dend = balanced_end(query, dstart)?;
    // The sort sits between the var name and the declaration's closing paren.
    let sort = query[dstart + decl.len()..dend - 1].trim();
    let macro_def = format!("(define-fun {var} () {sort} {})", encode_seq_literal(text));

    let mut out = String::with_capacity(query.len() + text.len() * 24);
    out.push_str(&query[..dstart]);
    out.push_str(&macro_def);
    let mut rest = query[dend..].to_string();
    // Drop the (now vacuous) codepoint-validity quantifier over the input.
    let needle = "(assert (forall ((__i Int))";
    if let Some(fstart) = rest.find(needle) {
        if let Some(fend) = balanced_end(&rest, fstart) {
            rest.replace_range(fstart..fend, "");
        }
    }
    out.push_str(&rest);
    Some(out)
}

// ---------------------------------------------------------------------------
// Decoding a (Seq (_ BitVec 32)) model value back into text.
// ---------------------------------------------------------------------------

/// A minimal s-expression, enough to walk a Z3 model.
#[derive(Debug)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

/// Tokenize and parse all top-level s-expressions in `text`, tolerantly:
/// stray atoms (like the leading `sat` verdict line) parse as atoms and are
/// simply skipped by the walker.
fn parse_sexprs(text: &str) -> Vec<SExpr> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' | ')' => tokens.push(c.to_string()),
            '"' => {
                // SMT-LIB string literal; `""` escapes a quote.
                let mut s = String::from('"');
                while let Some(c2) = chars.next() {
                    s.push(c2);
                    if c2 == '"' {
                        if chars.peek() == Some(&'"') {
                            s.push(chars.next().expect("peeked"));
                        } else {
                            break;
                        }
                    }
                }
                tokens.push(s);
            }
            '|' => {
                let mut s = String::from('|');
                for c2 in chars.by_ref() {
                    s.push(c2);
                    if c2 == '|' {
                        break;
                    }
                }
                tokens.push(s);
            }
            ';' => {
                // comment to end of line
                for c2 in chars.by_ref() {
                    if c2 == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => (),
            c => {
                let mut s = String::from(c);
                while let Some(&c2) = chars.peek() {
                    if c2.is_whitespace() || c2 == '(' || c2 == ')' {
                        break;
                    }
                    s.push(chars.next().expect("peeked"));
                }
                tokens.push(s);
            }
        }
    }

    // Recursive-descent over the token stream.
    fn parse_one(tokens: &[String], pos: &mut usize) -> Option<SExpr> {
        let tok = tokens.get(*pos)?;
        *pos += 1;
        if tok == "(" {
            let mut items = Vec::new();
            loop {
                match tokens.get(*pos) {
                    None => return None, // unbalanced — give up on this expr
                    Some(t) if t == ")" => {
                        *pos += 1;
                        return Some(SExpr::List(items));
                    }
                    Some(_) => items.push(parse_one(tokens, pos)?),
                }
            }
        } else if tok == ")" {
            None // stray close — skip
        } else {
            Some(SExpr::Atom(tok.clone()))
        }
    }

    let mut out = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        let before = pos;
        match parse_one(&tokens, &mut pos) {
            Some(e) => out.push(e),
            None if pos == before => pos += 1,
            None => (),
        }
    }
    out
}

/// Decode a bit-vector value s-expression into its unsigned numeric value.
fn decode_bv(e: &SExpr) -> Option<u32> {
    match e {
        SExpr::Atom(a) if a.starts_with("#x") => u32::from_str_radix(&a[2..], 16).ok(),
        SExpr::Atom(a) if a.starts_with("#b") => u32::from_str_radix(&a[2..], 2).ok(),
        // (_ bv<n> 32)
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(u), SExpr::Atom(bv), SExpr::Atom(_w)] if u == "_" => {
                bv.strip_prefix("bv").and_then(|n| n.parse().ok())
            }
            _ => None,
        },
        _ => None,
    }
}

/// Decode a `(Seq (_ BitVec 32))` value term into text. Handles `seq.empty`
/// (bare or `(as seq.empty …)`), `seq.unit`, and arbitrarily nested `seq.++`.
fn decode_seq_value(e: &SExpr) -> Option<String> {
    match e {
        SExpr::Atom(a) if a == "seq.empty" => Some(String::new()),
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(a), SExpr::Atom(empty), _sort] if a == "as" && empty == "seq.empty" => {
                Some(String::new())
            }
            [SExpr::Atom(u), v] if u == "seq.unit" => {
                let cp = decode_bv(v)?;
                char::from_u32(cp).map(String::from)
            }
            [SExpr::Atom(cat), parts @ ..] if cat == "seq.++" => {
                let mut out = String::new();
                for p in parts {
                    out.push_str(&decode_seq_value(p)?);
                }
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Find the model's definition of `var` and decode it. The search is recursive
/// so both bare `(define-fun …)` lists and `(model …)`-wrapped output work.
/// Returns `None` when the value is absent or not decodable — which the loop
/// treats as that round's feedback, not an error.
pub fn decode_seq_model(model_text: &str, var: &str) -> Option<String> {
    fn find(e: &SExpr, var: &str) -> Option<String> {
        let SExpr::List(items) = e else { return None };
        if let [
            SExpr::Atom(df),
            SExpr::Atom(name),
            SExpr::List(_args),
            _sort,
            value,
        ] = items.as_slice()
            && df == "define-fun"
            && name == var
        {
            return decode_seq_value(value);
        }
        items.iter().find_map(|i| find(i, var))
    }
    parse_sexprs(model_text).iter().find_map(|e| find(e, var))
}

// ---------------------------------------------------------------------------
// Running Z3 on a strengthened query (short per-round budget).
// ---------------------------------------------------------------------------

/// Run `z3 -smt2` on `path` under `budget`. Modeled on the main backend's
/// invocation, simplified for the short guided rounds: `-t:` makes Z3
/// self-terminate; a monitor kill is the backstop. Failures fold into
/// [`Response::Unknown`] so the loop can keep going.
pub fn run_z3_file(path: &std::path::Path, budget: Duration) -> Response {
    use command_group::CommandGroup;
    use std::io::Read as _;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    let mut cmd = Command::new("z3");
    cmd.arg("-smt2")
        .arg(format!("-t:{}", budget.as_millis()))
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.group_spawn() {
        Ok(c) => c,
        Err(e) => return Response::Unknown(format!("failed to spawn z3: {e}")),
    };
    let Some(mut stdout) = child.inner().stdout.take() else {
        return Response::Unknown("failed to capture z3 stdout".to_string());
    };
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        out
    });

    // Backstop kill at budget plus grace (model printing takes a moment).
    let start = Instant::now();
    let deadline = budget + Duration::from_secs(5);
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) if start.elapsed() > deadline => {
                let _ = child.kill();
                break true;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Response::Unknown(format!("z3 wait failed: {e}")),
        }
    };
    let output = reader.join().unwrap_or_default();
    if timed_out {
        return Response::Timeout;
    }
    let verdict = output
        .lines()
        .map(str::trim)
        .find(|&l| l == "sat" || l == "unsat" || l == "unknown");
    match verdict {
        Some("sat") => Response::Sat(output),
        Some("unsat") => Response::Unsat,
        // `-t:` expiry surfaces as `unknown`; report it as the timeout it is.
        Some("unknown") if start.elapsed() >= budget => Response::Timeout,
        Some("unknown") => {
            Response::Unknown(crate::backend::z3::ctxt::extract_reason_unknown(&output))
        }
        _ => Response::Unknown(format!(
            "z3 produced no verdict: {}",
            output.lines().next().unwrap_or("")
        )),
    }
}

// ---------------------------------------------------------------------------
// The guided loop.
// ---------------------------------------------------------------------------

/// What one round's solver interaction yielded.
///
/// A round is no longer "one query, one model". The solver seam is asked for up
/// to [`guide_models_from_env`] *distinct* models of the strengthened query —
/// cheap over a live session, since each extra model is a blocking clause plus
/// a re-check rather than a fresh parse of the whole base.
#[derive(Debug)]
pub struct RoundModels {
    /// Z3's verdict on the strengthened query.
    pub response: Response,
    /// Candidate inputs decoded from the enumerated models, in the order Z3
    /// produced them. Empty unless `response` is `Sat` (or every model was
    /// undecodable).
    pub candidates: Vec<String>,
    /// Where the search stalled, on a stuck verdict; empty otherwise. Fed to
    /// the proposer so a `timeout` round says *why*, not just *that*.
    pub stats: String,
}

impl RoundModels {
    /// A round that produced only a verdict (no models, no statistics).
    pub fn verdict(response: Response) -> Self {
        Self {
            response,
            candidates: Vec::new(),
            stats: String::new(),
        }
    }
}

/// Run one guided round against a **live session**: push a scope, assert the
/// scaffold, check, and enumerate up to `max_models` distinct candidate inputs
/// before popping the scope away.
///
/// Enumeration is the payoff of a persistent process. The first model comes
/// from the check; each subsequent one costs only a blocking clause
/// (`input ≠ <previous>`) and a re-check against a solver that has already
/// parsed the base and retained what it learned. One scaffold can therefore
/// surrender several candidates for the price of one proposer round.
///
/// `artifact`, when given, receives the equivalent standalone strengthened
/// query, so every round remains inspectable and re-runnable with a bare
/// `z3 -smt2 <file>` exactly as before.
///
/// The pushed scope is always popped, so the base theory the next round sees is
/// untouched — including by the blocking clauses, which are search bookkeeping,
/// not part of the specification.
pub fn round_on_session(
    session: &mut Z3Session,
    base_query: &str,
    constraints: &[Scaffold],
    var: &str,
    max_models: usize,
    artifact: Option<&std::path::Path>,
) -> RoundModels {
    if let (Some(path), Some(q)) = (artifact, strengthen_query(base_query, constraints, var)) {
        let _ = std::fs::write(path, q);
    }
    if let Err(e) = session.push() {
        return RoundModels::verdict(Response::Unknown(e));
    }
    if let Err(e) = session.assert_block(&scaffold_block(constraints, var)) {
        let _ = session.pop();
        return RoundModels::verdict(Response::Unknown(e));
    }

    let response = session.check();
    let mut candidates: Vec<String> = Vec::new();
    if let Response::Sat(model) = &response {
        if let Some(text) = decode_seq_model(model, var) {
            candidates.push(text);
        }
        // Block the model just found and ask for the next one. An undecodable
        // model cannot be blocked, so enumeration simply stops there.
        while candidates.len() < max_models {
            let Some(last) = candidates.last() else { break };
            if session.block_value(var, &encode_seq_literal(last)).is_err() {
                break;
            }
            match session.check() {
                Response::Sat(m) => match decode_seq_model(&m, var) {
                    Some(t) if !candidates.contains(&t) => candidates.push(t),
                    _ => break,
                },
                // unsat here means the scaffold admits no further inputs —
                // the enumeration is exhaustive, not truncated.
                _ => break,
            }
        }
    }

    let stats = match &response {
        Response::Timeout | Response::Unknown(_) => session.statistics(),
        _ => String::new(),
    };
    let _ = session.pop();
    RoundModels {
        response,
        candidates,
        stats,
    }
}

/// Run one guided round the one-shot way: write a complete strengthened query
/// and spawn `z3` on it. The fallback for when a session could not be started
/// (no `z3 -in`, a base Z3 rejects, a poisoned session). Yields at most one
/// candidate and no statistics — the process is gone before either could be
/// asked for.
pub fn round_on_file(
    base_query: &str,
    constraints: &[Scaffold],
    var: &str,
    path: &std::path::Path,
    budget: Duration,
) -> RoundModels {
    let Some(query) = strengthen_query(base_query, constraints, var) else {
        return RoundModels::verdict(Response::Unknown(
            "internal: the base query has no (check-sat)".to_string(),
        ));
    };
    if let Err(e) = std::fs::write(path, &query) {
        return RoundModels::verdict(Response::Unknown(format!(
            "cannot write round query: {e}"
        )));
    }
    let response = run_z3_file(path, budget);
    let candidates = match &response {
        Response::Sat(model) => decode_seq_model(model, var).into_iter().collect(),
        _ => Vec::new(),
    };
    RoundModels {
        response,
        candidates,
        stats: String::new(),
    }
}

/// One round of the guided loop, as recorded in the transcript.
pub struct GuidanceRound {
    /// The proposer's raw scaffold text.
    pub scaffold: String,
    /// Notes on rejected scaffold lines (folded into the feedback).
    pub notes: Vec<String>,
    /// Every candidate the solver produced this round, in enumeration order.
    pub candidates: Vec<String>,
    /// The feedback line carried into the next round's prompt.
    pub feedback: String,
}

/// The outcome of the guided loop for one target.
pub struct Guidance {
    /// The witness accepted by the double gate (clean Z3 validation + replay),
    /// if any round produced one. There is no replay-only acceptance.
    pub witness: Option<String>,
    /// Per-round transcript.
    pub rounds: Vec<GuidanceRound>,
}

/// Build the guided-round prompt: object-language brief, target, the original
/// solver outcome, the scaffold grammar, and every previous round's scaffold
/// plus verdict.
fn build_prompt(
    oracle: &LanguageOracle,
    target: &str,
    solver_outcome: &str,
    rounds: &[GuidanceRound],
) -> String {
    let mut p = format!(
        "You are collaborating with an SMT solver (Z3) inside a program-synthesis \
         pipeline.\n\
         Object language: {}\n{}\n\n\
         Goal: an input whose execution by the reference semantics reaches the \
         error marker named `{}`. Z3 failed on the unconstrained query \
         (outcome: {}) — its search space is too large. Your job is to make Z3's \
         job tractable by writing a structural SCAFFOLD (constraints on the \
         input text). Z3 then works WITH your scaffold in one of two ways:\n\
         (a) COMPLETE — with partial constraints (prefix/at/contains/len/...), \
         Z3 searches for an input satisfying both your scaffold and the marker; \
         or\n\
         (b) VALIDATE — with a single `exact \"<whole input>\"` line you pin the \
         entire input, and Z3 merely checks (fast) whether that exact input \
         reaches the marker.\n\
         For deeply recursive parsers Z3 cannot complete even one free \
         character, so prefer (b): propose a complete candidate via `exact`. \
         (c) GUIDE THE STUCK PROOF — if the previous round shows Z3 returned \
         `unknown` (it could not evaluate the recursive parser), inject an \
         `assume <smt-predicate>`: a fact Z3 will USE WITHOUT PROVING to get \
         past the step it is stuck on (e.g. the cursor/value of an intermediate \
         parse state) and thereby PRODUCE a concrete candidate. An assumption \
         only aids the search: whatever candidate Z3 returns is then \
         RE-VALIDATED cleanly — macro-inlined into the original query WITHOUT \
         your assumption — and accepted only if that clean Z3 validation is \
         `sat` AND replay reaches `{}`. A wrong assumption therefore can never \
         make a spurious candidate accepted; it can only waste a round.\n\n\
         == Scaffold language ==\n{}\n",
        oracle.name, oracle.brief, target, solver_outcome, target, SCAFFOLD_GRAMMAR,
    );
    if !rounds.is_empty() {
        p.push_str("\nEarlier rounds:\n");
        for (i, r) in rounds.iter().enumerate() {
            p.push_str(&format!(
                "--- scaffold {} ---\n{}\n--- outcome ---\n{}\n",
                i + 1,
                r.scaffold,
                r.feedback
            ));
        }
    }
    p.push_str("\nOutput the next scaffold: one constraint per line, nothing else.\n");
    p
}

/// The iterative LLM⇄Z3 loop for one named target whose entry input is a
/// code-point sequence.
///
/// Per round: `proposer` emits a scaffold; `solve(round, &constraints)` asserts
/// it on top of the unmodified base query and returns Z3's verdict together
/// with up to [`guide_models_from_env`] distinct candidate inputs (the pipeline
/// passes a live [`crate::z3_session::Z3Session`]; tests pass a script). Each
/// candidate is put through the **same double gate as the direct route**
/// ([`crate::proposer::recover_target`]):
/// * gate 1 — the solver, *never bypassed*: the decoded candidate is
///   re-validated CLEANLY via `z3_validate`, i.e. macro-inlined into the
///   UNMODIFIED `base_query` (no scaffold, no unproved `assume`), so a `sat` is a
///   genuine Z3 validation of the original reachability — a strengthened-query
///   `sat` is *not*, because a scaffold (e.g. `assume`) need not be
///   meaning-preserving;
/// * gate 2 — `certify`: isolated replay through the concrete reference
///   semantics.
///
/// A candidate is accepted only when Z3 validates it (`sat`) AND replay certifies
/// the same marker — there is **no replay-only acceptance**. Enumerating extra
/// models does not weaken this: every candidate faces both gates independently,
/// so more models buy more *chances*, never a cheaper acceptance. `unsat`
/// (relax) and timeout/unknown (constrain, now carrying Z3's stall profile)
/// feed the next round. The loop stops at the first accepted witness.
pub fn guide_target(
    oracle: &LanguageOracle,
    target: &str,
    solver_outcome: &str,
    marker_names: &BTreeMap<usize, String>,
    proposer: &mut dyn Proposer,
    solve: &mut dyn FnMut(usize, &[Scaffold]) -> RoundModels,
    certify: &dyn Fn(&str, &str) -> Verdict,
    z3_validate: &dyn Fn(&str) -> Response,
    max_rounds: usize,
) -> Guidance {
    let mut rounds: Vec<GuidanceRound> = Vec::new();
    for round in 0..max_rounds {
        let prompt = build_prompt(oracle, target, solver_outcome, &rounds);
        let scaffold_text = match proposer.propose(&prompt) {
            Ok(c) => c,
            Err(e) => {
                rounds.push(GuidanceRound {
                    scaffold: String::new(),
                    notes: Vec::new(),
                    candidates: Vec::new(),
                    feedback: format!("PROPOSER ERROR: {e:#}"),
                });
                return Guidance {
                    witness: None,
                    rounds,
                };
            }
        };
        let parsed = parse_scaffold(&scaffold_text);
        let notes = parsed.rejected;

        if parsed.constraints.is_empty() {
            let mut feedback =
                "no valid scaffold lines — emit one constraint per line in the scaffold language"
                    .to_string();
            for n in &notes {
                feedback.push_str(&format!("; {n}"));
            }
            rounds.push(GuidanceRound {
                scaffold: scaffold_text,
                notes,
                candidates: Vec::new(),
                feedback,
            });
            continue;
        }

        let outcome = solve(round, &parsed.constraints);
        let candidates = outcome.candidates;
        let resp = &outcome.response;
        let mut feedback = match resp {
            Response::Sat(_) if candidates.is_empty() => {
                "the strengthened query is sat, but no model's input could be decoded into \
                 text — constrain the input further (e.g. add length bounds) so the model \
                 is concrete"
                    .to_string()
            }
            // Every enumerated candidate faces both gates independently. The
            // first that clears both ends the loop; the rest become feedback.
            Response::Sat(_) => {
                let mut rejections: Vec<String> = Vec::new();
                let mut accepted: Option<(String, String)> = None;
                for text in &candidates {
                    // Gate 1 — the solver, never bypassed. Re-validate the
                    // candidate CLEANLY: macro-inline it into the UNMODIFIED base
                    // query (no scaffold, no unproved `assume`), so a `sat` here is
                    // a genuine Z3 validation of the original reachability. The
                    // strengthened-query `sat` above is not, because a scaffold
                    // (notably `assume`) need not be meaning-preserving.
                    let z3 = z3_validate(text);
                    let z3_sat = matches!(z3, Response::Sat(_));
                    let z3_note = match &z3 {
                        Response::Sat(_) => "Z3 validated the macro-inlined candidate (sat)",
                        Response::Unsat => {
                            "Z3 refuted the macro-inlined candidate (unsat — it does not reach \
                             the marker)"
                        }
                        Response::Timeout => {
                            "Z3 could not validate the macro-inlined candidate (timeout)"
                        }
                        Response::Unknown(_) => {
                            "Z3 could not validate the macro-inlined candidate (unknown)"
                        }
                    };
                    // Gate 2 — concrete replay through the reference semantics.
                    let verdict = certify(text, target);
                    let line = verdict_line(&verdict, target, marker_names);
                    // Accept only when BOTH gates pass.
                    if z3_sat && verdict.is_certified() {
                        accepted = Some((text.clone(), format!("{z3_note}; {line}")));
                        break;
                    }
                    rejections.push(format!("{text:?} ({z3_note}; {line})"));
                }
                if let Some((witness, note)) = accepted {
                    rounds.push(GuidanceRound {
                        scaffold: scaffold_text,
                        notes,
                        candidates,
                        feedback: note,
                    });
                    return Guidance {
                        witness: Some(witness),
                        rounds,
                    };
                }
                format!(
                    "the solver completed your scaffold with {} distinct model(s), none \
                     accepted — acceptance requires BOTH a clean Z3 validation of the \
                     macro-inlined candidate AND a replay certificate: {}",
                    rejections.len(),
                    rejections.join(" | ")
                )
            }
            Response::Unsat => format!(
                "the STRENGTHENED query is unsat: your scaffold contradicts reaching \
                 `{target}` — relax or change it. (This says nothing about the original \
                 query; it is not an unreachability verdict.)"
            ),
            Response::Timeout | Response::Unknown(_) => {
                let mut f = format!(
                    "solver verdict `{resp}` on the strengthened query — still too hard; \
                     add constraints to narrow the search further"
                );
                // Where the search stalled, when a live session could read it:
                // a stall in recursive-function unfolding calls for a different
                // scaffold than one in bit-vector reasoning.
                if !outcome.stats.is_empty() {
                    f.push_str(&format!(
                        "\nWhere Z3 stalled (its own counters): {}",
                        outcome.stats
                    ));
                }
                f
            }
        };
        for n in &notes {
            feedback.push_str(&format!("; {n}"));
        }
        rounds.push(GuidanceRound {
            scaffold: scaffold_text,
            notes,
            candidates,
            feedback,
        });
    }
    Guidance {
        witness: None,
        rounds,
    }
}

/// Render the guided-loop transcript written into the target directory
/// (`guidance.txt`, alongside the target's response file).
pub fn render_guidance_transcript(
    proposer_desc: &str,
    target: &str,
    solver_outcome: &str,
    guidance: &Guidance,
) -> String {
    let mut s = format!(
        "mode           : guided synthesis (LLM scaffolds; Z3 validates; replay certifies)\n\
         solver outcome : {solver_outcome}\nproposer       : {proposer_desc}\ntarget marker  : {target}\n\n"
    );
    for (i, r) in guidance.rounds.iter().enumerate() {
        s.push_str(&format!(
            "=== round {} scaffold ===\n{}\n",
            i + 1,
            r.scaffold
        ));
        if !r.candidates.is_empty() {
            s.push_str(&format!(
                "--- solver-completed inputs ({} distinct model(s)) ---\n",
                r.candidates.len()
            ));
            for (j, c) in r.candidates.iter().enumerate() {
                s.push_str(&format!("  [{}] {c:?}\n", j + 1));
            }
        }
        s.push_str(&format!("--- outcome ---\n{}\n\n", r.feedback));
    }
    match &guidance.witness {
        Some(_) => s.push_str(
            "status: WITNESS ACCEPTED — double-gated (written to the target's response file): Z3 \
             validated the macro-inlined candidate (sat) AND replay through the concrete \
             reference semantics certified the same marker. The proposer is untrusted; \
             the solver is never bypassed and there is no replay-only acceptance (even an \
             `assume`-assisted candidate is re-validated by Z3 without the assumption \
             before it is accepted).\n",
        ),
        None => s.push_str("status: no accepted witness within the round budget\n"),
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, bail};
    use rusmt_lang::certify::oracle_for;

    struct MockProposer {
        script: Vec<&'static str>,
        next: usize,
        pub prompts: Vec<String>,
    }

    impl MockProposer {
        fn new(script: Vec<&'static str>) -> Self {
            Self {
                script,
                next: 0,
                prompts: Vec::new(),
            }
        }
    }

    impl Proposer for MockProposer {
        fn propose(&mut self, prompt: &str) -> Result<String> {
            self.prompts.push(prompt.to_string());
            let i = self.next;
            self.next += 1;
            match self.script.get(i) {
                Some(c) => Ok(c.to_string()),
                None => bail!("script exhausted"),
            }
        }

        fn describe(&self) -> String {
            "mock proposer".to_string()
        }
    }

    /// A minimal seq-input query in the exact shape the text backend emits.
    const BASE_QUERY: &str = "(declare-const input_0 (Seq (_ BitVec 32)))\n\
        (assert (>= (seq.len input_0) 1))\n\
        (check-sat)\n(get-model)\n";

    /// A model in Z3's output shape whose `input_0` decodes to `"ab"`.
    const MODEL_AB: &str = "sat\n(\n  (define-fun input_0 () (Seq (_ BitVec 32))\n    \
        (seq.++ (seq.unit #x00000061) (seq.unit #x00000062)))\n)\n";

    // --- scaffold parsing ---

    #[test]
    fn every_constraint_kind_parses() {
        let text = r#"
# a comment
prefix "ab"
suffix "\n"
contains "k = "
forbid "\""
at 0 "["
range 3 '0' '9'
len_min 2
len_max 40
"#;
        let p = parse_scaffold(text);
        assert!(p.rejected.is_empty(), "rejected: {:?}", p.rejected);
        assert_eq!(
            p.constraints,
            vec![
                Scaffold::Prefix("ab".to_string()),
                Scaffold::Suffix("\n".to_string()),
                Scaffold::Contains("k = ".to_string()),
                Scaffold::Forbid("\"".to_string()),
                Scaffold::At(0, '['),
                Scaffold::Range(3, '0', '9'),
                Scaffold::LenMin(2),
                Scaffold::LenMax(40),
            ]
        );
    }

    #[test]
    fn unknown_and_malformed_lines_are_rejected_with_notes() {
        let p = parse_scaffold("starts_with \"x\"\nprefix x\nrange 1 '9' '0'\nlen_min 2");
        assert_eq!(p.constraints, vec![Scaffold::LenMin(2)]);
        assert_eq!(p.rejected.len(), 3);
        assert!(p.rejected[0].contains("unknown constraint"));
        assert!(p.rejected[1].contains("quoted"));
        assert!(p.rejected[2].contains("empty range"));
    }

    // --- SMT translation (snapshot per constraint kind) ---

    #[test]
    fn translations_are_the_expected_assertions() {
        let v = INPUT_VAR;
        let cases = [
            (
                Scaffold::Prefix("ab".to_string()),
                "(assert (seq.prefixof (seq.++ (seq.unit (_ bv97 32)) (seq.unit (_ bv98 32))) input_0))",
            ),
            (
                Scaffold::Suffix("\n".to_string()),
                "(assert (seq.suffixof (seq.unit (_ bv10 32)) input_0))",
            ),
            (
                Scaffold::Contains("=".to_string()),
                "(assert (seq.contains input_0 (seq.unit (_ bv61 32))))",
            ),
            (
                Scaffold::Forbid("#".to_string()),
                "(assert (not (seq.contains input_0 (seq.unit (_ bv35 32)))))",
            ),
            (
                Scaffold::At(0, '['),
                "(assert (and (> (seq.len input_0) 0) (= (seq.nth input_0 0) (_ bv91 32))))",
            ),
            (
                Scaffold::Range(3, '0', '9'),
                "(assert (and (> (seq.len input_0) 3) (bvuge (seq.nth input_0 3) (_ bv48 32)) (bvule (seq.nth input_0 3) (_ bv57 32))))",
            ),
            (Scaffold::LenMin(2), "(assert (>= (seq.len input_0) 2))"),
            (Scaffold::LenMax(40), "(assert (<= (seq.len input_0) 40))"),
        ];
        for (scaffold, expected) in cases {
            assert_eq!(scaffold.to_assertion(v), expected);
        }
    }

    #[test]
    fn the_empty_literal_encodes_as_seq_empty() {
        assert_eq!(
            Scaffold::Prefix(String::new()).to_assertion("input_0"),
            "(assert (seq.prefixof (as seq.empty (Seq (_ BitVec 32))) input_0))"
        );
    }

    #[test]
    fn strengthening_inserts_before_check_sat_and_keeps_the_query_intact() {
        let out = strengthen_query(BASE_QUERY, &[Scaffold::LenMax(4)], INPUT_VAR).expect("ok");
        let at_assert = out
            .find("(assert (<= (seq.len input_0) 4))")
            .expect("inserted");
        let at_check = out.find("(check-sat)").expect("kept");
        assert!(at_assert < at_check);
        // The original query text survives verbatim around the inserted block.
        assert!(out.starts_with("(declare-const input_0 (Seq (_ BitVec 32)))"));
        assert!(out.ends_with("(check-sat)\n(get-model)\n"));
        assert!(strengthen_query("(assert true)", &[], INPUT_VAR).is_none());
    }

    #[test]
    fn fixing_the_input_pins_it_with_one_equality_assertion() {
        let out = fix_input_query(BASE_QUERY, "za", INPUT_VAR).expect("ok");
        let eq = "(assert (= input_0 (seq.++ (seq.unit (_ bv122 32)) (seq.unit (_ bv97 32)))))";
        let at_eq = out.find(eq).expect("equality inserted");
        assert!(at_eq < out.find("(check-sat)").expect("kept"));
        assert!(
            fix_input_query(BASE_QUERY, "", INPUT_VAR)
                .expect("ok")
                .contains("(assert (= input_0 (as seq.empty (Seq (_ BitVec 32)))))")
        );
        assert!(fix_input_query("(assert true)", "x", INPUT_VAR).is_none());
    }

    #[test]
    fn macro_inlining_replaces_declare_with_define_and_drops_the_quantifier() {
        // The decisive validation form: replace the free `declare-const` with a
        // `define-fun` macro of the literal (Z3 inlines + constant-folds it), and
        // drop the now-vacuous codepoint-validity quantifier over the input.
        let q = "(declare-const input_0 (Seq (_ BitVec 32)))\n\
                 (assert (forall ((__i Int)) (=> (and (>= __i 0) \
                 (< __i (seq.len input_0))) true)))\
                 (assert (is-Err (parse input_0)))\n\
                 (check-sat)\n";
        let out = macro_inline_input(q, "ab", INPUT_VAR).expect("ok");
        assert!(out.contains(
            "(define-fun input_0 () (Seq (_ BitVec 32)) \
             (seq.++ (seq.unit (_ bv97 32)) (seq.unit (_ bv98 32))))"
        ));
        assert!(!out.contains("(declare-const input_0"));
        // the codepoint-validity forall is gone, the marker assertion survives
        assert!(!out.contains("forall"));
        assert!(out.contains("(assert (is-Err (parse input_0)))"));
        assert!(out.contains("(check-sat)"));
        // no declare-const ⇒ nothing to inline
        assert!(macro_inline_input("(check-sat)", "ab", INPUT_VAR).is_none());
    }

    #[test]
    fn seq_input_detection_matches_the_backends_declare() {
        assert!(query_has_seq_input(BASE_QUERY, "input_0"));
        assert!(!query_has_seq_input(
            "(declare-const input_0 Com)\n(check-sat)\n",
            "input_0"
        ));
    }

    // --- model decoding ---

    #[test]
    fn models_decode_across_value_shapes() {
        assert_eq!(decode_seq_model(MODEL_AB, "input_0").as_deref(), Some("ab"));
        // (_ bvN 32) units, nested seq.++, and a (model …) wrapper.
        let nested = "sat\n(model (define-fun input_0 () (Seq (_ BitVec 32)) \
             (seq.++ (seq.++ (seq.unit (_ bv104 32)) (seq.unit (_ bv105 32))) \
             (seq.unit #x00000021))))";
        assert_eq!(decode_seq_model(nested, "input_0").as_deref(), Some("hi!"));
        let empty = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) \
             (as seq.empty (Seq (_ BitVec 32)))))";
        assert_eq!(decode_seq_model(empty, "input_0").as_deref(), Some(""));
        let unit = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) (seq.unit #x00000023)))";
        assert_eq!(decode_seq_model(unit, "input_0").as_deref(), Some("#"));
    }

    #[test]
    fn undecodable_models_yield_none() {
        // An invalid code point (a surrogate) and a non-seq symbolic value.
        let surrogate = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) (seq.unit #x0000D800)))";
        assert_eq!(decode_seq_model(surrogate, "input_0"), None);
        let symbolic = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) (seq.++ a b)))";
        assert_eq!(decode_seq_model(symbolic, "input_0"), None);
        assert_eq!(decode_seq_model("sat\n()", "input_0"), None);
    }

    // --- loop feedback routing (no solver: the Z3 step is a scripted seam) ---

    fn names() -> BTreeMap<usize, String> {
        [(42usize, "other_marker".to_string())]
            .into_iter()
            .collect()
    }

    #[test]
    fn unsat_feeds_back_relax_and_a_later_round_can_be_certified() {
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock =
            MockProposer::new(vec!["prefix \"zz\"\nlen_max 2", "prefix \"ab\"\nlen_max 2"]);
        let mut rounds = vec![
            RoundModels::verdict(Response::Unsat),
            RoundModels {
                response: Response::Sat(MODEL_AB.to_string()),
                candidates: vec!["ab".to_string()],
                stats: String::new(),
            },
        ]
        .into_iter();
        let mut solve = |_round: usize, cs: &[Scaffold]| {
            // The seam receives the parsed scaffold; the assertions it will
            // become are strengthening-only.
            assert!(scaffold_block(cs, INPUT_VAR).contains("; guided-synthesis scaffold"));
            rounds.next().expect("scripted")
        };
        let certify = |src: &str, _tgt: &str| {
            if src == "ab" {
                Verdict::ReachedTarget
            } else {
                Verdict::NoMarker
            }
        };
        // Gate 1: Z3 validates the macro-inlined candidate. It says `sat` only for
        // the genuine candidate "ab" (the scripted seam stands in for real Z3).
        let z3_validate = |s: &str| -> Response {
            if s == "ab" {
                Response::Sat(MODEL_AB.to_string())
            } else {
                Response::Unsat
            }
        };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            4,
        );
        assert_eq!(g.witness.as_deref(), Some("ab"));
        assert_eq!(g.rounds.len(), 2);
        assert!(g.rounds[0].feedback.contains("relax"));
        assert!(
            g.rounds[0]
                .feedback
                .contains("not an unreachability verdict")
        );
        // Acceptance is double-gated: the feedback records both gates.
        assert!(g.rounds[1].feedback.contains("Z3 validated"));
        assert!(g.rounds[1].feedback.contains("CERTIFIED"));
        assert_eq!(g.rounds[1].candidates, vec!["ab".to_string()]);
        // The second prompt must carry the first round's verdict (iterative).
        assert!(mock.prompts[1].contains("contradicts reaching"));
    }

    #[test]
    fn every_enumerated_model_faces_both_gates_and_the_first_clean_one_wins() {
        // A single scaffold surrenders three distinct models. The first two fail
        // a gate each (Z3 refutes one, replay misfires on the other); the third
        // clears both. Enumeration must reach it without another proposer round.
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["len_max 2"]);
        let mut solve = |_r: usize, _cs: &[Scaffold]| RoundModels {
            response: Response::Sat(MODEL_AB.to_string()),
            candidates: vec!["zz".to_string(), "qq".to_string(), "ab".to_string()],
            stats: String::new(),
        };
        let certify = |src: &str, _t: &str| match src {
            "ab" => Verdict::ReachedTarget,
            // "qq" passes Z3 but fires the wrong marker.
            _ => Verdict::ReachedOtherMarker(vec![42]),
        };
        let z3_validate = |s: &str| -> Response {
            if s == "zz" {
                Response::Unsat // Z3 refutes the first candidate outright
            } else {
                Response::Sat(MODEL_AB.to_string())
            }
        };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            1,
        );
        assert_eq!(g.witness.as_deref(), Some("ab"));
        assert_eq!(g.rounds.len(), 1, "one round sufficed for three models");
        // Exactly one proposer call: the extra models cost no extra round.
        assert_eq!(mock.prompts.len(), 1);
        assert_eq!(g.rounds[0].candidates.len(), 3);
    }

    #[test]
    fn when_no_enumerated_model_passes_every_rejection_is_fed_back() {
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["len_max 2", "len_max 3"]);
        let mut solve = |_r: usize, _cs: &[Scaffold]| RoundModels {
            response: Response::Sat(MODEL_AB.to_string()),
            candidates: vec!["zz".to_string(), "qq".to_string()],
            stats: String::new(),
        };
        let certify = |_s: &str, _t: &str| Verdict::ReachedOtherMarker(vec![42]);
        let z3_validate = |_s: &str| Response::Sat(MODEL_AB.to_string());
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            2,
        );
        assert!(g.witness.is_none());
        let fb = &g.rounds[0].feedback;
        assert!(fb.contains("2 distinct model(s), none accepted"), "{fb}");
        assert!(fb.contains("\"zz\"") && fb.contains("\"qq\""), "{fb}");
        // Both rejections reach the next prompt, so the proposer can steer away.
        assert!(mock.prompts[1].contains("none accepted"));
    }

    #[test]
    fn a_stuck_round_feeds_back_where_z3_stalled() {
        // The steering signal a persistent session makes available: not just
        // "timeout", but which part of the search consumed the budget.
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["len_min 1", "len_min 2"]);
        let mut solve = |_r: usize, _cs: &[Scaffold]| RoundModels {
            response: Response::Timeout,
            candidates: Vec::new(),
            stats: "[z3-stats] recfun_body_expansion=2390 bv_bit2core=115776".to_string(),
        };
        let certify = |_s: &str, _t: &str| -> Verdict { unreachable!("no sat model") };
        let z3_validate = |_s: &str| -> Response { unreachable!("no sat model") };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            2,
        );
        assert!(g.witness.is_none());
        assert!(g.rounds[0].feedback.contains("Where Z3 stalled"));
        assert!(g.rounds[0].feedback.contains("recfun_body_expansion=2390"));
        // The profile reaches the proposer, not just the transcript.
        assert!(mock.prompts[1].contains("recfun_body_expansion=2390"));
    }

    #[test]
    fn a_sat_model_that_misfires_is_rejected_with_the_marker_name() {
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["len_max 2"]);
        let mut solve = |_r: usize, _cs: &[Scaffold]| RoundModels {
            response: Response::Sat(MODEL_AB.to_string()),
            candidates: vec!["ab".to_string()],
            stats: String::new(),
        };
        let certify = |_s: &str, _t: &str| Verdict::ReachedOtherMarker(vec![42]);
        // Z3 validates the macro-inlined candidate, but replay fires a different
        // marker — so gate 2 fails and the candidate is rejected.
        let z3_validate = |_s: &str| Response::Sat(MODEL_AB.to_string());
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            1,
        );
        assert!(g.witness.is_none());
        assert!(g.rounds[0].feedback.contains("other_marker"));
        assert!(g.rounds[0].feedback.contains("none accepted"));
    }

    #[test]
    fn timeout_feeds_back_constrain_further() {
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["len_min 1", "len_min 1"]);
        let mut solve = |_r: usize, _cs: &[Scaffold]| RoundModels::verdict(Response::Timeout);
        let certify = |_s: &str, _t: &str| -> Verdict { unreachable!("no sat model") };
        let z3_validate = |_s: &str| -> Response { unreachable!("no sat model to validate") };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            2,
        );
        assert!(g.witness.is_none());
        assert_eq!(g.rounds.len(), 2);
        assert!(g.rounds[0].feedback.contains("narrow the search"));
        assert!(mock.prompts[1].contains("narrow the search"));
    }

    #[test]
    fn an_invalid_scaffold_skips_the_solver_and_feeds_back_the_rejections() {
        let oracle = oracle_for("toml").expect("toml is registered");
        let mut mock = MockProposer::new(vec!["here is my plan:", "len_max 2"]);
        let mut calls = 0usize;
        let mut solve = |_r: usize, _cs: &[Scaffold]| {
            calls += 1;
            RoundModels::verdict(Response::Unsat)
        };
        let certify = |_s: &str, _t: &str| -> Verdict { unreachable!("no sat model") };
        let z3_validate = |_s: &str| -> Response { unreachable!("no sat model to validate") };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &z3_validate,
            2,
        );
        assert!(g.witness.is_none());
        assert_eq!(calls, 1, "round 1 had no valid lines, so no solver call");
        assert!(g.rounds[0].feedback.contains("no valid scaffold lines"));
        assert!(g.rounds[0].feedback.contains("unknown constraint"));
    }

    // --- end-to-end with the real solver (ignored: needs the z3 binary) ---

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a 2-character seq query)"]
    fn guided_loop_completes_a_scaffold_with_real_z3() {
        let oracle = oracle_for("toml").expect("toml is registered");
        // prefix "ab" + len_max 2 forces the model input_0 = "ab" exactly, so
        // the test is deterministic across Z3 versions.
        let mut mock = MockProposer::new(vec!["prefix \"ab\"\nlen_max 2"]);
        let dir = tempfile::tempdir().expect("tempdir");
        let mut solve = |round: usize, cs: &[Scaffold]| {
            let p = dir.path().join(format!("guided_round_{round}.smt2"));
            round_on_file(BASE_QUERY, cs, INPUT_VAR, &p, Duration::from_secs(30))
        };
        let certify = |src: &str, _tgt: &str| {
            if src == "ab" {
                Verdict::ReachedTarget
            } else {
                Verdict::NoMarker
            }
        };
        // Gate 1 with real Z3: macro-inline the decoded candidate into the
        // unmodified base query and let Z3 validate it.
        let validate = |candidate: &str| -> Response {
            match macro_inline_input(BASE_QUERY, candidate, INPUT_VAR) {
                Some(q) => {
                    let p = dir.path().join("validate.smt2");
                    std::fs::write(&p, q).expect("write validate query");
                    run_z3_file(&p, Duration::from_secs(30))
                }
                None => Response::Unknown("no seq input to macro-inline".to_string()),
            }
        };
        let g = guide_target(
            oracle,
            "boolean_invalid",
            "timeout",
            &names(),
            &mut mock,
            &mut solve,
            &certify,
            &validate,
            1,
        );
        assert_eq!(g.witness.as_deref(), Some("ab"));
    }

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a short seq query)"]
    fn a_session_round_enumerates_several_real_models_from_one_scaffold() {
        // The persistent-session payoff, end to end against real Z3: a single
        // scaffold that admits many inputs yields several *distinct* candidates
        // from one round, each blocked and re-checked in the same process.
        let base = split_at_check_sat(BASE_QUERY).expect("base has a check-sat");
        let mut session =
            Z3Session::start(base, Duration::from_secs(30)).expect("z3 -in session starts");
        // `len_max 3` leaves the solver plenty of freedom, so it can produce
        // several different inputs.
        let outcome = round_on_session(
            &mut session,
            BASE_QUERY,
            &[Scaffold::LenMax(3)],
            INPUT_VAR,
            3,
            None,
        );
        assert!(matches!(outcome.response, Response::Sat(_)));
        assert_eq!(outcome.candidates.len(), 3, "got {:?}", outcome.candidates);
        let unique: std::collections::BTreeSet<&String> = outcome.candidates.iter().collect();
        assert_eq!(unique.len(), 3, "models repeated: {:?}", outcome.candidates);

        // The scope was popped, so the next round starts from the pristine base:
        // a scaffold contradicting the *previous* round is satisfiable again.
        let again = round_on_session(
            &mut session,
            BASE_QUERY,
            &[Scaffold::Exact("zz".to_string())],
            INPUT_VAR,
            1,
            None,
        );
        assert!(
            matches!(again.response, Response::Sat(_)),
            "blocking clauses must not survive the pop: {:?}",
            again.response
        );
        assert_eq!(again.candidates, vec!["zz".to_string()]);
    }

    #[test]
    fn transcript_reports_double_gated_acceptance_never_replay_only() {
        // Any accepted witness is reported as double-gated (Z3 validation AND
        // replay); the transcript never advertises a replay-only acceptance path.
        let accepted = Guidance {
            witness: Some("x".to_string()),
            rounds: vec![GuidanceRound {
                scaffold: "prefix \"x\"".to_string(),
                notes: vec![],
                candidates: vec!["x".to_string()],
                feedback: "Z3 validated the macro-inlined candidate (sat); CERTIFIED".to_string(),
            }],
        };
        let t = render_guidance_transcript("desc", "m", "timeout", &accepted);
        assert!(t.contains("double-gated"), "accepted: {t}");
        assert!(t.contains("no replay-only acceptance"), "accepted: {t}");
        assert!(
            !t.contains("REPLAY ONLY"),
            "must not advertise a replay-only path: {t}"
        );

        let none = Guidance {
            witness: None,
            rounds: vec![],
        };
        let t2 = render_guidance_transcript("desc", "m", "timeout", &none);
        assert!(t2.contains("no accepted witness"), "none: {t2}");
    }
}

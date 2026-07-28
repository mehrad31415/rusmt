//! Query surgery and solver primitives for the co-solving loop.
//!
//! Everything here operates on the SMT-LIB text the backend emitted: it adds
//! assertions over the entry input, pins an input to a concrete value, splits a
//! query for a persistent session, and decodes a `(Seq (_ BitVec 32))` model
//! back into text. The loop that decides *which* surgery to apply lives in
//! [`crate::cosolve`].
//!
//! The one rule this module exists to enforce: a constraint may only be *added*
//! to a query. Definitions and the marker assertion are never rewritten, so
//! every model of `Q ∧ A` is a model of `Q` — the strengthening lemma. Its dual
//! matters as much: `unsat` of `Q ∧ A` says nothing about `Q`.

pub use crate::backend::response::Response;
use std::time::Duration;

/// The SMT constant the per-target query declares for the entry input.
pub const INPUT_VAR: &str = "input_0";

/// Default per-solve Z3 budget, in seconds.
pub const DEFAULT_Z3_SECS: u64 = 20;

/// Read `RUSMT_STAGE1_SECS`, falling back to [`z3_budget_from_env`].
///
/// Stage 1 — Z3 alone on the unmodified query — deserves its own, much smaller
/// budget. On the TOML parser it has never once returned a model (0/182 at 5 s,
/// and no verdict at all at 300 s), so a large T1 buys nothing and is subtracted
/// from the co-solving rounds, which do finish. Keeping the knobs separate stops
/// a tight per-round budget from starving the solve that matters.
pub fn stage1_budget_from_env() -> Duration {
    match std::env::var("RUSMT_STAGE1_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        Some(secs) => Duration::from_secs(secs),
        None => z3_budget_from_env(),
    }
}

/// Read `RUSMT_Z3_SECS` (default [`DEFAULT_Z3_SECS`]).
pub fn z3_budget_from_env() -> Duration {
    Duration::from_secs(
        std::env::var("RUSMT_Z3_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_Z3_SECS),
    )
}

// ---------------------------------------------------------------------------
// The constraint vocabulary.
// ---------------------------------------------------------------------------

/// One structural constraint on the entry input.
///
/// This is the whole vocabulary a proposal may use for the input. It is
/// deliberately unable to express "the input is exactly this": see
/// [`Constraints::fully_determines`], which rejects a set that leaves the solver
/// nothing to find. The solver produces every model; a proposal only narrows
/// where it looks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    /// `prefix "…"` — the input starts with the given text.
    Prefix(String),
    /// `suffix "…"` — the input ends with the given text.
    Suffix(String),
    /// `contains "…"` — the text occurs somewhere in the input.
    Contains(String),
    /// `forbid "…"` — the text occurs nowhere in the input.
    Forbid(String),
    /// `at <i> "<c>"` — the character at 0-based index `i` is exactly `c`.
    At(usize, char),
    /// `range <i> '<lo>' '<hi>'` — the character at `i` is in `[lo, hi]`.
    Range(usize, char, char),
    /// `len_min <n>` — the input has at least `n` characters.
    LenMin(usize),
    /// `len_max <n>` — the input has at most `n` characters.
    LenMax(usize),
}

impl Constraint {
    /// This constraint as a single SMT-LIB assertion over `var`.
    pub fn to_assertion(&self, var: &str) -> String {
        match self {
            Constraint::Prefix(t) => {
                format!("(assert (seq.prefixof {} {var}))", encode_seq_literal(t))
            }
            Constraint::Suffix(t) => {
                format!("(assert (seq.suffixof {} {var}))", encode_seq_literal(t))
            }
            Constraint::Contains(t) => {
                format!("(assert (seq.contains {var} {}))", encode_seq_literal(t))
            }
            Constraint::Forbid(t) => format!(
                "(assert (not (seq.contains {var} {})))",
                encode_seq_literal(t)
            ),
            Constraint::At(i, c) => format!(
                "(assert (and (> (seq.len {var}) {i}) (= (seq.nth {var} {i}) (_ bv{} 32))))",
                *c as u32
            ),
            Constraint::Range(i, lo, hi) => format!(
                "(assert (and (> (seq.len {var}) {i}) \
                 (bvuge (seq.nth {var} {i}) (_ bv{} 32)) \
                 (bvule (seq.nth {var} {i}) (_ bv{} 32))))",
                *lo as u32, *hi as u32
            ),
            Constraint::LenMin(n) => format!("(assert (>= (seq.len {var}) {n}))"),
            Constraint::LenMax(n) => format!("(assert (<= (seq.len {var}) {n}))"),
        }
    }

    /// How many character positions this pins to a single value.
    fn pinned_positions(&self) -> usize {
        match self {
            Constraint::Prefix(t) | Constraint::Suffix(t) => t.chars().count(),
            Constraint::At(..) => 1,
            _ => 0,
        }
    }
}

/// A set of constraints, checked as a unit.
#[derive(Debug, Clone, Default)]
pub struct Constraints(pub Vec<Constraint>);

impl Constraints {
    /// Whether these constraints leave the solver nothing to find.
    ///
    /// A proposal that pins every position of a length-bounded input has *named*
    /// the answer rather than narrowed the search, which is the one thing a
    /// proposal may not do. Rejecting it keeps the solver the source of every
    /// model instead of a checker for someone else's guess.
    pub fn fully_determines(&self) -> bool {
        let Some(max_len) = self
            .0
            .iter()
            .filter_map(|c| match c {
                Constraint::LenMax(n) => Some(*n),
                _ => None,
            })
            .min()
        else {
            return false; // unbounded length: always something left to find
        };
        let pinned: usize = self.0.iter().map(Constraint::pinned_positions).sum();
        max_len > 0 && pinned >= max_len
    }

    /// The assertion block these constraints contribute to a query.
    pub fn block(&self, var: &str) -> String {
        let mut b = String::from("; co-solving constraints (strengthening only)\n");
        for c in &self.0 {
            b.push_str(&c.to_assertion(var));
            b.push('\n');
        }
        b
    }
}

/// The constraint grammar as shown to the proposer.
pub const CONSTRAINT_GRAMMAR: &str = r#"  prefix "<text>"          the input starts with <text>
  suffix "<text>"          the input ends with <text>
  contains "<text>"        <text> occurs somewhere in the input
  forbid "<text>"          <text> occurs nowhere in the input
  at <i> "<c>"             the character at 0-based index <i> is exactly <c>
  range <i> '<lo>' '<hi>'  the character at <i> is in the inclusive range
  len_min <n>              the input has at least <n> characters
  len_max <n>              the input has at most <n> characters"#;

/// Parse one constraint line. `Ok(None)` for blank lines and `#` comments.
pub fn parse_constraint_line(line: &str) -> Result<Option<Constraint>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    let (kw, rest) = match line.split_once(char::is_whitespace) {
        Some((k, r)) => (k, r.trim_start()),
        None => (line, ""),
    };
    let c = match kw {
        "prefix" | "suffix" | "contains" | "forbid" => {
            let (t, after) = parse_any_quoted(rest)?;
            expect_end(after)?;
            match kw {
                "prefix" => Constraint::Prefix(t),
                "suffix" => Constraint::Suffix(t),
                "contains" => Constraint::Contains(t),
                _ => Constraint::Forbid(t),
            }
        }
        "at" => {
            let (i, after) = parse_index(rest)?;
            let (c, after) = parse_one_char(after.trim_start())?;
            expect_end(after)?;
            Constraint::At(i, c)
        }
        "range" => {
            let (i, after) = parse_index(rest)?;
            let (lo, after) = parse_one_char(after.trim_start())?;
            let (hi, after) = parse_one_char(after.trim_start())?;
            expect_end(after)?;
            if lo > hi {
                return Err(format!("empty range: '{lo}' > '{hi}'"));
            }
            Constraint::Range(i, lo, hi)
        }
        "len_min" | "len_max" => {
            let (n, after) = parse_index(rest)?;
            expect_end(after)?;
            if kw == "len_min" {
                Constraint::LenMin(n)
            } else {
                Constraint::LenMax(n)
            }
        }
        other => return Err(format!("unknown constraint `{other}`")),
    };
    Ok(Some(c))
}

/// Parse a quoted literal starting at `s`; returns the text and the remainder.
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
                // `\u{XX}` — most error markers are about control characters,
                // which cannot be written literally on a single line.
                Some((_, 'u')) => {
                    if !matches!(chars.next(), Some((_, '{'))) {
                        return Err("expected `{` after \\u".to_string());
                    }
                    let mut hex = String::new();
                    let mut closed = false;
                    for (_, h) in chars.by_ref() {
                        if h == '}' {
                            closed = true;
                            break;
                        }
                        hex.push(h);
                    }
                    if !closed {
                        return Err("unterminated \\u{…}".to_string());
                    }
                    let cp = u32::from_str_radix(&hex, 16)
                        .map_err(|_| format!("bad hex in \\u{{{hex}}}"))?;
                    out.push(char::from_u32(cp).ok_or(format!("not a code point: \\u{{{hex}}}"))?);
                }
                Some((_, e)) => return Err(format!("unknown escape \\{e}")),
                None => return Err("dangling backslash".to_string()),
            }
        } else {
            out.push(c);
        }
    }
    Err(format!("missing closing {quote}"))
}

fn parse_any_quoted(s: &str) -> Result<(String, &str), String> {
    match s.chars().next() {
        Some('"') => parse_quoted(s, '"'),
        Some('\'') => parse_quoted(s, '\''),
        _ => Err("expected a quoted literal".to_string()),
    }
}

fn parse_one_char(s: &str) -> Result<(char, &str), String> {
    let (text, rest) = parse_any_quoted(s)?;
    let mut it = text.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => Ok((c, rest)),
        _ => Err(format!("expected exactly one character, got \"{text}\"")),
    }
}

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

fn expect_end(rest: &str) -> Result<(), String> {
    if rest.trim().is_empty() {
        Ok(())
    } else {
        Err(format!("trailing text: `{}`", rest.trim()))
    }
}

// ---------------------------------------------------------------------------
// Query surgery.
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

/// Whether the query declares `var` with the code-point-sequence sort.
pub fn query_has_seq_input(query: &str, var: &str) -> bool {
    query.contains(&format!("(declare-const {var} (Seq (_ BitVec 32)))"))
}

/// Insert a block immediately before the query's final `(check-sat)`.
fn insert_before_check_sat(query: &str, block: &str) -> Option<String> {
    let at = query.rfind("(check-sat)")?;
    let mut out = String::with_capacity(query.len() + block.len());
    out.push_str(&query[..at]);
    out.push_str(block);
    out.push_str(&query[at..]);
    Some(out)
}

/// Conjoin constraints to an unmodified query — added assertions only.
pub fn strengthen_query(query: &str, cs: &Constraints, var: &str) -> Option<String> {
    insert_before_check_sat(query, &cs.block(var))
}

/// Everything before the final `(check-sat)`: the base a session loads once.
pub fn split_at_check_sat(query: &str) -> Option<&str> {
    query.rfind("(check-sat)").map(|at| &query[..at])
}

/// Pin the entry input to `text` with a single equality assertion.
///
/// This is the acceptance check. It is pure strengthening of the *unmodified*
/// query — no definition is touched, the marker assertion is untouched — so a
/// `sat` here is a model of the original reachability question with this input.
/// That is what turns a candidate found over a sliced or constrained query into
/// a witness, and it is the only thing that does.
pub fn pin_input(query: &str, text: &str, var: &str) -> Option<String> {
    insert_before_check_sat(
        query,
        &format!(
            "; acceptance: the input is pinned; the query is otherwise unmodified\n\
             (assert (= {var} {}))\n",
            encode_seq_literal(text)
        ),
    )
}

/// Pin the entry input concretely for the authoring spec-example gate.
pub fn fix_input_query(query: &str, text: &str, var: &str) -> Option<String> {
    pin_input(query, text, var)
}

// ---------------------------------------------------------------------------
// Model decoding.
// ---------------------------------------------------------------------------

/// A minimal s-expression, enough to walk a Z3 model.
#[derive(Debug)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

/// Tokenize and parse the top-level s-expressions of `text`, tolerantly: a
/// stray atom such as the leading `sat` line parses as an atom and is skipped.
fn parse_sexprs(text: &str) -> Vec<SExpr> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' | ')' => tokens.push(c.to_string()),
            '"' => {
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

/// Decode a bit-vector value into its unsigned numeric value.
fn decode_bv(e: &SExpr) -> Option<u32> {
    match e {
        SExpr::Atom(a) if a.starts_with("#x") => u32::from_str_radix(&a[2..], 16).ok(),
        SExpr::Atom(a) if a.starts_with("#b") => u32::from_str_radix(&a[2..], 2).ok(),
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(u), SExpr::Atom(bv), SExpr::Atom(_w)] if u == "_" => {
                bv.strip_prefix("bv").and_then(|n| n.parse().ok())
            }
            _ => None,
        },
        _ => None,
    }
}

/// Decode a `(Seq (_ BitVec 32))` value term into text.
fn decode_seq_value(e: &SExpr) -> Option<String> {
    match e {
        SExpr::Atom(a) if a == "seq.empty" => Some(String::new()),
        SExpr::List(items) => match items.as_slice() {
            [SExpr::Atom(a), SExpr::Atom(empty), _sort] if a == "as" && empty == "seq.empty" => {
                Some(String::new())
            }
            [SExpr::Atom(u), v] if u == "seq.unit" => {
                char::from_u32(decode_bv(v)?).map(String::from)
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

/// Find the model's definition of `var` and decode it into text. `None` when
/// the value is absent or not concrete — which the loop treats as feedback.
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
// Running Z3 on a file.
// ---------------------------------------------------------------------------

/// Run `z3 -smt2` on `path` under `budget`.
///
/// `-t:` makes Z3 self-terminate; a monitor kill is the backstop, because on
/// these queries Z3 can stall in a phase that ignores its own timeout. A kill
/// with no verdict is reported as [`Response::Timeout`]; a crash — Z3 4.15.4
/// segfaults on some partially-symbolic variants of these queries — folds into
/// [`Response::Unknown`] with the signal named, so the loop can tell a dead
/// solver from a slow one.
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

    let start = Instant::now();
    let deadline = budget + Duration::from_secs(5);
    let mut status = None;
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(s)) => {
                status = Some(s);
                break false;
            }
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
        _ => Response::Unknown(match status {
            Some(s) => format!("z3 produced no verdict ({s})"),
            None => "z3 produced no verdict".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL_AB: &str = "sat\n(\n  (define-fun input_0 () (Seq (_ BitVec 32))\n    \
        (seq.++ (seq.unit #x00000061) (seq.unit #x00000062)))\n)\n";

    fn parse_all(text: &str) -> (Vec<Constraint>, Vec<String>) {
        let mut cs = Vec::new();
        let mut bad = Vec::new();
        for l in text.lines() {
            match parse_constraint_line(l) {
                Ok(Some(c)) => cs.push(c),
                Ok(None) => (),
                Err(e) => bad.push(e),
            }
        }
        (cs, bad)
    }

    #[test]
    fn every_constraint_kind_parses() {
        let (cs, bad) = parse_all(
            "# comment\nprefix \"ab\"\nsuffix \"\\n\"\ncontains \"k = \"\nforbid \"\\\"\"\n\
             at 0 \"[\"\nrange 3 '0' '9'\nlen_min 2\nlen_max 40",
        );
        assert!(bad.is_empty(), "rejected: {bad:?}");
        assert_eq!(
            cs,
            vec![
                Constraint::Prefix("ab".to_string()),
                Constraint::Suffix("\n".to_string()),
                Constraint::Contains("k = ".to_string()),
                Constraint::Forbid("\"".to_string()),
                Constraint::At(0, '['),
                Constraint::Range(3, '0', '9'),
                Constraint::LenMin(2),
                Constraint::LenMax(40),
            ]
        );
    }

    #[test]
    fn control_characters_can_be_named_with_a_unicode_escape() {
        // Most error markers are about control characters, so the grammar has to
        // be able to name them at all.
        let (cs, bad) = parse_all("at 1 \"\\u{0}\"\nforbid \"\\u{7f}\"");
        assert!(bad.is_empty(), "rejected: {bad:?}");
        assert_eq!(
            cs,
            vec![
                Constraint::At(1, '\0'),
                Constraint::Forbid("\u{7f}".to_string())
            ]
        );
        assert!(parse_constraint_line("at 1 \"\\u{d800}\"").is_err());
        assert!(parse_constraint_line("at 1 \"\\u{0\"").is_err());
    }

    #[test]
    fn malformed_lines_are_reported_not_silently_dropped() {
        let (cs, bad) = parse_all("starts_with \"x\"\nprefix x\nrange 1 '9' '0'\nlen_min 2");
        assert_eq!(cs, vec![Constraint::LenMin(2)]);
        assert_eq!(bad.len(), 3);
        assert!(bad[0].contains("unknown constraint"));
        assert!(bad[1].contains("quoted"));
        assert!(bad[2].contains("empty range"));
    }

    #[test]
    fn a_constraint_set_that_names_the_whole_input_is_rejected() {
        // prefix "ab" with len_max 2 leaves the solver nothing to find: that is
        // a proposal naming the answer, which the loop must not accept.
        assert!(
            Constraints(vec![
                Constraint::Prefix("ab".to_string()),
                Constraint::LenMax(2)
            ])
            .fully_determines()
        );
        // The same prefix with room to grow is legitimate narrowing.
        assert!(
            !Constraints(vec![
                Constraint::Prefix("ab".to_string()),
                Constraint::LenMax(4)
            ])
            .fully_determines()
        );
        // Unbounded length always leaves something free.
        assert!(!Constraints(vec![Constraint::Prefix("ab".to_string())]).fully_determines());
        // Per-position pinning counts the same way.
        assert!(
            Constraints(vec![
                Constraint::At(0, 'a'),
                Constraint::At(1, 'b'),
                Constraint::LenMax(2)
            ])
            .fully_determines()
        );
    }

    #[test]
    fn translations_are_the_expected_assertions() {
        let v = INPUT_VAR;
        let cases = [
            (
                Constraint::Prefix("ab".to_string()),
                "(assert (seq.prefixof (seq.++ (seq.unit (_ bv97 32)) (seq.unit (_ bv98 32))) input_0))",
            ),
            (
                Constraint::Contains("=".to_string()),
                "(assert (seq.contains input_0 (seq.unit (_ bv61 32))))",
            ),
            (
                Constraint::Forbid("#".to_string()),
                "(assert (not (seq.contains input_0 (seq.unit (_ bv35 32)))))",
            ),
            (
                Constraint::At(0, '['),
                "(assert (and (> (seq.len input_0) 0) (= (seq.nth input_0 0) (_ bv91 32))))",
            ),
            (Constraint::LenMin(2), "(assert (>= (seq.len input_0) 2))"),
            (Constraint::LenMax(40), "(assert (<= (seq.len input_0) 40))"),
        ];
        for (c, expected) in cases {
            assert_eq!(c.to_assertion(v), expected);
        }
        assert_eq!(
            Constraint::Prefix(String::new()).to_assertion(v),
            "(assert (seq.prefixof (as seq.empty (Seq (_ BitVec 32))) input_0))"
        );
    }

    #[test]
    fn strengthening_adds_assertions_and_changes_nothing_else() {
        let q = "(declare-const input_0 (Seq (_ BitVec 32)))\n(assert true)\n(check-sat)\n(get-model)\n";
        let cs = Constraints(vec![Constraint::LenMax(4)]);
        let out = strengthen_query(q, &cs, INPUT_VAR).expect("has check-sat");
        let at_assert = out
            .find("(assert (<= (seq.len input_0) 4))")
            .expect("inserted");
        assert!(at_assert < out.rfind("(check-sat)").expect("kept"));
        // The original text survives verbatim on both sides of the insertion.
        assert!(out.starts_with("(declare-const input_0 (Seq (_ BitVec 32)))"));
        assert!(out.contains("(assert true)"));
        assert!(out.ends_with("(check-sat)\n(get-model)\n"));
        assert!(strengthen_query("(assert true)", &cs, INPUT_VAR).is_none());
    }

    #[test]
    fn pinning_an_input_only_adds_an_equality() {
        let q = "(declare-const input_0 (Seq (_ BitVec 32)))\n(assert true)\n(check-sat)\n";
        let out = pin_input(q, "za", INPUT_VAR).expect("has check-sat");
        let eq = "(assert (= input_0 (seq.++ (seq.unit (_ bv122 32)) (seq.unit (_ bv97 32)))))";
        assert!(out.find(eq).expect("pinned") < out.rfind("(check-sat)").unwrap());
        // Nothing else moved: the declaration and the original assertion remain.
        assert!(out.contains("(declare-const input_0 (Seq (_ BitVec 32)))"));
        assert!(out.contains("(assert true)"));
        assert!(
            pin_input(q, "", INPUT_VAR)
                .expect("ok")
                .contains("(assert (= input_0 (as seq.empty (Seq (_ BitVec 32)))))")
        );
        assert!(pin_input("(assert true)", "x", INPUT_VAR).is_none());
    }

    #[test]
    fn models_decode_across_value_shapes() {
        assert_eq!(decode_seq_model(MODEL_AB, INPUT_VAR).as_deref(), Some("ab"));
        let nested = "sat\n(model (define-fun input_0 () (Seq (_ BitVec 32)) \
             (seq.++ (seq.++ (seq.unit (_ bv104 32)) (seq.unit (_ bv105 32))) \
             (seq.unit #x00000021))))";
        assert_eq!(decode_seq_model(nested, INPUT_VAR).as_deref(), Some("hi!"));
        let empty = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) \
             (as seq.empty (Seq (_ BitVec 32)))))";
        assert_eq!(decode_seq_model(empty, INPUT_VAR).as_deref(), Some(""));
        assert_eq!(decode_seq_model(MODEL_AB, "nope"), None);
    }

    #[test]
    fn undecodable_models_yield_none() {
        // A surrogate is not a code point, and a symbolic value is not concrete.
        let surrogate = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) (seq.unit #x0000D800)))";
        assert_eq!(decode_seq_model(surrogate, INPUT_VAR), None);
        let symbolic = "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) (seq.++ a b)))";
        assert_eq!(decode_seq_model(symbolic, INPUT_VAR), None);
        assert_eq!(decode_seq_model("sat\n()", INPUT_VAR), None);
    }

    #[test]
    fn a_seq_input_declaration_is_recognized() {
        assert!(query_has_seq_input(
            "(declare-const input_0 (Seq (_ BitVec 32)))\n(check-sat)\n",
            INPUT_VAR
        ));
        assert!(!query_has_seq_input(
            "(declare-const input_0 Com)\n(check-sat)\n",
            INPUT_VAR
        ));
    }
}

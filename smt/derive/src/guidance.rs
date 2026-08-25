//! Query surgery and the one Z3 runner.
//!
//! Pins the entry input to a concrete value, decodes a `(Seq (_ BitVec 32))` or
//! bit-vector model, and runs `z3 -smt2` under a budget.
//!
//! A query is only ever added to; definitions and the marker assertion are never
//! rewritten. That is what makes a `sat` on a pinned query a statement about the
//! original one.
pub use crate::backend::response::Response;
use std::time::Duration;

/// The SMT constant the per-target query declares for the entry input.
pub const INPUT_VAR: &str = "input_0";

/// The SMT constant an observation query binds the returned `Path` to.
pub const OBSERVED_PATH: &str = "observed_path";

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

/// Which bit positions are set in a bit-vector value in a model.
///
/// Z3 prints a `(_ BitVec N)` value as `#b…`, or `#x…` when the width divides
/// by four, or as `(_ bvV N)`. Bit 0 is the least significant, which is the
/// rank order [`crate::backend::z3::path::bit_index`] assigns.
pub fn decode_bitvec_bits(model_text: &str, var: &str) -> Option<Vec<usize>> {
    fn digits(e: &SExpr) -> Option<String> {
        match e {
            SExpr::Atom(a) if a.starts_with("#b") => Some(a[2..].to_string()),
            SExpr::Atom(a) if a.starts_with("#x") => Some(
                a[2..]
                    .chars()
                    .map(|c| {
                        let v = c.to_digit(16).unwrap_or(0);
                        format!("{v:04b}")
                    })
                    .collect(),
            ),
            SExpr::List(items) => match items.as_slice() {
                [SExpr::Atom(u), SExpr::Atom(bv), SExpr::Atom(w)] if u == "_" => {
                    let n: u128 = bv.strip_prefix("bv")?.parse().ok()?;
                    let width: usize = w.parse().ok()?;
                    Some(format!("{n:0width$b}"))
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn find(e: &SExpr, var: &str) -> Option<String> {
        let SExpr::List(items) = e else { return None };
        if let [
            SExpr::Atom(df),
            SExpr::Atom(name),
            SExpr::List(_),
            _sort,
            value,
        ] = items.as_slice()
            && df == "define-fun"
            && name == var
        {
            return digits(value);
        }
        items.iter().find_map(|i| find(i, var))
    }
    let bits = parse_sexprs(model_text).iter().find_map(|e| find(e, var))?;
    let n = bits.len();
    Some(
        bits.char_indices()
            .filter(|(_, c)| *c == '1')
            .map(|(i, _)| n - 1 - i)
            .collect(),
    )
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
    fn a_pin_goes_before_the_last_check_sat_and_keeps_the_text_around_it() {
        let q = "(declare-const input_0 (Seq (_ BitVec 32)))\n(assert true)\n\
                 (check-sat)\n(get-info :reason-unknown)\n(get-model)\n";
        let out = pin_input(q, "a", INPUT_VAR).expect("has check-sat");
        assert!(out.starts_with("(declare-const input_0 (Seq (_ BitVec 32)))"));
        assert!(out.ends_with("(check-sat)\n(get-info :reason-unknown)\n(get-model)\n"));
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

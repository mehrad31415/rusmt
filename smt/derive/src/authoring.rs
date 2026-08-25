//! Drafts a reference semantics from a prose spec, behind mechanical gates.
//!
//! A [`Proposer`] writes the `#[smt_type]` / `#[smt_fn]` source; the framework's
//! own machinery judges it. Gates run cheapest first:
//!
//! 1. front end -- the DSL parser and sort checker accept it
//! 2. back end -- it transpiles to well-formed SMT-LIB
//! 3. marker -- it declares at least one `Path::named`
//! 4. reachability -- no declared marker is dead under the draft's own semantics
//! 5. spec example -- it agrees with the author's examples
//!
//! Failures are fed back verbatim and the loop repeats to a round budget.
//! Admission means the conjunction of those gates held: the draft is internally
//! consistent, not correct. No reference exists to check it against.

use crate::guidance::Response;
use crate::ir::ctxt::IRContext;
use crate::ir::index::UsrFunId;
use crate::ir::sort::Sort;
use crate::proposer::Proposer;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

/// Default number of drafting rounds.
pub const DEFAULT_MAX_ROUNDS: usize = 4;
// A spending limit, not a tuned parameter. Same status as `cosolve::DEFAULT_ROUNDS`.

/// Default per-query Z3 budget for the behavioral gates, in seconds.
pub const DEFAULT_AUTHOR_Z3_SECS: u64 = 20;

/// Read `RUSMT_AUTHOR_Z3_SECS` (default [`DEFAULT_AUTHOR_Z3_SECS`]).
pub fn author_z3_budget_from_env() -> Duration {
    let secs = std::env::var("RUSMT_AUTHOR_Z3_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_AUTHOR_Z3_SECS);
    Duration::from_secs(secs)
}

/// The conventions the task imposes, and where the DSL itself is defined.
///
/// Not an API summary: a curated list is a second artifact that drifts from the
/// crate it describes. The proposer reads the crates.
pub const DSL_GUIDE: &str = r#"A RuSMT specification is one self-contained Rust
source file of `#[smt_type]` data types and `#[smt_fn]` functions, written in a
restricted subset that translates to SMT.

== Requirements, and where each is enforced ==
These are requirements of the task. None is a matter of taste: each is checked
by code you can read, cited so you can confirm the wording against it.

- Exactly ONE top-level entry function — the unique function no other function
  calls — and for a text language it takes exactly one `Seq<U32>` parameter,
  the input as Unicode code points. It must fix every other piece of context
  internally, so its parameters are exactly the inputs to be synthesized.
  Enforced: `detect_entry` and `entry_takes_one_text_input`,
  smt/derive/src/authoring.rs.
- Every error or edge condition the test suite should reach is a NAMED marker,
  `Path::named("snake_case_name")`, carried in a result enum:
      #[smt_type]
      pub enum EvalResult { Err(Path), Ok(I64) }
  The guard pattern is to special-case the diverging input and fall through to
  the ordinary operation otherwise:
      if *divisor.eq(I64::from(0)) {
          EvalResult::Err(Path::named("division_by_zero"))
      } else {
          EvalResult::Ok(lhs.bv_div(divisor))
      }
  Enforced: marker collection in smt/remark/src/marker.rs.
- Recursion is the ONLY iteration: no loops, no mutation, no references, no
  closures, no `self` receivers, no `?`; `else` is mandatory on every `if`.
  Enforced: smt/derive/src/parser/ (ctxt.rs, func.rs) — what the front end
  rejects, you may not write.
- Branch on an SMT `Boolean` by dereferencing: `if *a.eq(b) { .. } else { .. }`.
- Self-referential ADTs use `Cloak<T>` at every recursive position
  (`Add(Cloak<Aexp>, Cloak<Aexp>)`); build with `Cloak::shield(x)`, read with
  `x.reveal()`. At least one variant must be non-recursive.
- Imports: `use rusmt_smt_remark_derive::{smt_fn, smt_type};` and
  `use rusmt_smt_stdlib::{...};` (plus trait imports such as
  `bitvector::BitvectorOps` and `smt::SMT` when their methods are used).

== Where the DSL is defined ==
Three crates define the language. They are AUTHORITATIVE and you can read all
of them; read what you need rather than guessing, and where this text and the
source disagree, the source is right.

  smt/stdlib/    every sort you can use and every operation on it.
                 `src/dt/` has one file per sort — `seq.rs` (text and
                 sequences), `int.rs` (unbounded `Integer`, radix decoding,
                 range predicates), `bitvector.rs` (`I64`/`U32` arithmetic),
                 `boolean.rs`, `string.rs`, `array.rs`, `set.rs`,
                 `path.rs` (markers), `cloak.rs`, `real.rs`, `float.rs`,
                 `smt.rs` (`eq`/`ne`) — and `src/exp.rs` has the quantified
                 expressions. Every `pub fn` is callable from a specification;
                 anything named `test_*` is a unit test, not API.
  smt/remark/    the `#[smt_type]` / `#[smt_fn]` macros: which item shapes are
                 accepted, and how markers are collected from a function body.
  smt/derive/    the compiler. `src/parser/` is the front end and therefore the
                 definition of the accepted subset — what it rejects, you may
                 not write; `src/ir/` is the sorts a specification lowers into;
                 `src/backend/` is the SMT-LIB emitted for each construct.

Nothing else in the repository is readable. In particular the reference
semantics for the language you are specifying is withheld on purpose: the
exercise is to write it from the specification, not to recover it.
"#;

// ---------------------------------------------------------------------------
// Spec examples (the human's behavioral test intent).
// ---------------------------------------------------------------------------

/// The expected observable class of one spec example, phrased in terms of the
/// draft's *named markers* (its declared error intent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    /// The input is accepted: no named marker fires.
    Ok,
    /// The given named marker fires.
    Err(String),
    /// The input is rejected (some named marker fires, no particular one).
    NoMatch,
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expectation::Ok => write!(f, "ok"),
            Expectation::Err(m) => write!(f, "err:{m}"),
            Expectation::NoMatch => write!(f, "nomatch"),
        }
    }
}

/// One concrete behavioral example from the prose spec / the human author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    /// The concrete input text the entry function receives.
    pub input: String,
    /// The expected observable class.
    pub expect: Expectation,
}

/// Decode the `\n \t \r \\` escapes allowed in the INPUT column (a literal
/// tab would collide with the column separator, a literal newline with the
/// line format).
fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(e) => return Err(format!("unknown escape \\{e}")),
            None => return Err("dangling backslash".to_string()),
        }
    }
    Ok(out)
}

/// Parse an examples file: one `INPUT<TAB>EXPECT` per line, where EXPECT is
/// `ok`, `err:<marker_name>`, or `nomatch`; blank lines and `#` comments are
/// skipped. INPUT may use the `\n \t \r \\` escapes. Strict: any malformed
/// line is an error (examples are trusted test intent, not model output).
pub fn parse_examples(text: &str) -> Result<Vec<Example>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (raw_input, raw_expect) = line
            .split_once('\t')
            .ok_or(format!("line {n}: expected INPUT<TAB>EXPECT"))?;
        let input = unescape(raw_input).map_err(|e| format!("line {n}: {e}"))?;
        let expect = match raw_expect.trim() {
            "ok" => Expectation::Ok,
            "nomatch" => Expectation::NoMatch,
            e if e.starts_with("err:") => {
                let m = e["err:".len()..].trim();
                if m.is_empty() {
                    return Err(format!("line {n}: err: needs a marker name"));
                }
                Expectation::Err(m.to_string())
            }
            other => {
                return Err(format!(
                    "line {n}: unknown expectation `{other}` (ok | err:<marker> | nomatch)"
                ));
            }
        };
        out.push(Example { input, expect });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// The drafting rounds.
// ---------------------------------------------------------------------------

/// One drafting round: the candidate, the gate failures it triggered
/// (empty ⇒ the candidate passed all gates), and non-failure gate outcomes
/// worth recording (reachability witnesses, timeout warnings, confirmations).
pub struct Round {
    /// The proposer's draft for this round.
    pub candidate: String,
    /// Gate-failure feedback (empty if the draft was admitted).
    pub failures: Vec<String>,
    /// Non-failure gate outcomes (the per-round transcript of gates 4–5).
    pub notes: Vec<String>,
}

/// The outcome of an authoring session.
pub struct AuthoringOutcome {
    /// The admitted draft, if any round passed all gates.
    pub accepted: Option<String>,
    /// The names of the draft's named markers (its declared test intent).
    pub markers: Vec<String>,
    /// Per-round transcript.
    pub rounds: Vec<Round>,
}

/// Renders the session: every draft, its gate failures, and the solver's findings.
pub fn render_transcript(
    spec_label: &str,
    examples: &[Example],
    outcome: &AuthoringOutcome,
) -> String {
    let mut s = format!(
        "specification : {}\nspec examples : {}\nrounds        : {}\n\n",
        spec_label,
        examples.len(),
        outcome.rounds.len()
    );
    for (i, r) in outcome.rounds.iter().enumerate() {
        s.push_str(&format!(
            "=== round {} ===\n--- draft ---\n{}\n",
            i + 1,
            r.candidate
        ));
        if r.failures.is_empty() {
            s.push_str("--- gates: ADMITTED ---\n");
        } else {
            s.push_str("--- gate failures ---\n");
            for f in &r.failures {
                s.push_str(&format!("* {f}\n"));
            }
        }
        for n in &r.notes {
            s.push_str(&format!("note: {n}\n"));
        }
        s.push('\n');
    }
    match &outcome.accepted {
        Some(_) => s.push_str(&format!(
            "status: ADMITTED by all gates (named markers: {})\n\
             the draft is mechanically admissible, NOT trusted: review it, then \
             run synthesis against its markers.\n",
            outcome.markers.join(", ")
        )),
        None => s.push_str("status: no draft passed the gates within the round budget\n"),
    }
    s
}

/// Extract a printable message from a caught panic payload.
fn panic_msg(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(non-string panic payload)".to_string())
}

/// Run the mechanical gates (1–3) on one candidate draft. Returns the
/// failures (empty ⇒ admitted), the IR when the front end accepted the draft,
/// and the base SMT-LIB when the back end emitted it (both are reused by the
/// behavioral gates).
fn run_gates(candidate: &str) -> (Vec<String>, Option<IRContext>, Option<String>) {
    let mut failures = Vec::new();

    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return (
                vec![format!("internal: cannot create temp dir: {e}")],
                None,
                None,
            );
        }
    };
    let path = dir.path().join("draft.rs");
    if let Err(e) = fs::write(&path, candidate) {
        return (
            vec![format!("internal: cannot write draft: {e}")],
            None,
            None,
        );
    }

    // Gate 1: the DSL front end (parser + sort checker). The front end uses
    // panics for some internal invariants, so a panic is converted into
    // feedback rather than propagated.
    let ir = match catch_unwind(AssertUnwindSafe(|| crate::model(&path))) {
        Err(p) => {
            failures.push(format!(
                "DSL front-end rejected the draft: {}",
                panic_msg(p)
            ));
            return (failures, None, None);
        }
        Ok(Err(e)) => {
            failures.push(format!("DSL parse/type error: {e}"));
            return (failures, None, None);
        }
        Ok(Ok(ir)) => ir,
    };

    // Gate 2: SMT emission through the text backend. The emitted base code is
    // kept: the behavioral gates build their per-marker queries on top of it.
    let mut base_smt = None;
    {
        use crate::backend::codegen::CodeGen;
        use crate::backend::z3::ctxt::CodeGenZ3;
        match catch_unwind(AssertUnwindSafe(|| CodeGenZ3::new().process(&ir, 0))) {
            Err(p) => failures.push(format!("SMT emission panicked: {}", panic_msg(p))),
            Ok(Err(e)) => failures.push(format!("SMT emission failed: {e:?}")),
            Ok(Ok(code)) => base_smt = Some(code),
        }
    }

    // Gate 3: declared test intent — at least one named marker.
    if ir.marker_names.is_empty() {
        failures.push(
            "the draft declares no `Path::named(...)` markers; every error/edge \
             condition the test suite should reach must be a named marker"
                .to_string(),
        );
    }

    (failures, Some(ir), base_smt)
}

// ---------------------------------------------------------------------------
// The behavioral gates (4–5): the framework's synthesis machinery, turned
// against the draft itself.
// ---------------------------------------------------------------------------

/// The draft's top-level entry: the unique function that no *other* function
/// calls (self-recursion does not disqualify). The DSL guide requires drafts
/// to declare exactly one; anything else is gate feedback.
fn detect_entry(ir: &IRContext) -> Result<String, String> {
    use crate::backend::z3::fun::collect_called_functions;

    let mut ids: Vec<(&str, UsrFunId)> = Vec::new();
    for (name, insts) in ir.fn_registry.lookup() {
        for id in insts.values() {
            ids.push((name.as_ref(), *id));
        }
    }
    let mut callees: BTreeSet<UsrFunId> = BTreeSet::new();
    for (_, id) in &ids {
        let def = ir.fn_registry.retrieve_def(*id);
        for callee in collect_called_functions(&def.body, &def.root_exp_id) {
            if callee != *id {
                callees.insert(callee);
            }
        }
    }
    let mut roots: Vec<&str> = ids
        .iter()
        .filter(|(_, id)| !callees.contains(id))
        .map(|(name, _)| *name)
        .collect();
    roots.dedup();
    match roots.as_slice() {
        [one] => Ok(one.to_string()),
        [] => Err(
            "the draft has no top-level entry function (every function is called \
             by another); declare exactly ONE entry whose parameters are the \
             inputs to synthesize"
                .to_string(),
        ),
        many => Err(format!(
            "the draft must declare exactly ONE top-level entry function (one \
             called by no other function); found {}: {}",
            many.len(),
            many.join(", ")
        )),
    }
}

/// Whether the entry takes exactly one code-point-sequence parameter
/// (`Seq<U32>`) — the input shape the spec-example gate (and witness
/// decoding) speaks.
fn entry_takes_one_text_input(ir: &IRContext, entry: &str) -> bool {
    let Some(insts) = ir
        .fn_registry
        .lookup()
        .iter()
        .find(|(name, _)| name.as_ref() == entry)
        .map(|(_, insts)| insts)
    else {
        return false;
    };
    let Some(id) = insts.values().next() else {
        return false;
    };
    let sig = ir.fn_registry.retrieve_sig(*id);
    matches!(sig.params.as_slice(),
        [(_, Sort::Seq(inner))] if matches!(**inner, Sort::U32))
}

/// Run the behavioral gates on an admitted-so-far draft. `solve` is the
/// solver seam: `(label, query) -> Response` (the CLI passes a real Z3
/// invocation; tests pass a script). Failures and notes are appended to the
/// round's transcript.
fn behavioral_gates(
    ir: &IRContext,
    base_smt: &str,
    examples: &[Example],
    solve: &mut dyn FnMut(&str, &str) -> Response,
    failures: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    use crate::backend::codegen::CodeGen;
    use crate::backend::z3::ctxt::CodeGenZ3;

    let entry = match detect_entry(ir) {
        Ok(e) => e,
        Err(msg) => {
            failures.push(msg);
            return;
        }
    };
    notes.push(format!("entry function: `{entry}`"));
    let seq_input = entry_takes_one_text_input(ir, &entry);

    // Per-marker query builder: the SAME query the synthesis pipeline would
    // run against this draft, at k=0 (genuine recursive semantics, so unsat
    // is a real verdict). The backend panics on structural misuse (e.g. the
    // entry's return type carries no `Path`); that becomes feedback.
    let solver = CodeGenZ3::new();
    let build_query = |id: usize| -> Result<String, String> {
        catch_unwind(AssertUnwindSafe(|| {
            solver.process_path_queries(base_smt, ir, &entry, &BTreeSet::from([id]))
        }))
        .map_err(|p| {
            format!(
                "cannot build a synthesis query against entry `{entry}`: {}",
                panic_msg(p)
            )
        })
    };

    // Gate 4: marker reachability — the draft must be able to meet its own
    // declared test intent.
    for (id, name) in &ir.marker_names {
        let query = match build_query(*id) {
            Ok(q) => q,
            Err(msg) => {
                failures.push(msg);
                return;
            }
        };
        match solve(&format!("reach:{name}"), &query) {
            Response::Unsat => failures.push(format!(
                "marker `{name}` is unreachable dead code in YOUR draft \
                 (solver-proved): no input to `{entry}` reaches it — fix the \
                 logic guarding it or remove the marker"
            )),
            Response::Sat(model) => {
                let witness = if seq_input {
                    crate::guidance::decode_seq_model(&model, crate::guidance::INPUT_VAR)
                } else {
                    None
                };
                match witness {
                    Some(w) => notes.push(format!(
                        "marker `{name}`: reachable (solver witness input {w:?})"
                    )),
                    None => notes.push(format!("marker `{name}`: reachable (sat)")),
                }
            }
            resp @ (Response::Timeout | Response::Unknown(_)) => notes.push(format!(
                "marker `{name}`: reachability undecided within the budget \
                 ({resp}) — warning, not a gate failure"
            )),
        }
    }

    // Gate 5: spec examples — the draft must reproduce the human's concrete
    // behavioral expectations.
    if examples.is_empty() {
        return;
    }
    if !seq_input {
        failures.push(format!(
            "the spec-example gate needs the entry function to take exactly one \
             text input (`Seq<U32>`); `{entry}` does not"
        ));
        return;
    }
    for (i, ex) in examples.iter().enumerate() {
        let n = i + 1;
        // Does marker `name` fire on this concrete input? With the input
        // pinned by an equality assertion, sat/unsat decides; None = budget.
        let mut fires = |id: usize, name: &str| -> Result<Option<bool>, String> {
            let query = build_query(id)?;
            let query = crate::guidance::pin_input(&query, &ex.input, crate::guidance::INPUT_VAR)
                .ok_or_else(|| "internal: the query has no (check-sat)".to_string())?;
            Ok(match solve(&format!("example{n}:{name}"), &query) {
                Response::Sat(_) => Some(true),
                Response::Unsat => Some(false),
                Response::Timeout | Response::Unknown(_) => None,
            })
        };
        match &ex.expect {
            Expectation::Err(want) => {
                let Some((id, _)) = ir
                    .marker_names
                    .iter()
                    .find(|(_, name)| name.as_str() == want)
                else {
                    let declared: Vec<&str> =
                        ir.marker_names.values().map(String::as_str).collect();
                    failures.push(format!(
                        "spec example {n} (input {:?}) expects marker `{want}`, but \
                         your draft declares no marker with that name (declared: {})",
                        ex.input,
                        declared.join(", ")
                    ));
                    continue;
                };
                match fires(*id, want) {
                    Err(msg) => failures.push(msg),
                    Ok(Some(true)) => notes.push(format!(
                        "spec example {n} (input {:?}): confirmed — marker `{want}` fires",
                        ex.input
                    )),
                    Ok(Some(false)) => {
                        // The expected marker does not fire: scan the draft's
                        // other markers so the counterexample reports what the
                        // draft DID do, not just what it failed to do.
                        let mut other: Option<&str> = None;
                        let mut undecided_other = false;
                        for (oid, oname) in &ir.marker_names {
                            if oname == want {
                                continue;
                            }
                            match fires(*oid, oname) {
                                Ok(Some(true)) => {
                                    other = Some(oname.as_str());
                                    break;
                                }
                                Ok(None) => undecided_other = true,
                                Ok(Some(false)) | Err(_) => (),
                            }
                        }
                        let did = match other {
                            Some(m) => format!("it fires marker `{m}` instead"),
                            None if undecided_other => "the solver could not \
                                decide what it does instead within the budget"
                                .to_string(),
                            None => "it fires no named marker at all (the draft \
                                accepts this input, or rejects it via an unnamed \
                                path — make that rejection a named marker)"
                                .to_string(),
                        };
                        failures.push(format!(
                            "on input {:?} the spec expects marker `{want}` to fire, \
                             but in your draft it does not: {did}",
                            ex.input
                        ));
                    }
                    Ok(None) => notes.push(format!(
                        "spec example {n} (input {:?}): undecided within the \
                         budget — warning, not a gate failure",
                        ex.input
                    )),
                }
            }
            // `ok` and `nomatch` both scan the whole named-marker set: `ok`
            // requires that NO named marker fires, `nomatch` that SOME does.
            expect @ (Expectation::Ok | Expectation::NoMatch) => {
                let mut fired: Option<&str> = None;
                let mut undecided = false;
                for (id, name) in &ir.marker_names {
                    match fires(*id, name) {
                        Err(msg) => {
                            failures.push(msg);
                            return;
                        }
                        Ok(Some(true)) => {
                            fired = Some(name);
                            break;
                        }
                        Ok(Some(false)) => (),
                        Ok(None) => undecided = true,
                    }
                }
                match (expect, fired) {
                    (Expectation::Ok, Some(m)) => failures.push(format!(
                        "on input {:?} the spec says ok, but your draft fires \
                         marker `{m}`",
                        ex.input
                    )),
                    (Expectation::Ok, None) if !undecided => notes.push(format!(
                        "spec example {n} (input {:?}): confirmed — no named \
                         marker fires (ok)",
                        ex.input
                    )),
                    (Expectation::NoMatch, Some(m)) => notes.push(format!(
                        "spec example {n} (input {:?}): confirmed — rejected via \
                         marker `{m}`",
                        ex.input
                    )),
                    (Expectation::NoMatch, None) if !undecided => failures.push(format!(
                        "on input {:?} the spec says the input must be rejected, \
                         but no named marker of your draft fires — the draft \
                         either accepts it or rejects it via an unnamed path; \
                         make the rejecting condition a `Path::named(...)` marker",
                        ex.input
                    )),
                    _ => notes.push(format!(
                        "spec example {n} (input {:?}): undecided within the \
                         budget — warning, not a gate failure",
                        ex.input
                    )),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The drafting loop.
// ---------------------------------------------------------------------------

/// Build the drafting prompt for one round.
fn build_prompt(spec: &str, examples: &[Example], markers: &[String], rounds: &[Round]) -> String {
    // Optional: supplied only when the caller fixes the taxonomy in advance.
    let markers_block = if markers.is_empty() {
        String::new()
    } else {
        format!(
            "== The markers you must declare ==\n\
             Use `Path::named` with EXACTLY these names, one per error condition. \
             The name states the rule; detect precisely that and nothing else. Do \
             not invent, rename or omit any.\n{}\n\n",
            markers
                .iter()
                .map(|m| format!("  {m}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let mut p = format!(
        "You are drafting an executable reference semantics in the RuSMT DSL.\n\n\
         == The DSL ==\n{DSL_GUIDE}\n\n\
         == The language to specify ==\n{spec}\n\n\
         {markers_block}\
         == Requirements ==\n\
         - One self-contained Rust source file, nothing else.\n\
         - Mark every error/edge condition with `Path::named(\"...\")`.\n\
         - One top-level entry function whose parameters are exactly the \
           inputs to synthesize.\n"
    );
    if !examples.is_empty() {
        p.push_str(
            "\n== Spec examples (solver-checked against your draft) ==\n\
             Your single top-level entry function MUST take exactly one \
             `Seq<U32>` parameter (the input text as a code-point sequence): \
             each example input below is fed to it and checked symbolically.\n\
             `ok` = no named marker fires; `err:<m>` = marker <m> fires; \
             `nomatch` = some named marker fires.\n",
        );
        for ex in examples {
            p.push_str(&format!("input {:?} -> {}\n", ex.input, ex.expect));
        }
    }
    if !rounds.is_empty() {
        p.push_str("\nEarlier drafts were rejected by the gates:\n");
        for (i, r) in rounds.iter().enumerate() {
            p.push_str(&format!(
                "--- draft {} ---\n{}\n--- gate failures ---\n",
                i + 1,
                r.candidate
            ));
            for f in &r.failures {
                p.push_str(&format!("* {f}\n"));
            }
            // Feed back what the solver *established* about the draft, not just
            // what failed: reachability witnesses and confirmed examples tell
            // the proposer which parts already work and which concrete inputs
            // reach each marker — material a chat session cannot produce.
            if !r.notes.is_empty() {
                p.push_str("--- what the solver established about this draft ---\n");
                for n in &r.notes {
                    p.push_str(&format!("* {n}\n"));
                }
            }
        }
    }
    p.push_str(
        "\nOutput exactly the Rust source text and nothing else: no prose, no \
         markdown fences, no explanation.\n",
    );
    p
}

/// Ask the proposer for one draft, retrying once on a transient error. A
/// dropped connection or rate-limit from the LLM command should not throw away
/// a whole session's accumulated gate feedback.
fn propose_with_retry(proposer: &mut dyn Proposer, prompt: &str) -> Result<String, String> {
    match proposer.propose(prompt) {
        Ok(c) => Ok(c),
        Err(first) => match proposer.propose(prompt) {
            Ok(c) => Ok(c),
            Err(second) => Err(format!(
                "proposer failed twice: {first:#}; on retry: {second:#}"
            )),
        },
    }
}

/// The shared loop behind [`author_semantics`] (mechanical gates only) and
/// [`author_semantics_validated`] (mechanical + behavioral gates).
fn author_core(
    spec: &str,
    examples: &[Example],
    markers: &[String],
    proposer: &mut dyn Proposer,
    mut solve: Option<&mut dyn FnMut(&str, &str) -> Response>,
    max_rounds: usize,
) -> AuthoringOutcome {
    let mut rounds: Vec<Round> = Vec::new();
    for _ in 0..max_rounds {
        let prompt = build_prompt(spec, examples, markers, &rounds);
        let candidate = match propose_with_retry(proposer, &prompt) {
            Ok(c) => c,
            Err(e) => {
                rounds.push(Round {
                    candidate: String::new(),
                    failures: vec![format!("PROPOSER ERROR: {e}")],
                    notes: Vec::new(),
                });
                break;
            }
        };
        let (mut failures, ir, base_smt) = run_gates(&candidate);
        let mut notes = Vec::new();
        if failures.is_empty()
            && let (Some(ir), Some(base_smt)) = (ir.as_ref(), base_smt.as_deref())
            && let Some(solve) = solve.as_deref_mut()
        {
            behavioral_gates(ir, base_smt, examples, solve, &mut failures, &mut notes);
        }
        let admitted = failures.is_empty();
        rounds.push(Round {
            candidate: candidate.clone(),
            failures,
            notes,
        });
        if admitted {
            let markers = ir
                .map(|ir| ir.marker_names.values().cloned().collect())
                .unwrap_or_default();
            return AuthoringOutcome {
                accepted: Some(candidate),
                markers,
                rounds,
            };
        }
    }
    AuthoringOutcome {
        accepted: None,
        markers: Vec::new(),
        rounds,
    }
}

/// Drafts `spec` under the mechanical gates only (front end, back end, marker).
///
/// Prefer [`author_semantics_validated`], which adds the behavioral gates.
pub fn author_semantics(
    spec: &str,
    markers: &[String],
    proposer: &mut dyn Proposer,
    max_rounds: usize,
) -> AuthoringOutcome {
    author_core(spec, &[], markers, proposer, None, max_rounds)
}

/// Drafts `spec` under all five gates.
///
/// `solve` is the solver seam, `(label, query) -> Response`. `examples` is the
/// author's test intent; `&[]` runs only the reachability gate.
pub fn author_semantics_validated(
    spec: &str,
    examples: &[Example],
    markers: &[String],
    proposer: &mut dyn Proposer,
    solve: &mut dyn FnMut(&str, &str) -> Response,
    max_rounds: usize,
) -> AuthoringOutcome {
    author_core(spec, examples, markers, proposer, Some(solve), max_rounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, bail};

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
                Some(&"__ERROR__") => bail!("scripted transient error"),
                Some(c) => Ok(c.to_string()),
                None => bail!("script exhausted"),
            }
        }

        fn describe(&self) -> String {
            "mock proposer".to_string()
        }
    }

    /// A minimal admissible draft: parses, transpiles, declares a named marker.
    const GOOD_DRAFT: &str = r#"
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{I64, Path, smt::SMT};

#[smt_type]
pub enum CheckResult {
    Err(Path),
    Ok(I64),
}

#[smt_fn]
pub fn check_nonzero(v: I64) -> CheckResult {
    if *v.eq(I64::from(0)) {
        CheckResult::Err(Path::named("zero_input"))
    } else {
        CheckResult::Ok(v)
    }
}
"#;

    /// Like [`GOOD_DRAFT`] but with no named marker: must fail gate 3.
    const UNMARKED_DRAFT: &str = r#"
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{I64, smt::SMT};

#[smt_fn]
pub fn identity(v: I64) -> I64 {
    v
}
"#;

    /// A text-input draft (entry takes `Seq<U32>`): classifies by the first
    /// character — empty input and a leading 'z' are the two named markers.
    const SEQ_DRAFT: &str = r#"
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{Integer, Path, Seq, U32, smt::SMT};

#[smt_type]
pub enum CheckResult {
    Err(Path),
    Ok(U32),
}

#[smt_fn]
pub fn classify(s: Seq<U32>) -> CheckResult {
    if *s.is_empty() {
        CheckResult::Err(Path::named("empty_input"))
    } else {
        if *s.at(Integer::from(0)).eq(U32::from(122)) {
            CheckResult::Err(Path::named("starts_with_z"))
        } else {
            CheckResult::Ok(s.at(Integer::from(0)))
        }
    }
}
"#;

    /// Two functions, neither calling the other: no unique entry.
    const TWO_ROOTS_DRAFT: &str = r#"
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{I64, Path, smt::SMT};

#[smt_type]
pub enum CheckResult {
    Err(Path),
    Ok(I64),
}

#[smt_fn]
pub fn first(v: I64) -> CheckResult {
    if *v.eq(I64::from(0)) {
        CheckResult::Err(Path::named("zero_a"))
    } else {
        CheckResult::Ok(v)
    }
}

#[smt_fn]
pub fn second(v: I64) -> CheckResult {
    if *v.eq(I64::from(1)) {
        CheckResult::Err(Path::named("one_b"))
    } else {
        CheckResult::Ok(v)
    }
}
"#;

    // --- the mechanical gates (unchanged behavior) ---

    #[test]
    fn a_draft_that_fails_the_front_end_gets_feedback_and_a_repair_is_admitted() {
        let mut mock = MockProposer::new(vec!["fn broken(", GOOD_DRAFT]);
        let outcome = author_semantics("a toy nonzero-check language", &[], &mut mock, 4);
        assert!(outcome.accepted.is_some());
        assert_eq!(outcome.rounds.len(), 2);
        assert!(!outcome.rounds[0].failures.is_empty());
        assert!(outcome.rounds[1].failures.is_empty());
        assert_eq!(outcome.markers, vec!["zero_input".to_string()]);
        // Round 2's prompt must carry round 1's gate failure (gated repair).
        assert!(mock.prompts[1].contains("gate failures"));
    }

    #[test]
    fn a_draft_without_named_markers_is_rejected_by_the_marker_gate() {
        let mut mock = MockProposer::new(vec![UNMARKED_DRAFT]);
        let outcome = author_semantics("a toy identity language", &[], &mut mock, 1);
        assert!(outcome.accepted.is_none());
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("Path::named"))
        );
    }

    #[test]
    fn the_round_budget_is_respected() {
        let mut mock = MockProposer::new(vec!["x", "y", "z", "w"]);
        let outcome = author_semantics("anything", &[], &mut mock, 2);
        assert!(outcome.accepted.is_none());
        assert_eq!(outcome.rounds.len(), 2);
    }

    #[test]
    fn a_transient_proposer_error_is_retried_within_the_round() {
        // Round 1's first proposer call errors; the immediate retry returns a
        // good draft, so a transient failure does not discard the session.
        let mut mock = MockProposer::new(vec!["__ERROR__", GOOD_DRAFT]);
        let outcome = author_semantics("a toy nonzero-check language", &[], &mut mock, 1);
        assert!(outcome.accepted.is_some());
        assert_eq!(outcome.rounds.len(), 1);
        assert!(outcome.rounds[0].failures.is_empty());
        assert_eq!(
            mock.prompts.len(),
            2,
            "the failing call and its retry both ran"
        );
    }

    #[test]
    fn two_proposer_errors_in_one_round_end_the_session() {
        let mut mock = MockProposer::new(vec!["__ERROR__", "__ERROR__", GOOD_DRAFT]);
        let outcome = author_semantics("anything", &[], &mut mock, 3);
        assert!(outcome.accepted.is_none());
        assert_eq!(outcome.rounds.len(), 1);
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("proposer failed twice"))
        );
    }

    // --- the examples file ---

    #[test]
    fn examples_parse_with_escapes_and_reject_malformed_lines() {
        let text = "# comment\nk = true\tok\nk = tru\\te\terr:boolean_invalid\n\\n\tnomatch\n";
        let ex = parse_examples(text).expect("parses");
        assert_eq!(ex.len(), 3);
        assert_eq!(ex[0].input, "k = true");
        assert_eq!(ex[0].expect, Expectation::Ok);
        assert_eq!(ex[1].input, "k = tru\te");
        assert_eq!(
            ex[1].expect,
            Expectation::Err("boolean_invalid".to_string())
        );
        assert_eq!(ex[2].input, "\n");
        assert_eq!(ex[2].expect, Expectation::NoMatch);

        assert!(
            parse_examples("no tab here")
                .unwrap_err()
                .contains("line 1")
        );
        assert!(
            parse_examples("x\tmaybe")
                .unwrap_err()
                .contains("unknown expectation")
        );
        assert!(
            parse_examples("x\terr:")
                .unwrap_err()
                .contains("marker name")
        );
        assert!(
            parse_examples("bad\\q\tok")
                .unwrap_err()
                .contains("unknown escape")
        );
    }

    // --- gate 4: marker reachability (scripted solver seam) ---

    #[test]
    fn a_dead_marker_is_a_solver_proved_failure_and_feeds_the_repair() {
        let mut mock = MockProposer::new(vec![GOOD_DRAFT, GOOD_DRAFT]);
        let mut calls = 0usize;
        let mut solve = |label: &str, query: &str| {
            assert_eq!(label, "reach:zero_input");
            assert!(query.contains("(check-sat)"));
            calls += 1;
            if calls == 1 {
                Response::Unsat
            } else {
                Response::Sat("sat".to_string())
            }
        };
        let outcome = author_semantics_validated(
            "a toy nonzero-check language",
            &[],
            &[],
            &mut mock,
            &mut solve,
            4,
        );
        assert!(outcome.accepted.is_some());
        assert_eq!(outcome.rounds.len(), 2);
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("dead code") && f.contains("zero_input"))
        );
        assert!(outcome.rounds[1].failures.is_empty());
        assert!(
            outcome.rounds[1]
                .notes
                .iter()
                .any(|n| n.contains("zero_input") && n.contains("reachable"))
        );
        // Round 2's prompt carries the solver-proved counterexample.
        assert!(mock.prompts[1].contains("dead code"));
    }

    #[test]
    fn a_draft_with_two_entry_candidates_fails_the_entry_check() {
        let mut mock = MockProposer::new(vec![TWO_ROOTS_DRAFT]);
        let mut calls = 0usize;
        let mut solve = |_l: &str, _q: &str| {
            calls += 1;
            Response::Unsat
        };
        let outcome = author_semantics_validated("anything", &[], &[], &mut mock, &mut solve, 1);
        assert!(outcome.accepted.is_none());
        assert_eq!(calls, 0, "no solver call without a unique entry");
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("exactly ONE") && f.contains("first") && f.contains("second"))
        );
    }

    #[test]
    fn a_reachability_timeout_is_a_warning_not_a_failure() {
        let mut mock = MockProposer::new(vec![GOOD_DRAFT]);
        let mut solve = |_l: &str, _q: &str| Response::Timeout;
        let outcome = author_semantics_validated(
            "a toy nonzero-check language",
            &[],
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_some());
        assert!(
            outcome.rounds[0]
                .notes
                .iter()
                .any(|n| n.contains("undecided") && n.contains("warning"))
        );
    }

    // --- gate 5: spec examples (scripted solver seam) ---

    #[test]
    fn an_example_mismatch_is_solver_checked_and_feeds_the_repair() {
        let examples = vec![Example {
            input: "za".to_string(),
            expect: Expectation::Err("starts_with_z".to_string()),
        }];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT, SEQ_DRAFT]);
        let mut example_calls = 0usize;
        let mut solve = |label: &str, query: &str| {
            if label == "example1:starts_with_z" {
                // The example query pins the input concretely.
                assert!(query.contains(
                    "(assert (= input_0 (seq.++ (seq.unit (_ bv122 32)) (seq.unit (_ bv97 32)))))"
                ));
                example_calls += 1;
                if example_calls == 1 {
                    Response::Unsat
                } else {
                    Response::Sat("sat".to_string())
                }
            } else if label == "example1:empty_input" {
                // Round 1: after the expected marker misses, the gate scans the
                // other markers to report what fired instead (here: nothing).
                Response::Unsat
            } else {
                assert!(label.starts_with("reach:"));
                Response::Sat("sat".to_string())
            }
        };
        let outcome = author_semantics_validated(
            "a toy classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            4,
        );
        assert!(outcome.accepted.is_some());
        assert_eq!(outcome.rounds.len(), 2);
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("starts_with_z") && f.contains("\"za\""))
        );
        // The behavioral counterexample reaches the proposer; the examples
        // themselves are in the prompt from round 1.
        assert!(mock.prompts[0].contains("Spec examples"));
        assert!(mock.prompts[1].contains("does not"));
    }

    #[test]
    fn an_err_mismatch_reports_which_marker_fires_instead() {
        // Spec says "z" should fire `empty_input`; the draft fires
        // `starts_with_z` instead. The feedback must name what DID fire, not
        // just that the expected marker missed.
        let examples = vec![Example {
            input: "z".to_string(),
            expect: Expectation::Err("empty_input".to_string()),
        }];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT]);
        let mut solve = |label: &str, _q: &str| {
            if label == "example1:empty_input" {
                Response::Unsat // expected marker does NOT fire on "z"
            } else if label == "example1:starts_with_z" {
                Response::Sat("sat".to_string()) // this one fires instead
            } else {
                assert!(label.starts_with("reach:"));
                Response::Sat("sat".to_string())
            }
        };
        let outcome = author_semantics_validated(
            "a toy classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_none());
        assert!(outcome.rounds[0].failures.iter().any(|f| {
            f.contains("empty_input") && f.contains("starts_with_z") && f.contains("instead")
        }));
    }

    #[test]
    fn the_transcript_records_drafts_failures_and_observations() {
        let mut mock = MockProposer::new(vec!["fn broken(", GOOD_DRAFT]);
        let outcome = author_semantics("a toy nonzero-check language", &[], &mut mock, 4);
        let t = render_transcript("toy.md", &[], &outcome);
        assert!(t.contains("round 1") && t.contains("gate failures"));
        assert!(t.contains("round 2") && t.contains("ADMITTED"));
        assert!(t.contains("status: ADMITTED") && t.contains("zero_input"));
    }

    #[test]
    fn ok_and_nomatch_examples_check_the_named_marker_set() {
        let examples = vec![
            Example {
                input: "a".to_string(),
                expect: Expectation::Ok,
            },
            Example {
                input: String::new(),
                expect: Expectation::NoMatch,
            },
        ];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT]);
        let mut labels: Vec<String> = Vec::new();
        let mut solve = |label: &str, _q: &str| {
            labels.push(label.to_string());
            if label.starts_with("reach:") {
                Response::Sat("sat".to_string())
            } else if label.starts_with("example1:") {
                Response::Unsat // "a" fires nothing -> ok confirmed
            } else {
                Response::Sat("sat".to_string()) // "" fires a marker -> nomatch confirmed
            }
        };
        let outcome = author_semantics_validated(
            "a toy classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_some());
        // `ok` must have been checked against EVERY named marker.
        assert_eq!(
            labels.iter().filter(|l| l.starts_with("example1:")).count(),
            2
        );
        let notes = &outcome.rounds[0].notes;
        assert!(
            notes
                .iter()
                .any(|n| n.contains("example 1") && n.contains("confirmed"))
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("example 2") && n.contains("rejected via"))
        );
    }

    #[test]
    fn an_ok_example_that_fires_a_marker_and_a_missed_nomatch_are_failures() {
        let examples = vec![
            Example {
                input: "z".to_string(),
                expect: Expectation::Ok,
            },
            Example {
                input: "a".to_string(),
                expect: Expectation::NoMatch,
            },
        ];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT]);
        let mut solve = |label: &str, _q: &str| {
            if label.starts_with("reach:") || label == "example1:starts_with_z" {
                Response::Sat("sat".to_string())
            } else {
                Response::Unsat
            }
        };
        let outcome = author_semantics_validated(
            "a toy classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_none());
        let f = &outcome.rounds[0].failures;
        assert!(
            f.iter()
                .any(|m| m.contains("spec says ok") && m.contains("starts_with_z"))
        );
        assert!(f.iter().any(|m| m.contains("must be rejected")));
    }

    #[test]
    fn examples_require_a_text_entry_and_a_declared_marker() {
        // GOOD_DRAFT's entry takes I64, not Seq<U32>.
        let examples = vec![Example {
            input: "0".to_string(),
            expect: Expectation::Ok,
        }];
        let mut mock = MockProposer::new(vec![GOOD_DRAFT]);
        let mut solve = |label: &str, _q: &str| {
            assert!(label.starts_with("reach:"));
            Response::Sat("sat".to_string())
        };
        let outcome = author_semantics_validated(
            "a toy nonzero-check language",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_none());
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| f.contains("Seq<U32>"))
        );

        // An expectation naming a marker the draft does not declare.
        let examples = vec![Example {
            input: "x".to_string(),
            expect: Expectation::Err("no_such_marker".to_string()),
        }];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT]);
        let mut solve = |_l: &str, _q: &str| Response::Sat("sat".to_string());
        let outcome = author_semantics_validated(
            "a toy classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(outcome.accepted.is_none());
        assert!(
            outcome.rounds[0]
                .failures
                .iter()
                .any(|f| { f.contains("no_such_marker") && f.contains("declares no marker") })
        );
    }

    // --- end-to-end with the real solver (ignored: needs the z3 binary) ---

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a one-function toy draft)"]
    fn behavioral_gates_admit_a_correct_draft_with_real_z3() {
        let examples = vec![
            Example {
                input: String::new(),
                expect: Expectation::Err("empty_input".to_string()),
            },
            Example {
                input: "za".to_string(),
                expect: Expectation::Err("starts_with_z".to_string()),
            },
            Example {
                input: "a".to_string(),
                expect: Expectation::Ok,
            },
            Example {
                input: "z".to_string(),
                expect: Expectation::NoMatch,
            },
        ];
        let mut mock = MockProposer::new(vec![SEQ_DRAFT]);
        let dir = tempfile::tempdir().expect("tempdir");
        let mut n = 0usize;
        let mut solve = |_label: &str, query: &str| {
            n += 1;
            let p = dir.path().join(format!("gate_{n}.smt2"));
            std::fs::write(&p, query).expect("write query");
            crate::guidance::run_z3_file(&p, Duration::from_secs(20))
        };
        let outcome = author_semantics_validated(
            "a toy first-character classifier",
            &examples,
            &[],
            &mut mock,
            &mut solve,
            1,
        );
        assert!(
            outcome.accepted.is_some(),
            "failures: {:?}",
            outcome
                .rounds
                .iter()
                .map(|r| &r.failures)
                .collect::<Vec<_>>()
        );
        let notes = &outcome.rounds[0].notes;
        assert!(notes.iter().any(|nt| nt.contains("reachable")));
        assert!(notes.iter().filter(|nt| nt.contains("confirmed")).count() >= 4);
    }
}

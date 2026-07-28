//! AI⇄Z3 co-solving: the pipeline stage that reaches markers Z3 alone cannot.
//!
//! Stage 1 puts the unmodified per-marker query to Z3. When that does not
//! return a model, this stage runs — always, not as an option. It never
//! rewrites the question. It changes two things only:
//!
//! * **what Z3 must expand** — a marker-directed slice replaces definitions
//!   that cannot contribute to the target with a constant of their return sort
//!   ([`crate::slice`]);
//! * **where Z3 must look** — added assertions over the entry input
//!   ([`crate::guidance::Constraints`]).
//!
//! Both are search steering, and neither is trusted. A model over a sliced or
//! constrained query is a *candidate*; it becomes a witness only when the
//! UNMODIFIED query, with the input pinned to it, is itself `sat`
//! ([`crate::guidance::pin_input`]). That check is pure strengthening, so its
//! `sat` is a model of the original reachability question. The proposer never
//! produces a witness and never approves one.
//!
//! # Why the loop is symptom-directed
//!
//! Z3 fails on these queries in distinct ways, and the right move differs per
//! way — so the loop reads Z3's own verdict and reason rather than guessing:
//!
//! | Z3 says | what it means | move |
//! |---|---|---|
//! | no verdict / killed | it died expanding definitions over a symbolic input | slice harder |
//! | `unknown`, `incomplete (theory seq)` | the symbolic-length sequence is the blocker | bound the length |
//! | `timeout` (+ statistics) | in a fragment it can decide, but looking too widely | constrain the input |
//! | `unsat` on a SLICED query | the slice removed the path to the marker | restore stubs |
//! | `unsat` on a CONSTRAINED query | the constraints contradict the marker | drop constraints |
//! | `sat`, acceptance fails | the slice changed behaviour: spurious | restore stubs |
//! | `sat`, acceptance `sat` | witness | block it and look for the next |
//!
//! Neither `unsat` above is an unreachability verdict: one is about a different
//! program, the other about a strengthened query.

use crate::guidance::{
    CONSTRAINT_GRAMMAR, Constraint, Constraints, INPUT_VAR, Response, decode_seq_model,
    parse_constraint_line, pin_input, run_z3_file, strengthen_query,
};
use crate::proposer::Proposer;
use crate::slice::StubPlan;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

/// Which division of labour the loop runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The proposer restores sliced definitions and constrains the input; Z3
    /// searches for the model.
    Guide,
    /// The proposer generates a whole candidate input from the emitted SMT-LIB;
    /// Z3 certifies it against the unmodified query. The solver is a decision
    /// procedure rather than a search.
    Certify,
}

/// Read `RUSMT_MODE` (`certify` or `guide`; default `certify`).
pub fn mode_from_env() -> Mode {
    match std::env::var("RUSMT_MODE").ok().as_deref() {
        Some("guide") => Mode::Guide,
        _ => Mode::Certify,
    }
}

/// Default co-solving rounds per marker.
pub const DEFAULT_ROUNDS: usize = 4;

/// Default number of witnesses to collect per marker.
///
/// One witness per marker is thin: the solver may return an input every
/// implementation already handles. Extra witnesses cost one blocking assertion
/// and a re-solve each.
pub const DEFAULT_WITNESSES: usize = 3;

/// Read `RUSMT_ROUNDS` (default [`DEFAULT_ROUNDS`]).
pub fn rounds_from_env() -> usize {
    std::env::var("RUSMT_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_ROUNDS)
        .max(1)
}

/// Read `RUSMT_WITNESSES` (default [`DEFAULT_WITNESSES`]).
pub fn witnesses_from_env() -> usize {
    std::env::var("RUSMT_WITNESSES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_WITNESSES)
        .max(1)
}

/// What Z3's answer says about *why* it did not hand over a model, and hence
/// which lever the next round should pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnosis {
    /// A model came back and survived acceptance.
    Witness,
    /// A model came back but the unmodified query rejected it: the slice
    /// changed behaviour, so the candidate was never a real witness.
    Spurious,
    /// Z3 produced no verdict at all (killed, or crashed): it died expanding
    /// definitions. Slice harder.
    Died(String),
    /// Z3 gave up on a theory it cannot decide here — in practice the
    /// symbolic-length sequence. Bound the length.
    Incomplete(String),
    /// Budget expired inside a fragment Z3 can decide. Narrow the input.
    Stalled(String),
    /// `unsat` while definitions were stubbed: the slice removed the path.
    OverSliced,
    /// `unsat` with no stubs but added constraints: they contradict the marker.
    OverConstrained,
    /// `unsat` on the unmodified, unconstrained query: genuinely unreachable
    /// under this encoding and bound. The only `unsat` that means anything.
    Unreachable,
}

impl Diagnosis {
    /// Read Z3's answer in the context that produced it.
    pub fn of(resp: &Response, sliced: bool, constrained: bool) -> Diagnosis {
        match resp {
            Response::Sat(_) => Diagnosis::Witness,
            Response::Unsat if sliced => Diagnosis::OverSliced,
            Response::Unsat if constrained => Diagnosis::OverConstrained,
            Response::Unsat => Diagnosis::Unreachable,
            Response::Timeout => Diagnosis::Stalled("budget expired".to_string()),
            Response::Unknown(r) if r.contains("no verdict") => Diagnosis::Died(r.clone()),
            Response::Unknown(r) if r.contains("incomplete") => Diagnosis::Incomplete(r.clone()),
            Response::Unknown(r) => Diagnosis::Stalled(r.clone()),
        }
    }

    /// The instruction handed to the proposer for this symptom.
    fn advice(&self) -> &'static str {
        match self {
            Diagnosis::Witness => "a witness was accepted",
            Diagnosis::Spurious => {
                "Z3 found a model of the SLICED program, but the unmodified query REJECTED that \
                 input. A stubbed definition changed the behaviour that matters. Restore the \
                 stub(s) that decide this marker's condition."
            }
            Diagnosis::Died(_) => {
                "Z3 produced no verdict at all — it died expanding definitions over a symbolic \
                 input. The query is still too large: restore FEWER stubs, and bound the input \
                 length with len_max so the sequence is small."
            }
            Diagnosis::Incomplete(_) => {
                "Z3 gave up because it cannot decide this theory with a symbolic-length sequence. \
                 Bound the length: give len_max (and len_min) so the input has a fixed small size."
            }
            Diagnosis::Stalled(_) => {
                "Z3 is in a fragment it can decide but is searching too widely. Narrow the input: \
                 pin characters that any input reaching this marker must have (prefix / at / \
                 range), and tighten len_max."
            }
            Diagnosis::OverSliced => {
                "The SLICED program is unsat, so the stub set removed the path to the marker. \
                 This says NOTHING about reachability. Restore the stubbed definitions that an \
                 input must pass through to reach this marker — think about what syntax has to \
                 parse first (a key, a separator, an opening delimiter)."
            }
            Diagnosis::OverConstrained => {
                "The STRENGTHENED query is unsat: your constraints contradict reaching the \
                 marker. This says NOTHING about reachability. Relax or replace them."
            }
            Diagnosis::Unreachable => {
                "The unmodified, unconstrained query is unsat: the marker is unreachable under \
                 this encoding and bound."
            }
        }
    }
}

/// What a proposal may ask for.
#[derive(Debug, Clone, Default)]
pub struct Proposal {
    /// A complete candidate input, given as `input <<<…>>>`.
    ///
    /// This is the certification mode: the proposer generates a candidate from
    /// the emitted SMT-LIB and Z3 *decides* it against the unmodified query, so
    /// the solver moves from searching for a model to certifying one (CEGIS with
    /// the model as generator). It is only reachable in
    /// [`Mode::Certify`] — in [`Mode::Guide`] a proposal may not name an input.
    pub input: Option<String>,
    /// Stubbed definitions to emit in full again (by function name).
    pub restore: Vec<String>,
    /// Added assertions over the entry input.
    pub constraints: Constraints,
    /// Notes on lines that did not parse, fed back verbatim.
    pub rejected: Vec<String>,
}

impl Proposal {
    /// Whether this proposal asks for anything at all.
    pub fn is_empty(&self) -> bool {
        self.restore.is_empty() && self.constraints.0.is_empty() && self.input.is_none()
    }
}

/// Parse a proposal. Unknown lines become feedback rather than errors, because
/// the round after a malformed reply is cheaper than a failed run.
pub fn parse_proposal(text: &str) -> Proposal {
    let mut p = Proposal::default();
    // `input <<<…>>>` carries a whole candidate, newlines included, so it is
    // extracted before the line-oriented directives.
    if let Some(start) = text.find("<<<") {
        if let Some(end) = text[start + 3..].find(">>>") {
            let mut body = &text[start + 3..start + 3 + end];
            body = body.strip_prefix('\n').unwrap_or(body);
            body = body.strip_suffix('\n').unwrap_or(body);
            p.input = Some(body.to_string());
        }
    }
    for line in text.lines() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix("restore ") {
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                p.rejected
                    .push(format!("ignored `{t}`: restore needs one function name"));
            } else {
                p.restore.push(name.to_string());
            }
            continue;
        }
        match parse_constraint_line(t) {
            Ok(Some(c)) => p.constraints.0.push(c),
            Ok(None) => (),
            Err(e) => p.rejected.push(format!("ignored `{t}`: {e}")),
        }
    }
    p
}

/// The prompt for one round.
///
/// The proposer sees the marker name, the shape of the query, Z3's diagnosis,
/// what is currently stubbed, and the history. It does not see the Rust
/// semantics — the point of the exercise is that it reasons about the emitted
/// SMT-LIB and the solver's behaviour, not about source it could read the
/// answer out of.
fn build_prompt(
    marker: &str,
    language: &str,
    holder: &str,
    stub_names: &[String],
    diagnosis: &Diagnosis,
    detail: &str,
    rounds: &[Round],
) -> String {
    let mut p = format!(
        "You are steering an SMT solver (Z3) inside a test-input synthesis pipeline.\n\
         Object language: {language}\n\
         Goal: an input whose execution reaches the error marker `{marker}`, raised inside the \
         function `{holder}`.\n\n\
         YOU DO NOT WRITE THE INPUT — Z3 constructs it. You write a SKETCH: a partial input with \
         positions left FREE, which Z3 fills in. A proposal that pins every position of a \
         length-bounded input is REJECTED, because then nothing is left for the solver to \
         construct. Always leave at least one position free: make `len_max` strictly greater than \
         the length of any `prefix` you fix.\n\n\
         Two levers:\n\
         (1) RESTORE definitions. To make the query tractable, every function that cannot \
         contribute to this marker has had its body replaced by a constant. If that removed \
         something an input must pass THROUGH to reach the marker, Z3 answers `unsat` and you must \
         restore it. Ask what syntax has to parse successfully first — a key, a separator, an \
         opening delimiter — and restore exactly those. Restore as FEW as you can: each one makes \
         the query harder for Z3 to finish.\n\
         (2) SKETCH the input. Fix only the scaffolding characters any input reaching this marker \
         must have, bound the length, and leave the semantically interesting positions free.\n\n\
         A good proposal, for a marker about a capital-T boolean literal:\n\
         \x20 restore parse_simple_key\n\
         \x20 restore parse_unquoted_key\n\
         \x20 prefix \"a=Tr\"\n\
         \x20 len_max 7\n\
         Z3 then constructs the remaining characters itself.\n\n\
         == Z3's current diagnosis ==\n{}\n{}\n\n\
         == Reply grammar (one directive per line, nothing else) ==\n\
         restore <function_name>  emit that function's real body again\n{}\n\
         Escapes in quotes: \\\" \\\\ \\n \\t \\r \\u{{XX}} (use \\u{{XX}} for control characters).\n\
         Lines starting with # are comments.\n\n\
         == Currently stubbed ({} functions, largest first) ==\n{}\n",
        diagnosis.advice(),
        if detail.is_empty() {
            String::new()
        } else {
            format!("Z3 reported: {detail}")
        },
        CONSTRAINT_GRAMMAR,
        stub_names.len(),
        if stub_names.is_empty() {
            "(none)".to_string()
        } else {
            // Never truncate: a name that is not shown cannot be restored, and
            // the load-bearing one is often a small predicate that sorts last.
            stub_names.join(", ")
        }
    );
    if !rounds.is_empty() {
        p.push_str("\n== Earlier rounds ==\n");
        for (i, r) in rounds.iter().enumerate() {
            let _ = write!(
                p,
                "--- round {} directives ---\n{}\n--- outcome ---\n{}\n",
                i + 1,
                r.directives.trim(),
                r.outcome
            );
        }
    }
    p.push_str("\nNext directives:\n");
    p
}

/// The prompt for a certification round: the proposer sees the emitted SMT-LIB
/// and writes a candidate input; Z3 decides it.
///
/// `smt_excerpt` is the emitted query, truncated — the point is that the
/// proposal is derived from the *lifted semantics as Z3 sees them*, not from the
/// Rust source, which the proposer never receives.
fn build_certify_prompt(
    marker: &str,
    language: &str,
    holder: &str,
    smt_excerpt: &str,
    rounds: &[Round],
) -> String {
    let mut p = format!(
        "You are generating a test input inside a program-synthesis pipeline.\n\
         Object language: {language}\n\
         Goal: an input whose execution reaches the error marker `{marker}`, which is raised \
         inside `{holder}`.\n\n\
         Z3 cannot search for this input: the lifted parser is too large to solve with a \
         symbolic input. So the division of labour is inverted — YOU propose a concrete \
         candidate, and Z3 DECIDES it by pinning it into the unmodified query. A candidate is \
         accepted only when Z3 answers `sat`, which means the lifted semantics genuinely reach \
         `{marker}` on that input.\n\n\
         Two things make a candidate fail, and both are reported back to you:\n\
         * `unsat` — the input does not reach `{marker}`. Usually it reaches a DIFFERENT error \
           first (an earlier syntax error shadows the one you want), or it is simply valid.\n\
         * `unknown`/timeout — rare for a pinned input; treat as a hint to simplify.\n\n\
         Guidance that has worked:\n\
         * Keep it MINIMAL — the shortest document that reaches the marker. Extra keys, tables \
           and whitespace create chances for a different error to fire first.\n\
         * Make everything BEFORE the intended error perfectly valid, so nothing shadows it.\n\
         * The marker name describes the rule. Read it literally and violate exactly that.\n\
         * Control characters and EOF conditions matter: a marker ending `_eof` wants the \
           document to STOP mid-construct, with no trailing newline.\n\
         * Only the FIRST error matters — the parser stops there.\n\n\
         == The emitted SMT-LIB for this marker (truncated) ==\n{smt_excerpt}\n\n"
    );
    if !rounds.is_empty() {
        p.push_str("== Candidates already rejected ==\n");
        for (i, r) in rounds.iter().enumerate() {
            let _ = write!(
                p,
                "--- attempt {} ---\n{}\n--- Z3 said ---\n{}\n",
                i + 1,
                r.candidates.first().map(String::as_str).unwrap_or("(none)"),
                r.outcome
            );
        }
        p.push('\n');
    }
    p.push_str(
        "Reply with the candidate input between <<< and >>> and nothing else:\n\
         input <<<\n(your candidate here)\n>>>\n",
    );
    p
}

/// One round, as recorded in the transcript.
pub struct Round {
    /// The proposer's raw reply.
    pub directives: String,
    /// Names it asked to restore that were actually stubbed.
    pub restored: Vec<String>,
    /// Constraints it added.
    pub constraints: Vec<Constraint>,
    /// Candidates Z3 produced.
    pub candidates: Vec<String>,
    /// The line carried into the next prompt.
    pub outcome: String,
    /// Wall-clock for the round's solving.
    pub elapsed: Duration,
}

/// The result of co-solving one marker.
pub struct Outcome {
    /// Accepted witnesses, in the order found. Each is a model of the
    /// unmodified query.
    pub witnesses: Vec<String>,
    /// Per-round transcript.
    pub rounds: Vec<Round>,
    /// Z3's verdict on the unmodified, unconstrained query (Stage 1).
    pub stage1: String,
}

/// How the loop talks to Z3. The pipeline supplies a real implementation; tests
/// supply a script, so the loop's routing is testable without a solver.
pub trait Solver {
    /// Emit the query for `plan` and solve it with `cs` conjoined, returning
    /// Z3's response.
    fn solve(&mut self, plan: &StubPlan, cs: &Constraints, round: usize) -> Response;

    /// Solve the UNMODIFIED query with the input pinned to `candidate`. This is
    /// the acceptance check and the only thing that admits a witness.
    fn accept(&mut self, candidate: &str) -> Response;

    /// Names of the currently stubbed functions, largest body first.
    fn stub_names(&self, plan: &StubPlan) -> Vec<String>;

    /// Move `names` out of `plan`'s stub set; returns how many moved.
    fn restore(&self, plan: &mut StubPlan, names: &[String]) -> usize;
}

/// Co-solve one marker: run the rounds, collect accepted witnesses.
///
/// `plan` starts at the aggressive end of slicing ([`crate::slice::plan`]) and
/// is relaxed as rounds restore stubs. Each round's candidates go through
/// [`Solver::accept`] — the unmodified query — and only survivors are returned.
pub fn co_solve(
    marker: &str,
    language: &str,
    holder: &str,
    stage1: &Response,
    plan: &mut StubPlan,
    ladder: &[StubPlan],
    proposer: Option<&mut dyn Proposer>,
    solver: &mut dyn Solver,
    max_rounds: usize,
    want: usize,
) -> Outcome {
    let mut rounds: Vec<Round> = Vec::new();
    let mut witnesses: Vec<String> = Vec::new();
    let mut constraints = Constraints::default();
    // Round 0 below sets these from its own verdict, which is strictly more
    // informative than Stage 1's: it is the same question over the sliced query.
    let mut diagnosis;
    let mut detail;

    // Round 0 is mechanical: walk the slice ladder with no constraints and no
    // model call. Each rung restores more definitions; a rung costs one solve
    // (0.5-1.6 s on TOML) where a model round costs a real call. An `unsat` on a
    // rung means that rung over-sliced, so the next rung is the answer — there is
    // nothing for a proposer to contribute until the ladder is exhausted.
    //
    // It also keeps the ablation honest: whatever the ladder reaches is what
    // slicing achieves WITHOUT AI, so the loop's contribution is exactly the
    // residue.
    {
        let started = Instant::now();
        let mut verdicts: Vec<String> = Vec::new();
        let mut candidates = Vec::new();
        let mut last = Response::Unknown("no rung ran".to_string());
        let mut hit = false;
        for (i, rung) in ladder.iter().enumerate() {
            let resp = solver.solve(rung, &constraints, i);
            let tag = match &resp {
                Response::Sat(_) => "sat",
                Response::Unsat => "unsat",
                Response::Timeout => "timeout",
                Response::Unknown(_) => "unknown",
            };
            verdicts.push(format!(
                "rung {} ({} stubbed): {tag}",
                i + 1,
                rung.stub.len()
            ));
            last = resp;
            if let Response::Sat(model) = &last {
                if let Some(text) = decode_seq_model(model, INPUT_VAR) {
                    candidates.push(text.clone());
                    if matches!(solver.accept(&text), Response::Sat(_)) {
                        witnesses.push(text);
                        hit = true;
                    }
                }
            }
            if hit {
                // Adopt the rung that worked, so later rounds build on it.
                *plan = rung.clone();
                break;
            }
        }
        if !hit {
            // Hand later rounds the most permissive rung: the aggressive one is
            // already known to over-slice.
            if let Some(l) = ladder.last() {
                *plan = l.clone();
            }
        }
        let outcome = if hit {
            format!(
                "ACCEPTED (no model call): the slice ladder sufficed — {}; Z3 found {:?} and the \
                 unmodified query with it pinned is sat",
                verdicts.join("; "),
                witnesses.last().expect("just pushed")
            )
        } else if !candidates.is_empty() {
            // A rung produced a model the unmodified query rejected. Say so: it
            // means a stub changed the behaviour that decides this marker, which
            // is a different problem from the ladder simply running out.
            format!(
                "SPURIOUS (no model call): the ladder produced {:?} but the unmodified query \
                 rejected it — {}",
                candidates.last().expect("non-empty"),
                verdicts.join("; ")
            )
        } else {
            format!(
                "the slice ladder produced no witness — {}",
                verdicts.join("; ")
            )
        };
        diagnosis = if hit {
            Diagnosis::Witness
        } else if !candidates.is_empty() {
            Diagnosis::Spurious
        } else {
            Diagnosis::of(&last, !plan.stub.is_empty(), false)
        };
        detail = match &last {
            Response::Unknown(r) => r.clone(),
            Response::Timeout => "budget expired".to_string(),
            _ => String::new(),
        };
        rounds.push(Round {
            directives: "(mechanical: slice ladder, no constraints)".to_string(),
            restored: Vec::new(),
            constraints: Vec::new(),
            candidates,
            outcome,
            elapsed: started.elapsed(),
        });
    }

    // Round 0 is mechanical and has already run. The model-driven rounds need a
    // model; without one the run reports what slicing alone reached and says so.
    let Some(proposer) = proposer else {
        return Outcome {
            witnesses,
            rounds,
            stage1: stage1.to_string(),
        };
    };

    for round in 0..max_rounds {
        if witnesses.len() >= want {
            break;
        }
        let prompt = build_prompt(
            marker,
            language,
            holder,
            &solver.stub_names(plan),
            &diagnosis,
            &detail,
            &rounds,
        );
        let reply = match proposer.propose(&prompt) {
            Ok(r) => r,
            Err(e) => {
                rounds.push(Round {
                    directives: String::new(),
                    restored: Vec::new(),
                    constraints: Vec::new(),
                    candidates: Vec::new(),
                    outcome: format!("PROPOSER ERROR: {e:#}"),
                    elapsed: Duration::ZERO,
                });
                break;
            }
        };
        let proposal = parse_proposal(&reply);
        let mut notes = proposal.rejected.clone();

        if proposal.is_empty() {
            let mut outcome =
                "no usable directives — reply with one `restore <fn>` or constraint per line"
                    .to_string();
            for n in &notes {
                let _ = write!(outcome, "; {n}");
            }
            rounds.push(Round {
                directives: reply,
                restored: Vec::new(),
                constraints: Vec::new(),
                candidates: Vec::new(),
                outcome,
                elapsed: Duration::ZERO,
            });
            continue;
        }

        // A round's constraints REPLACE the running set rather than add to it.
        // Accumulating them would make "relax or replace them" unactionable: an
        // earlier len_max could never be lifted, and every later proposal would
        // be judged against a bound its author had already abandoned.
        let trial = Constraints(proposal.constraints.0.clone());
        if trial.fully_determines() {
            rounds.push(Round {
                directives: reply,
                restored: Vec::new(),
                constraints: Vec::new(),
                candidates: Vec::new(),
                outcome: "REFUSED: those constraints pin every position of a length-bounded \
                          input, which names the answer instead of narrowing the search. Leave \
                          at least one position for Z3, or raise len_max."
                    .to_string(),
                elapsed: Duration::ZERO,
            });
            continue;
        }

        let restored = {
            let n = solver.restore(plan, &proposal.restore);
            if n < proposal.restore.len() {
                notes.push(format!(
                    "{} of {} names were not stubbed (already emitted, or no such function)",
                    proposal.restore.len() - n,
                    proposal.restore.len()
                ));
            }
            proposal.restore.clone()
        };
        constraints = trial;

        let started = Instant::now();
        let resp = solver.solve(plan, &constraints, round + 1);
        let sliced = !plan.stub.is_empty();
        let mut candidates: Vec<String> = Vec::new();
        let mut outcome;

        match &resp {
            Response::Sat(model) => match decode_seq_model(model, INPUT_VAR) {
                None => {
                    outcome = "Z3 returned a model but the input was not concrete in it — \
                               bound the length so the model is fully determined"
                        .to_string();
                }
                Some(text) => {
                    candidates.push(text.clone());
                    // Acceptance: the unmodified query, input pinned.
                    match solver.accept(&text) {
                        Response::Sat(_) => {
                            witnesses.push(text.clone());
                            outcome = format!(
                                "ACCEPTED: Z3 found {text:?} and the unmodified query with that \
                                 input pinned is sat — a witness for `{marker}`"
                            );
                            diagnosis = Diagnosis::Witness;
                        }
                        other => {
                            diagnosis = Diagnosis::Spurious;
                            detail = other.to_string();
                            outcome = format!(
                                "SPURIOUS: Z3 found {text:?} over the sliced program, but the \
                                 unmodified query answered `{other}` for it. {}",
                                Diagnosis::Spurious.advice()
                            );
                        }
                    }
                }
            },
            other => {
                diagnosis = Diagnosis::of(other, sliced, !constraints.0.is_empty());
                detail = match other {
                    Response::Unknown(r) => r.clone(),
                    Response::Timeout => "budget expired".to_string(),
                    _ => String::new(),
                };
                outcome = format!("Z3 answered `{other}`. {}", diagnosis.advice());
            }
        }
        for n in &notes {
            let _ = write!(outcome, "; {n}");
        }
        rounds.push(Round {
            directives: reply,
            restored,
            constraints: proposal.constraints.0,
            candidates,
            outcome,
            elapsed: started.elapsed(),
        });
    }

    Outcome {
        witnesses,
        rounds,
        stage1: stage1.to_string(),
    }
}

/// Certification mode: the proposer generates candidate inputs from the emitted
/// SMT-LIB, and Z3 decides each one against the UNMODIFIED query.
///
/// The solver's role changes from search to decision, which is what makes this
/// tractable: a pinned input decides in well under a second where a symbolic one
/// does not return at all. What does NOT change is the soundness argument — the
/// candidate enters as `(assert (= input_0 …))` on the unmodified query, so it is
/// a strengthening, and a `sat` is a genuine model of the original reachability
/// question. The proposer never sees the Rust semantics and never decides
/// acceptance.
///
/// A rejected candidate is fed back with Z3's verdict, so the next attempt knows
/// which shadowing error to avoid (CEGIS, with the model as generator).
pub fn certify(
    marker: &str,
    language: &str,
    holder: &str,
    smt_excerpt: &str,
    stage1: &Response,
    proposer: &mut dyn Proposer,
    solver: &mut dyn Solver,
    max_rounds: usize,
    want: usize,
) -> Outcome {
    let mut rounds: Vec<Round> = Vec::new();
    let mut witnesses: Vec<String> = Vec::new();

    for _ in 0..max_rounds {
        if witnesses.len() >= want {
            break;
        }
        let prompt = build_certify_prompt(marker, language, holder, smt_excerpt, &rounds);
        let reply = match proposer.propose(&prompt) {
            Ok(r) => r,
            Err(e) => {
                rounds.push(Round {
                    directives: String::new(),
                    restored: Vec::new(),
                    constraints: Vec::new(),
                    candidates: Vec::new(),
                    outcome: format!("PROPOSER ERROR: {e:#}"),
                    elapsed: Duration::ZERO,
                });
                break;
            }
        };
        let proposal = parse_proposal(&reply);
        let Some(candidate) = proposal.input.clone() else {
            rounds.push(Round {
                directives: reply,
                restored: Vec::new(),
                constraints: Vec::new(),
                candidates: Vec::new(),
                outcome: "no candidate found — put the input between <<< and >>>".to_string(),
                elapsed: Duration::ZERO,
            });
            continue;
        };
        // Already tried and rejected: say so rather than spending a solve.
        if rounds
            .iter()
            .any(|r| r.candidates.first() == Some(&candidate))
        {
            rounds.push(Round {
                directives: reply,
                restored: Vec::new(),
                constraints: Vec::new(),
                candidates: vec![candidate],
                outcome: "that candidate was already rejected — propose a DIFFERENT one"
                    .to_string(),
                elapsed: Duration::ZERO,
            });
            continue;
        }

        let started = Instant::now();
        let verdict = solver.accept(&candidate);
        let outcome = match &verdict {
            Response::Sat(_) => {
                witnesses.push(candidate.clone());
                format!(
                    "ACCEPTED: Z3 certified this input against the unmodified query (sat) — a \
                     witness for `{marker}`"
                )
            }
            Response::Unsat => format!(
                "`unsat`: this input does not reach `{marker}`. It is either valid, or an \
                 earlier error shadows the one you want. Make everything before the intended \
                 error valid, and keep it shorter."
            ),
            other => format!(
                "`{other}`: Z3 could not decide this input. Simplify it — shorter, fewer \
                 constructs."
            ),
        };
        rounds.push(Round {
            directives: reply,
            restored: Vec::new(),
            constraints: Vec::new(),
            candidates: vec![candidate],
            outcome,
            elapsed: started.elapsed(),
        });
    }

    Outcome {
        witnesses,
        rounds,
        stage1: stage1.to_string(),
    }
}

/// Render the per-marker transcript written beside the target's response.
pub fn render_transcript(
    proposer_desc: &str,
    marker: &str,
    holder: &str,
    outcome: &Outcome,
) -> String {
    let mut s = format!(
        "marker          : {marker}\n\
         raised in       : {holder}\n\
         stage 1 (Z3 on the unmodified query): {}\n\
         proposer        : {proposer_desc}\n\n\
         The proposer restores sliced-away definitions and adds constraints over the input. It\n\
         never writes an input. Every witness below is a model of the UNMODIFIED query, obtained\n\
         by pinning Z3's candidate into it and re-solving.\n\n",
        outcome.stage1
    );
    for (i, r) in outcome.rounds.iter().enumerate() {
        let _ = writeln!(s, "=== round {} ({} ms) ===", i + 1, r.elapsed.as_millis());
        if !r.restored.is_empty() {
            let _ = writeln!(s, "restored: {}", r.restored.join(", "));
        }
        if !r.constraints.is_empty() {
            let _ = writeln!(s, "constraints: {} added", r.constraints.len());
        }
        let _ = writeln!(s, "--- directives ---\n{}", r.directives.trim());
        for c in &r.candidates {
            let _ = writeln!(s, "Z3 candidate: {c:?}");
        }
        let _ = writeln!(s, "--- outcome ---\n{}\n", r.outcome);
    }
    if outcome.witnesses.is_empty() {
        s.push_str("status: no witness within the round budget\n");
    } else {
        let _ = writeln!(
            s,
            "status: {} witness(es) accepted:",
            outcome.witnesses.len()
        );
        for w in &outcome.witnesses {
            let _ = writeln!(s, "  {w:?}");
        }
    }
    s
}

/// The pipeline's [`Solver`]: emits per-round queries to files under `dir` so
/// every round stays inspectable and re-runnable with a bare `z3 -smt2`.
pub struct FileSolver<'a, F>
where
    F: FnMut(&StubPlan) -> String,
{
    /// Renders the per-target query for a stub plan.
    pub emit: F,
    /// The unmodified query, used for acceptance.
    pub unmodified: &'a str,
    /// Where round artifacts are written.
    pub dir: &'a Path,
    /// Per-solve Z3 budget.
    pub budget: Duration,
    /// Names for the plan's stubs, supplied by the caller (which holds the IR).
    pub names: &'a dyn Fn(&StubPlan) -> Vec<String>,
    /// Restores stubs by name, supplied by the caller.
    pub restorer: &'a dyn Fn(&mut StubPlan, &[String]) -> usize,
}

impl<F> Solver for FileSolver<'_, F>
where
    F: FnMut(&StubPlan) -> String,
{
    fn solve(&mut self, plan: &StubPlan, cs: &Constraints, round: usize) -> Response {
        let base = (self.emit)(plan);
        let query = match strengthen_query(&base, cs, INPUT_VAR) {
            Some(q) => q,
            None => return Response::Unknown("emitted query has no (check-sat)".to_string()),
        };
        let p = self.dir.join(format!("round_{round}.smt2"));
        if let Err(e) = std::fs::write(&p, &query) {
            return Response::Unknown(format!("cannot write round query: {e}"));
        }
        run_z3_file(&p, self.budget)
    }

    fn accept(&mut self, candidate: &str) -> Response {
        let Some(q) = pin_input(self.unmodified, candidate, INPUT_VAR) else {
            return Response::Unknown("unmodified query has no (check-sat)".to_string());
        };
        let p = self.dir.join("accept.smt2");
        if let Err(e) = std::fs::write(&p, &q) {
            return Response::Unknown(format!("cannot write acceptance query: {e}"));
        }
        run_z3_file(&p, self.budget)
    }

    fn stub_names(&self, plan: &StubPlan) -> Vec<String> {
        (self.names)(plan)
    }

    fn restore(&self, plan: &mut StubPlan, names: &[String]) -> usize {
        (self.restorer)(plan, names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, bail};

    struct MockProposer {
        script: Vec<&'static str>,
        next: usize,
        prompts: Vec<String>,
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
            "mock".to_string()
        }
    }

    /// A scripted solver: `solves` are returned in order, and `accepts` decides
    /// which candidates survive the unmodified query.
    struct MockSolver {
        solves: Vec<Response>,
        next: usize,
        accepts: Vec<&'static str>,
        stubs: Vec<String>,
        solve_calls: usize,
    }

    impl MockSolver {
        fn new(solves: Vec<Response>, accepts: Vec<&'static str>) -> Self {
            Self {
                solves,
                next: 0,
                accepts,
                stubs: vec!["parse_float".to_string(), "parse_key".to_string()],
                solve_calls: 0,
            }
        }
    }

    impl Solver for MockSolver {
        fn solve(&mut self, _p: &StubPlan, _cs: &Constraints, _r: usize) -> Response {
            self.solve_calls += 1;
            let i = self.next;
            self.next += 1;
            self.solves
                .get(i)
                .cloned()
                .unwrap_or(Response::Unknown("script exhausted".to_string()))
        }
        fn accept(&mut self, candidate: &str) -> Response {
            if self.accepts.contains(&candidate) {
                Response::Sat(String::new())
            } else {
                Response::Unsat
            }
        }
        fn stub_names(&self, _p: &StubPlan) -> Vec<String> {
            self.stubs.clone()
        }
        fn restore(&self, plan: &mut StubPlan, names: &[String]) -> usize {
            // The mock has no IR; model a successful restore by shrinking the set.
            let n = names.len().min(plan.stub.len());
            for _ in 0..n {
                let first = *plan.stub.iter().next().expect("non-empty");
                plan.stub.remove(&first);
            }
            n
        }
    }

    fn model_of(text: &str) -> Response {
        let units: Vec<String> = text
            .chars()
            .map(|c| format!("(seq.unit (_ bv{} 32))", c as u32))
            .collect();
        let body = if units.len() == 1 {
            units[0].clone()
        } else {
            format!("(seq.++ {})", units.join(" "))
        };
        Response::Sat(format!(
            "sat\n((define-fun input_0 () (Seq (_ BitVec 32)) {body}))"
        ))
    }

    fn plan_with_stubs(n: usize) -> StubPlan {
        let mut p = StubPlan::default();
        for i in 0..n {
            p.stub.insert(crate::ir::index::UsrFunId { index: i });
        }
        p
    }

    #[test]
    fn a_diagnosis_distinguishes_the_two_meaningless_unsats_from_the_real_one() {
        assert_eq!(
            Diagnosis::of(&Response::Unsat, true, false),
            Diagnosis::OverSliced
        );
        assert_eq!(
            Diagnosis::of(&Response::Unsat, false, true),
            Diagnosis::OverConstrained
        );
        // Only an unsat with nothing stubbed and nothing added means anything.
        assert_eq!(
            Diagnosis::of(&Response::Unsat, false, false),
            Diagnosis::Unreachable
        );
        // And the advice for the two says so explicitly.
        assert!(
            Diagnosis::OverSliced
                .advice()
                .contains("NOTHING about reachability")
        );
        assert!(
            Diagnosis::OverConstrained
                .advice()
                .contains("NOTHING about reachability")
        );
    }

    #[test]
    fn z3_failure_modes_route_to_the_matching_lever() {
        let died = Diagnosis::of(
            &Response::Unknown("z3 produced no verdict (signal: 11)".to_string()),
            true,
            false,
        );
        assert!(matches!(died, Diagnosis::Died(_)));
        assert!(died.advice().contains("bound the input length"));

        let incomplete = Diagnosis::of(
            &Response::Unknown("incomplete (theory seq)".to_string()),
            true,
            false,
        );
        assert!(matches!(incomplete, Diagnosis::Incomplete(_)));
        assert!(incomplete.advice().contains("Bound the length"));

        let stalled = Diagnosis::of(&Response::Timeout, true, false);
        assert!(matches!(stalled, Diagnosis::Stalled(_)));
        assert!(stalled.advice().contains("Narrow the input"));
    }

    #[test]
    fn a_candidate_is_a_witness_only_if_the_unmodified_query_accepts_it() {
        // Z3 finds "#\0" over the sliced program; acceptance approves it.
        let mut prop = MockProposer::new(vec!["restore is_comment_start_symbol\nlen_max 4"]);
        let mut solver = MockSolver::new(vec![model_of("#\u{0}")], vec!["#\u{0}"]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(3);
        let out = co_solve(
            "comment_invalid_char",
            "toml",
            "parse_comment_rest",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            2,
            1,
        );
        assert_eq!(out.witnesses, vec!["#\u{0}".to_string()]);
        assert!(out.rounds[0].outcome.contains("ACCEPTED"));
        assert!(out.rounds[0].outcome.contains("unmodified query"));
    }

    #[test]
    fn a_model_the_unmodified_query_rejects_is_reported_spurious_not_accepted() {
        let mut prop = MockProposer::new(vec!["len_max 4", "restore parse_key\nlen_max 6"]);
        // Round 0 (mechanical) and then two model-driven rounds all hand back
        // "zz"; acceptance never approves it.
        let mut solver =
            MockSolver::new(vec![model_of("zz"), model_of("zz"), model_of("zz")], vec![]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(3);
        let out = co_solve(
            "m",
            "toml",
            "h",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            2,
            1,
        );
        assert!(out.witnesses.is_empty());
        // Round 0 is the mechanical one, so its rejection is what the FIRST
        // model prompt has to carry, so the proposer can restore the stub that
        // changed behaviour.
        assert!(out.rounds[0].outcome.contains("SPURIOUS"));
        assert!(out.rounds[0].directives.contains("mechanical"));
        assert!(prop.prompts[0].contains("REJECTED that input"));
    }

    #[test]
    fn a_proposal_that_names_the_whole_input_is_refused_without_solving() {
        let mut prop = MockProposer::new(vec!["prefix \"ab\"\nlen_max 2", "len_max 8"]);
        // Round 0 finds nothing, so the model rounds run.
        let mut solver = MockSolver::new(vec![Response::Unsat, model_of("abc")], vec!["abc"]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(2);
        let out = co_solve(
            "m",
            "toml",
            "h",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            2,
            1,
        );
        // rounds[0] is mechanical; rounds[1] is the refused proposal.
        assert!(out.rounds[1].outcome.contains("REFUSED"));
        // The refused round never reached the solver: only round 0 and the
        // following accepted round did.
        assert_eq!(solver.solve_calls, 2);
        assert!(prop.prompts[1].contains("names the answer"));
    }

    #[test]
    fn over_sliced_unsat_is_fed_back_as_restore_advice_not_unreachability() {
        let mut prop = MockProposer::new(vec!["restore parse_key\nlen_max 4"]);
        // Round 0 over-slices (unsat); the model round then restores and wins.
        let mut solver = MockSolver::new(vec![Response::Unsat, model_of("a=T")], vec!["a=T"]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(3);
        let out = co_solve(
            "boolean_invalid_capital_true",
            "toml",
            "parse_boolean",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            2,
            1,
        );
        assert_eq!(out.witnesses, vec!["a=T".to_string()]);
        // Round 0's over-sliced unsat must reach the first model prompt as
        // restore advice, never as an unreachability verdict.
        assert!(prop.prompts[0].contains("NOTHING about reachability"));
        assert!(prop.prompts[0].contains("has to parse first"));
    }

    #[test]
    fn the_prompt_never_carries_the_rust_source_only_smt_level_facts() {
        let mut prop = MockProposer::new(vec!["len_max 4"]);
        let mut solver = MockSolver::new(vec![Response::Unsat], vec![]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(2);
        let _ = co_solve(
            "comment_invalid_char",
            "toml",
            "parse_comment_rest",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            1,
            1,
        );
        let p = &prop.prompts[0];
        // It gets the marker, the holder, the stub list, and the grammar.
        assert!(p.contains("comment_invalid_char"));
        assert!(p.contains("parse_comment_rest"));
        assert!(p.contains("parse_float"));
        assert!(p.contains("len_max <n>"));
        // It is told explicitly that it does not write the input.
        assert!(p.contains("YOU DO NOT WRITE THE INPUT"));
    }

    #[test]
    fn extra_witnesses_are_collected_up_to_the_requested_count() {
        let mut prop = MockProposer::new(vec!["len_max 4", "len_max 5", "len_max 6"]);
        // Round 0 finds the first; model rounds find the rest.
        let mut solver = MockSolver::new(
            vec![model_of("#\u{0}"), model_of("#\u{1}"), model_of("#\u{2}")],
            vec!["#\u{0}", "#\u{1}", "#\u{2}"],
        );
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(3);
        let out = co_solve(
            "m",
            "toml",
            "h",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            3,
            2,
        );
        assert_eq!(out.witnesses.len(), 2, "stopped at the requested count");
    }

    #[test]
    fn a_reply_with_no_usable_directive_costs_no_solver_call() {
        let mut prop = MockProposer::new(vec!["Here is my plan:", "len_max 4"]);
        // Round 0 finds nothing; the junk reply then costs no solver call.
        let mut solver = MockSolver::new(vec![Response::Unsat, model_of("ab")], vec!["ab"]);
        let ladder = vec![plan_with_stubs(1)];
        let mut plan = plan_with_stubs(2);
        let out = co_solve(
            "m",
            "toml",
            "h",
            &Response::Timeout,
            &mut plan,
            &ladder,
            Some(&mut prop as &mut dyn Proposer),
            &mut solver,
            2,
            1,
        );
        // Round 0 plus the round after the junk reply — the junk one itself
        // never reached the solver.
        assert_eq!(solver.solve_calls, 2);
        assert!(out.rounds[1].outcome.contains("no usable directives"));
    }

    #[test]
    fn proposals_parse_restores_and_constraints_and_report_junk() {
        let p = parse_proposal(
            "restore parse_key\n# a comment\nlen_max 8\nrestore \nprefix \"a\"\nnonsense 3",
        );
        assert_eq!(p.restore, vec!["parse_key".to_string()]);
        assert_eq!(p.constraints.0.len(), 2);
        assert_eq!(p.rejected.len(), 2);
        assert!(!p.is_empty());
        assert!(parse_proposal("# nothing here").is_empty());
    }

    #[test]
    fn candidate_delimiter_newlines_are_not_part_of_the_input() {
        let p = parse_proposal("input <<<\na=[\n>>>");
        assert_eq!(p.input.as_deref(), Some("a=["));
    }
}

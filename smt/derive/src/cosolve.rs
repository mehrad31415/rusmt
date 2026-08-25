//! Stage 2: the proposer⇄Z3 loop, for markers Z3 alone cannot reach.
//!
//! The proposer reads the emitted SMT-LIB and returns one concrete candidate.
//! The candidate is pinned into the UNMODIFIED query as a single equality and Z3
//! decides it; a rejection is fed back with the marker that actually fired.
//!
//! This is CEGIS (Solar-Lezama et al., ASPLOS 2006) with a model in the
//! generator position, as in Jha et al. (MILCOM 2023). Nothing here is novel.
//! See `book/src/dev/design.md` for why each part is the way it is.

use crate::guidance::{INPUT_VAR, Response, pin_input, run_z3_file};
use crate::proposer::Proposer;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

/// Default Stage-2 rounds per marker.
///
/// A spending limit, not a property of the method. Reaching it says nothing about
/// whether a witness exists.
pub const DEFAULT_ROUNDS: usize = 9;

/// Why the loop stopped.
///
/// Only [`Stop::Witness`] is a conclusion; only Stage-1 `unsat` means a marker is
/// unreachable. Every marker gets the same number of attempts — the proposer is
/// stochastic, so nothing seen in a run predicts that more sampling is futile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The requested number of witnesses was accepted.
    Witness,
    /// The round budget ran out. Says nothing about whether a witness exists.
    Budget,
    /// The proposer failed.
    ProposerError,
}

impl Stop {
    /// The word used in the ledger and the run summary.
    pub fn as_str(self) -> &'static str {
        match self {
            Stop::Witness => "witness",
            Stop::Budget => "budget",
            Stop::ProposerError => "proposer-error",
        }
    }

    /// Whether this stop leaves the marker's reachability open. Every stop but a
    /// witness does, so any coverage figure including them is a lower bound.
    pub fn is_inconclusive(self) -> bool {
        !matches!(self, Stop::Witness)
    }
}

/// Default number of witnesses to collect per marker. One input per marker is
/// what the suite reports; `RUSMT_WITNESSES` raises it.
pub const DEFAULT_WITNESSES: usize = 1;

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

/// The candidate a proposal carries, given as `input <<<…>>>`.
///
/// The *last* block wins. Some transports echo the prompt before the answer, and
/// the prompt contains the reply template, so taking the first block would
/// return `(your candidate here)`.
pub fn parse_proposal(text: &str) -> Option<String> {
    let start = text.rfind("<<<")?;
    let end = text[start + 3..].find(">>>")?;
    let mut body = &text[start + 3..start + 3 + end];
    body = body.strip_prefix('\n').unwrap_or(body);
    body = body.strip_suffix('\n').unwrap_or(body);
    Some(decode_escapes(body))
}

/// Decode `\u{HEX}` and leave every other byte alone.
///
/// A control character cannot cross the text channel raw, so it needs a spelling.
/// Only `\u{…}` is decoded — `\q` and `\uZZZZ` are themselves markers and must
/// reach the parser untouched.
fn decode_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.char_indices();
    while let Some((i, c)) = it.next() {
        if c != '\\' || !s[i..].starts_with("\\u{") {
            out.push(c);
            continue;
        }
        let rest = &s[i + 3..];
        match rest.find('}') {
            Some(j) => match u32::from_str_radix(&rest[..j], 16)
                .ok()
                .and_then(char::from_u32)
            {
                Some(ch) => {
                    out.push(ch);
                    for _ in 0..(3 + j) {
                        it.next();
                    }
                }
                None => out.push(c),
            },
            None => out.push(c),
        }
    }
    out
}

/// Everything a round's prompt says about the marker being attempted.
pub struct Target<'a> {
    /// The marker name.
    pub marker: &'a str,
    /// The object language, for the oracle registry and the prompt.
    pub language: &'a str,
    /// The function whose body raises the marker. Kept for human transcripts;
    /// it is deliberately not shown to the proposer.
    pub holder: &'a str,
    /// The emitted query, truncated — the proposal is derived from the lifted
    /// semantics as Z3 sees them, never from the Rust source.
    pub smt_excerpt: &'a str,
}

/// The prompt for one round: the emitted SMT-LIB, the marker name, and every
/// earlier candidate with Z3's verdict.
fn build_certify_prompt(t: &Target, rounds: &[Round]) -> String {
    let (marker, language, smt_excerpt) = (t.marker, t.language, t.smt_excerpt);
    let mut p = format!(
        "You are generating a test input inside a program-synthesis pipeline.\n\
         Object language: {language}\n\
         Goal: an input whose execution reaches the error marker `{marker}`.\n\n\
         Z3 cannot search for this input: the lifted parser is too large to solve with a \
         symbolic input. So the division of labour is inverted — YOU propose a concrete \
         candidate, and Z3 DECIDES it by pinning it into the unmodified query. A candidate is \
         accepted only when Z3 answers `sat`, which means the lifted semantics genuinely reach \
         `{marker}` on that input.\n\n\
         Two things make a candidate fail, and both are reported back to you:\n\
         * `unsat` — the input does not reach `{marker}`. Usually it reaches a DIFFERENT error \
           first (an earlier syntax error shadows the one you want), or it is simply valid.\n\
         * `unknown`/timeout — rare for a pinned input; treat as a hint to simplify.\n\n\
         == The channel (a contract, not advice) ==\n\
         A control character cannot reach us as a raw byte. Spell it `\\u{{XX}}` — e.g. \
         `\\u{{0}}` for NUL, `\\u{{7f}}` for DEL. That is the ONLY escape decoded; every other \
         backslash is passed through verbatim, so a TOML escape such as `\\q` or `\\uZZZZ` \
         reaches the parser exactly as you wrote it.\n\n\
         == A domain prior (hints only; ignore any that do not fit) ==\n\
         These shape where you look. They decide nothing: acceptance is Z3's alone.\n\
         * Only the FIRST error matters — the parser stops there, so everything before the \
           intended violation must be valid or it will shadow it.\n\
         * Shorter is better: extra keys, tables and whitespace add chances to trip an \
           earlier rule.\n\
         * The marker name states the rule. Violate exactly that and nothing else.\n\n\
         == The emitted SMT-LIB for this marker (truncated) ==\n{smt_excerpt}\n\n"
    );
    if !rounds.is_empty() {
        p.push_str("== Candidates already rejected ==\n");
        for (i, r) in rounds.iter().enumerate() {
            let _ = write!(
                p,
                "--- attempt {} ---\n{}\n--- Z3 said ---\n{}\n",
                i + 1,
                r.candidate.as_deref().unwrap_or("(none)"),
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
    pub reply: String,
    /// The candidate parsed out of it.
    pub candidate: Option<String>,
    /// Z3's verdict, phrased as the line carried into the next prompt.
    pub outcome: String,
    /// Wall-clock for this round's acceptance solve.
    pub elapsed: Duration,
}

/// The result of Stage 2 for one marker.
pub struct Outcome {
    /// Accepted witnesses, in the order found. Each is a model of the
    /// unmodified query.
    pub witnesses: Vec<String>,
    /// Per-round transcript.
    pub rounds: Vec<Round>,
    /// Z3's verdict on the unmodified, unconstrained query (Stage 1).
    pub stage1: String,
    /// Which rule ended the loop.
    pub stop: Stop,
    /// Wall-clock Z3 spent on Stage 1, in milliseconds.
    pub stage1_ms: u128,
}

impl Outcome {
    /// How many proposals Z3 rejected as `unsat` — the count reviewer 5A's
    /// question is answered with.
    pub fn rejected(&self) -> usize {
        self.rounds
            .iter()
            .filter(|r| r.outcome.starts_with("`unsat`"))
            .count()
    }
}

/// The acceptance check: solve the UNMODIFIED query with the input pinned to
/// `candidate`. This is the only thing that admits a witness.
pub trait Solver {
    /// Z3's verdict on the unmodified query with `candidate` pinned into it.
    fn accept(&mut self, candidate: &str) -> Response;

    /// Which marker the candidate actually fired. A bare `unsat` leaves the next
    /// proposal guessing; this is the counterexample.
    fn observe(&self, _candidate: &str) -> Option<String> {
        None
    }
}

/// Stage 2 for one marker: propose, decide, feed the verdict back.
pub fn certify(
    target: &Target,
    stage1: &Response,
    proposer: &mut dyn Proposer,
    solver: &mut dyn Solver,
    max_rounds: usize,
    want: usize,
) -> Outcome {
    let marker = target.marker;
    let mut rounds: Vec<Round> = Vec::new();
    let mut witnesses: Vec<String> = Vec::new();
    let mut stop = Stop::Budget;

    while witnesses.len() < want {
        if rounds.len() >= max_rounds {
            stop = Stop::Budget;
            break;
        }
        let prompt = build_certify_prompt(target, &rounds);
        let reply = match proposer.propose(&prompt) {
            Ok(r) => r,
            Err(e) => {
                rounds.push(Round {
                    reply: String::new(),
                    candidate: None,
                    outcome: format!("PROPOSER ERROR: {e:#}"),
                    elapsed: Duration::ZERO,
                });
                stop = Stop::ProposerError;
                break;
            }
        };
        let Some(candidate) = parse_proposal(&reply) else {
            rounds.push(Round {
                reply,
                candidate: None,
                outcome: "no candidate found — put the input between <<< and >>>".to_string(),
                elapsed: Duration::ZERO,
            });
            continue;
        };
        // Already decided: say so rather than spending a solve on it again.
        if rounds
            .iter()
            .any(|r| r.candidate.as_ref() == Some(&candidate))
        {
            rounds.push(Round {
                reply,
                candidate: Some(candidate),
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
            Response::Unsat => match solver.observe(&candidate) {
                Some(obs) if obs.starts_with("ERR ") => format!(
                    "`unsat`: this input does not reach `{marker}`. Z3 reports that it \
                     reaches {obs} instead — a DIFFERENT rule fired first and shadowed the one you \
                     want. Fix that earlier violation so the document is valid up to the \
                     point `{marker}` is about."
                ),
                Some(obs) if obs.starts_with("OK") => format!(
                    "`unsat`: this input does not reach `{marker}`. Z3 reports {obs} \
                     for it — the document is simply valid, so it violates no \
                     rule at all. It has to break exactly the rule `{marker}` names."
                ),
                Some(obs) => format!(
                    "`unsat`: this input does not reach `{marker}`. Z3 reports {obs} for it."
                ),
                None => format!(
                    "`unsat`: this input does not reach `{marker}`. It is either valid, or an \
                     earlier error shadows the one you want."
                ),
            },
            other => format!(
                "`{other}`: Z3 could not decide this input within the budget. Simplify it — \
                 shorter, fewer constructs."
            ),
        };
        rounds.push(Round {
            reply,
            candidate: Some(candidate),
            outcome,
            elapsed: started.elapsed(),
        });
    }

    if witnesses.len() >= want {
        stop = Stop::Witness;
    }
    Outcome {
        witnesses,
        rounds,
        stage1: stage1.to_string(),
        stop,
        stage1_ms: 0,
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
         The proposer reads the emitted SMT-LIB and returns one candidate. Every witness below\n\
         is a model of the UNMODIFIED query, obtained by pinning the candidate into it and\n\
         re-solving; a rejected candidate is a counterexample fed into the next round.\n\n",
        outcome.stage1
    );
    for (i, r) in outcome.rounds.iter().enumerate() {
        let _ = writeln!(s, "=== round {} ({} ms) ===", i + 1, r.elapsed.as_millis());
        let _ = writeln!(s, "--- reply ---\n{}", r.reply.trim());
        if let Some(c) = &r.candidate {
            let _ = writeln!(s, "candidate: {c:?}");
        }
        let _ = writeln!(s, "--- outcome ---\n{}\n", r.outcome);
    }
    if outcome.witnesses.is_empty() {
        let _ = writeln!(s, "status: no witness (stopped: {})", outcome.stop.as_str());
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

/// Writes each query to a file under `dir`, so any round re-runs with `z3 -smt2`.
pub struct FileSolver<'a> {
    /// The unmodified query.
    pub unmodified: &'a str,
    /// Where the acceptance query is written.
    pub dir: &'a Path,
    /// Per-solve Z3 budget.
    pub budget: Duration,
    /// Serial number, so a round's query survives the next round.
    pub round: usize,
    /// A query that binds the returned `Path` to a constant instead of
    /// asserting a marker, used to explain a rejection.
    pub observation: &'a str,
    /// Marker names by bit position, for decoding that constant.
    pub marker_at_bit: &'a [String],
}

impl Solver for FileSolver<'_> {
    fn accept(&mut self, candidate: &str) -> Response {
        let Some(q) = pin_input(self.unmodified, candidate, INPUT_VAR) else {
            return Response::Unknown("unmodified query has no (check-sat)".to_string());
        };
        self.round += 1;
        let p = self.dir.join("accept.smt2");
        if let Err(e) = std::fs::write(&p, &q) {
            return Response::Unknown(format!("cannot write acceptance query: {e}"));
        }
        run_z3_file(&p, self.budget)
    }

    fn observe(&self, candidate: &str) -> Option<String> {
        let q = pin_input(self.observation, candidate, INPUT_VAR)?;
        let p = self.dir.join("observe.smt2");
        std::fs::write(&p, &q).ok()?;
        let Response::Sat(model) = run_z3_file(&p, self.budget) else {
            return None;
        };
        let bits = crate::guidance::decode_bitvec_bits(&model, crate::guidance::OBSERVED_PATH)?;
        let named: Vec<&str> = bits
            .iter()
            .filter_map(|&b| self.marker_at_bit.get(b).map(String::as_str))
            .collect();
        Some(match named.as_slice() {
            [] => "OK".to_string(),
            [one] => format!("ERR {one}"),
            many => format!("ERR {}", many.join(", ")),
        })
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

    fn mock(script: Vec<&'static str>) -> MockProposer {
        MockProposer {
            script,
            next: 0,
            prompts: Vec::new(),
        }
    }

    /// Accepts exactly the candidates it was given; everything else is `unsat`.
    struct MockSolver {
        accepts: Vec<&'static str>,
        calls: usize,
    }

    impl Solver for MockSolver {
        fn accept(&mut self, candidate: &str) -> Response {
            self.calls += 1;
            if self.accepts.contains(&candidate) {
                Response::Sat(String::new())
            } else {
                Response::Unsat
            }
        }
    }

    fn run(p: &mut MockProposer, s: &mut dyn Solver, rounds: usize, want: usize) -> Outcome {
        certify(
            &Target {
                marker: "comment_invalid_char",
                language: "toml",
                holder: "parse_comment_rest",
                smt_excerpt: "(declare-const input_0 (Seq (_ BitVec 32)))",
            },
            &Response::Timeout,
            p,
            s,
            rounds,
            want,
        )
    }

    #[test]
    fn a_candidate_is_a_witness_only_when_the_unmodified_query_accepts_it() {
        let mut p = mock(vec!["input <<<\nzz\n>>>", "input <<<\n#\u{0}\n>>>"]);
        let mut s = MockSolver {
            accepts: vec!["#\u{0}"],
            calls: 0,
        };
        let out = run(&mut p, &mut s, 3, 1);
        assert_eq!(out.witnesses, vec!["#\u{0}".to_string()]);
        assert_eq!(out.rejected(), 1, "the first candidate was rejected");
        assert!(out.rounds[1].outcome.contains("ACCEPTED"));
    }

    #[test]
    fn a_rejection_reaches_the_next_prompt_as_the_counterexample() {
        let mut p = mock(vec!["input <<<\nzz\n>>>", "input <<<\nyy\n>>>"]);
        let mut s = MockSolver {
            accepts: vec![],
            calls: 0,
        };
        let out = run(&mut p, &mut s, 2, 1);
        assert!(out.witnesses.is_empty());
        assert_eq!(out.rejected(), 2);
        assert!(p.prompts[1].contains("Candidates already rejected"));
        assert!(p.prompts[1].contains("zz"));
    }

    #[test]
    fn solver_observation_is_the_rejection_counterexample() {
        struct ObservingSolver;
        impl Solver for ObservingSolver {
            fn accept(&mut self, _candidate: &str) -> Response {
                Response::Unsat
            }

            fn observe(&self, _candidate: &str) -> Option<String> {
                Some("ERR array_open_eof".to_string())
            }
        }

        let mut p = mock(vec!["input <<<\na=[\n>>>", "input <<<\nyy\n>>>"]);
        let mut s = ObservingSolver;
        let out = run(&mut p, &mut s, 2, 1);
        assert!(out.rounds[0].outcome.contains("Z3 reports"));
        assert!(out.rounds[0].outcome.contains("array_open_eof"));
        assert!(p.prompts[1].contains("array_open_eof"));
    }

    #[test]
    fn the_prompt_carries_smt_level_facts_and_never_the_rust_source() {
        let mut p = mock(vec!["input <<<\nzz\n>>>"]);
        let mut s = MockSolver {
            accepts: vec![],
            calls: 0,
        };
        let _ = run(&mut p, &mut s, 1, 1);
        let prompt = &p.prompts[0];
        assert!(prompt.contains("comment_invalid_char"));
        assert!(!prompt.contains("parse_comment_rest"));
        assert!(prompt.contains("(declare-const input_0 (Seq (_ BitVec 32)))"));
        assert!(prompt.contains("Z3 DECIDES it"));
    }

    #[test]
    fn a_repeated_candidate_costs_no_solve() {
        let mut p = mock(vec!["input <<<\nzz\n>>>", "input <<<\nzz\n>>>"]);
        let mut s = MockSolver {
            accepts: vec![],
            calls: 0,
        };
        let out = run(&mut p, &mut s, 2, 1);
        assert_eq!(s.calls, 1);
        assert!(out.rounds[1].outcome.contains("already rejected"));
    }

    #[test]
    fn a_reply_with_no_candidate_costs_no_solve() {
        let mut p = mock(vec!["I would suggest a comment.", "input <<<\n#\u{0}\n>>>"]);
        let mut s = MockSolver {
            accepts: vec!["#\u{0}"],
            calls: 0,
        };
        let out = run(&mut p, &mut s, 2, 1);
        assert_eq!(s.calls, 1);
        assert!(out.rounds[0].outcome.contains("no candidate found"));
        assert_eq!(out.witnesses.len(), 1);
    }

    #[test]
    fn extra_witnesses_are_collected_up_to_the_requested_count() {
        let mut p = mock(vec![
            "input <<<\n#\u{0}\n>>>",
            "input <<<\n#\u{1}\n>>>",
            "input <<<\n#\u{2}\n>>>",
        ]);
        let mut s = MockSolver {
            accepts: vec!["#\u{0}", "#\u{1}", "#\u{2}"],
            calls: 0,
        };
        let out = run(&mut p, &mut s, 3, 2);
        assert_eq!(out.witnesses.len(), 2, "stopped at the requested count");
    }

    #[test]
    fn candidate_delimiter_newlines_are_not_part_of_the_input() {
        assert_eq!(
            parse_proposal("input <<<\na=[\n>>>").as_deref(),
            Some("a=[")
        );
        assert_eq!(parse_proposal("no delimiters here"), None);
    }

    #[test]
    fn a_prompt_echo_does_not_steal_the_candidate() {
        // Transports that echo the prompt replay the reply template first.
        let echoed = "input <<<\n(your candidate here)\n>>>\n\nHere is mine:\n\
                      input <<<\na=[\n>>>";
        assert_eq!(parse_proposal(echoed).as_deref(), Some("a=["));
    }

    #[test]
    fn a_control_character_reaches_us_only_through_the_escape() {
        // The model cannot put a raw NUL on stdout, so it spells it.
        assert_eq!(
            parse_proposal("input <<<\n#\\u{0}\n>>>").as_deref(),
            Some("#\u{0}")
        );
        assert_eq!(
            parse_proposal("input <<<\na = \"\\u{7f}\"\n>>>").as_deref(),
            Some("a = \"\u{7f}\"")
        );
    }

    #[test]
    fn every_other_backslash_is_passed_through_verbatim() {
        // These ARE the markers: an invalid string escape and a bad \u payload
        // have to reach the parser exactly as written.
        assert_eq!(
            parse_proposal("input <<<\na = \"\\q\"\n>>>").as_deref(),
            Some("a = \"\\q\"")
        );
        assert_eq!(
            parse_proposal("input <<<\na = \"\\uZZZZ\"\n>>>").as_deref(),
            Some("a = \"\\uZZZZ\"")
        );
        // Malformed or non-scalar escapes are left alone rather than guessed at.
        assert_eq!(
            parse_proposal("input <<<\na\\u{zz}b\n>>>").as_deref(),
            Some("a\\u{zz}b")
        );
        assert_eq!(
            parse_proposal("input <<<\na\\u{d800}b\n>>>").as_deref(),
            Some("a\\u{d800}b")
        );
    }
}

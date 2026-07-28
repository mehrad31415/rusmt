//! Untrusted candidate proposers and the solver-first, proposer-fallback loop.
//!
//! The synthesis pipeline treats the SMT solver as just one *proposer* of
//! candidate inputs. When the solver fails on a target (timeout, `unknown`, or
//! a bound-limited `unsat`), the pipeline can fall back to a second, equally
//! untrusted proposer — typically a language model — that suggests candidate
//! object-language inputs directly. Soundness never depends on the proposer:
//! every candidate is accepted only if concrete replay through the reference
//! semantics (the [`LanguageOracle`]'s certifier) fires the *targeted* named
//! marker. A wrong guess costs one cheap concrete run; it can never become a
//! false witness.
//!
//! The asymmetry this exploits: for the deep, multi-theory queries on which Z3
//! times out, *checking* a candidate (a millisecond concrete run of the
//! reference semantics) is far cheaper than *finding* one (solving the lifted
//! query). The loop is counterexample-guided: each rejected candidate's verdict
//! (parse error, no marker fired, wrong marker fired, non-termination) is fed
//! back to the proposer for the next round.
//!
//! Configuration (environment):
//! * `RUSMT_LLM_CMD` — a shell command that reads a prompt on stdin and writes
//!   a candidate on stdout (e.g. `claude -p`, `llm -m <model>`, or any local
//!   script). Unset ⇒ the fallback is disabled and the pipeline behaves as
//!   before.
//! * `RUSMT_LLM_MAX_GUESSES` — proposal rounds per target (default
//!   [`DEFAULT_MAX_GUESSES`]).

use anyhow::{Context, Result, bail};
use rusmt_lang::certify::{LanguageOracle, Verdict};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Default number of proposal rounds per target.
pub const DEFAULT_MAX_GUESSES: usize = 4;

/// Wall-clock budget for replaying one candidate through the reference
/// semantics. Replay is near-instant; the budget only bounds adversarial or
/// non-terminating candidates.
pub const REPLAY_BUDGET: Duration = Duration::from_secs(5);

/// An untrusted source of candidates: a language model, an enumerator, or a
/// mock in tests. Implementations are *never* part of the trusted base — every
/// candidate is certified by concrete replay before it is accepted.
pub trait Proposer {
    /// Produce one candidate for the given prompt.
    fn propose(&mut self, prompt: &str) -> Result<String>;

    /// Short provenance label written into the fallback transcript.
    fn describe(&self) -> String;
}

/// A [`Proposer`] that pipes the prompt to a user-configured shell command and
/// reads the candidate from its stdout. Vendor-agnostic by construction: the
/// pipeline never hard-codes a model or an API.
pub struct CommandProposer {
    /// The shell command (run via `sh -c`).
    cmd: String,
}

impl CommandProposer {
    /// Build a proposer from an explicit shell command.
    pub fn new(cmd: impl Into<String>) -> Self {
        Self { cmd: cmd.into() }
    }

    /// Build the proposer configured by `RUSMT_LLM_CMD`, or `None` if the
    /// variable is unset or empty (fallback disabled).
    pub fn from_env() -> Option<Self> {
        match std::env::var("RUSMT_LLM_CMD") {
            Ok(cmd) if !cmd.trim().is_empty() => Some(Self::new(cmd)),
            _ => None,
        }
    }
}

impl Proposer for CommandProposer {
    fn propose(&mut self, prompt: &str) -> Result<String> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn proposer command `{}`", self.cmd))?;
        child
            .stdin
            .take()
            .expect("stdin piped")
            .write_all(prompt.as_bytes())
            .context("failed to write prompt to proposer stdin")?;
        let out = child
            .wait_with_output()
            .context("failed to wait for proposer command")?;
        if !out.status.success() {
            bail!(
                "proposer command `{}` exited with {}: {}",
                self.cmd,
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let candidate = strip_fences(&String::from_utf8_lossy(&out.stdout));
        if candidate.is_empty() {
            bail!("proposer command `{}` produced no candidate", self.cmd);
        }
        Ok(candidate)
    }

    fn describe(&self) -> String {
        format!("command proposer `{}`", self.cmd)
    }
}

/// Strip a surrounding Markdown code fence (with optional language tag) that
/// chat-tuned models often wrap answers in, and trim whitespace.
pub fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let Some(rest) = t.strip_prefix("```") else {
        return t.to_string();
    };
    let Some(body) = rest.split_once('\n').map(|(_tag, body)| body) else {
        return t.to_string();
    };
    match body.rfind("```") {
        Some(end) => body[..end].trim().to_string(),
        None => body.trim().to_string(),
    }
}

/// The outcome of the fallback loop for one target.
pub struct Recovery {
    /// The replay-certified witness, if any round produced one.
    pub witness: Option<String>,
    /// Per-round transcript: `(candidate, verdict line)`.
    pub attempts: Vec<(String, String)>,
}

/// Render a [`Verdict`] as the feedback line shown to the proposer (and
/// recorded in the transcript). Marker ids are translated back to names where
/// the model declares them, so the proposer gets actionable feedback.
pub fn verdict_line(
    verdict: &Verdict,
    target: &str,
    marker_names: &BTreeMap<usize, String>,
) -> String {
    match verdict {
        Verdict::ReachedTarget => {
            format!("CERTIFIED: replay through the reference semantics fired `{target}`")
        }
        Verdict::ReachedOtherMarker(ids) => {
            let named: Vec<&str> = ids
                .iter()
                .filter_map(|id| marker_names.get(id).map(String::as_str))
                .collect();
            if named.is_empty() {
                format!(
                    "REJECTED: execution fired a different (unnamed) marker, not `{target}` \
                     (ids {ids:?})"
                )
            } else {
                format!(
                    "REJECTED: execution fired marker(s) {named:?} instead of `{target}`; \
                     adjust the input so the `{target}` condition is the one that fires"
                )
            }
        }
        Verdict::NoMarker => {
            "REJECTED: the input parses and executes to completion without firing any marker"
                .to_string()
        }
        Verdict::ParseError(e) => format!("REJECTED: the input does not parse: {e}"),
        Verdict::Timeout => {
            "REJECTED: replay exceeded its wall-clock budget (likely non-terminating)".to_string()
        }
        Verdict::Crashed(e) => format!(
            "REJECTED: replay crashed (e.g. unbounded recursion overflowing the stack): {e}"
        ),
    }
}

/// Build the proposal prompt for one round.
fn build_prompt(
    oracle: &LanguageOracle,
    target: &str,
    solver_outcome: &str,
    attempts: &[(String, String)],
) -> String {
    let mut p = String::new();
    p.push_str(&format!(
        "You are a test-input proposer inside a program-synthesis pipeline.\n\
         Object language: {}\n{}\n\n\
         Task: write ONE {} input (plain `.{}` source text) whose execution by \
         the reference semantics reaches the error marker named `{}`.\n\
         The SMT solver failed to synthesize an input for this marker \
         (solver outcome: {}). Your candidate will be checked by re-executing \
         the trusted reference semantics; it is accepted only if `{}` actually \
         fires. Prefer the smallest input you can.\n",
        oracle.name, oracle.brief, oracle.name, oracle.ext, target, solver_outcome, target,
    ));
    if !attempts.is_empty() {
        p.push_str("\nEarlier candidates and the checker's verdicts:\n");
        for (i, (candidate, verdict)) in attempts.iter().enumerate() {
            p.push_str(&format!(
                "--- candidate {} ---\n{}\n--- verdict ---\n{}\n",
                i + 1,
                candidate,
                verdict
            ));
        }
    }
    p.push_str(
        "\nOutput exactly the candidate source text and nothing else: \
         no prose, no markdown fences, no explanation.\n",
    );
    p
}

/// The solver-first, proposer-fallback loop for one *named* marker target.
///
/// Asks `proposer` for up to `max_guesses` candidates; each is certified by
/// `certify(candidate, target)` — concrete replay through the reference
/// semantics (the pipeline passes the process-isolated
/// `rusmt_lang::certify::certify_isolated`, so a crashing candidate is merely
/// rejected) — and the loop stops at the first certified witness. Rejected
/// candidates' verdicts are fed back into the next prompt
/// (counterexample-guided). The returned transcript records every round.
///
/// The solver stays **in the loop** and is never bypassed: each proposed
/// candidate is pinned as a `define-fun` macro and handed to Z3, which validates
/// that the lifted semantics reach the target marker (see
/// [`crate::guidance::macro_inline_input`]). Acceptance is **double-gated**: Z3
/// must return `sat` AND concrete replay must certify the same marker — the
/// identical reachability fact checked by two independent executions of the
/// semantics (symbolic and concrete). A candidate Z3 cannot validate (`unsat`,
/// `unknown`, or timeout) is rejected, never accepted on replay alone, so there
/// is no replay-only bypass. The route therefore applies to inputs Z3 can
/// macro-inline (a code-point sequence); the caller supplies `z3_validate`
/// accordingly.
pub fn recover_target(
    oracle: &LanguageOracle,
    target: &str,
    solver_outcome: &str,
    marker_names: &BTreeMap<usize, String>,
    proposer: &mut dyn Proposer,
    certify: &dyn Fn(&str, &str) -> Verdict,
    max_guesses: usize,
    z3_validate: &dyn Fn(&str) -> crate::guidance::Response,
) -> Recovery {
    use crate::guidance::Response;
    let mut attempts: Vec<(String, String)> = Vec::new();
    for _round in 0..max_guesses {
        let prompt = build_prompt(oracle, target, solver_outcome, &attempts);
        let candidate = match proposer.propose(&prompt) {
            Ok(c) => c,
            Err(e) => {
                // A proposer failure (command missing, API error) ends the loop:
                // record it and report no witness.
                attempts.push((String::new(), format!("PROPOSER ERROR: {e:#}")));
                return Recovery {
                    witness: None,
                    attempts,
                };
            }
        };
        // Solver in the loop (gate 1): Z3 must validate the macro-inlined
        // candidate. This is required, never bypassed.
        let z3 = z3_validate(&candidate);
        let z3_sat = matches!(z3, Response::Sat(_));
        let z3_note = match &z3 {
            Response::Sat(_) => "Z3 validated (sat); ",
            Response::Unsat => "Z3 refuted (unsat — does not reach the marker); ",
            Response::Timeout => "Z3 could not validate (timeout); ",
            Response::Unknown(_) => "Z3 could not validate (unknown); ",
        };
        // Cross-check (gate 2): concrete replay through the reference semantics.
        let verdict = certify(&candidate, target);
        let certified = verdict.is_certified();
        let line = format!("{z3_note}{}", verdict_line(&verdict, target, marker_names));
        attempts.push((candidate.clone(), line));
        // Accept only when the solver validates AND replay certifies: the witness
        // is endorsed by both the symbolic and the concrete semantics.
        if z3_sat && certified {
            return Recovery {
                witness: Some(candidate),
                attempts,
            };
        }
    }
    Recovery {
        witness: None,
        attempts,
    }
}

/// Read `RUSMT_LLM_MAX_GUESSES`, defaulting to [`DEFAULT_MAX_GUESSES`].
pub fn max_guesses_from_env() -> usize {
    std::env::var("RUSMT_LLM_MAX_GUESSES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_GUESSES)
}

/// Render the fallback transcript written into the target directory
/// (`fallback.txt`, alongside the target's response file).
pub fn render_transcript(
    proposer_desc: &str,
    target: &str,
    solver_outcome: &str,
    recovery: &Recovery,
) -> String {
    let mut s = format!(
        "solver outcome : {solver_outcome}\nproposer       : {proposer_desc}\ntarget marker  : {target}\n\n"
    );
    for (i, (candidate, verdict)) in recovery.attempts.iter().enumerate() {
        s.push_str(&format!(
            "=== attempt {} ===\n{}\n--- verdict ---\n{}\n\n",
            i + 1,
            candidate,
            verdict
        ));
    }
    match &recovery.witness {
        Some(_) => s.push_str(
            "status: WITNESS ACCEPTED — double-gated (written to the target's response file): Z3 \
             validated the macro-inlined candidate (sat) AND replay through the concrete \
             reference semantics certified the same marker. The proposer is untrusted; \
             the solver is never bypassed and there is no replay-only acceptance.\n",
        ),
        None => s.push_str("status: no accepted witness within the guess budget\n"),
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusmt_lang::certify::oracle_for;

    /// A scripted proposer for tests: returns its canned candidates in order.
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

    #[test]
    fn fences_are_stripped() {
        assert_eq!(strip_fences("```imp\nA := 1\n```"), "A := 1");
        assert_eq!(strip_fences("```\nA := 1\n```\n"), "A := 1");
        assert_eq!(strip_fences("A := 1\n"), "A := 1");
    }

    #[test]
    fn command_proposer_pipes_stdin_to_stdout() {
        // `cat` echoes the prompt back: enough to test the plumbing.
        let mut p = CommandProposer::new("cat");
        let out = p.propose("hello proposer").expect("cat runs");
        assert_eq!(out, "hello proposer");
    }

    /// In-process certifier for tests (the pipeline uses the process-isolated
    /// variant; the candidates here all terminate).
    fn in_process(
        oracle: &'static rusmt_lang::certify::LanguageOracle,
    ) -> impl Fn(&str, &str) -> Verdict {
        |src: &str, tgt: &str| (oracle.certify)(src, tgt, REPLAY_BUDGET)
    }

    #[test]
    fn rejected_candidates_get_feedback_and_a_later_round_can_be_certified() {
        let oracle = oracle_for("imp").expect("imp is registered");
        // Round 1 proposes a program that fires no marker; round 2 a genuine
        // division-by-zero witness (deliberately not the solver's `A := (0/0)`).
        let mut mock = MockProposer::new(vec!["A := 1", "B := (7 / (2 - 2))"]);
        let rec = recover_target(
            oracle,
            "division_by_zero",
            "timeout",
            &BTreeMap::new(),
            &mut mock,
            &in_process(oracle),
            4,
            &|_: &str| crate::guidance::Response::Sat(String::new()),
        );
        assert_eq!(rec.witness.as_deref(), Some("B := (7 / (2 - 2))"));
        assert_eq!(rec.attempts.len(), 2);
        assert!(rec.attempts[0].1.contains("REJECTED"));
        assert!(rec.attempts[1].1.contains("CERTIFIED"));
        // Z3 validation is in the loop on every attempt (no bypass).
        assert!(rec.attempts[1].1.contains("Z3 validated (sat)"));
        // The second prompt must carry the first verdict (counterexample-guided).
        assert!(mock.prompts[1].contains("without firing any marker"));
    }

    #[test]
    fn a_candidate_for_the_wrong_marker_is_rejected_with_the_named_marker() {
        let oracle = oracle_for("imp").expect("imp is registered");
        let names: BTreeMap<usize, String> = [
            ("division_by_zero", "division_by_zero"),
            ("undefined_variable", "undefined_variable"),
        ]
        .into_iter()
        .map(|(n, v)| (rusmt_smt_stdlib::path::marker_id(n), v.to_string()))
        .collect();
        // `A := (0 / 0)` fires division_by_zero, not undefined_variable.
        let mut mock = MockProposer::new(vec!["A := (0 / 0)"]);
        let rec = recover_target(
            oracle,
            "undefined_variable",
            "timeout",
            &names,
            &mut mock,
            &in_process(oracle),
            1,
            &|_: &str| crate::guidance::Response::Sat(String::new()),
        );
        assert!(rec.witness.is_none());
        assert!(rec.attempts[0].1.contains("division_by_zero"));
    }

    #[test]
    fn the_guess_budget_is_respected() {
        let oracle = oracle_for("imp").expect("imp is registered");
        let mut mock = MockProposer::new(vec!["skip", "skip", "skip", "skip", "skip"]);
        let rec = recover_target(
            oracle,
            "division_by_zero",
            "unknown",
            &BTreeMap::new(),
            &mut mock,
            &in_process(oracle),
            3,
            &|_: &str| crate::guidance::Response::Sat(String::new()),
        );
        assert!(rec.witness.is_none());
        assert_eq!(rec.attempts.len(), 3);
    }
}

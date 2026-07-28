//! A persistent `z3 -in` session: one long-lived solver process per target.
//!
//! The one-shot path ([`crate::guidance::run_z3_file`]) writes a complete
//! `.smt2` file and spawns `z3` for every query. That is the right shape for a
//! single independent check, but the guided loop is not that: every round
//! conjoins a *strengthening* block to the **same** base theory. Re-spawning
//! makes Z3 re-read and re-typecheck the whole base each round — measured at
//! ~0.9 s for the 206 KB TOML base — and, worse, discards everything the
//! previous round's search learned.
//!
//! A session keeps one `z3 -in` process alive with the base theory loaded once.
//! Rounds are scoped with `(push)`/`(pop)`, so the base is paid for exactly
//! once per target and the solver's internal state survives across rounds. Two
//! capabilities follow directly, and neither is practical over the one-shot
//! path:
//!
//! * **Model enumeration.** Asserting a blocking clause and re-checking yields
//!   the *next* distinct model in milliseconds. A single scaffold can therefore
//!   surrender several candidate inputs, each put through the acceptance gates,
//!   before the loop spends another (far more expensive) proposer round.
//! * **Stuck-search introspection.** `(get-info :all-statistics)` after a
//!   `-t:`-expired check reports *where* the search stalled — recursive-function
//!   unfolding, bit-vector reasoning, quantifier instantiation — which becomes
//!   steering feedback for the proposer instead of the single bit "timeout".
//!
//! The session is a transport, not a policy: it moves SMT-LIB text and verdicts,
//! and every soundness decision (what may be asserted, what a `sat` licenses)
//! stays in [`crate::guidance`]. Queries remain plain SMT-LIB, so the artifacts
//! a reviewer inspects are unchanged.
//!
//! Robustness: a wedged Z3 that ignores its own `:timeout` would otherwise hang
//! the pipeline, so every read is deadline-bounded. On expiry the process group
//! is killed and the session is **poisoned** — every later call returns a
//! failure verdict rather than blocking, and the caller falls back to the
//! one-shot path.

use crate::backend::response::Response;
use crate::backend::z3::ctxt::extract_reason_unknown;
use command_group::{CommandGroup, GroupChild};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

/// Echoed after every command batch to mark the end of that batch's output.
/// Z3 prints the string bare (no quotes), one line of its own.
const SENTINEL: &str = "@@RUSMT_SYNC@@";

/// Grace added to a check's own `:timeout` before the session concludes Z3 is
/// wedged (stalled in a preprocessing phase that ignores the timeout) and kills
/// it. Model printing on a large model can take a moment, hence not zero.
const GRACE: Duration = Duration::from_secs(5);

/// A live `z3 -in` process with a base theory already loaded.
pub struct Z3Session {
    child: GroupChild,
    stdin: ChildStdin,
    /// Lines of Z3's stdout, drained continuously by a reader thread so a large
    /// base write can never deadlock against a full stdout pipe.
    lines: Receiver<String>,
    /// Set once a read deadline expires or the process dies; every subsequent
    /// operation short-circuits instead of blocking.
    poisoned: bool,
    /// Per-check budget, also the base of every read deadline.
    budget: Duration,
}

impl Z3Session {
    /// Spawn `z3 -in`, load `base` (a query with its trailing `(check-sat)` and
    /// friends removed — see [`crate::guidance::split_at_check_sat`]), and wait
    /// for Z3 to finish reading it.
    ///
    /// `budget` is the per-check timeout handed to Z3 via `(set-option
    /// :timeout …)`. Returns the parse diagnostics as `Err` if Z3 rejected any
    /// of the base, so a malformed base is a caller-visible failure rather than
    /// a silent stream of `unknown`s.
    pub fn start(base: &str, budget: Duration) -> Result<Self, String> {
        let mut cmd = Command::new("z3");
        cmd.arg("-in")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .group_spawn()
            .map_err(|e| format!("failed to spawn z3 -in: {e}"))?;
        let stdin = child
            .inner()
            .stdin
            .take()
            .ok_or_else(|| "failed to capture z3 stdin".to_string())?;
        let stdout = child
            .inner()
            .stdout
            .take()
            .ok_or_else(|| "failed to capture z3 stdout".to_string())?;

        // Start draining stdout *before* the (potentially large) base write.
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });

        let mut session = Self {
            child,
            stdin,
            lines: rx,
            poisoned: false,
            budget,
        };

        session.send(base)?;
        // The emitted base already sets these, but a hand-supplied one may not,
        // and re-setting is harmless.
        session.send("(set-option :produce-models true)")?;
        session.send("(set-option :print-success false)")?;

        // Loading a large base is parse-bound, not search-bound; allow the full
        // budget plus grace for it.
        let banner = session
            .sync(budget + GRACE)
            .ok_or_else(|| "z3 did not acknowledge the base theory".to_string())?;
        if banner.contains("(error") {
            let detail: Vec<&str> = banner
                .lines()
                .filter(|l| l.contains("(error"))
                .take(3)
                .collect();
            return Err(format!("z3 rejected the base theory: {}", detail.join(" ")));
        }
        Ok(session)
    }

    /// Whether the session is still usable. A poisoned session answers every
    /// query with a failure verdict; the caller should fall back to the
    /// one-shot path.
    pub fn is_alive(&self) -> bool {
        !self.poisoned
    }

    /// Write one command (or a whole block of them) to Z3.
    fn send(&mut self, cmd: &str) -> Result<(), String> {
        if self.poisoned {
            return Err("z3 session is poisoned".to_string());
        }
        let write = self
            .stdin
            .write_all(cmd.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush());
        if let Err(e) = write {
            self.poison();
            return Err(format!("cannot write to z3: {e}"));
        }
        Ok(())
    }

    /// Kill the process group and mark the session unusable.
    fn poison(&mut self) {
        if !self.poisoned {
            self.poisoned = true;
            let _ = self.child.kill();
        }
    }

    /// Emit the sentinel and collect every line Z3 produced up to it.
    /// `None` means the deadline expired (or the process died); the session is
    /// poisoned in that case.
    fn sync(&mut self, deadline: Duration) -> Option<String> {
        if self.send(&format!("(echo \"{SENTINEL}\")")).is_err() {
            return None;
        }
        let start = Instant::now();
        let mut out = String::new();
        loop {
            let remaining = match deadline.checked_sub(start.elapsed()) {
                Some(r) if !r.is_zero() => r,
                _ => {
                    self.poison();
                    return None;
                }
            };
            match self.lines.recv_timeout(remaining) {
                Ok(line) if line.trim() == SENTINEL => return Some(out),
                Ok(line) => {
                    out.push_str(&line);
                    out.push('\n');
                }
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                    self.poison();
                    return None;
                }
            }
        }
    }

    /// Open a new assertion scope. Everything asserted until the matching
    /// [`Self::pop`] is retracted together, leaving the base untouched.
    pub fn push(&mut self) -> Result<(), String> {
        self.send("(push)")
    }

    /// Close the innermost assertion scope.
    pub fn pop(&mut self) -> Result<(), String> {
        self.send("(pop)")
    }

    /// Assert a block of raw SMT-LIB (one or more `(assert …)` forms) into the
    /// current scope.
    pub fn assert_block(&mut self, block: &str) -> Result<(), String> {
        self.send(block)
    }

    /// Assert that `var` differs from `term` — the blocking clause that makes
    /// the next [`Self::check`] return a *different* model. `term` must already
    /// be an SMT-LIB term of `var`'s sort (see
    /// [`crate::guidance::encode_seq_literal`]).
    pub fn block_value(&mut self, var: &str, term: &str) -> Result<(), String> {
        self.send(&format!("(assert (not (= {var} {term})))"))
    }

    /// Run `(check-sat)` under the session budget and return the verdict, with
    /// the model text attached on `sat` in the same shape the one-shot path
    /// produces (`"sat\n(…)"`), so model decoding is identical either way.
    pub fn check(&mut self) -> Response {
        if self.poisoned {
            return Response::Unknown("z3 session is poisoned".to_string());
        }
        let ms = self.budget.as_millis();
        if self.send(&format!("(set-option :timeout {ms})")).is_err()
            || self.send("(check-sat)").is_err()
        {
            return Response::Unknown("z3 session write failed".to_string());
        }
        let start = Instant::now();
        let Some(out) = self.sync(self.budget + GRACE) else {
            // Z3 ignored its own timeout and had to be killed.
            return Response::Timeout;
        };
        let verdict = out
            .lines()
            .map(str::trim)
            .find(|&l| l == "sat" || l == "unsat" || l == "unknown");
        match verdict {
            Some("sat") => match self.get_model() {
                Some(model) => Response::Sat(format!("sat\n{model}")),
                None => Response::Sat("sat\n(model unavailable)".to_string()),
            },
            Some("unsat") => Response::Unsat,
            Some("unknown") => {
                if start.elapsed() >= self.budget {
                    return Response::Timeout;
                }
                let reason = self.reason_unknown();
                if reason.contains("timeout")
                    || reason.contains("canceled")
                    || reason.contains("interrupted")
                {
                    Response::Timeout
                } else {
                    Response::Unknown(reason)
                }
            }
            _ => Response::Unknown(format!(
                "z3 produced no verdict: {}",
                out.lines().next().unwrap_or("")
            )),
        }
    }

    /// Fetch the current model as text.
    fn get_model(&mut self) -> Option<String> {
        self.send("(get-model)").ok()?;
        let out = self.sync(self.budget + GRACE)?;
        if out.trim().is_empty() || out.contains("(error") {
            None
        } else {
            Some(out)
        }
    }

    /// Ask Z3 why it gave up. Empty when unavailable.
    fn reason_unknown(&mut self) -> String {
        if self.send("(get-info :reason-unknown)").is_err() {
            return String::new();
        }
        match self.sync(GRACE) {
            Some(out) => extract_reason_unknown(&out),
            None => String::new(),
        }
    }

    /// Read the live solver's statistics as one compact, greppable line.
    ///
    /// This is the introspection a stuck search affords over a persistent
    /// session: the process is still alive after a `:timeout`-expired check, so
    /// the counters that say *where* the search stalled — quantifier
    /// instantiations, recursive-function unfoldings, bit-vector reasoning,
    /// peak memory — are still readable. Spawning a fresh `z3` per query cannot
    /// do this: the process is gone before the verdict is even parsed. Empty if
    /// the session is poisoned (a hard-killed Z3 leaves nothing to read).
    pub fn statistics(&mut self) -> String {
        if self.poisoned || self.send("(get-info :all-statistics)").is_err() {
            return String::new();
        }
        let Some(out) = self.sync(GRACE) else {
            return String::new();
        };
        let entries: Vec<String> = out
            .split_whitespace()
            .collect::<Vec<_>>()
            .chunks(2)
            .filter_map(|pair| match pair {
                [k, v] => {
                    let key = k.trim_start_matches(['(', ':']);
                    let val = v.trim_end_matches(')');
                    if key.is_empty() || val.is_empty() {
                        None
                    } else {
                        Some(format!("{key}={val}"))
                    }
                }
                _ => None,
            })
            .collect();
        if entries.is_empty() {
            String::new()
        } else {
            format!("[z3-stats] {}", entries.join(" "))
        }
    }
}

impl Drop for Z3Session {
    fn drop(&mut self) {
        // Ask nicely, then make sure. `(exit)` lets Z3 flush and close cleanly;
        // the kill covers a process already wedged past listening.
        let _ = self.stdin.write_all(b"(exit)\n");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A base with a single integer constant, enough to exercise the protocol.
    const BASE: &str = "(declare-const x Int)\n(assert (> x 0))\n(assert (< x 10))\n";

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a tiny integer query)"]
    fn a_session_loads_a_base_and_scopes_rounds_with_push_pop() {
        let mut s = Z3Session::start(BASE, Duration::from_secs(10)).expect("session starts");
        assert!(s.is_alive());

        // Round 1: a satisfiable strengthening.
        s.push().expect("push");
        s.assert_block("(assert (= x 5))").expect("assert");
        assert!(matches!(s.check(), Response::Sat(_)));
        s.pop().expect("pop");

        // Round 2: the previous round's assertion is gone, so a contradictory
        // one is unsat on its own terms rather than trivially so.
        s.push().expect("push");
        s.assert_block("(assert (> x 100))").expect("assert");
        assert_eq!(s.check(), Response::Unsat);
        s.pop().expect("pop");

        // The base survived both rounds.
        assert!(matches!(s.check(), Response::Sat(_)));
    }

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a tiny integer query)"]
    fn blocking_clauses_enumerate_distinct_models() {
        let mut s = Z3Session::start(BASE, Duration::from_secs(10)).expect("session starts");
        let mut seen: Vec<i64> = Vec::new();
        for _ in 0..4 {
            let Response::Sat(model) = s.check() else {
                break;
            };
            // The model prints as `(define-fun x () Int N)`; pull out N.
            let n: i64 = model
                .split_whitespace()
                .filter_map(|t| t.trim_end_matches(')').parse::<i64>().ok())
                .next()
                .expect("a value in the model");
            assert!(!seen.contains(&n), "z3 repeated model x={n}");
            seen.push(n);
            s.block_value("x", &n.to_string()).expect("block");
        }
        assert_eq!(seen.len(), 4, "expected four distinct models, got {seen:?}");
    }

    #[test]
    #[ignore = "invokes the real z3 binary (fast: a tiny integer query)"]
    fn statistics_are_readable_from_a_live_session() {
        let mut s = Z3Session::start(BASE, Duration::from_secs(10)).expect("session starts");
        let _ = s.check();
        let stats = s.statistics();
        assert!(stats.starts_with("[z3-stats]"), "stats: {stats}");
        assert!(stats.contains('='), "stats: {stats}");
    }

    #[test]
    #[ignore = "invokes the real z3 binary"]
    fn a_malformed_base_is_reported_rather_than_silently_accepted() {
        match Z3Session::start("(declare-const x NoSuchSort)\n", Duration::from_secs(10)) {
            Ok(_) => panic!("an unknown sort must be reported, not silently accepted"),
            Err(e) => assert!(e.contains("rejected the base theory"), "err: {e}"),
        }
    }
}

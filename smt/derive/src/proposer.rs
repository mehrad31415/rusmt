//! The proposer transport: moves a prompt to a command and text back.
//!
//! No decision-making — [`crate::cosolve`] owns that. Each call runs in a fresh
//! temporary directory that is deleted afterwards, so nothing one marker's
//! proposal leaves behind can reach another.
//!
//! * `RUSMT_LLM_CMD` — a command reading a prompt on stdin, writing a candidate
//!   on stdout. It must not be able to read the semantics under test.
//! * `RUSMT_LLM_CACHE` — a directory of recorded exchanges, keyed by prompt hash.
//!   A hit is served from disk, so a recorded run replays without model access.

use anyhow::{Context, Result, bail};
use rusmt_lang::certify::Verdict;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Wall-clock budget for replaying one input through the reference semantics.
/// Replay is near-instant; the budget only bounds a non-terminating input.
pub const REPLAY_BUDGET: Duration = Duration::from_secs(5);

/// A source of directives: a model, a script, or a mock in tests.
pub trait Proposer {
    /// Produce directives for the given prompt.
    fn propose(&mut self, prompt: &str) -> Result<String>;

    /// Short provenance label for the transcript.
    fn describe(&self) -> String;
}

/// Content-addressed key for a prompt: two FNV-1a passes with different offset
/// bases, plus the length.
///
/// 128 bits over a few thousand prompts per run makes an accidental collision
/// not worth a dependency on a cryptographic hash. This is a cache key and a
/// filename, not a security boundary.
fn prompt_key(prompt: &str) -> String {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x9e37_79b9_7f4a_7c15;
    for byte in prompt.as_bytes() {
        a = (a ^ *byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
        b = (b ^ *byte as u64)
            .wrapping_mul(0x0000_0100_0000_01b3)
            .rotate_left(7);
    }
    format!("{a:016x}{b:016x}-{}", prompt.len())
}

/// A [`Proposer`] that pipes the prompt to a shell command, optionally backed by
/// an on-disk cache of previous exchanges.
pub struct CommandProposer {
    /// The shell command (run via `sh -c`).
    cmd: String,
    /// Where to record and look up exchanges, if configured.
    cache: Option<PathBuf>,
    /// Exchanges served from the cache this run.
    hits: usize,
    /// Exchanges that actually invoked the command.
    misses: usize,
}

impl CommandProposer {
    /// Build a proposer from an explicit shell command, with no cache.
    pub fn new(cmd: impl Into<String>) -> Self {
        Self {
            cmd: cmd.into(),
            cache: None,
            hits: 0,
            misses: 0,
        }
    }

    /// Record and replay exchanges under `dir`.
    pub fn with_cache(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache = Some(dir.into());
        self
    }

    /// The proposer named on the command line, falling back to `RUSMT_LLM_CMD`.
    ///
    /// `RUSMT_LLM_CACHE` is the transcript directory when set. `None` if neither
    /// names a command.
    pub fn from_cli_or_env(cli: Option<&str>) -> Option<Self> {
        if let Some(c) = cli.filter(|c| !c.trim().is_empty()) {
            let mut p = Self::new(c);
            if let Ok(dir) = std::env::var("RUSMT_LLM_CACHE") {
                if !dir.trim().is_empty() {
                    p = p.with_cache(dir);
                }
            }
            return Some(p);
        }
        Self::from_env()
    }

    /// The proposer configured by `RUSMT_LLM_CMD`, with `RUSMT_LLM_CACHE` as the
    /// transcript directory when set. `None` if no command is configured.
    pub fn from_env() -> Option<Self> {
        let cmd = match std::env::var("RUSMT_LLM_CMD") {
            Ok(c) if !c.trim().is_empty() => c,
            _ => return None,
        };
        let mut p = Self::new(cmd);
        if let Ok(dir) = std::env::var("RUSMT_LLM_CACHE") {
            if !dir.trim().is_empty() {
                p = p.with_cache(dir);
            }
        }
        Some(p)
    }

    /// How many exchanges were served from cache versus invoked.
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }

    /// Invoke the command, returning its stdout with any code fence stripped.
    ///
    /// The command runs in a fresh empty directory that is deleted afterwards.
    /// Each marker is an independent trial, so nothing a proposer writes — or
    /// that a model runner keeps per working directory, since such state is
    /// keyed by it — may reach the next one. Enforcing it here rather than in
    /// the operator's command string is what lets the independence claim be a
    /// property of the framework rather than of the reader's configuration.
    fn invoke(&self, prompt: &str) -> Result<String> {
        let jail = tempfile::tempdir().context("cannot create a proposer directory")?;
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.cmd)
            .current_dir(jail.path())
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
        let text = strip_fences(&String::from_utf8_lossy(&out.stdout));
        if text.is_empty() {
            bail!("proposer command `{}` produced no output", self.cmd);
        }
        Ok(text)
    }
}

impl Proposer for CommandProposer {
    fn propose(&mut self, prompt: &str) -> Result<String> {
        let entry = self
            .cache
            .as_ref()
            .map(|d| d.join(format!("{}.txt", prompt_key(prompt))));
        if let Some(path) = &entry {
            if let Ok(recorded) = std::fs::read_to_string(path) {
                self.hits += 1;
                // The prompt is stored above a separator so an exchange can be
                // read back in full; only the response is returned.
                return Ok(match recorded.split_once("\n===RESPONSE===\n") {
                    Some((_prompt, response)) => response.to_string(),
                    None => recorded,
                });
            }
        }
        let response = self.invoke(prompt)?;
        self.misses += 1;
        if let Some(path) = &entry {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, format!("{prompt}\n===RESPONSE===\n{response}"));
        }
        Ok(response)
    }

    fn describe(&self) -> String {
        match &self.cache {
            Some(d) => format!(
                "command proposer `{}` (cache {}, {} hit / {} invoked)",
                self.cmd,
                d.display(),
                self.hits,
                self.misses
            ),
            None => format!("command proposer `{}`", self.cmd),
        }
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

/// Render a replay [`Verdict`] as one line for `replay.txt`.
///
/// Replay compares the concrete Rust semantics against the SMT lift on an input
/// Z3 already proved reaches the marker, so this line reports a
/// transpilation-fidelity check, not a decision about the witness.
pub fn verdict_line(
    verdict: &Verdict,
    target: &str,
    marker_names: &BTreeMap<usize, String>,
) -> String {
    match verdict {
        Verdict::ReachedTarget => {
            format!("AGREES: concrete replay also reaches `{target}`")
        }
        Verdict::ReachedOtherMarker(ids) => {
            let named: Vec<&str> = ids
                .iter()
                .filter_map(|id| marker_names.get(id).map(String::as_str))
                .collect();
            if named.is_empty() {
                format!("DIVERGES: concrete replay fired unnamed marker(s) {ids:?}, not `{target}`")
            } else {
                format!("DIVERGES: concrete replay fired {named:?}, not `{target}`")
            }
        }
        Verdict::NoMarker => {
            "DIVERGES: concrete replay completed without firing any marker".to_string()
        }
        Verdict::ParseError(e) => format!("DIVERGES: the input does not parse concretely: {e}"),
        Verdict::Timeout => "DIVERGES: concrete replay exceeded its budget".to_string(),
        Verdict::Crashed(e) => format!("DIVERGES: concrete replay crashed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fences_are_stripped() {
        assert_eq!(strip_fences("```\nlen_max 4\n```"), "len_max 4");
        assert_eq!(strip_fences("```text\nlen_max 4\n```\n"), "len_max 4");
        assert_eq!(strip_fences("len_max 4\n"), "len_max 4");
    }

    #[test]
    fn the_command_receives_the_prompt_on_stdin() {
        let mut p = CommandProposer::new("cat");
        assert_eq!(p.propose("len_max 4").expect("cat runs"), "len_max 4");
    }

    #[test]
    fn distinct_prompts_get_distinct_cache_keys() {
        assert_ne!(prompt_key("a"), prompt_key("b"));
        assert_ne!(prompt_key("ab"), prompt_key("ba"));
        assert_eq!(prompt_key("same"), prompt_key("same"));
    }

    #[test]
    fn a_cached_exchange_is_replayed_without_invoking_the_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        // First run: `cat` echoes the prompt, and the exchange is recorded.
        let mut p = CommandProposer::new("cat").with_cache(dir.path());
        assert_eq!(p.propose("len_max 4").expect("runs"), "len_max 4");
        assert_eq!(p.cache_stats(), (0, 1));

        // Second run with a command that would FAIL if invoked: the answer must
        // come from the cache, which is what makes a recorded run replayable.
        let mut q = CommandProposer::new("exit 1").with_cache(dir.path());
        assert_eq!(
            q.propose("len_max 4").expect("served from cache"),
            "len_max 4"
        );
        assert_eq!(q.cache_stats(), (1, 0));
        // A prompt that was never recorded still goes to the command, and fails.
        assert!(q.propose("a different prompt").is_err());
    }

    #[test]
    fn a_recorded_exchange_keeps_the_prompt_for_inspection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut p = CommandProposer::new("cat").with_cache(dir.path());
        p.propose("the prompt").expect("runs");
        let file = std::fs::read_dir(dir.path())
            .expect("readable")
            .next()
            .expect("one entry")
            .expect("entry");
        let body = std::fs::read_to_string(file.path()).expect("readable");
        assert!(body.contains("the prompt"));
        assert!(body.contains("===RESPONSE==="));
    }

    #[test]
    fn a_failing_command_is_an_error_not_an_empty_proposal() {
        let mut p = CommandProposer::new("exit 3");
        let e = p.propose("x").expect_err("must fail");
        assert!(format!("{e:#}").contains("exited with"));
        // Silence is also an error: an empty reply is not a valid round.
        let mut q = CommandProposer::new("true");
        assert!(q.propose("x").is_err());
    }

    #[test]
    fn replay_verdicts_read_as_fidelity_checks_not_witness_decisions() {
        let names: BTreeMap<usize, String> = [(7usize, "other".to_string())].into_iter().collect();
        assert!(verdict_line(&Verdict::ReachedTarget, "m", &names).starts_with("AGREES"));
        let d = verdict_line(&Verdict::ReachedOtherMarker(vec![7]), "m", &names);
        assert!(d.starts_with("DIVERGES") && d.contains("other"));
        assert!(verdict_line(&Verdict::NoMarker, "m", &names).starts_with("DIVERGES"));
    }
}

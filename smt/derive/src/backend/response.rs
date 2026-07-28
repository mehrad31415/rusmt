//! Module containing the response enum, the execution timeout, and the number of CPU cores.

use lazy_static::lazy_static;
use std::fmt::{Display, Formatter};
use std::time::Duration;

/// Default execution timeout for the backend in seconds: 10 minutes.
pub const DEFAULT_BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

/// Execution timeout for the backend.
///
/// `RUSMT_BACKEND_TIMEOUT_SECS` overrides the default and is intentionally a
/// runtime setting so large target sweeps can be reproduced under a bounded
/// budget without changing normal behavior.
pub fn backend_timeout() -> Duration {
    std::env::var("RUSMT_BACKEND_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_BACKEND_TIMEOUT)
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The response returned by the backend solver.
pub enum Response {
    /// Satisfiable model found
    Sat(String),
    /// Unsatisfiable
    Unsat,
    /// Unknown result; carries the reason string Z3 reported via
    /// `(get-info :reason-unknown)` (empty if Z3 didn't emit one).
    Unknown(String),
    /// Timeout occurred
    Timeout,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => f.write_str("timeout"),
            Self::Unknown(reason) if reason.is_empty() => f.write_str("unknown"),
            Self::Unknown(reason) => write!(f, "unknown\nreason: {reason}"),
            Self::Sat(model) => f.write_str(model),
            Self::Unsat => f.write_str("unsat"),
        }
    }
}

lazy_static! {
    /// Number of CPU cores available on this machine.
    pub static ref NUM_CPU_CORES: usize = num_cpus::get();
}

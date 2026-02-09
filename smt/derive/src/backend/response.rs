//! Module containing the response enum, the execution timeout, and the number of CPU cores.

use lazy_static::lazy_static;
use std::fmt::{Display, Formatter};
use std::time::Duration;

/// Execution timeout for the backend in seconds: by default 10 minutes (600 seconds).
pub const BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

#[derive(Debug, Clone)]
/// The response returned by the backend solver.
pub(crate) enum Response {
    /// Satisfiable model found
    Sat(String),
    /// Unsatisfiable
    Unsat,
    /// Unknown result
    Unknown,
    /// Timeout occurred
    Timeout,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Timeout => "timeout",
            Self::Unknown => "unknown",
            Self::Sat(model) => &format!("sat: model found\n {}", model),
            Self::Unsat => "unsat",
        };
        f.write_str(text)
    }
}

lazy_static! {
    /// Number of CPU cores available on this machine.
    pub static ref NUM_CPU_CORES: usize = num_cpus::get();
}

//! Module containing the response enum, the execution timeout, and the number of CPU cores.

use lazy_static::lazy_static;
use std::fmt::{Display, Formatter};

/// Execution timeout for the backend.
///
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

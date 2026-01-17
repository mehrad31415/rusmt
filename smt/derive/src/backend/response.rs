//! Module containing the response enum and the execution timeout.

use std::fmt::{Display, Formatter};
use std::time::Duration;

/// Execution timeout for the backend in seconds: by default 10 minutes (600 seconds).
pub const BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

#[derive(Debug, Clone)]
/// The response returned by the backend solver.
pub enum Response {
    Sat(String),
    Unsat,
    Unknown,
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

//! Module containing the response enum, the execution timeout, and the number of CPU cores.

use std::fmt::{Display, Formatter};
use std::time::Duration;
use lazy_static::lazy_static;

/// Execution timeout for the backend in seconds: by default 10 minutes (600 seconds).
pub const BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

#[derive(Debug, Clone, PartialEq, Eq)]
/// The response returned by the backend solver.
pub enum Response {
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
        match self {                                                                                                                                                           
            Self::Timeout => f.write_str("timeout"),
            Self::Unknown => f.write_str("unknown"),                                                                                                                           
            Self::Sat(model) => f.write_str(model),                                                                                                                          
            Self::Unsat => f.write_str("unsat"),                                                                                                                               
        }
    }                                                                                                                                                                          
}


lazy_static! {
    /// Number of CPU cores available on this machine.
    pub static ref NUM_CPU_CORES: usize = num_cpus::get();
}
//! Module containing the response enum, the execution timeout, and the number of CPU cores.

use std::fmt::{Display, Formatter};
use std::time::Duration;

/// Execution timeout for the backend in seconds: by default 10 minutes (600 seconds).
pub(crate) const BACKEND_TIMEOUT: Duration = Duration::from_secs(60 * 10);

#[derive(Debug, Clone, PartialEq, Eq)]
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
        match self {                                                                                                                                                           
            Self::Timeout => f.write_str("timeout"),
            Self::Unknown => f.write_str("unknown"),                                                                                                                           
            Self::Sat(model) => f.write_str(model),                                                                                                                          
            Self::Unsat => f.write_str("unsat"),                                                                                                                               
        }
    }                                                                                                                                                                          
}   

static NUM_CPU_CORES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
pub(crate) fn num_cpu_cores() -> usize {                                                                                                                                              
    *NUM_CPU_CORES.get_or_init(num_cpus::get)                                                                                                                                  
}     
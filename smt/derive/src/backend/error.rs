//! Error types for the backend generator

use std::fmt::Display;

/// An error for backend generator
#[derive(Debug)]
pub struct BackendError;

impl Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("backend error")
    }
}
pub type BackendResult<T> = Result<T, BackendError>;

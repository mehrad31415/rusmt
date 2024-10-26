/// An error for backend generator
#[derive(Debug)]
pub enum BackendError {
    NotSupported,
}

pub type BackendResult<T> = Result<T, BackendError>;

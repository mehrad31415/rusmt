use crate::Error;
use internment::Intern;
use std::collections::BTreeSet;

/// A context to manage unique ID generation
pub struct ErrorContext {
    counter: usize,
}

impl ErrorContext {
    /// Create a new ErrorContext
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Generate a fresh error
    pub fn fresh_error(&mut self) -> Error {
        let id = self.counter;
        self.counter += 1;
        Error::fresh(id)
    }
}

impl Error {
    /// Create a new error
    /// Every time the fresh() method is called, a new error state is created with a unique inner value.
    /// The inner values are incremented by one each time a new error state is created.
    pub fn fresh(id: usize) -> Self {
        let mut set = BTreeSet::new();
        set.insert(id);
        Self {
            inner: Intern::new(set),
        }
    }

    /// Merge two errors
    /// The merge method is used to merge two error states where duplicates are not allowed.
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&r.inner).copied().collect()),
        }
    }
}

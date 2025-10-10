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
        let result = Error::fresh(id);
        self.counter += 1;
        result
    }
}

impl Error {
    /// Every time the fresh() method is called, a new error state is created with a unique inner value.
    /// The inner values are incremented by one each time a new error state is created.
    pub(crate) fn fresh(id: usize) -> Self {
        let mut set = BTreeSet::new();
        set.insert(id);
        Self {
            inner: Intern::new(set),
        }
    }

    /// Merge two errors (duplicates are not allowed)
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&r.inner).copied().collect()),
        }
    }
}

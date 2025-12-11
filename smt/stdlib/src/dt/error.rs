//! Errors of the Executable semantic model.

use crate::Error;
use internment::Intern;
use std::{
    collections::BTreeSet,
    sync::atomic::{self, AtomicUsize},
};

/// An error counter to generate unique error states.
static _ERROR_COUNTER_: AtomicUsize = AtomicUsize::new(0);

impl Error {
    /// Every time the fresh() method is called, a new error state is created with a unique inner value.
    /// The inner values are incremented by one each time a new error state is created.
    pub fn fresh() -> Self {
        let id = _ERROR_COUNTER_.fetch_add(1, atomic::Ordering::SeqCst);
        
        // Panic on overflow (practical limit is 2^63 errors)
        if id == usize::MAX {
            panic!("Error counter overflow: generated 2^64 unique errors");
        }
        let mut set = BTreeSet::new();
        set.insert(id);
        Self {
            inner: Intern::new(set),
        }
    }

    /// Merge two errors (no duplicates)
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&r.inner).copied().collect()),
        }
    }
}

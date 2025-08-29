use crate::Error;
use internment::Intern;
use std::collections::BTreeSet;
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;

static _ERROR_COUNTER_: AtomicUsize = AtomicUsize::new(0);

impl Error {
    /// Create a new error
    pub fn fresh() -> Self {
        let mut set = BTreeSet::new();
        set.insert(_ERROR_COUNTER_.fetch_add(1, atomic::Ordering::SeqCst));
        Self {
            inner: Intern::new(set),
        }
    }

    /// Merge two errors
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.iter().chain(r.inner.iter()).copied().collect()),
        }
    }
}

//! Path-condition markers of the Executable semantic model.

use crate::Path;
use internment::Intern;
use std::{
    collections::BTreeSet,
    sync::atomic::{self, AtomicUsize},
};

/// A path counter to generate unique path-condition markers.
static _PATH_COUNTER_: AtomicUsize = AtomicUsize::new(0);

impl Path {
    /// Every time the fresh() method is called, a new path-condition marker is created with a unique inner value.
    /// The inner values are incremented by one each time a new marker is created.
    pub fn fresh() -> Self {
        let id = _PATH_COUNTER_.fetch_add(1, atomic::Ordering::SeqCst);

        // Panic on overflow (practical limit is 2^63 markers)
        if id == usize::MAX {
            panic!("Path counter overflow: generated 2^64 unique path markers");
        }
        let mut set = BTreeSet::new();
        set.insert(id);
        Self {
            inner: Intern::new(set),
        }
    }

    /// Merge two path markers (no duplicates)
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&r.inner).copied().collect()),
        }
    }
}

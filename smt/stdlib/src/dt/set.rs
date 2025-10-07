use crate::smt::SMT;
use crate::{Boolean, Integer, Set, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeSet;

impl<T: SMT> Set<T> {
    /// create an new set: `Set::new()`
    pub fn new() -> Self {
        Self {
            inner: Intern::new(BTreeSet::new()),
        }
    }

    /// return the length of the set: `s.length()`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// a non in-place operation to insert an element into the set: `s.insert(e)`
    pub fn insert(self, e: T) -> Self {
        let mut new_set = (*self.inner).clone();
        new_set.insert(SMTWrap(e));
        Self {
            inner: Intern::new(new_set),
        }
    }

    /// a non in-place operation to remove an element from the set: `s.remove(e)`
    pub fn remove(self, e: T) -> Self {
        let mut new_set = (*self.inner).clone();
        new_set.remove(&SMTWrap(e));
        Self {
            inner: Intern::new(new_set),
        }
    }

    /// `v.contains(e)`
    pub fn contains(self, e: T) -> Boolean {
        self.inner.contains(&SMTWrap(e)).into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<T> {
        self.inner.iter().map(|i| i.0).collect()
    }

    /// checks if the set is empty: `s.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// take the intersection of two sets
    pub fn intersection(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.intersection(&other.inner).copied().collect()),
        }
    }

    /// take the union of two sets
    pub fn union(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&other.inner).copied().collect()),
        }
    }

    /// take the difference of two sets (self - other)
    pub fn difference(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.difference(&other.inner).copied().collect()),
        }
    }

    /// is subset of other (self <= other)
    pub fn is_subset(self, other: Self) -> Boolean {
        self.inner.is_subset(&other.inner).into()
    }
}

#[macro_export]
/// Example: set!(Integer::from(1), Integer::from(2));
macro_rules! set {
    ( $($e:expr),*) => {
        {
            let mut set = Set::new();
            $(
                set = set.insert($e);
            )*
            set
        }
    };
}

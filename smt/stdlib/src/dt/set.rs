//! Set<K> -- Array<K,Bool> datatype and operations

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

    /// iterator over the elements of the set: `s.iterator()`
    // this method should not be used in the direct implementation of the interpreters so it should not be translated to SMT-LIB.
    // it is used in the expression macros for iterating over the set.
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

    /// Symmetric difference (elements in either but not both)
    pub fn symmetric_difference(self, other: Self) -> Self {
        let diff1 = self.difference(other.clone());
        let diff2 = other.difference(self.clone());
        diff1.union(diff2)
    }

    /// is subset of other (self <= other)
    pub fn is_subset(self, other: Self) -> Boolean {
        self.inner.is_subset(&other.inner).into()
    }

    /// Checks if this is a proper subset (⊂, not ⊆)
    pub fn is_proper_subset(self, other: Self) -> Boolean {
        (self
            .is_subset(other.clone())
            .and(self.length().lt(other.length())))
        .into()
    }

    /// This is a concrete check. Z3's `set.has_size` is a symbolic predicate.
    pub fn has_size(self, k: Integer) -> Boolean {
        self.length().eq(k)
    }

    /// Checks if two sets are disjoint (no common elements)
    pub fn is_disjoint(self, other: Self) -> Boolean {
        self.inner.is_disjoint(&other.inner).into()
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

mod tests {
    #[test]
    fn test_set_empty_len_insert() {
        use crate::{Integer, Set, smt::SMT};

        let empty: Set<Integer> = Set::new();
        assert!(*empty.length().eq(Integer::from(0)));

        let s1 = empty.insert(Integer::from(10));
        assert!(*s1.length().eq(Integer::from(1)));

        let s2 = s1.insert(Integer::from(20));
        assert!(*s2.length().eq(Integer::from(2)));

        // Duplicate insert — length stays the same
        let s3 = s2.insert(Integer::from(10));
        assert!(*s3.length().eq(Integer::from(2)));
    }
}

mod test {
    #[test]
    fn test_set_operations() {
        use crate::{Integer, Set, smt::SMT};

        let empty: Set<Integer> = Set::new();
        assert!(*empty.is_empty());
        assert!(*empty.length().eq(Integer::from(0)));

        let s1 = empty.insert(Integer::from(10));
        assert!(*s1.length().eq(Integer::from(1)));
        assert!(*s1.contains(Integer::from(10)));
        assert!(*s1.contains(Integer::from(20)).not());

        let s2 = s1.insert(Integer::from(20));
        assert!(*s2.length().eq(Integer::from(2)));

        // duplicate insert
        let s3 = s2.insert(Integer::from(10));
        assert!(*s3.length().eq(Integer::from(2)));

        // remove
        let s4 = s3.remove(Integer::from(20));
        assert!(*s4.length().eq(Integer::from(1)));
        assert!(*s4.contains(Integer::from(10)));
        assert!(*s4.contains(Integer::from(20)).not());

        // remove non-existent
        let s5 = s4.remove(Integer::from(99));
        assert!(*s5.length().eq(Integer::from(1)));

        // is_subset
        assert!(*s4.is_subset(s2));
        assert!(*s2.is_subset(s4).not());
        assert!(*empty.is_subset(s2));

        // is_proper_subset
        assert!(*s4.is_proper_subset(s2));
        assert!(*s2.is_proper_subset(s2).not());

        // has_size
        assert!(*s2.has_size(Integer::from(2)));
        assert!(*s2.has_size(Integer::from(3)).not());

        // is_disjoint
        let a = Set::new().insert(Integer::from(1)).insert(Integer::from(2));
        let b = Set::new().insert(Integer::from(3)).insert(Integer::from(4));
        let c = Set::new().insert(Integer::from(2)).insert(Integer::from(3));
        assert!(*a.is_disjoint(b));
        assert!(*a.is_disjoint(c).not());
    }
}

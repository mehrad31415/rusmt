//! Array<K, V> data type and methods.

use crate::smt::SMT;
use crate::{Array, Boolean, Integer, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeMap;

/// SMT methods for symbolic arrays.
impl<K: SMT, V: SMT> Array<K, V> {
    /// Creates a new symbolic array where every key maps to a default value.
    /// let arr: Array<KeyType, ValueType> = Array::new(); -- transpiles to:
    /// (declare-const arr (Array KeyType ValueType))
    /// (declare-const value_type_sentinel ValueType)
    /// (declare-fun len ((Array KeyType ValueType)) Int)
    /// (assert (= (len arr) 0))
    /// (assert (= arr ((as const (Array KeyType ValueType)) value_type_sentinel)))
    pub fn new() -> Self {
        Self {
            inner: Intern::new(BTreeMap::new()),
        }
    }

    /// let arr: Array<KeyType, ValueType> = Array::new();
    /// let arr2 = arr.store(k, v); -- transpiles to:
    /// ... previous declarations ...
    /// (declare-const arr2 (Array KeyType ValueType))
    /// (assert (= arr2 (store arr k v)))
    /// (assert (ite (= (select arr k) value_type_sentinel) (= (len arr2) (+ (len arr) 1)) (= (len arr2) (len arr))))
    pub fn store(self, k: K, v: V) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.insert(SMTWrap(k), SMTWrap(v));
        Self {
            inner: Intern::new(new_map),
        }
    }

    /// Performs the low-level SMT `select` operation: `(select self k)`.
    /// 
    /// # Panics
    /// Panics if key does not exist. Use `contains_key` to check first.
    pub fn select(self, k: K) -> V {
        self.inner.get(&SMTWrap(k)).map(|v| v.0).unwrap()
    }

    /// `arr.contains_key(k)` - similar to (not (= (select arr2 k) value_type_sentinel))
    pub fn contains_key(self, k: K) -> Boolean {
        self.inner.contains_key(&SMTWrap(k)).into()
    }

    /// `m.del(k)` removes the key-value pair for `k` if it exists, no-op otherwise.
    ///
    /// This translates to `(store arr k value_type_sentinel)` and decreases the length by 1 if `k` existed.
    pub fn del(self, k: K) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.remove(&SMTWrap(k));
        Self {
            inner: Intern::new(new_map),
        }
    }

    /// return the length of the array: `m.length()`.
    /// This translates to `(len arr)` -- must maintain the length invariant.
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// iterator over the keys of the array: `m.iterator()`
    /// This is mainly useful for testing and translates to forall k in keys(arr)
    // this method should not be used in the direct implementation of the interpreters so it should not be translated to SMT-LIB.
    // it is used in the expression macros for iterating over the array.
    pub fn iterator(self) -> Vec<K> {
        self.inner.keys().map(|i| i.0).collect()
    }

    /// checks if the array is empty: `m.is_empty()`
    /// This translates to `(= (len arr) 0)`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }
}

#[macro_export]
/// Example: array!((Integer::from(1), String::from("one")), (Integer::from(2), String::from("two")));
macro_rules! array {
    ($( ($e1:expr, $e2:expr) ),*) => {
        {
            let mut arr = Array::new();
            $(
                arr = arr.store($e1, $e2);
            )*
            arr
        }
    };
}

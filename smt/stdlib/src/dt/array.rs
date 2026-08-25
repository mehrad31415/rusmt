//! Array<K, V> data type and methods.

use crate::smt::SMT;
use crate::{Array, Boolean, Integer, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeMap;

/// SMT methods for symbolic arrays.
impl<K: SMT, V: SMT> Array<K, V> {
    /// Creates a new empty symbolic array: no key is present, and the entry
    /// count is 0.
    ///
    /// Arrays are not bare SMT `(Array K V)` — they transpile to a record
    /// datatype pairing a value array with a **presence** array plus an entry count:
    /// (declare-datatypes ((RuSmtArray 2))
    ///   ((par (K V) ((mk-rarr (rarr-data (Array K V))
    ///                         (rarr-pres (Array K Bool))
    ///                         (rarr-card Int))))))
    ///
    /// `Array::new()` is a *term* that builds the empty record — every value
    /// slot is a don't-care default, every presence bit is false, count 0:
    /// (mk-rarr ((as const (Array KeyType ValueType)) <default-V>)
    ///          ((as const (Array KeyType Bool)) false)
    ///          0)
    ///
    /// Membership lives in `rarr-pres`, not in the value, so a key holding the
    /// value type's default is still distinguishable from an absent key.
    /// `Array::length` reads `(rarr-card arr)`.
    pub fn new() -> Self {
        Self {
            inner: Intern::new(BTreeMap::new()),
        }
    }

    /// `arr.store(k, v)` inserts/updates key `k` with value `v`, returning a
    /// new array. Transpiles to a term that rebuilds the record: it writes the
    /// value, sets the presence bit, and increments the count only when `k` was
    /// not already present:
    /// (mk-rarr (store (rarr-data arr) k v)
    ///          (store (rarr-pres arr) k true)
    ///          (ite (select (rarr-pres arr) k)
    ///               (rarr-card arr)
    ///               (+ (rarr-card arr) 1)))
    pub fn store(self, k: K, v: V) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.insert(SMTWrap(k), SMTWrap(v));
        Self {
            inner: Intern::new(new_map),
        }
    }

    /// `arr.del(k)` removes the pair for `k` if present, no-op otherwise.
    /// Transpiles to a term that clears the presence bit (and resets the value
    /// slot to the default), decrementing the count only when `k` was present:
    /// (mk-rarr (store (rarr-data arr) k <default-V>)
    ///          (store (rarr-pres arr) k false)
    ///          (ite (select (rarr-pres arr) k)
    ///               (- (rarr-card arr) 1)
    ///               (rarr-card arr)))
    pub fn del(self, k: K) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.remove(&SMTWrap(k));
        Self {
            inner: Intern::new(new_map),
        }
    }

    /// Low-level SMT `select`, reading through the record's backing array:
    /// `(select (rarr-data self) k)`.
    ///
    /// `V::default()` at an absent key, because that is the value the backend
    /// writes into every empty slot (`array_null_value`). It carries no meaning
    /// -- membership lives in `rarr-pres` -- so read it only after
    /// `contains_key`.
    pub fn select(self, k: K) -> V {
        self.inner.get(&SMTWrap(k)).map_or_else(V::default, |v| v.0)
    }

    /// `arr.contains_key(k)` -> transpiles to `(select (rarr-pres arr) k)`
    pub fn contains_key(self, k: K) -> Boolean {
        self.inner.contains_key(&SMTWrap(k)).into()
    }

    /// `arr.length()` -> transpiles to `(rarr-card arr)`.
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// `arr.is_empty()` -> transpiles to `(= (rarr-card arr) 0)`.
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// iterator over the keys of the array: `m.iterator()`
    // this method should not be used in the direct implementation of the interpreters so it should not be translated to SMT-LIB.
    // it is used in the expression macros for iterating over the array.
    pub fn iterator(self) -> Vec<K> {
        self.inner.keys().map(|i| i.0).collect()
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

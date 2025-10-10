use crate::smt::SMT;
use crate::{Array, Boolean, Integer, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeMap;

/// SMT methods for symbolic arrays.
impl<K: SMT, V: SMT> Array<K, V> {
    /// Creates a new symbolic array where every key maps to a default value `(const-array v)`.
    pub fn new(default: V) -> Self {
        Self {
            inner: Intern::new(BTreeMap::new()),
            default,
        }
    }

    /// Performs the low-level SMT `select` operation: `(select self k)`.
    ///
    /// If the key `k` has been explicitly stored, its value is returned.
    /// Otherwise, the array's `default` value is returned.
    pub fn select(self, k: K) -> V {
        self.get(k).unwrap_or(self.default)
    }

    /// `(store self k v)`.
    pub fn store(self, k: K, v: V) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.insert(SMTWrap(k), SMTWrap(v));
        Self {
            inner: Intern::new(new_map),
            default: self.default,
        }
    }

    /// Z3_mk_array_default
    pub fn array_default(self) -> V {
        self.default
    }

    /// This corresponds to the `(map f array)` SMT-LIB function. The transpiler
    /// will need to translate the closure `f` into a Z3 `FuncDecl`.
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(V) -> V,
    {
        let new_default = f(self.default);
        let new_map: BTreeMap<_, _> = self
            .inner
            .iter()
            .map(|(k, v)| (*k, SMTWrap(f(v.0))))
            .collect();

        Self {
            inner: Intern::new(new_map),
            default: new_default,
        }
    }
}

/// High-level operations that are not natively supported by SMT-LIB.
impl<K: SMT, V: SMT> Array<K, V> {
    /// `v.contains_key(e)` -- similar to (ne (select self k) default)
    pub fn contains_key(self, k: K) -> Boolean {
        self.inner.contains_key(&SMTWrap(k)).into()
    }

    /// receive the value for a key in the array `m.get(k)`
    /// The transpiler can model this with an `(ite (contains_key k) (some (select k)) (none))`.
    pub fn get(self, k: K) -> Option<V> {
        self.inner.get(&SMTWrap(k)).map(|wrapped_val| wrapped_val.0)
    }

    /// `m.del(k, v)`, will delete the (`k`, `v`) pair only when `k` exists
    ///
    /// This translates to `(store array key default_value)`.
    pub fn del(self, k: K) -> Self {
        let mut new_map = (*self.inner).clone();
        new_map.remove(&SMTWrap(k));
        Self {
            inner: Intern::new(new_map),
            default: self.default,
        }
    }
}

/// These methods only make sense in the concrete Rust execution context.
impl<K: SMT, V: SMT> Array<K, V> {
    /// return the length of the array: `m.length()` - number of explicit `store` operations in the array.
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<K> {
        self.inner.keys().map(|i| i.0).collect()
    }

    /// checks if the array is empty: `m.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }
}

#[macro_export]
/// Example: array!(String::default(), (Integer::from(1), String::from("one")), (Integer::from(2), String::from("two")));
macro_rules! array {
    ($default:expr; $( ($k:expr, $v:expr) ),* $(,)?) => {
        {
            let mut array = $crate::Array::new($default);
            $(
                array = array.store($k, $v);
            )*
            array
        }
    };
}

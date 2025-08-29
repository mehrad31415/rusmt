use crate::smt::SMT;
use crate::{Boolean, Integer, Map, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeMap;

impl<K: SMT, V: SMT> Map<K, V> {
    /// create a new map: `Map::new()`
    pub fn new() -> Self {
        Self {
            inner: Intern::new(BTreeMap::new()),
        }
    }

    /// return the length of the map: `m.length()`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// a non in-place operation to insert a key-value pair into the map
    /// `m.put(k, v)`, will override `v` if `k` already exists
    pub fn put_unchecked(self, k: K, v: V) -> Self {
        Self {
            inner: Intern::new(
                self.inner
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .chain(std::iter::once((SMTWrap(k), SMTWrap(v))))
                    .collect(),
            ),
        }
    }

    /// receive the value for a key and panic if the key does not exist
    /// `m.get(k)` with partial semantics (valid only when `k` exists)
    pub fn get_unchecked(self, k: K) -> V {
        self.inner
            .get(&SMTWrap(k))
            .unwrap_or_else(|| {
                panic!(
                    "key does not exist for SMT Array with key types {}",
                    std::any::type_name::<K>()
                )
            })
            .0
    }

    /// a non in-place operation to delete a key-value pair from the map
    /// if the key does not exist, the operation will not do anything
    /// `m.del(k, v)`, will delete the (`k`, `v`) pair only when `k` exists
    pub fn del_unchecked(self, k: K) -> Self {
        Self {
            inner: Intern::new(
                self.inner
                    .iter()
                    .filter_map(|(i, v)| if *K::eq(i.0, k) { None } else { Some((*i, *v)) })
                    .collect(),
            ),
        }
    }

    /// `v.contains_key(e)`
    pub fn contains_key(self, k: K) -> Boolean {
        self.inner.contains_key(&SMTWrap(k)).into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<K> {
        self.inner.keys().map(|i| i.0).collect()
    }

    /// checks if the map is empty: `m.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }
}

#[macro_export]
/// Example: map!((Integer::from(1), Text::from("one")), (Integer::from(2), Text::from("two")));
macro_rules! map {
($( ($e1:expr, $e2:expr) ),*) => {
        {
            let mut map = Map::new();
            $(
                map = map.put_unchecked($e1, $e2);
            )*
            map
        }
    };
}

use crate::Boolean;
use crate::smt::SMT;
use crate::{Integer, Seq, dt::SMTWrap};

use internment::Intern;
use num_traits::cast::ToPrimitive;

impl<T: SMT> Seq<T> {
    /// create a new sequence: `Seq::new()`
    pub fn new() -> Self {
        Self {
            inner: Intern::new(vec![]),
        }
    }

    /// get the length of the sequence: `v.length()`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// not in-place append operation to the sequence
    /// `let v = Seq::new(); let v = v.append(Integer::from(1));`
    /// This is equivalent to `(seq.++ self (seq.unit e))`.
    pub fn append(self, e: T) -> Self {
        let mut new_seq = (*self.inner).clone();
        new_seq.push(SMTWrap(e));
        Self {
            inner: Intern::new(new_seq),
        }
    }

    /// This corresponds to the `(seq.++ s1 s2)` SMT-LIB function.
    pub fn concat(self, other: Self) -> Self {
        let mut new_seq = (*self.inner).clone();
        new_seq.extend_from_slice(&other.inner);
        Self {
            inner: Intern::new(new_seq),
        }
    }

    /// `v[i]` with partial semantics (valid only when `i` is in bound)
    /// This corresponds to the `(seq.nth s i)` SMT-LIB function, but provides
    /// safe bounds checking via the `Option` type.
    pub fn at(self, i: Integer) -> Option<T> {
        i.inner
            .to_usize()
            .and_then(|idx| self.inner.get(idx))
            .map(|wrapped_val| wrapped_val.0)
    }

    /// This corresponds to `(seq.at s i)`.
    pub fn at_seq(self, i: Integer) -> Self {
        if let Some(elem) = self.at(i) {
            let mut new_vec = Vec::with_capacity(1);
            new_vec.push(SMTWrap(elem));
            Self {
                inner: Intern::new(new_vec),
            }
        } else {
            Self::new()
        }
    }

    /// This corresponds to the `(seq.extract s offset length)` SMT-LIB function.
    pub fn extract(self, offset: Integer, length: Integer) -> Option<Self> {
        let start = offset.inner.to_usize()?;
        let len = length.inner.to_usize()?;
        let end = start.checked_add(len)?;

        if end > self.inner.len() {
            return None;
        }

        let new_vec = self.inner[start..end].to_vec();
        Some(Self {
            inner: Intern::new(new_vec),
        })
    }

    /// This corresponds to `(seq.replace s src dst)`.
    pub fn replace_first(self, src: Self, dst: Self) -> Self {
        if src.is_empty().inner {
            return self;
        }
        if let Some(index) = self.index_of(src, 0.into()).inner.to_usize() {
            let mut new_vec = Vec::new();
            new_vec.extend_from_slice(&self.inner[..index]);
            new_vec.extend_from_slice(&dst.inner);
            new_vec.extend_from_slice(&self.inner[index + src.inner.len()..]);
            Self {
                inner: Intern::new(new_vec),
            }
        } else {
            self
        }
    }

    /// This corresponds to `(seq.indexof s substr offset)`.
    pub fn index_of(self, substr: Self, offset: Integer) -> Integer {
        let start = match offset.inner.to_usize() {
            Some(i) => i,
            None => return (-1).into(),
        };
        if start > self.inner.len() {
            return (-1).into();
        }

        self.inner[start..]
            .windows(substr.inner.len())
            .position(|window| window == &*substr.inner)
            .map(|i| Integer::from(i + start))
            .unwrap_or_else(|| Integer::from(-1))
    }

    /// This corresponds to `(seq.last_indexof s substr)`.
    pub fn last_index_of(self, substr: Self) -> Integer {
        if substr.is_empty().inner {
            return self.length();
        }
        self.inner
            .windows(substr.inner.len())
            .rposition(|window| window == &*substr.inner)
            .map(Integer::from)
            .unwrap_or_else(|| Integer::from(-1))
    }

    /// This corresponds to `(seq.map f s)`.
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(T) -> T,
    {
        let new_vec = self.inner.iter().map(|v| SMTWrap(f(v.0))).collect();
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `v.includes(e)`
    /// This corresponds to the `(seq.contains s (seq.unit e))` SMT-LIB function.
    pub fn contains(self, e: T) -> Boolean {
        self.inner.contains(&SMTWrap(e)).into()
    }

    /// This corresponds to the `(seq.prefixof other self)` SMT-LIB function.
    pub fn prefix_of(self, other: Self) -> Boolean {
        other.inner.starts_with(&self.inner).into()
    }

    /// This corresponds to the `(seq.suffixof other self)` SMT-LIB function.
    pub fn suffix_of(self, other: Self) -> Boolean {
        other.inner.ends_with(&self.inner).into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<Integer> {
        (0..self.inner.len()).map(Integer::from).collect()
    }

    /// checks if the sequence is empty: `v.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }
}
/// this is a sequence (list) of SMT values of type T where T is a type that implements the SMT trait.
#[macro_export]
/// Example: seq!(Integer::from(1), Integer::from(2));
macro_rules! seq {
    ($($e:expr),*) => {
        {
            let mut seq = Seq::new();
            $(
                seq = seq.append($e);
            )*
            seq
        }
    };
}

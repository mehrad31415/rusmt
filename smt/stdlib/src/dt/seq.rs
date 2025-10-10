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

    /// `v.length()`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// `(seq.unit e)`
    pub fn unit(e: T) -> Self {
        let mut new_vec = Vec::with_capacity(1);
        new_vec.push(SMTWrap(e));
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `(seq.++ self (seq.unit e))`
    pub fn append(self, e: T) -> Self {
        let mut new_seq = (*self.inner).clone();
        new_seq.push(SMTWrap(e));
        Self {
            inner: Intern::new(new_seq),
        }
    }

    /// `(seq.++ s1 s2)`
    pub fn concat(self, other: Self) -> Self {
        let mut new_seq = (*self.inner).clone();
        new_seq.extend_from_slice(&other.inner);
        Self {
            inner: Intern::new(new_seq),
        }
    }

    /// `(seq.nth s i)`
    pub fn at(self, i: Integer) -> Option<T> {
        i.inner
            .to_usize()
            .and_then(|idx| self.inner.get(idx))
            .map(|wrapped_val| wrapped_val.0)
    }

    /// `(seq.at s i)`
    pub fn at_seq(self, i: Integer) -> Option<Self> {
        if let Some(elem) = self.at(i) {
            let mut new_vec = Vec::with_capacity(1);
            new_vec.push(SMTWrap(elem));
            Some(Self {
                inner: Intern::new(new_vec),
            })
        } else {
            None
        }
    }

    /// `(seq.extract s offset length)`
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

    /// `(seq.map f s)`
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(T) -> T,
    {
        let new_vec = self.inner.iter().map(|v| SMTWrap(f(v.0))).collect();
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `(seq.contains s (seq.unit e))`
    pub fn contains(self, e: T) -> Boolean {
        self.inner.contains(&SMTWrap(e)).into()
    }

    /// `(seq.prefixof other self)`
    pub fn prefix_of(self, other: Self) -> Boolean {
        other.inner.starts_with(&self.inner).into()
    }

    /// `(seq.suffixof other self)`
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

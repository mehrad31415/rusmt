//! Sequence (list) data type and operations

use crate::smt::SMT;
use crate::{Boolean, Integer, Seq, dt::SMTWrap};
use internment::Intern;
use num_traits::cast::ToPrimitive;

impl<T: SMT> Seq<T> {
    /// create a new sequence
    /// let s: Seq<Type> = Seq::new(); traspiles to
    /// (declare-const s (Seq Type))
    /// (assert (= s (as seq.empty (Seq Type))))
    pub fn new() -> Self {
        Self {
            inner: Intern::new(vec![]),
        }
    }

    /// `(seq.unit e)` -- creates a sequence with a single element
    pub fn unit(e: T) -> Self {
        let mut new_vec = Vec::with_capacity(1);
        new_vec.push(SMTWrap(e));
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `(seq.++ (seq.unit e))`
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

    /// `s.length()` transpiles to `(seq.len s)`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// `(seq.extract s offset length)`
    /// Extracts a subsequence starting at `offset` with given `length`.
    /// 
    /// # Panics
    /// Panics if offset or length are negative or too large to convert to usize.
    /// Panics if offset is beyond the sequence length.
    /// If length extends beyond sequence, takes what's available (does not panic).
    pub fn extract(self, offset: Integer, length: Integer) -> Self {
        let start = offset.inner.to_usize().unwrap();
        let len = length.inner.to_usize().unwrap();
        
        // Clamp end to sequence length (like substr for strings)
        let end = (start + len).min(self.inner.len());
        
        let new_vec = self.inner[start..end].to_vec();
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `(seq.indexof s sub offset)`
    /// Returns the first index where subsequence `sub` appears in `self`, starting search at `offset`.
    /// 
    /// # Panics
    /// - Panics if offset is negative or too large to convert to usize
    /// - Panics if offset is beyond the sequence length
    /// - Panics if subsequence is not found
    pub fn index_of(self, sub: Self, offset: Integer) -> Integer {
        let start_pos = offset.inner.to_usize().unwrap();
        // If sub is empty, return offset (empty matches at any position)
        if sub.inner.is_empty() {
            return offset;
        }
        
        // Search for subsequence using a sliding window
        let sub_len = sub.inner.len();
        for i in start_pos..=(self.inner.len().saturating_sub(sub_len)) {
            let window = &self.inner[i..i + sub_len];
            if window == &sub.inner[..] {
                return Integer::from(i);
            }
        }
        
        panic!(
            "index_of: subsequence not found in sequence starting from offset {}",
            start_pos
        );
    }

    /// `(seq.indexof s sub)` - convenience method with offset 0
    pub fn index_of_default(self, sub: Self) -> Integer {
        self.index_of(sub, Integer::from(0))
    }

    /// `(seq.nth s i)`
    pub fn at(self, i: Integer) -> T {
        i.inner
            .to_usize()
            .and_then(|idx| self.inner.get(idx))
            .map(|wrapped_val| wrapped_val.0)
            .unwrap()
    }

    /// `(seq.at s i)`
    pub fn at_seq(self, i: Integer) -> Self {
        Self::unit(self.at(i))
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

    /// iterator over the indices of the sequence: `s.iterator()`
    /// This is mainly useful for testing and translates to forall k >= 0 and k < (seq.len s).
    // this method should not be used in the direct implementation of the interpreters so it should not be translated to SMT-LIB.
    // it is used in the expression macros for iterating over the sequence.
    pub fn iterator(self) -> Vec<Integer> {
        (0..self.inner.len()).map(Integer::from).collect()
    }

    /// checks if the sequence is empty: `v.is_empty()`
    /// This translates to `(= (seq.len s) 0)`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// Replace the first occurrence of src by dst in s: `s.replace(src, dst)`
    /// (seq.replace s src dst)
    pub fn replace(self, src: T, dst: T) -> Self {
        let mut new_vec = Vec::with_capacity(self.inner.len());
        let mut replaced = false;
        for item in self.inner.iter() {
            if !replaced && *item.0.eq(src) {
                new_vec.push(SMTWrap(dst));
                replaced = true;
            } else {
                new_vec.push(item.clone());
            }
        }
        Self {
            inner: Intern::new(new_vec),
        }
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

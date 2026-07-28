//! Sequence (list) data type and operations

use crate::smt::SMT;
use crate::{Boolean, Integer, Seq, dt::SMTWrap};
use internment::Intern;
use num_traits::cast::ToPrimitive;

impl<T: SMT> Seq<T> {
    /// create a new sequence
    ///
    /// `Seq::<Type>::new()` transpiles to the expression
    /// `(as seq.empty (Seq Type))`.
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

    /// `s.length()` transpiles to `(seq.len s)`
    pub fn length(self) -> Integer {
        self.inner.len().into()
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

    /// `self.prefix_of(other)` -- is `self` a prefix of `other`?
    /// Transpiles to `(seq.prefixof self other)`
    pub fn prefix_of(self, other: Self) -> Boolean {
        other.inner.starts_with(&self.inner).into()
    }

    /// `self.suffix_of(other)` -- is `self` a suffix of `other`?
    /// Transpiles to `(seq.suffixof self other)`
    pub fn suffix_of(self, other: Self) -> Boolean {
        other.inner.ends_with(&self.inner).into()
    }

    /// checks if the sequence is empty: `v.is_empty()`
    /// This translates to `(= (seq.len s) 0)`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
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
        let mut ret = None;
        let start_pos = offset.inner.to_usize().unwrap();
        if sub.inner.is_empty() {
            return offset;
        }

        let sub_len = sub.inner.len();
        if sub_len > self.inner.len() {
            ret.expect("index_of: subsequence is longer than the sequence");
        }

        if start_pos > self.inner.len() - sub_len {
            ret.expect("index_of: offset is beyond the sequence length");
        }

        for i in start_pos..=(self.inner.len() - sub_len) {
            let window = &self.inner[i..i + sub_len];
            if window == &sub.inner[..] {
                ret = Some(Integer::from(i));
                break;
            }
        }

        ret.unwrap_or_else(|| {
            panic!("index_of: subsequence not found in sequence starting from offset {start_pos}")
        })
    }

    /// `(seq.indexof s sub)` - convenience method with offset 0
    pub fn index_of_default(self, sub: Self) -> Integer {
        self.index_of(sub, Integer::from(0))
    }

    /// iterator over the indices of the sequence: `s.iterator()`
    // this method should not be used in the direct implementation of the interpreters so it should not be translated to SMT-LIB.
    // it is used in the expression macros for iterating over the sequence.
    pub fn iterator(self) -> Vec<Integer> {
        (0..self.inner.len()).map(Integer::from).collect()
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
                new_vec.push(*item);
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

mod tests {
    #[test]
    fn test_seq_len_push_concat() {
        use crate::{Integer, Seq, smt::SMT};

        let empty: Seq<Integer> = Seq::new();
        assert!(*empty.length().eq(Integer::from(0)));

        let s1 = Seq::unit(Integer::from(1));
        assert!(*s1.length().eq(Integer::from(1)));

        let s2 = s1.append(Integer::from(2));
        assert!(*s2.length().eq(Integer::from(2)));

        let s3 = Seq::unit(Integer::from(3));
        let s4 = s2.concat(s3);
        assert!(*s4.length().eq(Integer::from(3)));
    }

    #[test]
    fn test_seq_nth_at_seq() {
        use crate::{Integer, Seq, forall, smt::SMT};

        let s = Seq::unit(Integer::from(10))
            .append(Integer::from(20))
            .append(Integer::from(30));

        // SeqNth
        assert!(*s.at(Integer::from(0)).eq(Integer::from(10)));
        assert!(*s.at(Integer::from(1)).eq(Integer::from(20)));
        assert!(*s.at(Integer::from(2)).eq(Integer::from(30)));

        // SeqAtSeq
        let sub = s.at_seq(Integer::from(1));
        assert!(*sub.length().eq(Integer::from(1)));
        assert!(*sub.at(Integer::from(0)).eq(Integer::from(20)));

        assert!(*forall!(i in s => s.at_seq(i).at(Integer::from(0)).eq(s.at(i))));
    }

    #[test]
    #[should_panic]
    fn test_seq_nth_out_of_bounds() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        s.at(Integer::from(5));
    }

    #[test]
    #[should_panic]
    fn test_seq_nth_negative() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        s.at(Integer::from(-1));
    }

    #[test]
    fn test_seq_contains_prefix_suffix() {
        use crate::{Integer, Seq};

        let s = Seq::unit(Integer::from(1))
            .append(Integer::from(2))
            .append(Integer::from(3));

        // contains
        assert!(*s.contains(Integer::from(1)));
        assert!(*s.contains(Integer::from(2)));
        assert!(*s.contains(Integer::from(3)));
        assert!(*s.contains(Integer::from(4)).not());

        // prefix_of
        let prefix = Seq::unit(Integer::from(1)).append(Integer::from(2));
        assert!(*prefix.prefix_of(s));
        let not_prefix = Seq::unit(Integer::from(2)).append(Integer::from(3));
        assert!(*not_prefix.prefix_of(s).not());

        // suffix_of
        let suffix = Seq::unit(Integer::from(2)).append(Integer::from(3));
        assert!(*suffix.suffix_of(s));
        let not_suffix = Seq::unit(Integer::from(1)).append(Integer::from(2));
        assert!(*not_suffix.suffix_of(s).not());
    }

    #[test]
    fn test_seq_is_empty() {
        use crate::{Integer, Seq};

        let empty: Seq<Integer> = Seq::new();
        assert!(*empty.is_empty());

        let non_empty = Seq::unit(Integer::from(1));
        assert!(*non_empty.is_empty().not());
    }

    #[test]
    fn test_seq_extract() {
        use crate::{Integer, Seq, smt::SMT};

        let s = Seq::unit(Integer::from(10))
            .append(Integer::from(20))
            .append(Integer::from(30))
            .append(Integer::from(40));

        assert!(
            *s.extract(Integer::from(1), Integer::from(2))
                .length()
                .eq(Integer::from(2))
        );
        assert!(
            *s.extract(Integer::from(1), Integer::from(2))
                .at(Integer::from(0))
                .eq(Integer::from(20))
        );
        assert!(
            *s.extract(Integer::from(1), Integer::from(2))
                .at(Integer::from(1))
                .eq(Integer::from(30))
        );

        // length beyond sequence — clamps
        assert!(
            *s.extract(Integer::from(2), Integer::from(100))
                .length()
                .eq(Integer::from(2))
        );

        // zero length
        assert!(*s.extract(Integer::from(1), Integer::from(0)).is_empty());
    }

    #[test]
    #[should_panic]
    fn test_seq_extract_negative_offset() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        s.extract(Integer::from(-1), Integer::from(1));
    }

    #[test]
    #[should_panic]
    fn test_seq_extract_negative_length() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        s.extract(Integer::from(1), Integer::from(-1));
    }

    #[test]
    #[should_panic]
    fn test_seq_extract_out_of_bounds() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        s.extract(Integer::from(10), Integer::from(1));
    }

    #[test]
    fn test_seq_index_of() {
        use crate::{Integer, Seq, smt::SMT};

        let s = Seq::unit(Integer::from(10))
            .append(Integer::from(20))
            .append(Integer::from(30))
            .append(Integer::from(20))
            .append(Integer::from(30));

        // Find subsequence [20, 30] from 0
        let sub = Seq::unit(Integer::from(20)).append(Integer::from(30));
        assert!(*s.index_of(sub, Integer::from(0)).eq(Integer::from(1)));

        // Find from offset 2 — skips first occurrence
        let sub2 = Seq::unit(Integer::from(20)).append(Integer::from(30));
        assert!(*s.index_of(sub2, Integer::from(2)).eq(Integer::from(3)));

        // Find single element
        let single = Seq::unit(Integer::from(30));
        assert!(*s.index_of(single, Integer::from(0)).eq(Integer::from(2)));

        // Find empty subsequence — returns offset
        let empty: Seq<Integer> = Seq::new();
        assert!(*s.index_of(empty, Integer::from(3)).eq(Integer::from(3)));
    }

    #[test]
    #[should_panic(expected = "index_of: subsequence not found in sequence starting from offset 0")]
    fn test_seq_index_of_not_found() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        let sub = Seq::unit(Integer::from(99));
        s.index_of(sub, Integer::from(0));
    }

    #[test]
    #[should_panic]
    fn test_seq_index_of_negative_offset() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        let sub = Seq::unit(Integer::from(10));
        s.index_of(sub, Integer::from(-1));
    }

    #[test]
    #[should_panic(expected = "index_of: offset is beyond the sequence length")]
    fn test_seq_index_of_offset_out_of_bounds() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        let sub = Seq::unit(Integer::from(10));
        s.index_of(sub, Integer::from(20));
    }

    #[test]
    #[should_panic(expected = "index_of: subsequence is longer than the sequence")]
    fn test_seq_index_of_subsequence_longer_than_sequence() {
        use crate::{Integer, Seq};
        let s = Seq::unit(Integer::from(10));
        let sub = Seq::unit(Integer::from(10)).append(Integer::from(20));
        s.index_of(sub, Integer::from(0));
    }

    #[test]
    fn test_seq_replace() {
        use crate::{Integer, Seq, smt::SMT};

        let s = Seq::unit(Integer::from(1))
            .append(Integer::from(2))
            .append(Integer::from(3))
            .append(Integer::from(2));

        // Replace first 2 with 99
        let r = s.replace(Integer::from(2), Integer::from(99));
        assert!(*r.at(Integer::from(0)).eq(Integer::from(1)));
        assert!(*r.at(Integer::from(1)).eq(Integer::from(99)));
        assert!(*r.at(Integer::from(2)).eq(Integer::from(3)));
        assert!(*r.at(Integer::from(3)).eq(Integer::from(2))); // second 2 unchanged
    }
}

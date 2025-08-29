use crate::smt::SMT;
use crate::{Boolean, Integer, Seq, dt::SMTWrap};
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
    pub fn append(self, e: T) -> Self {
        Self {
            inner: Intern::new(
                self.inner
                    .iter()
                    .copied()
                    .chain(std::iter::once(SMTWrap(e)))
                    .collect(),
            ),
        }
    }

    /// `v[i]` with partial semantics (valid only when `i` is in bound)
    /// The method will panic if the index is too large or is out of bound.
    pub fn at_unchecked(self, i: Integer) -> T {
        self.inner
            .get(i.inner.to_usize().expect("index out of usize range"))
            .unwrap_or_else(|| {
                panic!(
                    "index {:?} out of bound for Seq with type {}",
                    i,
                    std::any::type_name::<T>()
                )
            })
            .0
    }

    /// `v.includes(e)`
    pub fn includes(self, e: T) -> Boolean {
        self.inner.iter().any(|i| *T::eq(i.0, e)).into()
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

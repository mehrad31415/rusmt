//! SMTWrap is a wrapper around SMT types to provide Rust-semantics enrichment such as Ord and Eq implementations.

use crate::smt::SMT;
use std::cmp::Ordering;
use std::hash::Hash;

// In SMTWrap, instead of using #[derive(Eq)] we implement the trait manually to avoid imposing the T: Eq constraint.
// this is because T does not necessarily need to implement the Eq trait as the eq method is a method in the SMT trait.
/// A wrapper around SMT types to provide Rust-semantics enrichment such as Ord and Eq implementations.
#[derive(Debug, Clone, Copy, Default)]
pub struct SMTWrap<T: SMT>(pub T);

impl<T: SMT> PartialEq for SMTWrap<T> {
    fn eq(&self, other: &Self) -> bool {
        *self.0.eq(other.0)
    }
}

impl<T: SMT> Eq for SMTWrap<T> {}

// because we manually implement the PartialEq for SMTWrap, we need to manually implement the Hash trait as well
// see https://rust-lang.github.io/rust-clippy/master/index.html#derived_hash_with_manual_eq
impl<T: SMT> Hash for SMTWrap<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: SMT> PartialOrd for SMTWrap<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: SMT> Ord for SMTWrap<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0._cmp(other.0)
    }
}

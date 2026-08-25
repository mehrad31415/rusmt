//! SMTWrap - Bridge between SMT types and Rust standard collections.
//!
//! # Purpose
//!
//! `SMTWrap<T>` is an adapter that makes custom SMT types compatible with Rust's standard
//! collections (`BTreeSet`, `BTreeMap`, `HashSet`, etc.) by implementing the required traits.
//!
//! # The Problem
//!
//! Our SMT types define custom comparison logic through the `SMT` trait's `_cmp()` method,
//! but Rust's standard collections require types to implement the standard `Ord` and `Eq` traits.
//! For example:
//!
//! - `BTreeSet<T>` requires `T: Ord`
//! - `BTreeMap<K, V>` requires `K: Ord`
//! - `HashSet<T>` requires `T: Eq + Hash`
//!
//! # The Solution
//!
//! `SMTWrap<T>` wraps an SMT type and implements Rust's standard traits by delegating to
//! the SMT trait methods:
//!
//! - `PartialEq::eq` → calls `SMT::eq()` → uses `SMT::_cmp()`
//! - `Ord::cmp` → calls `SMT::_cmp()` directly
//! - `Hash::hash` → delegates to the wrapped type's `Hash` implementation
//!
//! # Usage
//!
//! Used internally by collection types:
//!
//! ```ignore
//! pub struct Seq<T: SMT> {
//!     inner: Intern<Vec<SMTWrap<T>>>,
//! }
//!
//! pub struct Set<T: SMT> {
//!     inner: Intern<BTreeSet<SMTWrap<T>>>,  // BTreeSet requires Ord
//! }
//!
//! pub struct Array<K: SMT, V: SMT> {
//!     inner: Intern<BTreeMap<SMTWrap<K>, SMTWrap<V>>>,  // BTreeMap requires Ord
//! }
//! ```
//!
//! # Example
//!
//! ```ignore
//! let wrapped = SMTWrap(Integer::from(42));
//! let mut set = BTreeSet::new();
//! set.insert(wrapped);  // Works because SMTWrap implements Ord
//! ```

use crate::smt::SMT;
use std::cmp::Ordering;
use std::hash::Hash;

/// A wrapper around SMT types to provide Rust-semantics enrichment such as Ord and Eq implementations.
///
/// This adapter enables SMT types with custom comparison logic to be used in Rust's standard
/// collections that require `Ord`, `Eq`, and `Hash` traits.
///
/// # Note
///
/// We manually implement `Eq` instead of using `#[derive(Eq)]` to avoid imposing the
/// `T: Eq` constraint, since `T` only needs to implement the `SMT` trait which provides its
/// own `eq()` method.
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

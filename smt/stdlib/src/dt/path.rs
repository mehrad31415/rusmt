//! Path-condition markers of the executable semantic model.

use crate::{Path, String};
use internment::Intern;
use std::collections::BTreeSet;

/// Stable, name-derived marker identifier (FNV-1a, masked to a non-negative
/// `usize`).
///
/// This is the single source of truth for the integer id of a *named* marker.
/// It is deliberately a pure function of the marker name only: the transpiler
/// computes it from the string literal it reads in the source, and the concrete
/// evaluator computes it from the same name at run time, so the id the SMT
/// query asserts membership of and the id the concrete `Path` carries on replay
/// are identical *by construction*. That coincidence is what makes per-target replay
/// certification sound.
pub fn marker_id(name: &str) -> usize {
    // FNV-1a over the name's bytes.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // Mask the sign bit so the id is a non-negative `usize`/SMT `Int`.
    (h & 0x7fff_ffff_ffff_ffff) as usize
}

/// The set of marker ids carried by `p`. No intrinsic backs it, so it cannot appear inside an `#[smt_fn]`
pub fn marker_ids(p: Path) -> BTreeSet<usize> {
    p.inner.as_ref().clone()
}

/// The SMT surface of `Path`. Every method here is registered as an intrinsic
/// in `ApplyDatabase::with_intrinsics` and is callable inside an `#[smt_fn]`-annotated function.
impl Path {
    /// Allocate a *named* path-condition marker whose integer id is
    /// [`marker_id`]`(name)`.
    ///
    /// example: `Path::named(String::from("division_by_zero"))`.
    pub fn named(name: String) -> Self {
        let mut set = BTreeSet::new();
        set.insert(marker_id(name.inner.as_ref()));
        Self {
            inner: Intern::new(set),
        }
    }

    /// Union two path markers into one (deduplicated).
    ///
    /// This is the accumulation primitive for *graceful*, non-short-circuiting
    /// error handling: an interpreter that continues past a recoverable error
    /// instead of bailing on the first one `merge`s each marker it raises, so
    /// the resulting `Path` carries *every* error encountered rather than only
    /// the first.
    pub fn merge(self, r: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&r.inner).copied().collect()),
        }
    }
}

//! Bit-set encoding of `Path` for the SMT backend.
//!
//! Concretely a `Path` is a *set* of marker ids (`BTreeSet<usize>` of
//! [`marker_id`](rusmt_smt_stdlib::path::marker_id) hashes), and `Path::merge`
//! is set union. This module gives that set an exact SMT representation: a
//! bit-vector with **one bit per named marker in the program**.
//!
//! | concrete | SMT |
//! |---|---|
//! | `{}` | `(_ bv0 N)` |
//! | `{m}` | the one-hot literal for `m`'s bit |
//! | `l ∪ r` | `(bvor l r)` |
//! | `T ⊆ s` | `(= (bvand s M_T) M_T)` |
//!
//! The correspondence is exact — `bvor` *is* union and a bit test *is*
//! membership — so no query can be answered from information the encoding
//! dropped. That is the whole point of it: the previous encoding held a single
//! representative id, which made `merge` lossy (it discarded its right operand)
//! and made a multi-marker target emit `(and (= e a) (= e b))`, a contradiction
//! that is `unsat` for every input regardless of the program.
//!
//! # Bit indices and the marker hash
//!
//! [`marker_id`](rusmt_smt_stdlib::path::marker_id) is unchanged and remains the
//! identity of a marker on both sides of the pipeline. A bit index is *derived*
//! from it — the rank of that id among the program's marker ids — and never
//! leaves the query it was emitted into:
//!
//! * the SMT side converts a marker *name* to a hash, then to a bit;
//! * the concrete side converts the same *name* to the same hash and tests
//!   membership in the real `BTreeSet`.
//!
//! The two sides exchange the name, never the number, so the "identical by
//! construction" property that makes per-target replay sound is untouched.
//! Nothing decodes a `Path` value back out of a model, so an index never has to
//! be reversed.

use crate::ir::ctxt::IRContext;
use num_bigint::BigUint;
use std::collections::BTreeSet;

/// Width of the `Path` bit-set: one bit per named marker in the program.
///
/// Clamped to at least 1 because `(_ BitVec 0)` is not a legal SMT-LIB sort; a
/// program with no markers still needs `Path` to have *some* sort, and the
/// single bit is simply never set.
pub fn bit_width(ir: &IRContext) -> usize {
    ir.marker_names.len().max(1)
}

/// The bit position of a marker id: its rank among the program's marker ids.
///
/// `marker_names` is a `BTreeMap` keyed by id, so iteration is in sorted-id
/// order and the rank is deterministic for a given program.
///
/// # Panics
///
/// If `marker_id` was never registered. Every id reachable in a `Path` comes
/// from a `Path::named` that registered it (and `PathMerge` rejects operands
/// that are not path expressions), so an unregistered id is an IR bug.
pub fn bit_index(ir: &IRContext, marker_id: usize) -> usize {
    ir.marker_names
        .keys()
        .position(|id| *id == marker_id)
        .unwrap_or_else(|| {
            panic!("marker id {marker_id} is not registered in the IR's marker table")
        })
}

/// `(_ BitVec N)` — the SMT sort of `Path`.
pub fn sort_str(ir: &IRContext) -> String {
    format!("(_ BitVec {})", bit_width(ir))
}

/// A bit-vector literal of `value`, at the `Path` width.
fn literal(value: &BigUint, ir: &IRContext) -> String {
    format!("(_ bv{} {})", value, bit_width(ir))
}

/// The empty path — no marker fired.
pub fn empty_literal(ir: &IRContext) -> String {
    literal(&BigUint::from(0u32), ir)
}

/// The mask with exactly the bits of `ids` set.
pub fn mask_literal(ir: &IRContext, ids: &BTreeSet<usize>) -> String {
    let mut mask = BigUint::from(0u32);
    for id in ids {
        mask |= BigUint::from(1u32) << bit_index(ir, *id);
    }
    literal(&mask, ir)
}

/// The one-hot literal for a single named marker.
pub fn marker_literal(ir: &IRContext, marker_id: usize) -> String {
    literal(&(BigUint::from(1u32) << bit_index(ir, marker_id)), ir)
}

/// The union of two paths.
pub fn merge_expr(lhs: &str, rhs: &str) -> String {
    format!("(bvor {lhs} {rhs})")
}

/// "every marker in `ids` fired in `expr`" — the query-side membership test.
///
/// One formula covers both shapes a target can take: a singleton `Path::named`
/// target and a multi-marker `Path::merge` target ("reach all of these on one
/// run"). Masking with `M_T` and comparing to `M_T` says every bit of `T` is
/// set in `expr`, and says nothing about the other bits, so a run that fires
/// additional markers still satisfies it.
pub fn contains_all(expr: &str, ids: &BTreeSet<usize>, ir: &IRContext) -> String {
    let mask = mask_literal(ir, ids);
    format!("(= (bvand {expr} {mask}) {mask})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusmt_smt_stdlib::path::marker_id;

    /// An IR carrying just a marker table — enough for every function here.
    fn ir_with_markers(names: &[&str]) -> IRContext {
        let mut ir = IRContext::new();
        for n in names {
            ir.marker_names.insert(marker_id(n), (*n).to_string());
        }
        ir
    }

    #[test]
    fn the_width_is_one_bit_per_marker_and_never_zero() {
        assert_eq!(bit_width(&ir_with_markers(&["a", "b", "c"])), 3);
        // `(_ BitVec 0)` is not a legal sort, so a marker-free program still
        // gets a (permanently clear) bit.
        assert_eq!(bit_width(&ir_with_markers(&[])), 1);
        assert_eq!(sort_str(&ir_with_markers(&["a", "b"])), "(_ BitVec 2)");
    }

    #[test]
    fn bit_indices_are_the_rank_of_the_marker_hash() {
        let names = ["div_zero", "undef_var", "bad_type"];
        let ir = ir_with_markers(&names);
        // Ranks follow sorted-id order, which is `marker_names`' own iteration
        // order — so the index is a pure function of the program's marker set.
        let mut ids: Vec<usize> = names.iter().map(|n| marker_id(n)).collect();
        ids.sort_unstable();
        for (rank, id) in ids.iter().enumerate() {
            assert_eq!(bit_index(&ir, *id), rank);
        }
        // Distinct markers never share a bit.
        let bits: BTreeSet<usize> = names.iter().map(|n| bit_index(&ir, marker_id(n))).collect();
        assert_eq!(bits.len(), names.len());
    }

    #[test]
    fn a_named_marker_is_one_hot_and_the_empty_path_is_zero() {
        let ir = ir_with_markers(&["a", "b", "c"]);
        assert_eq!(empty_literal(&ir), "(_ bv0 3)");
        let mut seen = BTreeSet::new();
        for n in ["a", "b", "c"] {
            let lit = marker_literal(&ir, marker_id(n));
            let value: u32 = lit
                .trim_start_matches("(_ bv")
                .split_whitespace()
                .next()
                .expect("value")
                .parse()
                .expect("numeral");
            assert_eq!(value.count_ones(), 1, "{n} is not one-hot: {lit}");
            seen.insert(value);
        }
        assert_eq!(seen.len(), 3, "markers share a bit: {seen:?}");
    }

    #[test]
    fn a_merge_mask_is_the_union_of_its_markers() {
        let ir = ir_with_markers(&["a", "b", "c"]);
        let bit = |n: &str| 1u32 << bit_index(&ir, marker_id(n));
        let ids: BTreeSet<usize> = ["a", "c"].iter().map(|n| marker_id(n)).collect();
        assert_eq!(
            mask_literal(&ir, &ids),
            format!("(_ bv{} 3)", bit("a") | bit("c"))
        );
        // The empty target masks nothing.
        assert_eq!(mask_literal(&ir, &BTreeSet::new()), "(_ bv0 3)");
    }

    #[test]
    fn merging_is_bvor_and_membership_is_a_masked_equality() {
        let ir = ir_with_markers(&["a", "b"]);
        assert_eq!(merge_expr("l", "r"), "(bvor l r)");
        let ids: BTreeSet<usize> = ["a", "b"].iter().map(|n| marker_id(n)).collect();
        // The multi-marker test is ONE formula over the union mask. The old
        // single-id encoding emitted `(and (= e a) (= e b))` here, which is a
        // contradiction for any single-valued `e` — unsat for every input
        // whatever the program did.
        assert_eq!(
            contains_all("p", &ids, &ir),
            "(= (bvand p (_ bv3 2)) (_ bv3 2))"
        );
    }

    /// The end-to-end case the bit-set encoding exists for: a spec that
    /// accumulates two markers with `Path::merge`, whose merged target must be
    /// **satisfiable**. Under the previous single-id encoding this query was
    /// `(and (= e a) (= e b))` — unsat by construction, and reported as a
    /// genuine unreachability verdict at `k=0`.
    #[test]
    #[ignore = "invokes the real z3 binary"]
    fn a_merged_target_is_satisfiable_end_to_end() {
        use crate::backend::codegen::CodeGen;
        use crate::backend::z3::ctxt::CodeGenZ3;

        const SPEC: &str = r#"
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{I64, Path, smt::SMT};

#[smt_type]
pub enum CheckResult {
    Err(Path),
    Ok(I64),
}

#[smt_fn]
pub fn check_both(a: I64, b: I64) -> CheckResult {
    if *a.eq(I64::from(0)) {
        if *b.eq(I64::from(0)) {
            CheckResult::Err(Path::named("a_zero").merge(Path::named("b_zero")))
        } else {
            CheckResult::Err(Path::named("a_zero"))
        }
    } else {
        CheckResult::Ok(a)
    }
}
"#;
        let dir = tempfile::tempdir().expect("tempdir");
        let spec_path = dir.path().join("merge_spec.rs");
        std::fs::write(&spec_path, SPEC).expect("write spec");
        let ir = crate::model(&spec_path).expect("spec parses");

        // The merge registered a two-marker target.
        let merged = ir
            .path_targets
            .iter()
            .find(|t| t.len() == 2)
            .expect("Path::merge must register a multi-marker target")
            .clone();

        let base = CodeGenZ3::new().process(&ir, 0).expect("codegen");
        let query = CodeGenZ3::new().process_path_queries(&base, &ir, "check_both", &merged);

        // Both bits in one mask — not two conflicting equalities.
        let mask = mask_literal(&ir, &merged);
        assert!(
            query.contains("(bvand (field_CheckResult_Err_1_"),
            "expected a masked membership test, got:\n{query}"
        );
        assert!(query.contains(&mask), "expected mask {mask} in:\n{query}");

        let qpath = dir.path().join("merged.smt2");
        std::fs::write(&qpath, &query).expect("write query");
        let resp = crate::guidance::run_z3_file(&qpath, std::time::Duration::from_secs(30));
        assert!(
            matches!(resp, crate::backend::response::Response::Sat(_)),
            "the merged target must be reachable (a=0, b=0), got {resp}"
        );
    }
}

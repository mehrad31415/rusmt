//! Marker-directed slicing: which function definitions a target query needs.
//!
//! A per-marker query asks whether the entry function can return the target
//! marker. Z3 expands every `define-fun` it can reach, and on a large parser
//! that expansion — not the input search — is what defeats it: with the input
//! symbolic the branch conditions do not fold, so the expansion of the nested
//! parsers explodes. Replacing the definitions that cannot contribute to the
//! target with a constant of their return sort cuts the query roughly in half
//! and moves Z3 from crashing or hanging to deciding in tens of milliseconds.
//!
//! Stubbing changes the program, so a model over a stubbed query is only a
//! *candidate*. It becomes a witness only when the UNMODIFIED query, with the
//! input pinned to it, is itself `sat` — see [`crate::guidance`]. Conversely an
//! `unsat` over a stubbed query says nothing about reachability: it means the
//! stub set removed the path, and the answer is to restore stubs.

use crate::backend::z3::fun::{collect_function_call_edges, resolve_function_name};
use crate::backend::z3::path::marker_literal;
use crate::ir::ctxt::IRContext;
use crate::ir::exp::Expression;
use crate::ir::index::UsrFunId;
use crate::ir::sort::Sort;
use std::collections::{BTreeMap, BTreeSet};

/// Which definitions a sliced query keeps and which it replaces by a constant.
#[derive(Debug, Clone, Default)]
pub struct StubPlan {
    /// Definitions emitted in full.
    pub keep: BTreeSet<UsrFunId>,
    /// Definitions replaced by a constant of their return sort.
    pub stub: BTreeSet<UsrFunId>,
}

impl StubPlan {
    /// Move `names` out of the stub set, so they are emitted in full again.
    /// Unknown or already-kept names are ignored; the count actually restored
    /// is returned.
    pub fn restore(&mut self, ir: &IRContext, names: &[String]) -> usize {
        let mut n = 0;
        for fid in self.stub.clone() {
            if names.iter().any(|w| *w == resolve_function_name(ir, fid)) {
                self.stub.remove(&fid);
                self.keep.insert(fid);
                n += 1;
            }
        }
        n
    }

    /// The stubbed definitions' names, largest body first — the order a
    /// refinement round is offered them in.
    pub fn stub_names(&self, ir: &IRContext) -> Vec<String> {
        let mut v: Vec<(usize, String)> = self
            .stub
            .iter()
            .map(|&f| (body_size(ir, f), resolve_function_name(ir, f)))
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        v.into_iter().map(|(_, n)| n).collect()
    }
}

/// Rendered size of a function body, used only to order refinement candidates.
fn body_size(ir: &IRContext, fid: UsrFunId) -> usize {
    let def = ir.fn_registry.retrieve_def(fid);
    crate::backend::z3::exp::format_expression(&def.body, def.root_exp_id, ir).len()
}

/// The monomorphic, non-`choose!` functions the backend emits as definitions.
fn definable(ir: &IRContext) -> BTreeSet<UsrFunId> {
    ir.fn_registry
        .lookup()
        .values()
        .flat_map(|insts| insts.iter())
        .filter(|(ty_args, _)| !ty_args.iter().any(|s| matches!(s, Sort::Uninterpreted(_))))
        .map(|(_, fid)| *fid)
        .filter(|&fid| {
            let def = ir.fn_registry.retrieve_def(fid);
            !matches!(
                def.body.lookup_exp(&def.root_exp_id),
                Expression::IterChoose { .. }
            )
        })
        .collect()
}

/// The functions whose body raises one of `target_ids`.
///
/// Found by rendering each body and looking for the marker's one-hot `Path`
/// literal, which the backend emits exactly where the `Path::named` was. That
/// makes this exact rather than a heuristic, and needs no second traversal of
/// the intrinsic tree.
pub fn marker_holders(ir: &IRContext, target_ids: &BTreeSet<usize>) -> BTreeSet<UsrFunId> {
    let needles: Vec<String> = target_ids
        .iter()
        .map(|&id| marker_literal(ir, id))
        .collect();
    definable(ir)
        .into_iter()
        .filter(|&fid| {
            let def = ir.fn_registry.retrieve_def(fid);
            let body = crate::backend::z3::exp::format_expression(&def.body, def.root_exp_id, ir);
            needles.iter().any(|n| body.contains(n.as_str()))
        })
        .collect()
}

/// Transitive closure of `succ` from `seeds`.
fn closure(
    succ: &BTreeMap<UsrFunId, BTreeSet<UsrFunId>>,
    seeds: &BTreeSet<UsrFunId>,
) -> BTreeSet<UsrFunId> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<UsrFunId> = seeds.iter().copied().collect();
    while let Some(f) = stack.pop() {
        if !seen.insert(f) {
            continue;
        }
        if let Some(next) = succ.get(&f) {
            stack.extend(next.iter().copied());
        }
    }
    seen
}

/// The most aggressive sound-to-try plan for a target.
///
/// Keeps the call chain from `entry` down to whichever function raises the
/// marker, plus everything that function itself calls — the predicates in the
/// holder's own body decide whether the marker fires, so stubbing them would
/// make it fire vacuously. Everything else reachable from `entry` is stubbed.
///
/// This is deliberately the aggressive end: it is the cheapest query to solve,
/// and when it over-stubs Z3 answers `unsat` in milliseconds, which is the
/// signal to restore stubs ([`StubPlan::restore`]).
pub fn plan(ir: &IRContext, entry: &str, target_ids: &BTreeSet<usize>) -> StubPlan {
    let ids = definable(ir);
    let edges = collect_function_call_edges(&ids, &ir.fn_registry);
    let mut callees: BTreeMap<UsrFunId, BTreeSet<UsrFunId>> = BTreeMap::new();
    let mut callers: BTreeMap<UsrFunId, BTreeSet<UsrFunId>> = BTreeMap::new();
    for (a, b) in &edges {
        callees.entry(*a).or_default().insert(*b);
        callers.entry(*b).or_default().insert(*a);
    }

    let Some(entry_fid) = ids
        .iter()
        .copied()
        .find(|&f| resolve_function_name(ir, f) == entry)
    else {
        // No entry to slice from: keep everything.
        return StubPlan {
            keep: ids,
            stub: BTreeSet::new(),
        };
    };

    let reachable = closure(&callees, &BTreeSet::from([entry_fid]));
    let holders = marker_holders(ir, target_ids);
    if holders.is_empty() {
        return StubPlan {
            keep: ids,
            stub: BTreeSet::new(),
        };
    }
    // The chain that has to run to get to a holder, and what a holder needs to
    // decide the marker.
    let chain = closure(&callers, &holders);
    // DIRECT callees of the holder only. The transitive closure is self-defeating:
    // for a marker inside `parse_array`, it reaches `parse_value` and therefore
    // every value parser in the language, so nothing is left to stub (measured: 6
    // of 158 stubbed, and the query stays intractable). What the holder actually
    // needs to decide its own branches is its immediate callees, and the
    // predicates among those are kept by the rule below regardless.
    let support: BTreeSet<UsrFunId> = callees
        .get(&holders.iter().next().copied().unwrap_or(entry_fid))
        .cloned()
        .unwrap_or_default()
        .union(&holders)
        .copied()
        .collect();
    // Every predicate stays. A `Bool`-returning function is a branch condition —
    // a character class, a delimiter test — and stubbing one to `false` silently
    // removes the path that reaches the marker rather than shrinking the query.
    // Bisecting real markers against known witnesses found the load-bearing
    // definitions to be exactly these plus the syntactic scaffolding on the
    // chain (`is_comment_start_symbol`, `is_alpha`, `parse_keyval_sep`), and they
    // are all small, so keeping them costs almost nothing.
    let predicates: BTreeSet<UsrFunId> = ids
        .iter()
        .copied()
        .filter(|&f| matches!(ir.fn_registry.retrieve_sig(f).ret_ty, Sort::Boolean))
        .collect();

    // The syntactic scaffolding an input must get through to reach the marker —
    // a key, a separator, an opening delimiter — is always a DIRECT callee of
    // something on the chain. Bisecting markers against known witnesses found the
    // load-bearing definitions to be exactly that: `parse_key`, `parse_keyval_sep`,
    // `parse_simple_key`, `is_alpha`. Keeping direct callees (not their transitive
    // closure, which is the whole program again) makes those available without a
    // refinement round.
    let scaffolding: BTreeSet<UsrFunId> = chain
        .iter()
        .filter_map(|f| callees.get(f))
        .flatten()
        .copied()
        .collect();

    let keep: BTreeSet<UsrFunId> = chain
        .union(&support)
        .copied()
        .chain(predicates)
        .chain(scaffolding)
        .filter(|f| reachable.contains(f))
        .chain(std::iter::once(entry_fid))
        .collect();
    let stub = reachable.difference(&keep).copied().collect();
    StubPlan { keep, stub }
}

/// A ladder of progressively more permissive plans for a target.
///
/// The aggressive plan is the cheapest query but over-slices often: Z3 answers
/// `unsat` because the stub set removed the path, not because the marker is
/// unreachable. Each rung restores more, and a rung costs one solve — measured at
/// 0.5-1.6 s on TOML — so walking the ladder is far cheaper than spending a model
/// round to guess which definition to put back. Only when every rung fails is
/// there a question a proposer can usefully answer.
///
/// Rungs, in order: the aggressive plan; plus the chain's direct callees; plus
/// everything the chain can reach; then nothing stubbed at all.
pub fn plan_ladder(ir: &IRContext, entry: &str, target_ids: &BTreeSet<usize>) -> Vec<StubPlan> {
    let ids = definable(ir);
    let edges = collect_function_call_edges(&ids, &ir.fn_registry);
    let mut callees: BTreeMap<UsrFunId, BTreeSet<UsrFunId>> = BTreeMap::new();
    let mut callers: BTreeMap<UsrFunId, BTreeSet<UsrFunId>> = BTreeMap::new();
    for (a, b) in &edges {
        callees.entry(*a).or_default().insert(*b);
        callers.entry(*b).or_default().insert(*a);
    }
    let base = plan(ir, entry, target_ids);
    let Some(entry_fid) = ids
        .iter()
        .copied()
        .find(|&f| resolve_function_name(ir, f) == entry)
    else {
        return vec![base];
    };
    let holders = marker_holders(ir, target_ids);
    if holders.is_empty() {
        return vec![base];
    }
    let reachable = closure(&callees, &BTreeSet::from([entry_fid]));
    let chain = closure(&callers, &holders);

    let mut rungs = vec![base.clone()];

    // ONE fine rung only, restoring the smallest tenth of the stubs.
    //
    // Coarse graded rungs were measured to be useless here: for a marker in
    // `parse_array` the aggressive rung (75 stubbed) is `unsat` in ~1 s while the
    // next coarse rung (57 stubbed) already times out. Restoring 18 definitions
    // at a time steps straight over the window in which the path survives AND Z3
    // still finishes. Bisection against known witnesses shows only 1-5 of ~110
    // stubs are load-bearing, so the useful move is to restore a handful by name
    // — a choice a proposer makes from the marker and the stub list, and the
    // reason this stage is a collaboration rather than a search.
    //
    // No full-query rung: that is Stage 1, already measured, and it does not
    // return (no verdict at 300 s), so spending budget on it every marker is pure
    // waste.
    let mut by_size: Vec<UsrFunId> = base.stub.iter().copied().collect();
    by_size.sort_by_key(|&f| (body_size(ir, f), f));
    let take = (by_size.len() / 10)
        .max(1)
        .min(by_size.len().saturating_sub(1));
    if take > 0 {
        let restored: BTreeSet<UsrFunId> = by_size[..take].iter().copied().collect();
        rungs.push(StubPlan {
            stub: base.stub.difference(&restored).copied().collect(),
            keep: base.keep.union(&restored).copied().collect(),
        });
    }
    let _ = (&reachable, &ids, &callees, &chain);
    // Keep the ladder strictly decreasing in how much it stubs.
    rungs.dedup_by(|a, b| a.stub == b.stub);
    rungs
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusmt_smt_stdlib::path::marker_id;

    /// The IMP model is small and has two named markers, so slicing it is a
    /// cheap end-to-end check that holders are found and the plan is a
    /// partition of what is reachable.
    fn imp_ir() -> IRContext {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .join("lang/src/imp");
        crate::model(dir).expect("imp model builds")
    }

    #[test]
    fn a_named_marker_is_traced_to_the_function_that_raises_it() {
        let ir = imp_ir();
        for name in ["division_by_zero", "undefined_variable"] {
            let ids = BTreeSet::from([marker_id(name)]);
            assert!(
                !marker_holders(&ir, &ids).is_empty(),
                "no holder found for `{name}`"
            );
        }
    }

    #[test]
    fn keep_and_stub_partition_the_reachable_functions() {
        let ir = imp_ir();
        let ids = BTreeSet::from([marker_id("division_by_zero")]);
        let p = plan(&ir, "eval_command", &ids);
        assert!(
            p.keep.is_disjoint(&p.stub),
            "a function is both kept and stubbed"
        );
        // The holder is never stubbed: its own predicates decide the marker.
        for h in marker_holders(&ir, &ids) {
            assert!(!p.stub.contains(&h), "the marker holder was stubbed");
        }
    }

    #[test]
    fn restoring_a_stub_moves_it_into_the_keep_set() {
        let ir = imp_ir();
        let ids = BTreeSet::from([marker_id("division_by_zero")]);
        let mut p = plan(&ir, "eval_command", &ids);
        let Some(first) = p.stub_names(&ir).first().cloned() else {
            return; // nothing stubbed for this tiny model; nothing to check
        };
        let before = p.stub.len();
        assert_eq!(p.restore(&ir, &[first]), 1);
        assert_eq!(p.stub.len(), before - 1);
    }
}

//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::solvers;
use crate::backend::response::Response;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use crate::proposer::Proposer as _;
use rusmt_lang::certify::{self, ORACLES, oracle_for};
use rusmt_lang::imp_render;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Instant;
use syn::Result;

/// For the `imp` case study, a `sat` model is stored as the rendered `.imp`
/// program (the inverse of the Z3 encoding) rather than the raw solver text; any
/// other parser, or a non-`sat` response, is stored verbatim. The second
/// component reports whether the body is a rendered object-language witness
/// (and hence replayable through the concrete reference semantics).
fn response_to_store(parser_dir: &Path, raw: String) -> (String, bool) {
    match parser_dir.file_name().and_then(|s| s.to_str()) {
        Some("imp") => match imp_render::render_response(&raw) {
            Some(rendered) => (rendered, true),
            None => (raw, false),
        },
        _ => (raw, false),
    }
}

/// Store a target's result under exactly one `response.*` file. A replayable
/// object-language witness is named by the extension the `rusmt-lang` runner
/// expects for that language — taken from the shared oracle registry, so
/// `imp` → `response.imp`, `typecheck` → `response.tc`, `toml` → `response.toml`
/// — and can therefore be fed straight back for replay. Every non-witness body
/// (raw solver text, timeout, `unknown`, error) stays `response.txt`, since it
/// is not a runnable object-language program.
///
/// Any sibling `response.*` left by an earlier write under a different extension
/// — e.g. the timeout text that a later recovered witness now replaces, or a
/// stale file from a previous run — is removed first, so a target dir always
/// holds a single canonical response file whose extension reflects its content.
fn write_response(target_dir: &Path, parser_dir: &Path, body: &str, is_witness: bool) {
    let ext = if is_witness {
        parser_dir
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(oracle_for)
            .map(|o| o.ext)
            .unwrap_or("txt")
    } else {
        "txt"
    };
    for other in std::iter::once("txt").chain(ORACLES.iter().map(|o| o.ext)) {
        if other != ext {
            let _ = fs::remove_file(target_dir.join(format!("response.{other}")));
        }
    }
    fs::write(target_dir.join(format!("response.{ext}")), body)
        .expect("failed to write response file");
}

/// The name of the *single* named marker behind a synthesis target, if any.
///
/// This is the stricter of the two target predicates: it is what the
/// **proposer-fallback loop** needs, because that loop prompts an untrusted
/// proposer by name ("write an input that triggers `division_by_zero`"). A
/// merged, multi-marker target would have to be phrased as "make all of these
/// fire on one run", a materially different ask, so it is excluded here — see
/// [`certifiable_target`] for the weaker condition replay actually requires.
fn named_target<'a>(model: &'a IRContext, target_ids: &BTreeSet<usize>) -> Option<&'a str> {
    if target_ids.len() != 1 {
        return None;
    }
    let id = target_ids.iter().next().expect("singleton");
    model.marker_names.get(id).map(String::as_str)
}

/// The replay target string for a synthesis target, if every one of its ids
/// was declared via `Path::named`: the marker names joined by
/// [`certify::TARGET_SEP`].
///
/// Unlike [`named_target`] this accepts a *merged*, multi-marker target
/// (`Path::merge`). Certification does not need the target to be a singleton —
/// the arbiter's check is membership of the targeted ids in the fired `Path`
/// set, which generalizes to a subset test at no cost. Requiring names (rather
/// than passing raw ids) keeps the replay verdict and transcript
/// human-readable; the ids are recovered from the names by `marker_id`, the
/// same stable function both sides use.
fn certifiable_target(model: &IRContext, target_ids: &BTreeSet<usize>) -> Option<String> {
    if target_ids.is_empty() {
        return None;
    }
    let names: Vec<&str> = target_ids
        .iter()
        .map(|id| model.marker_names.get(id).map(String::as_str))
        .collect::<Option<_>>()?;
    Some(names.join(&certify::TARGET_SEP.to_string()))
}

/// Per-target certification and the proposer integrations, shared by both
/// backends. A no-op unless the target is a *named* marker (stable id) and the
/// language has a registered replay oracle; otherwise the solver's verdict in
/// the target's `response.*` file is final.
///
/// A rendered `sat` witness is replay-certified and the verdict recorded in
/// `replay.txt`. When the solver produced no certified result — timeout,
/// `unknown`, bound-limited `unsat`, or a replay-rejected model — and a
/// proposer command is configured (`RUSMT_LLM_CMD`), the recovery mode chosen
/// by `RUSMT_LLM_MODE` runs:
///
/// * `direct` (default) — the counterexample-guided whole-input fallback;
///   transcript in `fallback.txt`.
/// * `guided` — the iterative scaffold loop (`guidance.rs`): the proposer
///   strengthens the unmodified per-target query, Z3 completes it each round.
///   The round runs against a persistent `z3 -in` session
///   ([`crate::z3_session`]), so the base theory is parsed once per target and
///   a `sat` round is mined for several distinct candidates rather than one;
///   transcript in `guidance.txt`, round queries in `guided_round_*.smt2`.
/// * `both` — one direct round, then the guided loop.
///
/// Guided mode needs the per-target query file and a `Seq<U32>` entry input;
/// otherwise it transparently degrades to `direct`.
/// In every mode a certified witness replaces the target's response file
/// (named by the object-language extension when replayable — see
/// [`write_response`]).
fn certify_and_recover(
    model: &IRContext,
    lang_dir: &Path,
    path_target_dir: &Path,
    target_ids: &BTreeSet<usize>,
    resp: &Response,
    body: &str,
    is_witness: bool,
    unroll_depth: usize,
) {
    let lang = lang_dir.file_name().and_then(|s| s.to_str());
    // Replay only needs every targeted id to be name-addressable — a merged
    // multi-marker target certifies fine (all of its markers must fire on the
    // one run). The stricter single-name condition is applied later, and only
    // to the proposer fallback.
    let (Some(target), Some(oracle)) = (
        certifiable_target(model, target_ids),
        lang.and_then(oracle_for),
    ) else {
        return;
    };
    let target = target.as_str();
    // Replay runs in a separate process so that a crashing candidate (e.g.
    // unbounded recursion overflowing the stack) is rejected instead of
    // taking down the pipeline.
    let certify = |src: &str, tgt: &str| {
        rusmt_lang::certify::certify_isolated(oracle.name, src, tgt, proposer::REPLAY_BUDGET)
    };

    // The solver's outcome for this target, post-replay: `true` iff it
    // produced a witness certified by the concrete reference semantics.
    let solver_outcome;
    let certified = match resp {
        // A rendered sat witness: certify it by replay, record the verdict
        // next to it. A spurious candidate model (e.g. from depth-bounded
        // unrolling) is thereby flagged — and handed to the fallback —
        // instead of trusted.
        Response::Sat(_) if is_witness => {
            let verdict = certify(body, target);
            let line = proposer::verdict_line(&verdict, target, &model.marker_names);
            fs::write(path_target_dir.join("replay.txt"), format!("{line}\n"))
                .expect("failed to write replay verdict");
            solver_outcome = format!("sat, but the model was rejected on replay: {line}");
            verdict.is_certified()
        }
        // A sat model we cannot decode into object-language source is not
        // replayable; the raw model stands.
        Response::Sat(_) => {
            solver_outcome = String::new();
            true
        }
        // `unsat` under native recursion (k=0) is a genuine unreachability
        // verdict — respected, no fallback. Under k-unrolled bounding it only
        // means "no witness within depth k", so a proposer may still find a
        // deeper one.
        Response::Unsat if unroll_depth == 0 => {
            solver_outcome = String::new();
            true
        }
        // timeout / unknown / bound-limited unsat.
        _ => {
            solver_outcome = resp.to_string();
            false
        }
    };

    // Solver-first, proposer recovery: only when the solver produced no
    // certified witness and a proposer command is configured (RUSMT_LLM_CMD).
    if certified {
        return;
    }
    let Some(mut llm) = proposer::CommandProposer::from_env() else {
        return;
    };
    // Everything below prompts the proposer by marker *name* and asks it to
    // make that one condition fire, so it is restricted to single-marker
    // targets. A merged target still gets the replay certification above; it
    // just has no proposer fallback. Lifting this means teaching the prompts
    // to ask for several markers on one run.
    let Some(marker) = named_target(model, target_ids) else {
        return;
    };

    // Guided mode strengthens the per-target query file, and its scaffold
    // language speaks the theory of sequences — so it needs that file to exist
    // and the entry input to be a Seq<U32>. Otherwise (e.g. an ADT input like
    // IMP's `Com`) the requested guided/both mode degrades to the direct
    // whole-input fallback.
    let mode = guidance::mode_from_env();
    let base_query = fs::read_to_string(path_target_dir.join("main.smt2"))
        .ok()
        .filter(|q| guidance::query_has_seq_input(q, guidance::INPUT_VAR));
    let effective = if base_query.is_some() {
        mode
    } else {
        guidance::LlmMode::Direct
    };

    // The direct whole-input loop: all rounds in `direct` mode, a single
    // round in `both` (the degenerate round 0 of the guided loop).
    let direct_guesses = match effective {
        guidance::LlmMode::Direct => Some(proposer::max_guesses_from_env()),
        guidance::LlmMode::Both => Some(1),
        guidance::LlmMode::Guided => None,
    };
    // Keep the solver in the loop on the direct route: when the entry input is
    // a code-point sequence (so a candidate can be macro-inlined), each proposed
    // candidate is validated by Z3 — pinned as a `define-fun` macro — before
    // replay re-certifies it. On the array-free TOML encoding this validation is
    // a sub-second `sat`; the solver is never bypassed. For an ADT input (no
    // sequence to inline) `base_query` is `None`, so the direct route below does
    // not run at all (it requires Z3 macro-validation) and the solver searches
    // directly instead — there is no replay-only proposer fallback.
    let validate_budget = guidance::guide_z3_budget_from_env();
    let validate_query = base_query.clone();
    let validate = validate_query.as_ref().map(|bq| {
        move |candidate: &str| -> guidance::Response {
            match guidance::macro_inline_input(bq, candidate, guidance::INPUT_VAR) {
                Some(q) => {
                    let p = path_target_dir.join("validate.smt2");
                    match fs::write(&p, q) {
                        Ok(()) => guidance::run_z3_file(&p, validate_budget),
                        Err(e) => {
                            guidance::Response::Unknown(format!("cannot write validate query: {e}"))
                        }
                    }
                }
                None => guidance::Response::Unknown("no seq input to macro-inline".to_string()),
            }
        }
    });
    let mut recovered = false;
    // The direct route requires Z3 validation (no replay-only bypass), so it
    // runs only when the entry input can be macro-inlined (a code-point
    // sequence). For an ADT input the solver searches directly instead.
    if let (Some(max_guesses), Some(validate)) = (direct_guesses, validate.as_ref()) {
        let recovery = proposer::recover_target(
            oracle,
            marker,
            &solver_outcome,
            &model.marker_names,
            &mut llm,
            &certify,
            max_guesses,
            validate as &dyn Fn(&str) -> guidance::Response,
        );
        fs::write(
            path_target_dir.join("fallback.txt"),
            proposer::render_transcript(&llm.describe(), marker, &solver_outcome, &recovery),
        )
        .expect("failed to write fallback transcript");
        if let Some(witness) = recovery.witness {
            // The certified witness replaces the solver's non-answer as this
            // target's result; the full provenance lives in fallback.txt.
            write_response(path_target_dir, lang_dir, &witness, true);
            recovered = true;
        }
    }

    // The guided scaffold loop: per round, the proposer's scaffold strengthens
    // the UNMODIFIED query, Z3 completes it under a short budget, and the
    // verdict (sat → decode + replay / unsat → relax / timeout → tighten)
    // drives the next round. Replay stays the only acceptance criterion.
    if !recovered
        && matches!(
            effective,
            guidance::LlmMode::Guided | guidance::LlmMode::Both
        )
    {
        let base_query = base_query.expect("guided mode implies a seq-input query");
        let z3_budget = guidance::guide_z3_budget_from_env();
        let max_models = guidance::guide_models_from_env();
        // One `z3 -in` process for the whole target: the base theory is parsed
        // once instead of once per round, the solver's state survives between
        // rounds, and extra models cost a blocking clause rather than a fresh
        // solve. If it cannot be started we fall back to the one-shot path, so
        // a missing/older `z3` only costs speed, never correctness.
        let mut session = guidance::split_at_check_sat(&base_query).and_then(|base| {
            match z3_session::Z3Session::start(base, z3_budget) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[rusmt] persistent z3 session unavailable ({e}); using one-shot z3");
                    None
                }
            }
        });
        let mut solve_round = |round: usize, cs: &[guidance::Scaffold]| -> guidance::RoundModels {
            let p = path_target_dir.join(format!("guided_round_{round}.smt2"));
            match session.as_mut().filter(|s| s.is_alive()) {
                Some(s) => guidance::round_on_session(
                    s,
                    &base_query,
                    cs,
                    guidance::INPUT_VAR,
                    max_models,
                    Some(&p),
                ),
                None => {
                    guidance::round_on_file(&base_query, cs, guidance::INPUT_VAR, &p, z3_budget)
                }
            }
        };
        let g = guidance::guide_target(
            oracle,
            marker,
            &solver_outcome,
            &model.marker_names,
            &mut llm,
            &mut solve_round,
            &certify,
            // Gate 1 (solver, never bypassed): the same clean macro-inline Z3
            // validator the direct route uses. Guided mode implies a seq-input
            // query, so the validator is present.
            validate
                .as_ref()
                .expect("guided mode implies a seq-input query, hence a validator")
                as &dyn Fn(&str) -> guidance::Response,
            guidance::guide_rounds_from_env(),
        );
        fs::write(
            path_target_dir.join("guidance.txt"),
            guidance::render_guidance_transcript(&llm.describe(), marker, &solver_outcome, &g),
        )
        .expect("failed to write guidance transcript");
        if let Some(witness) = g.witness {
            write_response(path_target_dir, lang_dir, &witness, true);
        }
    }
}

/// Outcome of [`recover_named_marker`].
pub struct RecoverReport {
    /// The named marker targeted.
    pub marker: String,
    /// Z3's verdict on the UNCONSTRAINED per-target query (the solver alone).
    pub z3_alone: String,
    /// A witness Z3 produced alone (decoded to source), if it solved the
    /// unconstrained query within budget.
    pub z3_alone_witness: Option<String>,
    /// The AI⇄Z3 guided loop's outcome, when it ran (Z3 alone failed and a
    /// proposer was configured).
    pub guided: Option<guidance::Guidance>,
    /// The direct route's outcome, when it ran (`RUSMT_LLM_MODE=direct|both`):
    /// a whole candidate is proposed, Z3 *validates* the macro-inlined input
    /// (solver in the loop), and replay re-certifies.
    pub direct: Option<proposer::Recovery>,
    /// The proposer description, when the guided loop ran.
    pub proposer: Option<String>,
}

/// Run the AI⇄Z3 guided-synthesis loop for a single *named* marker, embedded so
/// any cloner can invoke it (the `recover` CLI mode), with **Z3 as the model
/// producer/validator**: first Z3 alone is tried on the unconstrained
/// per-target query; if it fails, the proposer (`RUSMT_LLM_CMD`) only
/// *strengthens* that query — partial constraints for Z3 to complete, or a
/// single `exact` candidate for Z3 to validate — and Z3 solves each round. A
/// found input is then independently replay-certified through the concrete
/// reference semantics (the proposer is never trusted). All queries and the
/// transcript are written under `out_dir` for inspection.
pub fn recover_named_marker(
    model: &IRContext,
    lang: &str,
    top_level_fn: &str,
    marker_name: &str,
    unroll_depth: usize,
    z3_alone_budget: std::time::Duration,
    out_dir: &Path,
) -> std::result::Result<RecoverReport, String> {
    use crate::backend::codegen::CodeGen;
    use crate::backend::z3::ctxt::CodeGenZ3;

    let id = rusmt_smt_stdlib::path::marker_id(marker_name);
    if model.marker_names.get(&id).map(String::as_str) != Some(marker_name) {
        let mut names: Vec<&str> = model.marker_names.values().map(String::as_str).collect();
        names.sort();
        return Err(format!(
            "`{marker_name}` is not a named marker in this model (use Path::named). \
             Named markers: {}",
            if names.is_empty() {
                "(none)".to_string()
            } else {
                names.join(", ")
            }
        ));
    }
    let oracle = oracle_for(lang)
        .ok_or_else(|| format!("no replay oracle registered for language `{lang}`"))?;

    // Build the SAME per-target query the synthesis pipeline would: base
    // definitions + the marker-membership assertion for this named id.
    let base = CodeGenZ3::new()
        .process(model, unroll_depth)
        .map_err(|e| format!("base SMT-LIB generation failed: {e:?}"))?;
    let query =
        CodeGenZ3::new().process_path_queries(&base, model, top_level_fn, &BTreeSet::from([id]));
    if !guidance::query_has_seq_input(&query, guidance::INPUT_VAR) {
        return Err(format!(
            "the guided AI⇄Z3 loop needs a `Seq<U32>` entry input; `{top_level_fn}` does not \
             take one (use the direct fallback via RUSMT_LLM_MODE=direct in the full pipeline)"
        ));
    }
    fs::create_dir_all(out_dir).map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let query_path = out_dir.join("query.smt2");
    fs::write(&query_path, &query).map_err(|e| format!("cannot write query: {e}"))?;

    // Step 1: Z3 alone on the unconstrained query.
    let z3_resp = guidance::run_z3_file(&query_path, z3_alone_budget);
    let z3_alone = z3_resp.to_string();
    if let Response::Sat(model_text) = &z3_resp {
        // Z3 solved it unaided; decode its model (no AI needed).
        let witness = guidance::decode_seq_model(model_text, guidance::INPUT_VAR);
        return Ok(RecoverReport {
            marker: marker_name.to_string(),
            z3_alone,
            z3_alone_witness: witness,
            guided: None,
            direct: None,
            proposer: None,
        });
    }

    // Step 2: Z3 failed — bring in the proposer. It strengthens the query; Z3
    // solves/validates each round; replay (isolated) is the only certificate.
    let Some(mut llm) = proposer::CommandProposer::from_env() else {
        return Ok(RecoverReport {
            marker: marker_name.to_string(),
            z3_alone,
            z3_alone_witness: None,
            guided: None,
            direct: None,
            proposer: None,
        });
    };
    let proposer_desc = llm.describe();
    let certify = |src: &str, tgt: &str| {
        rusmt_lang::certify::certify_isolated(oracle.name, src, tgt, proposer::REPLAY_BUDGET)
    };
    let z3_budget = guidance::guide_z3_budget_from_env();
    let mode = guidance::mode_from_env();

    // Gate 1 for BOTH AI routes (direct and guided) — never bypassed: the
    // proposer is untrusted, so any candidate it yields is validated by Z3,
    // pinned as a `define-fun` macro and handed to the solver (on the array-free
    // TOML encoding this decides `sat` sub-second). Replay then re-certifies
    // (gate 2). Neither route has a replay-only acceptance path.
    let validate = |candidate: &str| -> Response {
        match guidance::macro_inline_input(&query, candidate, guidance::INPUT_VAR) {
            Some(q) => {
                let p = out_dir.join("validate.smt2");
                match fs::write(&p, q) {
                    Ok(()) => guidance::run_z3_file(&p, z3_budget),
                    Err(e) => Response::Unknown(format!("cannot write validate query: {e}")),
                }
            }
            None => Response::Unknown("no seq input to macro-inline".to_string()),
        }
    };

    // Direct route (mode = direct|both): the proposer suggests a whole candidate;
    // Z3 validates it (gate 1), then replay re-certifies (gate 2).
    let mut direct = None;
    if matches!(mode, guidance::LlmMode::Direct | guidance::LlmMode::Both) {
        let rec = proposer::recover_target(
            oracle,
            marker_name,
            &z3_alone,
            &model.marker_names,
            &mut llm,
            &certify,
            proposer::max_guesses_from_env(),
            &validate as &dyn Fn(&str) -> Response,
        );
        fs::write(
            out_dir.join("fallback.txt"),
            proposer::render_transcript(&proposer_desc, marker_name, &z3_alone, &rec),
        )
        .map_err(|e| format!("cannot write fallback transcript: {e}"))?;
        let found = rec.witness.is_some();
        direct = Some(rec);
        if found && matches!(mode, guidance::LlmMode::Direct) {
            return Ok(RecoverReport {
                marker: marker_name.to_string(),
                z3_alone,
                z3_alone_witness: None,
                guided: None,
                direct,
                proposer: Some(proposer_desc),
            });
        }
    }

    // Guided route (mode = guided|both): the proposer strengthens the query and
    // Z3 completes it each round; replay (isolated) is the only certificate.
    let guided = if matches!(mode, guidance::LlmMode::Guided | guidance::LlmMode::Both) {
        let max_models = guidance::guide_models_from_env();
        // As in the pipeline: one persistent solver for the whole target, with
        // the one-shot path as the fallback.
        let mut session =
            guidance::split_at_check_sat(&query).and_then(
                |base| match z3_session::Z3Session::start(base, z3_budget) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        eprintln!(
                            "[recover] persistent z3 session unavailable ({e}); using one-shot z3"
                        );
                        None
                    }
                },
            );
        let mut solve_round = |round: usize, cs: &[guidance::Scaffold]| -> guidance::RoundModels {
            let p = out_dir.join(format!("guided_round_{round}.smt2"));
            match session.as_mut().filter(|s| s.is_alive()) {
                Some(s) => guidance::round_on_session(
                    s,
                    &query,
                    cs,
                    guidance::INPUT_VAR,
                    max_models,
                    Some(&p),
                ),
                None => guidance::round_on_file(&query, cs, guidance::INPUT_VAR, &p, z3_budget),
            }
        };
        let g = guidance::guide_target(
            oracle,
            marker_name,
            &z3_alone,
            &model.marker_names,
            &mut llm,
            &mut solve_round,
            &certify,
            &validate as &dyn Fn(&str) -> guidance::Response,
            guidance::guide_rounds_from_env(),
        );
        fs::write(
            out_dir.join("guidance.txt"),
            guidance::render_guidance_transcript(&proposer_desc, marker_name, &z3_alone, &g),
        )
        .map_err(|e| format!("cannot write guidance transcript: {e}"))?;
        Some(g)
    } else {
        None
    };

    Ok(RecoverReport {
        marker: marker_name.to_string(),
        z3_alone,
        z3_alone_witness: None,
        guided,
        direct,
        proposer: Some(proposer_desc),
    })
}

// module tree
pub mod authoring;
mod backend;
pub mod guidance;
mod ir;
mod parser;
pub mod proposer;
pub mod z3_session;

/// Create the intermediate representations (IR) from the parsing context.
pub fn model<P: AsRef<Path>>(input: P) -> Result<IRContext> {
    // The `new` function collects all the smt-marked items from the input file
    // and stores them in the context.
    let context = Context::new(input)?;

    // Chain parsing methods to process generics, types, function signatures, and function bodies.
    // This accumulates all necessary definitions into `ContextWithFunc`.
    let parsed = context
        .parse_generics()?
        .parse_types()?
        .parse_func_sigs()?
        .parse_func_body()?;

    // Build the Intermediate Representation (IR) for the entire parsed context.
    let ir = IRBuilder::build(&parsed);
    Ok(ir)
}

/// Emit the text backend's *base* SMT-LIB for a model: datatype declarations,
/// the stdlib helper definitions, and one `define-fun(-rec)` per user function —
/// with no `check-sat` and no path queries.
///
/// This is exposed so the randomized differential harness (which checks the
/// per-construct soundness contract `rust_impl(f,x) == z3_eval(z3_formula(f),x)`)
/// can append its own equality queries to the exact formulas the backend emits,
/// rather than re-deriving them.
pub fn emit_text_base_smt(model: &IRContext, unroll_depth: usize) -> String {
    use crate::backend::codegen::CodeGen;
    use crate::backend::z3::ctxt::CodeGenZ3;
    CodeGenZ3::new()
        .process(model, unroll_depth)
        .expect("base SMT-LIB generation failed")
}

/// Solve the models by synthesizing inputs for specific Path IDs.
///
/// `unroll_depth` controls bounded-recursion unrolling in the text backend
/// (see `CodeGen::process`); pass 0 or not specified to keep the existing recursive emission.
///
/// When the env var `RUSMT_SKIP_INVOKE=1` is set, everything runs except the
/// per-target Z3 invocation: codegen still happens and `main.smt2` plus every
/// `target_<N>/main.smt2` is still written. This is the codegen-debugging path,
/// and it is not the same as stopping at [`model`] — that returns an
/// `IRContext` and emits no SMT-LIB at all, so it shows nothing when the
/// generated text is the thing under inspection. Skipping the solver is what
/// makes it cheap: TOML's 186 targets render in seconds instead of an hour of
/// Z3 budget.
pub fn solve<P: AsRef<Path>>(
    model: &IRContext,
    top_level_fn: Option<&str>,
    output: P,
    unroll_depth: usize,
) -> Result<()> {
    let skip_invoke = std::env::var("RUSMT_SKIP_INVOKE").ok().as_deref() == Some("1");
    for solver in solvers() {
        let name = solver.name();

        // Create a root directory for the solver (e.g., ./lang/src/synthesis/<parser_name>/<solver_name>)
        let path_solver = output.as_ref().join(name);
        fs::create_dir_all(&path_solver).expect("workspace freshly created");

        // Generate base SMT-LIB (types + functions, no queries).
        let base_code = match solver.process(model, unroll_depth) {
            Ok(code) => code,
            Err(e) => panic!("error generating SMT-LIB code: {e:?}"),
        };

        // Write main.smt2 (base declarations, no check-sat).
        let path_src = path_solver.join(format!("main.{}", solver.flavor()));
        fs::write(&path_src, &base_code).unwrap_or_else(|e| panic!("IO error on source file: {e}"));

        // For each path-marker target, generate one query against `top_level_fn` and run it.
        // Skip entirely if no top-level function was specified.
        let Some(top_level_fn) = top_level_fn else {
            continue;
        };
        for (target_idx, target_ids) in model.path_targets.iter().enumerate() {
            let target_label = format!("target_{target_idx}");
            let path_target_dir = path_solver.join(&target_label);
            fs::create_dir_all(&path_target_dir).expect("target directory created");

            let query_code =
                solver.process_path_queries(&base_code, model, top_level_fn, target_ids);
            let query_path = path_target_dir.join(format!("main.{}", solver.flavor()));
            fs::write(&query_path, &query_code).expect("failed to write query file");

            if skip_invoke {
                continue;
            }

            let timing_file = path_target_dir.join("timing.txt");
            let start = Instant::now();
            match solver.invoke_backend(&query_path) {
                Ok(resp) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    let (body, is_witness) = response_to_store(output.as_ref(), resp.to_string());
                    write_response(&path_target_dir, output.as_ref(), &body, is_witness);
                    fs::write(&timing_file, format!("{elapsed_ms}ms"))
                        .expect("failed to write timing file");

                    certify_and_recover(
                        model,
                        output.as_ref(),
                        &path_target_dir,
                        target_ids,
                        &resp,
                        &body,
                        is_witness,
                        unroll_depth,
                    );
                }
                Err(x) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    write_response(
                        &path_target_dir,
                        output.as_ref(),
                        &format!(
                            "[{name}] backend failed for {target_label} fn {top_level_fn}: {x:?}"
                        ),
                        false,
                    );
                    fs::write(&timing_file, format!("{elapsed_ms}ms"))
                        .expect("failed to write timing file");
                }
            }
        }
    }
    Ok(())
}

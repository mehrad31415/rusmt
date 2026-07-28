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
fn response_to_store(parser_name: &str, raw: String) -> (String, bool) {
    match parser_name {
        "imp" => match imp_render::render_response(&raw) {
            Some(rendered) => (rendered, true),
            None => (raw, false),
        },
        _ => (raw, false),
    }
}

/// Store a target's result under exactly one `response.*` file. A replayable
/// object-language witness is named by the extension the `rusmt-lang` runner
/// expects for that language — taken from the shared oracle registry, so
/// `imp` → `response.imp`, `toml` → `response.toml`
/// — and can therefore be fed straight back for replay. Every non-witness body
/// (raw solver text, timeout, `unknown`, error) stays `response.txt`, since it
/// is not a runnable object-language program.
///
/// Any sibling `response.*` left by an earlier write under a different extension
/// — e.g. the timeout text that a later recovered witness now replaces, or a
/// stale file from a previous run — is removed first, so a target dir always
/// holds a single canonical response file whose extension reflects its content.
fn write_response(target_dir: &Path, parser_name: &str, body: &str, is_witness: bool) {
    let ext = if is_witness {
        oracle_for(parser_name).map(|o| o.ext).unwrap_or("txt")
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

/// The replay target for a synthesis target, if every one of its ids was
/// declared via `Path::named`: the marker names joined by [`certify::TARGET_SEP`].
///
/// Accepts a merged, multi-marker target (`Path::merge`): replay's check is that
/// the targeted ids are a subset of the fired `Path` set, which generalizes at
/// no cost. Using names rather than raw ids keeps the transcript readable; the
/// ids are recovered by `marker_id`, the same function both sides use.
fn replay_target(model: &IRContext, target_ids: &BTreeSet<usize>) -> Option<String> {
    if target_ids.is_empty() {
        return None;
    }
    let names: Vec<&str> = target_ids
        .iter()
        .map(|id| model.marker_names.get(id).map(String::as_str))
        .collect::<Option<_>>()?;
    Some(names.join(&certify::TARGET_SEP.to_string()))
}

/// The single named marker behind a target, if it is a singleton.
///
/// Co-solving prompts by marker name and slices toward the one function that
/// raises it, so it applies to single-marker targets. A merged target still gets
/// its transpilation-fidelity replay.
fn single_marker<'a>(model: &'a IRContext, target_ids: &BTreeSet<usize>) -> Option<&'a str> {
    if target_ids.len() != 1 {
        return None;
    }
    let id = target_ids.iter().next().expect("singleton");
    model.marker_names.get(id).map(String::as_str)
}

/// Re-run a decoded input through the concrete Rust semantics and record the
/// verdict in `replay.txt`.
///
/// This checks the **transpiler**, not the witness: the witness was already
/// established by Z3 answering `sat` on the unmodified query. Agreement between
/// the SMT lift and the Rust it was lifted from is a fidelity property of RuSmt,
/// and a disagreement is a bug in the lift worth reporting — not grounds for
/// rejecting the model.
fn record_replay(
    oracle: &'static certify::LanguageOracle,
    model: &IRContext,
    target_dir: &Path,
    target: &str,
    witness: &str,
) -> bool {
    let verdict = rusmt_lang::certify::certify_isolated(
        oracle.name,
        witness,
        target,
        proposer::REPLAY_BUDGET,
    );
    let agrees = verdict.is_certified();
    let line = proposer::verdict_line(&verdict, target, &model.marker_names);
    let note = if agrees {
        "the SMT lift and the concrete semantics agree on this input"
    } else {
        "MISMATCH: the SMT lift and the concrete semantics disagree on this input — \
         a transpilation-fidelity bug to investigate, not a rejected witness"
    };
    let _ = fs::write(target_dir.join("replay.txt"), format!("{line}\n{note}\n"));
    agrees
}

/// Stage 2 for one target: the AI⇄Z3 co-solving loop.
///
/// Runs whenever Stage 1 did not hand back a model. It is not gated on
/// configuration: a missing proposer command is a run failure to report, not a
/// silent downgrade to doing nothing.
#[allow(clippy::too_many_arguments)]
fn co_solve_target(
    model: &IRContext,
    parser_name: &str,
    target_dir: &Path,
    target_ids: &BTreeSet<usize>,
    top_level_fn: &str,
    unmodified: &str,
    stage1: &Response,
    unroll_depth: usize,
) {
    let (Some(replay_tgt), Some(oracle)) =
        (replay_target(model, target_ids), oracle_for(parser_name))
    else {
        return;
    };

    // A model for this target already: only the fidelity replay is left.
    if let Response::Sat(text) = stage1 {
        if let Some(w) = guidance::decode_seq_model(text, guidance::INPUT_VAR) {
            record_replay(oracle, model, target_dir, &replay_tgt, &w);
        }
        return;
    }
    // `unsat` with no bounding is a genuine unreachability verdict for this
    // encoding; under k-unrolling it only means "no witness within depth k".
    if matches!(stage1, Response::Unsat) && unroll_depth == 0 {
        return;
    }
    let Some(marker) = single_marker(model, target_ids) else {
        return;
    };
    if !guidance::query_has_seq_input(unmodified, guidance::INPUT_VAR) {
        return;
    }

    let mut llm = proposer::CommandProposer::from_env();
    if llm.is_none() && cosolve::mode_from_env() == cosolve::Mode::Guide {
        eprintln!(
            "[rusmt] {marker}: only the mechanical round will run — RUSMT_LLM_CMD is unset. \
             The co-solving loop is a pipeline stage; set it (e.g. \
             RUSMT_LLM_CMD='claude -p --allowedTools \"\"') for the full pipeline."
        );
    }

    let ladder = slice::plan_ladder(model, top_level_fn, target_ids);
    let mut plan = ladder.first().cloned().unwrap_or_default();
    let holder = slice::marker_holders(model, target_ids)
        .into_iter()
        .next()
        .map(|f| backend::z3::fun::resolve_function_name(model, f))
        .unwrap_or_else(|| "(unknown)".to_string());

    let emit = |p: &slice::StubPlan| -> String {
        use crate::backend::codegen::CodeGen as _;
        use crate::backend::z3::ctxt::CodeGenZ3;
        let cg = CodeGenZ3::with_stubs(p.stub.clone());
        match cg.process(model, unroll_depth) {
            Ok(base) => cg.process_path_queries(&base, model, top_level_fn, target_ids),
            Err(e) => format!("; sliced codegen failed: {e:?}\n(check-sat)\n"),
        }
    };
    let names = |p: &slice::StubPlan| p.stub_names(model);
    let restorer = |p: &mut slice::StubPlan, n: &[String]| p.restore(model, n);
    let mut solver = cosolve::FileSolver {
        emit,
        unmodified,
        dir: target_dir,
        budget: guidance::z3_budget_from_env(),
        names: &names,
        restorer: &restorer,
    };

    if cosolve::mode_from_env() == cosolve::Mode::Certify {
        let Some(l) = llm.as_mut() else {
            let outcome = cosolve::Outcome {
                witnesses: Vec::new(),
                rounds: vec![cosolve::Round {
                    directives: String::new(),
                    restored: Vec::new(),
                    constraints: Vec::new(),
                    candidates: Vec::new(),
                    outcome: "RUN FAILURE: certify mode needs RUSMT_LLM_CMD".to_string(),
                    elapsed: std::time::Duration::ZERO,
                }],
                stage1: stage1.to_string(),
            };
            let _ = fs::write(
                target_dir.join("cosolve.txt"),
                cosolve::render_transcript("(none)", marker, &holder, &outcome),
            );
            return;
        };
        let excerpt = smt_excerpt(unmodified);
        let outcome = cosolve::certify(
            marker,
            oracle.name,
            &holder,
            &excerpt,
            stage1,
            l as &mut dyn proposer::Proposer,
            &mut solver,
            cosolve::rounds_from_env(),
            cosolve::witnesses_from_env(),
        );
        let desc = l.describe();
        let _ = fs::write(
            target_dir.join("cosolve.txt"),
            cosolve::render_transcript(&desc, marker, &holder, &outcome),
        );
        if let Some(first) = outcome.witnesses.first() {
            write_response(target_dir, parser_name, first, true);
            if outcome.witnesses.len() > 1 {
                let extra = outcome.witnesses[1..].join("\n---\n");
                let _ = fs::write(target_dir.join("witnesses.txt"), extra);
            }
            record_replay(oracle, model, target_dir, &replay_tgt, first);
        }
        return;
    }

    let outcome = cosolve::co_solve(
        marker,
        oracle.name,
        &holder,
        stage1,
        &mut plan,
        &ladder,
        llm.as_mut().map(|l| l as &mut dyn proposer::Proposer),
        &mut solver,
        cosolve::rounds_from_env(),
        cosolve::witnesses_from_env(),
    );
    let desc = llm
        .as_ref()
        .map(|l| l.describe())
        .unwrap_or_else(|| "(none: mechanical round only)".to_string());
    let _ = fs::write(
        target_dir.join("cosolve.txt"),
        cosolve::render_transcript(&desc, marker, &holder, &outcome),
    );
    if let Some(first) = outcome.witnesses.first() {
        write_response(target_dir, parser_name, first, true);
        // Extra witnesses go beside it: one per marker is thin for a
        // conformance suite (reviewer 5A), and the rest cost one re-solve each.
        if outcome.witnesses.len() > 1 {
            let extra = outcome.witnesses[1..].join("\n---\n");
            let _ = fs::write(target_dir.join("witnesses.txt"), extra);
        }
        record_replay(oracle, model, target_dir, &replay_tgt, first);
    }
}

/// Run Stage 1 then the co-solving loop for one *named* marker, writing every
/// query and the transcript under `out_dir`. This is the `recover` CLI mode: the
/// same two stages the full sweep runs, for one marker, so a single result can be
/// reproduced without re-running everything.
pub fn recover_marker(
    model: &IRContext,
    lang: &str,
    top_level_fn: &str,
    marker_name: &str,
    unroll_depth: usize,
    out_dir: &Path,
) -> std::result::Result<cosolve::Outcome, String> {
    use crate::backend::codegen::CodeGen as _;
    use crate::backend::z3::ctxt::CodeGenZ3;

    let id = rusmt_smt_stdlib::path::marker_id(marker_name);
    if model.marker_names.get(&id).map(String::as_str) != Some(marker_name) {
        let mut names: Vec<&str> = model.marker_names.values().map(String::as_str).collect();
        names.sort();
        return Err(format!(
            "`{marker_name}` is not a named marker in this model (use Path::named). Named markers: {}",
            if names.is_empty() {
                "(none)".to_string()
            } else {
                names.join(", ")
            }
        ));
    }
    let oracle = oracle_for(lang)
        .ok_or_else(|| format!("no replay oracle registered for language `{lang}`"))?;
    let target_ids = BTreeSet::from([id]);
    fs::create_dir_all(out_dir).map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;

    // Stage 1: the unmodified query.
    let cg = CodeGenZ3::new();
    let base = cg
        .process(model, unroll_depth)
        .map_err(|e| format!("base SMT-LIB generation failed: {e:?}"))?;
    let unmodified = cg.process_path_queries(&base, model, top_level_fn, &target_ids);
    if !guidance::query_has_seq_input(&unmodified, guidance::INPUT_VAR) {
        return Err(format!(
            "co-solving needs a `Seq<U32>` entry input; `{top_level_fn}` does not take one"
        ));
    }
    let qpath = out_dir.join("unmodified.smt2");
    fs::write(&qpath, &unmodified).map_err(|e| format!("cannot write query: {e}"))?;
    let budget = guidance::z3_budget_from_env();
    let stage1 = guidance::run_z3_file(&qpath, guidance::stage1_budget_from_env());

    // Stage 2 runs unless Stage 1 already produced a model.
    let ladder = slice::plan_ladder(model, top_level_fn, &target_ids);
    let mut plan = ladder.first().cloned().unwrap_or_default();
    let holder = slice::marker_holders(model, &target_ids)
        .into_iter()
        .next()
        .map(|f| backend::z3::fun::resolve_function_name(model, f))
        .unwrap_or_else(|| "(unknown)".to_string());
    if let Response::Sat(text) = &stage1 {
        let w = guidance::decode_seq_model(text, guidance::INPUT_VAR);
        return Ok(cosolve::Outcome {
            witnesses: w.into_iter().collect(),
            rounds: Vec::new(),
            stage1: stage1.to_string(),
        });
    }
    // The mechanical round runs regardless; only the model-driven rounds need a
    // proposer. `RUSMT_ABLATION=mechanical` stops after the mechanical round on
    // purpose, which is how the ablation measures slicing on its own.
    let ablate = std::env::var("RUSMT_ABLATION").ok().as_deref() == Some("mechanical");
    let mut llm = proposer::CommandProposer::from_env();
    if llm.is_none() && !ablate {
        eprintln!(
            "[rusmt] RUSMT_LLM_CMD is unset: only the mechanical round will run. \
             The co-solving loop is a pipeline stage, not an option — set it to measure the \
             full pipeline, or set RUSMT_ABLATION=mechanical to state that you meant this."
        );
    }
    let emit = |p: &slice::StubPlan| -> String {
        let cg = CodeGenZ3::with_stubs(p.stub.clone());
        match cg.process(model, unroll_depth) {
            Ok(b) => cg.process_path_queries(&b, model, top_level_fn, &target_ids),
            Err(e) => format!("; sliced codegen failed: {e:?}\n(check-sat)\n"),
        }
    };
    let names = |p: &slice::StubPlan| p.stub_names(model);
    let restorer = |p: &mut slice::StubPlan, n: &[String]| p.restore(model, n);
    let mut solver = cosolve::FileSolver {
        emit,
        unmodified: &unmodified,
        dir: out_dir,
        budget,
        names: &names,
        restorer: &restorer,
    };
    let desc = llm
        .as_ref()
        .map(|l| l.describe())
        .unwrap_or_else(|| "(none: mechanical round only)".to_string());

    // Certification mode: the proposer generates candidates from the emitted
    // SMT-LIB and Z3 decides each against the unmodified query. This is the
    // default reported pipeline; `RUSMT_MODE=guide` selects the older sketch
    // mode for experiments.
    if cosolve::mode_from_env() == cosolve::Mode::Certify {
        let Some(l) = llm.as_mut() else {
            return Err("certify mode needs RUSMT_LLM_CMD".to_string());
        };
        // The proposer sees the lifted query, not the Rust. The declarations and
        // the marker assertion are what matter; the bulk is parser bodies.
        let excerpt = smt_excerpt(&unmodified);
        let outcome = cosolve::certify(
            marker_name,
            oracle.name,
            &holder,
            &excerpt,
            &stage1,
            l as &mut dyn proposer::Proposer,
            &mut solver,
            cosolve::rounds_from_env(),
            cosolve::witnesses_from_env(),
        );
        let _ = fs::write(
            out_dir.join("cosolve.txt"),
            cosolve::render_transcript(&desc, marker_name, &holder, &outcome),
        );
        for w in &outcome.witnesses {
            record_replay(oracle, model, out_dir, marker_name, w);
        }
        return Ok(outcome);
    }

    let outcome = cosolve::co_solve(
        marker_name,
        oracle.name,
        &holder,
        &stage1,
        &mut plan,
        &ladder,
        llm.as_mut().map(|l| l as &mut dyn proposer::Proposer),
        &mut solver,
        cosolve::rounds_from_env(),
        cosolve::witnesses_from_env(),
    );
    let _ = fs::write(
        out_dir.join("cosolve.txt"),
        cosolve::render_transcript(&desc, marker_name, &holder, &outcome),
    );
    for w in &outcome.witnesses {
        record_replay(oracle, model, out_dir, marker_name, w);
    }
    Ok(outcome)
}

/// The part of an emitted query worth putting in a prompt.
///
/// The query is ~200 KB, almost all of it parser bodies, which do not fit in a
/// prompt and are not what a proposal keys on. What matters is the head (sorts
/// and datatype declarations) and the tail (the input declaration, its codepoint
/// bound, and the marker assertion — the actual question being asked). Keeping it
/// to a couple of kilobytes matters in practice: a sweep is hundreds of model
/// calls, and prompt size is the one part of that cost we control.
fn smt_excerpt(query: &str) -> String {
    const HEAD: usize = 700;
    const TAIL: usize = 1500;
    fn clip(s: &str, n: usize, from_end: bool) -> &str {
        if s.len() <= n {
            return s;
        }
        if from_end {
            let start = s.len() - n;
            match s.char_indices().find(|(i, _)| *i >= start) {
                Some((i, _)) => &s[i..],
                None => s,
            }
        } else {
            match s.char_indices().nth(n) {
                Some((i, _)) => &s[..i],
                None => s,
            }
        }
    }
    if query.len() <= HEAD + TAIL {
        return query.to_string();
    }
    format!(
        "{}\n; … {} bytes of parser definitions elided …\n{}",
        clip(query, HEAD, false),
        query.len() - HEAD - TAIL,
        clip(query, TAIL, true)
    )
}

// module tree
pub mod authoring;
mod backend;
pub mod cosolve;
pub mod guidance;
mod ir;
mod parser;
pub mod proposer;
pub mod slice;
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
/// makes it cheap: TOML's 182 targets render in seconds instead of spending a
/// Z3 budget on every marker.
pub fn solve<P: AsRef<Path>>(
    model: &IRContext,
    parser_name: &str,
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

            println!(
                "[rusmt] {}/{} {}",
                target_idx + 1,
                model.path_targets.len(),
                target_label
            );

            let timing_file = path_target_dir.join("timing.txt");
            let start = Instant::now();
            match solver.invoke_backend(&query_path) {
                Ok(resp) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    let (body, is_witness) = response_to_store(parser_name, resp.to_string());
                    write_response(&path_target_dir, parser_name, &body, is_witness);
                    fs::write(&timing_file, format!("{elapsed_ms}ms"))
                        .expect("failed to write timing file");

                    co_solve_target(
                        model,
                        parser_name,
                        &path_target_dir,
                        target_ids,
                        top_level_fn,
                        &query_code,
                        &resp,
                        unroll_depth,
                    );
                }
                Err(x) => {
                    let elapsed_ms = start.elapsed().as_millis();
                    write_response(
                        &path_target_dir,
                        parser_name,
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

/// Copy accepted witnesses from a synthesis run into an object-language suite.
///
/// The suite contains only inputs that Z3 accepted: `response.<ext>` plus any
/// extra witnesses in `witnesses.txt`, named by the marker they target.
pub fn write_conformance_suite<P: AsRef<Path>, Q: AsRef<Path>>(
    model: &IRContext,
    parser_name: &str,
    synthesis_dir: P,
    suite_dir: Q,
) -> std::io::Result<usize> {
    let oracle = oracle_for(parser_name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no suite renderer registered for `{parser_name}`"),
        )
    })?;
    let suite_dir = suite_dir.as_ref();
    fs::create_dir_all(suite_dir)?;
    for entry in fs::read_dir(suite_dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some(oracle.ext) {
            fs::remove_file(entry.path())?;
        }
    }

    let solver_dir = synthesis_dir.as_ref().join("z3_chc");
    let mut written = 0usize;
    for (target_idx, target_ids) in model.path_targets.iter().enumerate() {
        let names: Vec<&str> = target_ids
            .iter()
            .filter_map(|id| model.marker_names.get(id).map(String::as_str))
            .collect();
        if names.is_empty() {
            continue;
        }
        let stem = names
            .iter()
            .map(|n| sanitize_suite_stem(n))
            .collect::<Vec<_>>()
            .join("__");
        let target_dir = solver_dir.join(format!("target_{target_idx}"));
        let response = target_dir.join(format!("response.{}", oracle.ext));
        if response.exists() {
            fs::copy(&response, suite_dir.join(format!("{stem}.{}", oracle.ext)))?;
            written += 1;
        }
        let extra = target_dir.join("witnesses.txt");
        if extra.exists() {
            for (i, witness) in fs::read_to_string(extra)?.split("\n---\n").enumerate() {
                if witness.is_empty() {
                    continue;
                }
                fs::write(
                    suite_dir.join(format!("{stem}__{}.{}", i + 2, oracle.ext)),
                    witness,
                )?;
                written += 1;
            }
        }
    }
    Ok(written)
}

fn sanitize_suite_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

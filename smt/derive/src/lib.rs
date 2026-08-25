//! Pipeline for deriving and solving models from Rust code using SMT solvers.

use crate::backend::codegen::CodeGen as _;
use crate::backend::response::Response;
use crate::backend::z3::ctxt::CodeGenZ3;
use crate::ir::ctxt::{IRBuilder, IRContext};
use crate::parser::ctxt::Context;
use crate::proposer::Proposer as _;
use rusmt_lang::certify::{self, LanguageOracle, oracle_for};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Instant;
use syn::Result;

// module tree
pub mod authoring;
mod backend;
pub mod cosolve;
pub mod guidance;
mod ir;
mod parser;
pub mod proposer;

/// Marker names a target covers: `"div_by_zero"`, or `"a,b"` for a merged target.
/// `None` if empty or if any id has no name.
fn target_name(model: &IRContext, target_ids: &BTreeSet<usize>) -> Option<String> {
    if target_ids.is_empty() {
        return None;
    }
    let names: Vec<&str> = target_ids
        .iter()
        .map(|id| model.marker_names.get(id).map(String::as_str))
        .collect::<Option<_>>()?;
    Some(names.join(&certify::TARGET_SEP.to_string()))
}

/// Object-language source for a Z3 model. A `Seq<U32>` input is already the
/// source; any other sort is an AST the language's renderer prints.
fn render_witness(
    oracle: &LanguageOracle,
    query: &str,
    model_text: &str,
) -> std::result::Result<String, String> {
    if guidance::query_has_seq_input(query, guidance::INPUT_VAR) {
        guidance::decode_seq_model(model_text, guidance::INPUT_VAR)
            .ok_or_else(|| "the model's input is not a concrete code-point sequence".to_string())
    } else {
        let render = oracle
            .render_model
            .ok_or_else(|| format!("no model renderer registered for `{}`", oracle.name))?;
        render(model_text).ok_or_else(|| "the model does not render as source".to_string())
    }
}

/// Write a target's witnesses and replay the first through the concrete
/// semantics. Z3 already decided reachability, so a disagreement here is a bug in
/// the lift; it is reported, not hidden.
fn write_witnesses(
    oracle: &'static LanguageOracle,
    model: &IRContext,
    dir: &Path,
    target: &str,
    witnesses: &[String],
) -> std::result::Result<(), String> {
    let Some(first) = witnesses.first() else {
        return Ok(());
    };
    let path = dir.join(format!("response.{}", oracle.ext));
    fs::write(&path, first).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    if witnesses.len() > 1 {
        let extra = witnesses[1..].join("\n---\n");
        let path = dir.join("witnesses.txt");
        fs::write(&path, extra).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }

    let verdict = certify::certify_isolated(oracle.name, first, target, proposer::REPLAY_BUDGET);
    let line = proposer::verdict_line(&verdict, target, &model.marker_names);
    let note = if verdict.is_certified() {
        "the SMT lift and the concrete semantics agree on this input"
    } else {
        "MISMATCH: the SMT lift and the concrete semantics disagree on this input — \
         a transpilation-fidelity bug to investigate, not a rejected witness"
    };
    let _ = fs::write(dir.join("replay.txt"), format!("{line}\n{note}\n"));
    if !verdict.is_certified() {
        eprintln!("[rusmt] FIDELITY MISMATCH on `{target}`: {line}");
    }
    Ok(())
}

/// Everything a run holds fixed across its markers.
pub struct Run<'a> {
    /// The IR the queries are emitted from.
    pub model: &'a IRContext,
    /// Base SMT-LIB: types and functions, no queries.
    pub base_code: &'a str,
    /// Object language, for the oracle registry.
    pub lang: &'a str,
    /// Entry function the queries call.
    pub top_level_fn: &'a str,
    /// Bounded-recursion depth; 0 uses `define-funs-rec`.
    pub unroll_depth: usize,
    /// Proposer command, overriding `RUSMT_LLM_CMD`.
    pub llm: Option<&'a str>,
}

/// Stage 1 then Stage 2 for one target, writing every artifact under `dir`.
///
/// Stage 1 is Z3 alone, input free. `sat` is rendered and replayed; `unsat` at
/// k=0 means unreachable and stops. Anything else escalates to Stage 2, always —
/// a missing proposer is a run failure, not a quieter run.
pub fn run_target(
    run: &Run<'_>,
    target_ids: &BTreeSet<usize>,
    dir: &Path,
) -> std::result::Result<cosolve::Outcome, String> {
    let (model, base_code, lang, top_level_fn, unroll_depth, llm) = (
        run.model,
        run.base_code,
        run.lang,
        run.top_level_fn,
        run.unroll_depth,
        run.llm,
    );
    let oracle =
        oracle_for(lang).ok_or_else(|| format!("no oracle registered for language `{lang}`"))?;
    let target = target_name(model, target_ids)
        .ok_or_else(|| "target has no named marker (use Path::named)".to_string())?;
    fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    // The suite is labelled from this file, not from the directory's position,
    // so collecting a run of different code cannot mislabel a witness.
    fs::write(dir.join("marker.txt"), &target)
        .map_err(|e| format!("cannot write marker.txt: {e}"))?;

    let unmodified =
        CodeGenZ3::new().process_path_queries(base_code, model, top_level_fn, target_ids);
    let qpath = dir.join("main.smt2");
    fs::write(&qpath, &unmodified).map_err(|e| format!("cannot write query: {e}"))?;

    let started = Instant::now();
    let stage1 = guidance::run_z3_file(&qpath, guidance::stage1_budget_from_env());
    let stage1_ms = started.elapsed().as_millis();

    if let Response::Sat(text) = &stage1 {
        let witness = render_witness(oracle, &unmodified, text)
            .map_err(|e| format!("Stage 1 solved `{target}` but {e}"))?;
        write_witnesses(oracle, model, dir, &target, std::slice::from_ref(&witness))?;
        return Ok(cosolve::Outcome {
            witnesses: vec![witness],
            rounds: Vec::new(),
            stage1: "sat".to_string(),
            stop: cosolve::Stop::Witness,
            stage1_ms,
        });
    }
    // Only the unmodified, unbounded query licenses an unreachability verdict;
    // under k-unrolling `unsat` means "no witness within depth k".
    if matches!(stage1, Response::Unsat) && unroll_depth == 0 {
        let _ = fs::write(
            dir.join("unreachable.txt"),
            format!("unsat on the unmodified, unconstrained query: no input reaches `{target}`\n"),
        );
        return Ok(cosolve::Outcome {
            witnesses: Vec::new(),
            rounds: Vec::new(),
            stage1: "unsat".to_string(),
            stop: cosolve::Stop::Witness,
            stage1_ms,
        });
    }

    if !guidance::query_has_seq_input(&unmodified, guidance::INPUT_VAR) {
        return Err(format!(
            "Stage 2 needs a `Seq<U32>` entry input; `{top_level_fn}` does not take one"
        ));
    }
    let mut llm = proposer::CommandProposer::from_cli_or_env(llm).ok_or_else(|| {
        "Stage 2 needs a proposer: name one on the command line, or set \
         RUSMT_LLM_CMD to a command that reads a prompt on stdin and writes a \
         candidate on stdout"
            .to_string()
    })?;
    let holder = backend::z3::fun::holder_name(model, target_ids);
    let excerpt = smt_excerpt(&unmodified);
    // Marker names in bit order, so a rejection's observed `Path` decodes.
    let marker_at_bit: Vec<String> = model.marker_names.values().cloned().collect();
    let observation = CodeGenZ3::new().process_path_observation(base_code, model, top_level_fn);
    let mut solver = cosolve::FileSolver {
        unmodified: &unmodified,
        dir,
        budget: guidance::z3_budget_from_env(),
        round: 0,
        observation: &observation,
        marker_at_bit: &marker_at_bit,
    };
    let outcome = cosolve::certify(
        &cosolve::Target {
            marker: &target,
            language: oracle.name,
            holder: &holder,
            smt_excerpt: &excerpt,
        },
        &stage1,
        &mut llm,
        &mut solver,
        cosolve::rounds_from_env(),
        cosolve::witnesses_from_env(),
    );
    let _ = fs::write(
        dir.join("cosolve.txt"),
        cosolve::render_transcript(&llm.describe(), &target, &holder, &outcome),
    );
    write_witnesses(oracle, model, dir, &target, &outcome.witnesses)?;
    let mut outcome = outcome;
    outcome.stage1_ms = stage1_ms;
    Ok(outcome)
}

/// Run both stages for one *named* marker under `out_dir` — the `recover` CLI
/// mode, and the unit the sweep drives. It is [`run_target`] with the marker
/// looked up by name, so it cannot drift from what the full run does.
pub fn recover_marker(
    model: &IRContext,
    lang: &str,
    top_level_fn: &str,
    marker_name: &str,
    unroll_depth: usize,
    out_dir: &Path,
    llm: Option<&str>,
) -> std::result::Result<cosolve::Outcome, String> {
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
    let base = CodeGenZ3::new()
        .process(model, unroll_depth)
        .map_err(|e| format!("base SMT-LIB generation failed: {e:?}"))?;
    run_target(
        &Run {
            model,
            base_code: &base,
            lang,
            top_level_fn,
            unroll_depth,
            llm,
        },
        &BTreeSet::from([id]),
        out_dir,
    )
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

/// One marker's result as a JSON line.
fn ledger_line(name: &str, outcome: &std::result::Result<cosolve::Outcome, String>) -> String {
    fn esc(s: &str) -> String {
        let mut o = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o.push('"');
        o
    }
    match outcome {
        Err(e) => format!(
            "{{\"marker\":{},\"status\":\"FAIL\",\"failure\":{},\"witnesses\":[],\"round_outcomes\":[],\"rounds\":0}}",
            esc(name),
            esc(e)
        ),
        Ok(o) => {
            let status = if !o.witnesses.is_empty() {
                if o.rounds.is_empty() {
                    "STAGE1"
                } else {
                    "WITNESS"
                }
            } else if o.stage1 == "unsat" {
                "UNREACH"
            } else {
                "none"
            };
            let ws: Vec<String> = o.witnesses.iter().map(|w| esc(w)).collect();
            let os: Vec<String> = o.rounds.iter().map(|r| esc(&r.outcome)).collect();
            format!(
                "{{\"marker\":{},\"status\":\"{status}\",\"stage1\":{},\"rounds\":{},\"stage1_ms\":{},\"witnesses\":[{}],\"round_outcomes\":[{}],\"rejected\":{},\"stop\":\"{}\"}}",
                esc(name),
                esc(&o.stage1),
                o.rounds.len(),
                o.stage1_ms,
                ws.join(","),
                os.join(","),
                o.rejected(),
                o.stop.as_str()
            )
        }
    }
}

/// What a whole-model run produced, per target.
pub struct SolveReport {
    /// One JSON line per marker.
    pub ledger: Vec<String>,
    /// Markers a witness was found for, and how many.
    pub covered: Vec<String>,
    /// Markers Z3 proved unreachable on the unmodified, unbounded query.
    pub unreachable: Vec<String>,
    /// Markers the round budget ran out on.
    pub missed: Vec<String>,
    /// Markers whose run failed, with the reason. Never counted as a miss.
    pub failed: Vec<(String, String)>,
}

/// Run both stages for every named marker in `model`, writing each target's
/// artifacts under `<output>/<backend>/target_<N>/`.
///
/// `unroll_depth` controls bounded-recursion unrolling in the text backend
/// (see `CodeGen::process`); pass 0 to keep the recursive emission.
///
/// `jobs` markers are attempted at a time.
///
/// With `RUSMT_SKIP_INVOKE=1` the queries are written and nothing is solved.
pub fn solve<P: AsRef<Path>>(
    model: &IRContext,
    parser_name: &str,
    top_level_fn: Option<&str>,
    output: P,
    unroll_depth: usize,
    jobs: usize,
    llm: Option<&str>,
) -> Result<SolveReport> {
    let skip_invoke = std::env::var("RUSMT_SKIP_INVOKE").ok().as_deref() == Some("1");
    let cg = CodeGenZ3::new();
    let dir = output.as_ref().join(cg.name());
    fs::create_dir_all(&dir).expect("workspace freshly created");

    let base_code = match cg.process(model, unroll_depth) {
        Ok(code) => code,
        Err(e) => panic!("error generating SMT-LIB code: {e:?}"),
    };
    fs::write(dir.join(format!("main.{}", cg.flavor())), &base_code)
        .unwrap_or_else(|e| panic!("IO error on source file: {e}"));

    let mut report = SolveReport {
        ledger: Vec::new(),
        covered: Vec::new(),
        unreachable: Vec::new(),
        missed: Vec::new(),
        failed: Vec::new(),
    };
    let Some(top_level_fn) = top_level_fn else {
        return Ok(report);
    };

    let targets: Vec<(usize, &BTreeSet<usize>)> = model.path_targets.iter().enumerate().collect();
    if skip_invoke {
        for (idx, target_ids) in targets {
            let name = target_name(model, target_ids).unwrap_or_else(|| format!("target_{idx}"));
            let target_dir = dir.join(format!("target_{idx}"));
            fs::create_dir_all(&target_dir).expect("target directory created");
            let q = cg.process_path_queries(&base_code, model, top_level_fn, target_ids);
            fs::write(target_dir.join(format!("main.{}", cg.flavor())), q)
                .expect("failed to write query file");
            let _ = fs::write(target_dir.join("marker.txt"), &name);
        }
        return Ok(report);
    }

    // Markers share no state, so this changes wall clock and nothing else.
    let jobs = jobs.max(1).min(targets.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::Mutex::new(&mut report);
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(idx, target_ids)) = targets.get(i) else {
                        break;
                    };
                    let name =
                        target_name(model, target_ids).unwrap_or_else(|| format!("target_{idx}"));
                    let outcome = run_target(
                        &Run {
                            model,
                            base_code: &base_code,
                            lang: parser_name,
                            top_level_fn,
                            unroll_depth,
                            llm,
                        },
                        target_ids,
                        &dir.join(format!("target_{idx}")),
                    );
                    let mut r = done.lock().expect("report lock");
                    r.ledger.push(ledger_line(&name, &outcome));
                    match outcome {
                        Ok(o) if !o.witnesses.is_empty() => r.covered.push(name),
                        Ok(o) if o.stage1 == "unsat" => r.unreachable.push(name),
                        Ok(_) => r.missed.push(name),
                        // A failed run is not a marker without a witness: it is
                        // a run to fix.
                        Err(e) => {
                            eprintln!("[rusmt] {name}: RUN FAILURE: {e}");
                            r.failed.push((name, e));
                        }
                    }
                }
            });
        }
    });
    Ok(report)
}

/// Copy accepted witnesses from a synthesis run into an object-language suite.
///
/// The suite holds only inputs Z3 accepted: each target's `response.<ext>` plus
/// any extra witnesses in `witnesses.txt`, named by the `marker.txt` the run
/// wrote beside them.
pub fn write_conformance_suite<P: AsRef<Path>, Q: AsRef<Path>>(
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

    let solver_dir = synthesis_dir.as_ref().join(CodeGenZ3::new().name());
    let mut written = 0usize;
    let mut dirs: Vec<_> = fs::read_dir(&solver_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for target_dir in dirs {
        let Ok(name) = fs::read_to_string(target_dir.join("marker.txt")) else {
            continue;
        };
        let stem = sanitize_suite_stem(name.trim());
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

/// A query that reports which markers an input fires, rather than asserting one.
///
/// The same query Stage 2 uses to explain a rejection. Pin an input into it with
/// [`guidance::pin_input`] and read [`guidance::OBSERVED_PATH`] out of the model
/// with [`guidance::decode_bitvec_bits`]; bit *i* is `marker_names`' *i*-th key.
pub fn observation_query(
    ir: &IRContext,
    top_level_fn: &str,
    unroll_depth: usize,
) -> std::result::Result<String, String> {
    let cg = CodeGenZ3::new();
    let base = cg
        .process(ir, unroll_depth)
        .map_err(|e| format!("does not transpile: {e:?}"))?;
    Ok(cg.process_path_observation(&base, ir, top_level_fn))
}

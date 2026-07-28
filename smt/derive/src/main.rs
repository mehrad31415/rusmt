//! This is the main entry point for the derive crate.
//! usage: cargo run -- <parser_name> <top_level_fn> [k=<N>] [--suite-out <dir>]
//!    or: cargo run -- author <spec_file> <out_file> [rounds=<N>] [--examples <path>]
//!
//! `k=<N>` enables bounded-recursion unrolling:
//!   k=0 (or omitted) -> recursive define-funs-rec.
//!   k=N (N≥1)        -> every recursive SCC is unrolled to depth N.
//!
//! The parser command emits one query per marker, runs Z3 first, escalates to
//! the AI-to-Z3 certification loop when needed, and writes the accepted
//! witnesses as a conformance suite. `RUSMT_LLM_CMD` names the model transport.
//!
//! `author` runs the gated AI-authoring loop: the proposer configured by
//! `RUSMT_LLM_CMD` drafts a reference semantics in the DSL for the prose
//! specification in `<spec_file>`; each draft must pass the mechanical gates
//! (DSL front end, SMT emission, named markers) AND the behavioral gates
//! (solver-proved marker reachability; conformance to the optional
//! `--examples` file of `INPUT<TAB>EXPECT` lines, EXPECT = ok | err:<marker>
//! | nomatch). The admitted draft is written to `<out_file>` for human
//! review. Per-gate Z3 budget: `RUSMT_AUTHOR_Z3_SECS` (default 20).

use rusmt_smt_derive::guidance::{self, Response};
use rusmt_smt_derive::proposer::{CommandProposer, Proposer as _};
use rusmt_smt_derive::{authoring, model, solve};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Run the gated authoring loop (the `author` CLI mode).
fn run_author(args: &[String]) -> Result<(), Box<dyn Error>> {
    let [spec_file, out_file, rest @ ..] = args else {
        return Err(
            "Usage: cargo run -- author <spec_file> <out_file> [rounds=<N>] [--examples <path>]"
                .into(),
        );
    };
    let mut rounds = authoring::DEFAULT_MAX_ROUNDS;
    let mut examples: Vec<authoring::Example> = Vec::new();
    let mut rest_it = rest.iter();
    while let Some(extra) = rest_it.next() {
        if let Some(n) = extra.strip_prefix("rounds=") {
            rounds = n
                .parse()
                .map_err(|_| format!("rounds= must be a positive integer, got '{n}'"))?;
            if rounds == 0 {
                return Err("rounds= must be at least 1".into());
            }
        } else if extra == "--examples" {
            let path = rest_it
                .next()
                .ok_or("--examples needs a file path argument")?;
            let text = fs::read_to_string(path)
                .map_err(|e| format!("cannot read examples file '{path}': {e}"))?;
            examples = authoring::parse_examples(&text)
                .map_err(|e| format!("examples file '{path}': {e}"))?;
        } else {
            return Err(format!("unrecognized argument: '{extra}'").into());
        }
    }
    let spec = fs::read_to_string(spec_file)
        .map_err(|e| format!("cannot read spec file '{spec_file}': {e}"))?;
    let mut proposer = CommandProposer::from_env().ok_or(
        "the author mode needs a proposer: set RUSMT_LLM_CMD to a command that \
         reads a prompt on stdin and writes a draft on stdout (e.g. `claude -p`)",
    )?;
    println!("[authoring] proposer: {}", proposer.describe());
    if !examples.is_empty() {
        println!("[authoring] spec examples: {}", examples.len());
    }

    // The behavioral gates' solver seam: each gate query goes to a real Z3
    // run under a short budget (a missing z3 binary folds into `unknown`,
    // which the gates treat as a warning, not a failure).
    let budget = authoring::author_z3_budget_from_env();
    let gate_dir = tempfile::tempdir().map_err(|e| format!("cannot create temp dir: {e}"))?;
    let mut gate_no = 0usize;
    let mut solve_gate = |label: &str, query: &str| -> Response {
        gate_no += 1;
        let path = gate_dir.path().join(format!("gate_{gate_no:03}.smt2"));
        if let Err(e) = fs::write(&path, query) {
            return Response::Unknown(format!("cannot write gate query: {e}"));
        }
        println!("[authoring]   z3 gate {label}");
        guidance::run_z3_file(&path, budget)
    };

    let outcome = authoring::author_semantics_validated(
        &spec,
        &examples,
        &mut proposer,
        &mut solve_gate,
        rounds,
    );
    for (i, round) in outcome.rounds.iter().enumerate() {
        if round.failures.is_empty() {
            println!("[authoring] round {}: admitted by all gates", i + 1);
        } else {
            println!("[authoring] round {}: rejected", i + 1);
            for f in &round.failures {
                println!("[authoring]   gate: {f}");
            }
        }
        for n in &round.notes {
            println!("[authoring]   note: {n}");
        }
    }

    // Persist the full session transcript next to the output, so a rejected
    // session (every draft, gate failure, and solver observation) can be
    // inspected — mirroring the synthesis pipeline's fallback.txt/guidance.txt.
    let transcript_path = format!("{out_file}.authoring.txt");
    let transcript = authoring::render_transcript(spec_file, &examples, &outcome);
    if let Err(e) = fs::write(&transcript_path, &transcript) {
        eprintln!("[authoring] warning: cannot write transcript to '{transcript_path}': {e}");
    } else {
        println!("[authoring] session transcript written to {transcript_path}");
    }

    match outcome.accepted {
        Some(draft) => {
            fs::write(out_file, &draft)
                .map_err(|e| format!("cannot write draft to '{out_file}': {e}"))?;
            println!(
                "[authoring] draft written to {out_file} (named markers: {}).",
                outcome.markers.join(", ")
            );
            println!(
                "[authoring] the draft is mechanically admissible, NOT trusted: \
                 review it, then run synthesis against its markers."
            );
            Ok(())
        }
        None => Err(format!(
            "no draft passed the gates within the round budget; see {transcript_path}"
        )
        .into()),
    }
}

/// Run Stage 1 then the AI⇄Z3 co-solving loop for one named marker (the
/// `recover` CLI mode). Z3 produces every model; the proposer only restores
/// sliced-away definitions and constrains the input.
fn run_recover(lang_src_dir: &std::path::Path, args: &[String]) -> Result<(), Box<dyn Error>> {
    let [parser_name, top_level_fn, marker_name, rest @ ..] = args else {
        return Err(
            "Usage: cargo run -- recover <parser> <top_level_fn> <marker_name> [k=<N>]\n\
             Example: RUSMT_LLM_CMD='claude -p' \\\n\
             \x20 cargo run -- recover toml parse_toml comment_invalid_char"
                .into(),
        );
    };
    let mut unroll_depth: usize = 0;
    for extra in rest {
        if let Some(n) = extra.strip_prefix("k=") {
            unroll_depth = n
                .parse()
                .map_err(|_| format!("k= must be a non-negative integer, got '{n}'"))?;
        } else {
            return Err(format!("unrecognized argument: '{extra}'").into());
        }
    }
    let parser_dir = lang_src_dir.join(parser_name);
    if !parser_dir.exists() {
        return Err(format!("Parser '{parser_name}' not found at {parser_dir:?}").into());
    }
    let model = model(&parser_dir)?;
    let out_dir = lang_src_dir
        .join("synthesis")
        .join(parser_name)
        .join("recover")
        .join(marker_name);

    println!("[recover] marker  : {marker_name}");
    println!("[recover] stage 1 : Z3 on the unmodified query");
    let outcome = rusmt_smt_derive::recover_marker(
        &model,
        parser_name,
        top_level_fn,
        marker_name,
        unroll_depth,
        &out_dir,
    )
    .map_err(|e| -> Box<dyn Error> { e.into() })?;

    println!("[recover] stage 1 verdict: {}", outcome.stage1);
    for (i, r) in outcome.rounds.iter().enumerate() {
        println!("[recover] round {} ({} ms)", i + 1, r.elapsed.as_millis());
        if !r.restored.is_empty() {
            println!("[recover]   restored: {}", r.restored.join(", "));
        }
        for c in &r.candidates {
            println!("[recover]   Z3 candidate: {c:?}");
        }
        println!("[recover]   outcome: {}", r.outcome);
    }
    if outcome.witnesses.is_empty() {
        println!("[recover] RESULT: no witness within the round budget");
    } else {
        println!(
            "[recover] RESULT: {} witness(es), each a model of the unmodified query:",
            outcome.witnesses.len()
        );
        for w in &outcome.witnesses {
            println!("[recover]   {w:?}");
        }
    }
    println!(
        "[recover] transcript: {}",
        out_dir.join("cosolve.txt").display()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Isolated-replay hook: when this process was re-spawned as a replay child
    // (by `certify_isolated`), certify the candidate on stdin and exit.
    rusmt_lang::certify::maybe_subprocess_entry();

    let root_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = root_crate_dir
        .parent()
        .expect("Failed to find workspace root")
        .parent()
        .expect("Failed to find project root");

    // Path to lang/src which contains the parsers/interpreters
    let lang_src_dir = workspace_root.join("lang").join("src");
    if !lang_src_dir.exists() {
        return Err(format!("Lang source directory not found at {lang_src_dir:?}").into());
    }
    // Path to lang/synthesis which contains the synthesis outputs
    let synthesis_base = lang_src_dir.join("synthesis");

    let args: Vec<String> = std::env::args().collect();

    // `author` mode: gated AI drafting of a reference semantics in the DSL.
    if args.len() >= 2 && args[1] == "author" {
        return run_author(&args[2..]);
    }

    // `recover` mode: the embedded AI⇄Z3 guided loop for one named marker.
    if args.len() >= 2 && args[1] == "recover" {
        return run_recover(&lang_src_dir, &args[2..]);
    }

    if args.len() >= 3 {
        let parser_name = &args[1];
        let top_level_fn = &args[2];
        let parser_dir = lang_src_dir.join(parser_name);
        if !parser_dir.exists() {
            return Err(format!("Parser '{parser_name}' not found at {parser_dir:?}").into());
        }
        let mut output_dir = synthesis_base.join(parser_name);
        let mut suite_out = Some(std::env::temp_dir().join("rusmt-suite").join(parser_name));

        let mut unroll_depth: usize = 0;
        let mut k_set = false;
        let mut rest = args.iter().skip(3);
        while let Some(extra) = rest.next() {
            if let Some(n_str) = extra.strip_prefix("k=") {
                if k_set {
                    return Err("k= specified more than once".to_string().into());
                }
                unroll_depth = n_str
                    .parse::<usize>()
                    .map_err(|_| format!("k= must be a non-negative integer, got '{n_str}'"))?;
                k_set = true;
            } else if extra == "--suite-out" {
                let path = rest.next().ok_or("--suite-out needs a directory")?;
                suite_out = Some(PathBuf::from(path));
            } else if let Some(path) = extra.strip_prefix("suite=") {
                suite_out = Some(PathBuf::from(path));
            } else if extra == "--no-suite" {
                suite_out = None;
            } else if extra == "--out-dir" {
                let path = rest.next().ok_or("--out-dir needs a directory")?;
                output_dir = PathBuf::from(path);
            } else if let Some(path) = extra.strip_prefix("out=") {
                output_dir = PathBuf::from(path);
            } else {
                return Err(format!("unrecognized argument: '{extra}'").into());
            }
        }

        if output_dir.exists() {
            fs::remove_dir_all(&output_dir)?;
        }
        fs::create_dir_all(&output_dir)?;

        let model = model(&parser_dir)?;

        solve(
            &model,
            parser_name,
            Some(top_level_fn.as_str()),
            &output_dir,
            unroll_depth,
        )?;

        if let Some(suite_out) = suite_out {
            match rusmt_smt_derive::write_conformance_suite(
                &model,
                parser_name,
                &output_dir,
                &suite_out,
            ) {
                Ok(n) => {
                    println!(
                        "[rusmt] conformance suite: {n} input(s) written to {}",
                        suite_out.display()
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                    println!("[rusmt] no conformance suite written: {e}");
                }
                Err(e) => return Err(format!("cannot write conformance suite: {e}").into()),
            }
        }
    } else {
        return Err(
            "Usage: cargo run -- <parser_name> <top_level_fn> [k=<N>] [--suite-out <dir>]\n\
             Example: cargo run -- toml parse_toml --suite-out /tmp/toml-suite"
                .into(),
        );
    }

    Ok(())
}

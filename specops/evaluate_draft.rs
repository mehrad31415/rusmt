//! Scores a drafted semantics against a reference suite, without using names.
//!
//! Paper tooling, not a framework feature: it needs a draft *and* a trusted
//! reference suite, which only we have. Reached via an `[[example]]` path.
//!
//! The draft is transpiled, never executed: each input is pinned into the
//! observation query built from the draft's own IR.
//!
//! Names are not compared. A draft invents its own, and supplying ours would
//! supply the rules -- 56 of 57 marker names contain their rule word. Both
//! metrics are therefore name-free:
//!
//! 1. accept/reject agreement, over invalid AND valid documents (invalid alone
//!    is degenerate: rejecting everything would score 100%)
//! 2. granularity: how finely the draft partitions the invalid documents. The
//!    reference gives each its own marker, so its partition is the discrete
//!    one; a draft reaching the same class count has reached the same partition.
//!
//! usage: cargo run -p rusmt-smt-derive --example evaluate_draft -- \
//!            <draft_dir> <entry_fn> <invalid_dir> <valid_dir> [k]

use rusmt_smt_derive::{guidance, model, observation_query};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// What the draft did with one input.
#[derive(PartialEq, Clone)]
enum Outcome {
    /// Fired at least one named marker: the draft rejects the document.
    Rejected(BTreeSet<String>),
    /// Fired nothing: the draft accepts the document.
    Accepted,
    /// The solver returned no usable verdict.
    Undecided(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [draft, entry, invalid_dir, valid_dir, rest @ ..] = a.as_slice() else {
        return Err(
            "usage: evaluate_draft <draft_dir> <entry_fn> <invalid_dir> <valid_dir> [k]".into(),
        );
    };
    let k: usize = rest.first().map(|s| s.parse()).transpose()?.unwrap_or(0);
    let budget = guidance::z3_budget_from_env();

    let ir = model(draft)?;
    let observation = observation_query(&ir, entry, k)?;
    if !guidance::query_has_seq_input(&observation, guidance::INPUT_VAR) {
        return Err(format!(
            "`{entry}` does not take a text input, so a document cannot be pinned into it"
        )
        .into());
    }
    let at_bit: Vec<String> = ir.marker_names.values().cloned().collect();
    let work = std::env::temp_dir().join("rusmt-evaluate");
    fs::create_dir_all(&work)?;

    let run = |path: &Path| -> Result<Outcome, Box<dyn std::error::Error>> {
        let input = fs::read_to_string(path)?;
        let Some(q) = guidance::pin_input(&observation, &input, guidance::INPUT_VAR) else {
            return Ok(Outcome::Undecided("no (check-sat)".into()));
        };
        let qp = work.join("draft_observe.smt2");
        fs::write(&qp, &q)?;
        Ok(match guidance::run_z3_file(&qp, budget) {
            guidance::Response::Sat(m) => {
                match guidance::decode_bitvec_bits(&m, guidance::OBSERVED_PATH) {
                    None => Outcome::Undecided("model did not decode".into()),
                    Some(bits) => {
                        let fired: BTreeSet<String> = bits
                            .iter()
                            .filter_map(|&b| at_bit.get(b).cloned())
                            .collect();
                        if fired.is_empty() {
                            Outcome::Accepted
                        } else {
                            Outcome::Rejected(fired)
                        }
                    }
                }
            }
            other => Outcome::Undecided(other.to_string()),
        })
    };

    let listing = |d: &str| -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
        let mut v: Vec<_> = fs::read_dir(d)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "toml"))
            .collect();
        v.sort();
        Ok(v)
    };

    // ---- 1. accept/reject agreement ----
    let mut false_accept = Vec::new(); // invalid, but the draft accepted it
    let mut false_reject = Vec::new(); // valid, but the draft rejected it
    let (mut agree, mut undecided) = (0usize, 0usize);
    let mut fired_on_invalid: Vec<(String, BTreeSet<String>)> = Vec::new();

    for p in listing(invalid_dir)? {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        match run(&p)? {
            Outcome::Rejected(m) => {
                agree += 1;
                fired_on_invalid.push((name, m));
            }
            Outcome::Accepted => false_accept.push(name),
            Outcome::Undecided(_) => undecided += 1,
        }
    }
    let n_invalid = listing(invalid_dir)?.len();

    for p in listing(valid_dir)? {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        match run(&p)? {
            Outcome::Accepted => agree += 1,
            Outcome::Rejected(m) => {
                false_reject.push(format!("{name} ({})", m.iter().cloned().collect::<Vec<_>>().join(",")))
            }
            Outcome::Undecided(_) => undecided += 1,
        }
    }
    let n_valid = listing(valid_dir)?.len();
    let total = n_invalid + n_valid;

    println!("== 1. accept/reject agreement ==");
    println!("  documents            : {total}  ({n_invalid} invalid, {n_valid} valid)");
    println!(
        "  agreed with oracle   : {agree}  ({:.1}%)",
        100.0 * agree as f64 / total.max(1) as f64
    );
    println!(
        "  accepted an invalid  : {:>4}   <- the draft is too permissive here",
        false_accept.len()
    );
    println!(
        "  rejected a valid     : {:>4}   <- the draft is too strict here",
        false_reject.len()
    );
    println!("  undecided (solver)   : {undecided:>4}");
    if !false_accept.is_empty() {
        println!("\n  invalid documents the draft accepted:");
        for m in &false_accept {
            println!("    {m}");
        }
    }
    if !false_reject.is_empty() {
        println!("\n  valid documents the draft rejected:");
        for m in &false_reject {
            println!("    {m}");
        }
    }

    // ---- 2. granularity ----
    let distinct_markers: BTreeSet<&String> =
        fired_on_invalid.iter().flat_map(|(_, m)| m).collect();
    // Documents sharing a marker-set are ones the draft cannot tell apart. The
    // reference separates all of them, so each such group is a merge.
    let mut classes: std::collections::BTreeMap<&BTreeSet<String>, Vec<&str>> =
        std::collections::BTreeMap::new();
    for (doc, m) in &fired_on_invalid {
        classes.entry(m).or_default().push(doc);
    }
    let distinct_classes = classes.len();
    println!("\n== 2. granularity (name-free) ==");
    println!("  reference markers over these documents : {n_invalid}");
    println!("  distinct draft markers fired           : {}", distinct_markers.len());
    println!("  distinct draft marker-sets (classes)   : {distinct_classes}");
    println!(
        "  resolution ratio                       : {:.2}  (1.00 = as fine as the reference)",
        distinct_classes as f64 / n_invalid.max(1) as f64
    );
    println!(
        "  markers the draft declares in total    : {}",
        ir.marker_names.len()
    );
    // The reference partition is discrete, so a ratio of 1.00 is not merely the
    // same class count: it is the same partition, there being only one partition
    // of n documents into n classes.
    let merged: Vec<_> = classes.values().filter(|d| d.len() > 1).collect();
    if merged.is_empty() {
        println!("\n  Every document lands in its own class: the same partition as the");
        println!("  reference. The draft separates every pair the reference separates.");
    } else {
        println!("\n  Documents the draft cannot tell apart (the reference separates each):");
        for g in &merged {
            println!("    {}", g.join("  "));
        }
    }
    Ok(())
}

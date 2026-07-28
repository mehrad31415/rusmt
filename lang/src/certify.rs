//! Replay checks for synthesized / proposed object-language programs.
//!
//! A candidate object-language program may come from Z3, a program emitted by a
//! language model, or a bounded enumerator. A candidate is accepted only if Z3
//! has decided the marker query for it and, when re-executed through the concrete
//! reference semantics, it reaches the *targeted* marker.
//!
//! Soundness rests on two facts:
//!
//!  1. Z3 acceptance is monotone: pinning a candidate input only strengthens the
//!     original marker query, so a `sat` pinned query is still a model of the
//!     original query.
//!  2. Replay checks that the SMT lift and concrete oracle agree on the
//!     candidate. A hallucinated or otherwise wrong proposal is rejected.
//!  3. A *named* marker's integer id is [`marker_id`]`(name)` on both the SMT
//!     side (the id the synthesis query asserts membership of) and here (the id
//!     the concrete `Path::named` carries). Hence "reached the targeted marker"
//!     is decided soundly by membership of `marker_id(target)` in the fired
//!     `Path` set — not merely "reached *some* marker".
//!
//! A target may name several markers at once (a `Path::merge` target, written
//! comma-separated); the candidate is accepted only if *every* one of
//! them fired on the same run, mirroring the SMT query's simultaneous
//! assertion of all their ids. See [`TARGET_SEP`].
//!
//! Replay runs under a wall-clock budget so that a non-terminating candidate
//! (IMP has `while`) is rejected as a timeout rather than hanging the run.

use crate::imp::{EvalResult, eval_com, parser::format_store, parser::parse_imp_source};
use crate::toml::{ParseResult as TomlParseResult, parse_toml};
use rusmt_smt_stdlib::path::{marker_id, marker_ids};
use rusmt_smt_stdlib::{Seq, U32};
use std::collections::BTreeSet;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// The marker names declared in the IMP reference semantics
/// (`lang/src/imp/mod.rs`). The arbiter refers to markers by these names.
pub const IMP_MARKERS: [&str; 2] = ["division_by_zero", "undefined_variable"];

/// The *observable behaviour* of running a program through the reference
/// semantics, as a canonical string suitable for differential comparison
/// against an independent implementation:
///
///  * `OK <store>`      — normal termination with the given final store;
///  * `ERR <marker>`    — a named error marker fired (e.g. `division_by_zero`);
///  * `PARSE_ERROR ...` — the source did not parse as IMP.
///
/// This is what an external (e.g. AI-generated) implementation must match to be
/// judged conformant; a difference is a conformance bug in that implementation.
pub fn observe_imp(source: &str) -> String {
    let program = match parse_imp_source(source) {
        Ok(p) => p,
        Err(e) => return format!("PARSE_ERROR {e}"),
    };
    match eval_com(program) {
        EvalResult::Ok(store) => {
            // Canonical, whitespace-free, sorted store: `OK a=1;b=2` (or `OK `
            // for the empty store), so an independent implementation can match
            // it byte-for-byte.
            let mut pairs: Vec<std::string::String> = format_store(store)
                .lines()
                .map(|l| l.replace(' ', ""))
                .collect();
            pairs.sort();
            format!("OK {}", pairs.join(";"))
        }
        EvalResult::Err(path) => {
            let ids = marker_ids(path);
            for name in IMP_MARKERS {
                if ids.contains(&marker_id(name)) {
                    return format!("ERR {name}");
                }
            }
            "ERR unknown_marker".to_string()
        }
    }
}

/// The outcome of certifying a candidate program against a target marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Parsed, terminated within budget, and the *targeted* marker fired.
    ReachedTarget,
    /// Parsed and terminated, but a *different* marker fired (ids carried).
    ReachedOtherMarker(Vec<usize>),
    /// Parsed and terminated normally, firing no marker.
    NoMarker,
    /// The candidate did not parse as object-language source.
    ParseError(String),
    /// Replay exceeded the wall-clock budget (e.g. a non-terminating loop).
    Timeout,
    /// The isolated replay process died abnormally (e.g. a stack overflow from
    /// unbounded recursion). Like [`Verdict::Timeout`], this rejects the
    /// candidate; it exists so a crash is reported as a crash.
    Crashed(String),
}

impl Verdict {
    /// Whether the candidate is an accepted witness for the target marker.
    pub fn is_certified(&self) -> bool {
        matches!(self, Verdict::ReachedTarget)
    }

    /// Encode this verdict as the single line the replay subprocess prints.
    pub fn to_wire(&self) -> String {
        match self {
            Verdict::ReachedTarget => "REACHED_TARGET".to_string(),
            Verdict::ReachedOtherMarker(ids) => {
                let ids: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                format!("REACHED_OTHER {}", ids.join(" "))
            }
            Verdict::NoMarker => "NO_MARKER".to_string(),
            Verdict::ParseError(msg) => format!("PARSE_ERROR {}", msg.replace('\n', " ")),
            Verdict::Timeout => "TIMEOUT".to_string(),
            Verdict::Crashed(msg) => format!("CRASHED {}", msg.replace('\n', " ")),
        }
    }

    /// Decode a verdict line printed by the replay subprocess.
    pub fn from_wire(line: &str) -> Option<Verdict> {
        let line = line.trim();
        if line == "REACHED_TARGET" {
            return Some(Verdict::ReachedTarget);
        }
        if line == "NO_MARKER" {
            return Some(Verdict::NoMarker);
        }
        if line == "TIMEOUT" {
            return Some(Verdict::Timeout);
        }
        if let Some(rest) = line.strip_prefix("REACHED_OTHER") {
            let ids: Option<Vec<usize>> = rest
                .split_whitespace()
                .map(|t| t.parse::<usize>().ok())
                .collect();
            return ids.map(Verdict::ReachedOtherMarker);
        }
        if let Some(rest) = line.strip_prefix("PARSE_ERROR") {
            return Some(Verdict::ParseError(rest.trim().to_string()));
        }
        if let Some(rest) = line.strip_prefix("CRASHED") {
            return Some(Verdict::Crashed(rest.trim().to_string()));
        }
        None
    }
}

/// Default replay budget. Replay of a synthesized witness is near-instant; the
/// budget exists only to bound adversarial / non-terminating candidates.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(5);

/// Separator between marker names in a multi-marker target.
///
/// Marker names are snake_case identifiers, so `,` cannot occur inside one.
pub const TARGET_SEP: char = ',';

/// Decide a replay verdict for `target` against the marker ids that actually
/// fired on the run.
///
/// `target` is one marker name, or several separated by [`TARGET_SEP`] — the
/// latter coming from a [`Path::merge`](rusmt_smt_stdlib::Path::merge) target,
/// where the SMT query asserts membership of *all* of the target's ids at once.
/// Replay mirrors that: the candidate is certified only if every listed marker
/// fired on the same execution.
///
/// Certification is a *subset* test, not equality: a run that fires the
/// targeted markers plus others still counts, because graceful (accumulating)
/// error handling makes extra markers routine.
fn verdict_for(target: &str, ids: BTreeSet<usize>) -> Verdict {
    let mut names = target
        .split(TARGET_SEP)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .peekable();
    // Guard the empty target: `all` on an empty iterator is vacuously true,
    // which would certify any candidate at all.
    let certified = names.peek().is_some() && names.all(|n| ids.contains(&marker_id(n)));
    if certified {
        Verdict::ReachedTarget
    } else {
        Verdict::ReachedOtherMarker(ids.into_iter().collect())
    }
}

/// Re-execute `source` through the concrete reference semantics and report
/// whether it reaches the marker(s) named by `target` (see `verdict_for`),
/// under a wall-clock `budget`.
///
/// See the module documentation for the soundness argument.
pub fn certify_imp(source: &str, target: &str, budget: Duration) -> Verdict {
    let program = match parse_imp_source(source) {
        Ok(p) => p,
        Err(e) => return Verdict::ParseError(e.to_string()),
    };

    // Run the (possibly non-terminating) replay on a worker thread so the
    // arbiter itself stays responsive and bounded.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(eval_com(program));
    });

    let result = match rx.recv_timeout(budget) {
        Ok(r) => r,
        Err(_) => return Verdict::Timeout,
    };

    match result {
        EvalResult::Ok(_) => Verdict::NoMarker,
        EvalResult::Err(path) => verdict_for(target, marker_ids(path)),
    }
}

// ---------------------------------------------------------------------------
// TOML: the same arbiter, with the (cheap, concrete) TOML parser as checker.
// ---------------------------------------------------------------------------

/// The named TOML markers. Every TOML `ParseResult::Err(Path::...)` branch is
/// named, giving each target a stable id shared between SMT and replay.
pub const TOML_NAMED_MARKERS: [&str; 182] = [
    "array_comment_missing_newline",
    "array_of_tables_inline_array",
    "array_open_after_ws_eof",
    "array_open_eof",
    "array_sep_eof_after_comma",
    "array_table_empty_name",
    "array_table_missing_close_char",
    "array_table_missing_close_eof",
    "array_table_open_eof",
    "array_table_redefines_closed_table",
    "array_table_redefines_inline_array",
    "array_table_redefines_inline_table",
    "array_table_redefines_std_table",
    "array_value_eof_after_value",
    "array_value_invalid_separator",
    "array_values_expected_value",
    "bare_key_invalid_start",
    "basic_string_invalid_char",
    "basic_string_missing_close_eof",
    "basic_string_newline",
    "boolean_invalid",
    "boolean_invalid_allcaps_true",
    "boolean_invalid_capital_false",
    "boolean_invalid_capital_true",
    "comment_invalid_char",
    "date_invalid_day",
    "date_invalid_month",
    "date_mday_first_char_not_digit",
    "date_mday_second_char_not_digit",
    "date_month_first_char_not_digit",
    "date_month_second_char_not_digit",
    "date_year_first_char_not_digit",
    "date_year_fourth_char_not_digit",
    "date_year_second_char_not_digit",
    "date_year_third_char_not_digit",
    "datetime_expect_partial_time_after_delim",
    "dotted_key_missing_segment",
    "dotted_key_redefines_array_table",
    "dotted_key_redefines_inline_table",
    "dotted_key_redefines_std_table",
    "float_duplicate_exponent",
    "float_exp_no_digit_after_e",
    "float_exp_only_exp_overflow_i32",
    "float_exp_only_exp_underflow_i32",
    "float_exp_only_final_overflow",
    "float_exp_only_pow10_overflow",
    "float_frac_combined_overflow",
    "float_frac_eof_combined_overflow",
    "float_frac_exp_overflow_i32",
    "float_frac_exp_underflow_i32",
    "float_frac_final_overflow",
    "float_frac_noexp_combined_overflow",
    "float_frac_pow10_overflow",
    "float_hex_char_after_underscore",
    "float_hex_char_in_part",
    "float_invalid_char_after_underscore",
    "float_invalid_inf_casing_allcaps",
    "float_invalid_inf_casing_titlecase",
    "float_invalid_nan_casing_allcaps",
    "float_invalid_nan_casing_camelcase",
    "float_invalid_nan_casing_titlecase",
    "float_multiple_underscores",
    "float_no_digit_after_dot",
    "float_no_digit_after_dot_eof",
    "float_underscore_at_end",
    "full_date_expect_mday_after_month",
    "full_date_first_dash_without_second",
    "full_date_first_dash_wrong_char",
    "full_date_second_dash_wrong_char",
    "inline_table_conflicting_keys",
    "inline_table_open_eof",
    "inline_table_redefined_closed",
    "inline_table_redefined_explicit",
    "inline_table_redefined_inline",
    "inline_table_sep_eof",
    "inline_table_sep_expected_comma",
    "inline_table_unterminated",
    "integer_bin_dec_digit_after_prefix",
    "integer_bin_dec_digit_after_underscore",
    "integer_bin_dec_digit_in_body",
    "integer_bin_double_underscore",
    "integer_bin_hex_digit_after_prefix",
    "integer_bin_hex_digit_after_underscore",
    "integer_bin_hex_digit_in_body",
    "integer_bin_invalid_char_after_prefix",
    "integer_bin_invalid_char_after_underscore",
    "integer_bin_no_digits_after_prefix",
    "integer_bin_oct_digit_after_prefix",
    "integer_bin_oct_digit_after_underscore",
    "integer_bin_oct_digit_in_body",
    "integer_bin_overflow",
    "integer_bin_prefix_uppercase",
    "integer_bin_underscore_after_prefix",
    "integer_bin_underscore_at_end",
    "integer_dec_double_underscore",
    "integer_dec_hex_char_after_underscore",
    "integer_dec_hex_char_in_body",
    "integer_dec_invalid_char_after_underscore",
    "integer_dec_invalid_char_after_zero",
    "integer_dec_leading_zero_digit",
    "integer_dec_leading_zero_underscore",
    "integer_dec_minus_sign_no_digits_eof",
    "integer_dec_minus_sign_no_digits_other",
    "integer_dec_overflow_plus_sign",
    "integer_dec_overflow_unsigned",
    "integer_dec_plus_sign_no_digits_eof",
    "integer_dec_plus_sign_no_digits_other",
    "integer_dec_underflow_minus_sign",
    "integer_dec_underscore_at_end",
    "integer_hex_double_underscore",
    "integer_hex_invalid_char_after_prefix",
    "integer_hex_invalid_char_after_underscore",
    "integer_hex_no_digits_after_prefix",
    "integer_hex_overflow",
    "integer_hex_prefix_uppercase",
    "integer_hex_underscore_after_prefix",
    "integer_hex_underscore_at_end",
    "integer_oct_dec_digit_after_prefix",
    "integer_oct_dec_digit_after_underscore",
    "integer_oct_dec_digit_in_body",
    "integer_oct_double_underscore",
    "integer_oct_hex_digit_after_prefix",
    "integer_oct_hex_digit_after_underscore",
    "integer_oct_hex_digit_in_body",
    "integer_oct_invalid_char_after_prefix",
    "integer_oct_invalid_char_after_underscore",
    "integer_oct_no_digits_after_prefix",
    "integer_oct_overflow",
    "integer_oct_prefix_uppercase",
    "integer_oct_underscore_after_prefix",
    "integer_oct_underscore_at_end",
    "integer_signed_bin_prefix",
    "integer_signed_hex_prefix",
    "integer_signed_oct_prefix",
    "key_value_missing_equals_char",
    "key_value_missing_equals_eof",
    "key_value_missing_value_eof",
    "key_value_missing_value_nomatch",
    "literal_string_invalid_char",
    "literal_string_missing_close_eof",
    "literal_string_newline",
    "ml_basic_escaped_newline_missing_newline",
    "ml_basic_missing_close_after_newline",
    "ml_basic_missing_close_no_newline",
    "ml_basic_open_eof",
    "ml_basic_quotes_without_content",
    "ml_literal_missing_close_after_newline",
    "ml_literal_missing_close_no_newline",
    "ml_literal_open_eof",
    "ml_literal_quotes_without_content",
    "numoffset_expect_colon_eof",
    "numoffset_expect_colon_wrong_char",
    "numoffset_expect_hour_after_sign",
    "numoffset_expect_minute_after_colon",
    "partial_time_expect_second_after_minute",
    "partial_time_second_colon_without_first",
    "quoted_key_multiline_basic",
    "quoted_key_multiline_literal",
    "std_table_duplicate",
    "std_table_empty_name",
    "std_table_missing_close_char",
    "std_table_missing_close_eof",
    "std_table_open_eof",
    "std_table_redefines_array_table",
    "std_table_redefines_implicit_table",
    "std_table_redefines_inline_table",
    "string_invalid_escape",
    "string_unicode_escape_invalid_hex",
    "string_unicode_escape_invalid_scalar",
    "time_hour_first_char_not_digit",
    "time_hour_out_of_range",
    "time_hour_second_char_not_digit",
    "time_minute_first_char_not_digit",
    "time_minute_out_of_range",
    "time_minute_second_char_not_digit",
    "time_secfrac_no_digit_after_dot_eof",
    "time_secfrac_no_digit_after_dot_nondigit",
    "time_second_first_char_not_digit",
    "time_second_out_of_range",
    "time_second_second_char_not_digit",
    "toml_expected_newline_between_expressions",
    "toml_table_merge_type_mismatch",
];

/// Send-able summary of a TOML parse outcome (we never move a `Value` across the
/// channel; only the fired marker ids, or Ok/NoMatch).
enum TomlOutcome {
    Err(BTreeSet<usize>),
    Ok,
    NoMatch,
}

fn toml_codepoints(source: &str) -> Seq<U32> {
    let mut seq: Seq<U32> = Seq::new();
    for ch in source.chars() {
        seq = seq.append(U32::from(ch as u32));
    }
    seq
}

/// Run the trusted TOML reference parser on `source` (under a wall-clock budget)
/// and report which marker ids fired. `Ok(None)` means the document parsed or no
/// marker fired; `Err(())` means the budget was exceeded.
pub fn toml_markers_fired(source: &str, budget: Duration) -> Result<Option<BTreeSet<usize>>, ()> {
    let input = toml_codepoints(source);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = match parse_toml(input) {
            TomlParseResult::Ok(_v, _) => TomlOutcome::Ok,
            TomlParseResult::Err(path) => TomlOutcome::Err(marker_ids(path)),
            TomlParseResult::NoMatch => TomlOutcome::NoMatch,
        };
        let _ = tx.send(outcome);
    });
    match rx.recv_timeout(budget) {
        Ok(TomlOutcome::Err(ids)) => Ok(Some(ids)),
        Ok(TomlOutcome::Ok) | Ok(TomlOutcome::NoMatch) => Ok(None),
        Err(_) => Err(()),
    }
}

/// Check that `source`, parsed by the TOML reference parser, reaches the
/// marker(s) named by `target`. The solver-first/model-second pipeline relies
/// on this after Z3 has accepted a pinned candidate: replay checks that the
/// concrete parser fires the same marker as the SMT query.
pub fn certify_toml(source: &str, target: &str, budget: Duration) -> Verdict {
    match toml_markers_fired(source, budget) {
        Ok(Some(ids)) => verdict_for(target, ids),
        Ok(None) => Verdict::NoMarker,
        Err(()) => Verdict::Timeout,
    }
}

/// Observable accept/reject behaviour of the TOML reference parser, as a
/// canonical string for differential comparison against an independent
/// implementation (cf. [`observe_imp`]). What matters across implementations is
/// whether a document is *accepted* and, if rejected, *which* spec rule the
/// reference flags:
///  * `OK`            — the document parsed (accepted);
///  * `ERR <marker>`  — a *named* error marker fired (rejected, labelled by rule);
///  * `ERR unnamed`   — a marker fired whose id is not in the catalog (rejected);
///  * `NOMATCH`       — rejected with no production matching;
///  * `TIMEOUT`       — replay exceeded the budget.
pub fn observe_toml(source: &str, budget: Duration) -> String {
    let input = toml_codepoints(source);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = match parse_toml(input) {
            TomlParseResult::Ok(_v, _) => TomlOutcome::Ok,
            TomlParseResult::Err(path) => TomlOutcome::Err(marker_ids(path)),
            TomlParseResult::NoMatch => TomlOutcome::NoMatch,
        };
        let _ = tx.send(outcome);
    });
    match rx.recv_timeout(budget) {
        Ok(TomlOutcome::Ok) => "OK".to_string(),
        Ok(TomlOutcome::NoMatch) => "NOMATCH".to_string(),
        Ok(TomlOutcome::Err(ids)) => {
            for name in TOML_NAMED_MARKERS {
                if ids.contains(&marker_id(name)) {
                    return format!("ERR {name}");
                }
            }
            "ERR unnamed".to_string()
        }
        Err(_) => "TIMEOUT".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Language registry: everything the model fallback loop needs per language.
// ---------------------------------------------------------------------------

/// A language registered with the synthesis pipeline.
///
/// The pipeline is language-agnostic: given a parser directory name it looks up
/// the language's oracle here and uses it to (a) describe the object language
/// to the proposer and (b) replay candidates through the reference semantics.
/// Registering a language here is what makes it eligible for the
/// solver-first/model-second synthesis loop in
/// `rusmt-smt-derive`.
pub struct LanguageOracle {
    /// Language name; equals the parser directory under `lang/src/`.
    pub name: &'static str,
    /// File extension of object-language sources (e.g. `imp`, `toml`).
    pub ext: &'static str,
    /// A short object-language description included in proposer prompts.
    pub brief: &'static str,
    /// Replay check: `(candidate source, target marker name(s), budget)`.
    /// The target is one marker name, or several separated by [`TARGET_SEP`]
    /// (a `Path::merge` target), in which case all of them must fire on the
    /// same run.
    pub certify: fn(&str, &str, Duration) -> Verdict,
}

/// Object-language brief for IMP (grammar of `lang/src/imp_parser.rs`).
const IMP_BRIEF: &str = "IMP/WHILE program. Grammar (whitespace-insensitive):\n\
  com  ::= stmt (';' stmt)*\n\
  stmt ::= 'skip' | ident ':=' aexp | '(' com ')'\n\
         | 'if' bexp 'then' stmt 'else' stmt | 'while' bexp 'do' stmt\n\
  aexp ::= integer arithmetic over '+' '-' '*' '/' with parentheses,\n\
           signed 64-bit literals, and identifiers\n\
  bexp ::= 'true' | 'false' | 'not' b | b 'and' b | b 'or' b\n\
         | aexp '==' aexp | aexp '<=' aexp\n\
Execution starts from an EMPTY store. Reading a variable that was never \
assigned is the error marker `undefined_variable`; dividing by zero is the \
error marker `division_by_zero`. Example program: A := (1 + 2); B := (A * A)";

/// Object-language brief for TOML (the v1.1.0 reference parser).
const TOML_BRIEF: &str = "A TOML v1.1.0 document (UTF-8 text): key/value pairs \
(bare or quoted keys, '=', values: strings, integers, floats, booleans, \
datetimes, arrays, inline tables), [table] and [[array-of-table]] headers, \
'#' comments. The parser flags specification violations with named markers.";

/// The registered language oracles.
pub static ORACLES: [LanguageOracle; 2] = [
    LanguageOracle {
        name: "imp",
        ext: "imp",
        brief: IMP_BRIEF,
        certify: certify_imp,
    },
    LanguageOracle {
        name: "toml",
        ext: "toml",
        brief: TOML_BRIEF,
        certify: certify_toml,
    },
];

/// Look up the oracle for a language by its parser-directory name.
pub fn oracle_for(name: &str) -> Option<&'static LanguageOracle> {
    ORACLES.iter().find(|o| o.name == name)
}

// ---------------------------------------------------------------------------
// Process-isolated replay.
//
// The in-process replay checks above bound *time* (a worker thread plus
// `recv_timeout`), but they cannot bound *stack*: a candidate like
// `while true do skip` drives the recursive reference evaluator into a stack
// overflow, which aborts the whole process — a thread cannot survive it. An
// external proposer or a solver model under depth-bounded unrolling can produce
// exactly such candidates, so the pipeline replays every candidate in a separate
// process: a crash there is reported as
// `Verdict::Crashed` and rejects the candidate instead of killing the run.
// ---------------------------------------------------------------------------

/// Env vars carrying the replay request to the subprocess.
const ENV_CERTIFY_LANG: &str = "RUSMT_CERTIFY_LANG";
/// See [`ENV_CERTIFY_LANG`].
const ENV_CERTIFY_TARGET: &str = "RUSMT_CERTIFY_TARGET";

/// Subprocess entry hook for isolated replay. Any binary that wants to *call*
/// [`certify_isolated`] must invoke this as the first statement of its `main`:
/// when the process was spawned as a replay child (the `RUSMT_CERTIFY_*` env
/// vars are set) it reads the candidate from stdin, certifies it in-process,
/// prints the wire-encoded [`Verdict`] on stdout, and exits — never returning
/// to the caller's `main`.
pub fn maybe_subprocess_entry() {
    let (Ok(lang), Ok(target)) = (
        std::env::var(ENV_CERTIFY_LANG),
        std::env::var(ENV_CERTIFY_TARGET),
    ) else {
        return;
    };
    let mut source = std::string::String::new();
    let verdict = match std::io::Read::read_to_string(&mut std::io::stdin(), &mut source) {
        Err(e) => Verdict::Crashed(format!("replay child could not read stdin: {e}")),
        Ok(_) => match oracle_for(&lang) {
            None => Verdict::Crashed(format!("no oracle registered for language `{lang}`")),
            // The budget here is a backstop; the parent enforces the real one.
            Some(oracle) => (oracle.certify)(&source, &target, DEFAULT_BUDGET),
        },
    };
    println!("{}", verdict.to_wire());
    std::process::exit(0);
}

/// Replay `source` against the marker(s) named by `target` (one name, or
/// several separated by [`TARGET_SEP`]) of language `lang` in a
/// **separate process** (see the module note above), under a wall-clock
/// `budget` enforced by the parent. The child is the current executable
/// re-spawned through [`maybe_subprocess_entry`].
pub fn certify_isolated(lang: &str, source: &str, target: &str, budget: Duration) -> Verdict {
    use std::process::{Command, Stdio};

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return Verdict::Crashed(format!("cannot locate current executable: {e}")),
    };
    let child = Command::new(exe)
        .env(ENV_CERTIFY_LANG, lang)
        .env(ENV_CERTIFY_TARGET, target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return Verdict::Crashed(format!("cannot spawn replay child: {e}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        // A write failure means the child died immediately; the wait loop below
        // will pick the failure up.
        let _ = std::io::Write::write_all(&mut stdin, source.as_bytes());
    }

    // Poll for completion under the budget; kill on expiry.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > budget {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Verdict::Timeout;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Verdict::Crashed(format!("replay child wait failed: {e}")),
        }
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return Verdict::Crashed(format!("replay child output unavailable: {e}")),
    };
    let stdout = std::string::String::from_utf8_lossy(&out.stdout);
    match stdout.lines().next().and_then(Verdict::from_wire) {
        Some(v) => v,
        None => {
            // No verdict line: the child died abnormally (abort, signal, stack
            // overflow). Surface its stderr head for diagnosis.
            let stderr = std::string::String::from_utf8_lossy(&out.stderr);
            Verdict::Crashed(format!(
                "replay child exited without a verdict ({}): {}",
                out.status,
                stderr.lines().next().unwrap_or("")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The two named markers of the IMP reference semantics (lang/src/imp/mod.rs).
    const DIV0: &str = "division_by_zero";
    const UNDEF: &str = "undefined_variable";

    fn certify(src: &str, target: &str) -> Verdict {
        certify_imp(src, target, DEFAULT_BUDGET)
    }

    #[test]
    fn division_by_zero_witness_is_certified_for_its_target() {
        assert_eq!(certify("A := (0 / 0)", DIV0), Verdict::ReachedTarget);
    }

    #[test]
    fn a_multi_marker_target_needs_every_marker_to_fire() {
        // IMP short-circuits on the first error, so one run cannot fire both
        // markers; the merged target is therefore rejected, while each
        // single-marker target is certified by its own witness.
        let both = format!("{DIV0}{TARGET_SEP}{UNDEF}");
        assert_eq!(certify("A := (0 / 0)", DIV0), Verdict::ReachedTarget);
        assert!(!certify("A := (0 / 0)", &both).is_certified());
        assert!(!certify("A := v0", &both).is_certified());
    }

    #[test]
    fn an_empty_target_certifies_nothing() {
        // `all` over no names is vacuously true; the guard in `verdict_for`
        // must stop that from certifying an arbitrary candidate.
        assert!(!certify("A := (0 / 0)", "").is_certified());
        assert!(!certify("A := (0 / 0)", ",").is_certified());
    }

    #[test]
    fn extra_markers_do_not_break_certification() {
        // Certification is a subset test: whitespace and ordering around the
        // names are tolerated, and a singleton target still matches.
        assert_eq!(
            certify("A := (0 / 0)", &format!(" {DIV0} ")),
            Verdict::ReachedTarget
        );
    }

    #[test]
    fn undefined_variable_witness_is_certified_for_its_target() {
        // Reading `v0` from the empty store fires the undefined-variable marker.
        assert_eq!(certify("A := v0", UNDEF), Verdict::ReachedTarget);
    }

    #[test]
    fn a_candidate_for_the_wrong_target_is_rejected() {
        // `A := (0 / 0)` reaches division-by-zero, NOT undefined-variable: the
        // arbiter certifies per-target, not "some marker fired".
        assert!(matches!(
            certify("A := (0 / 0)", UNDEF),
            Verdict::ReachedOtherMarker(_)
        ));
    }

    #[test]
    fn a_well_formed_program_that_reaches_no_marker_is_rejected() {
        assert_eq!(certify("A := 1", DIV0), Verdict::NoMarker);
    }

    #[test]
    fn malformed_source_is_rejected() {
        assert!(matches!(
            certify("@@@ not imp", DIV0),
            Verdict::ParseError(_)
        ));
    }

    #[test]
    fn an_llm_style_proposal_is_certified_for_its_target() {
        // The example used in the paper (a different div-by-zero program than the
        // solver's witness): proposed by a language model, certified by replay.
        assert_eq!(certify("B := (7 / (2 - 2))", DIV0), Verdict::ReachedTarget);
    }

    /// One model-free proposed document per named TOML target. These are the
    /// witnesses the *solver* could not synthesize within budget (TOML sweep:
    /// 0/182); each is checked here, in milliseconds, by concrete replay
    /// through the reference parser — the asymmetry the paper's evaluation
    /// reports (solving is out of budget; checking a determined candidate is a
    /// concrete parse). Together they are a small, replay-checked conformance
    /// suite for these ten specification violations.
    const TOML_DIRECT_WITNESSES: [(&str, &str); 10] = [
        ("boolean_invalid", "a = FALSE"),
        ("bare_key_invalid_start", "@ = 1"),
        ("time_hour_out_of_range", "a = 25:32:00"),
        ("time_minute_out_of_range", "a = 00:99:00"),
        ("time_second_out_of_range", "a = 00:00:99"),
        ("date_invalid_month", "a = 2020-13-01"),
        ("date_invalid_day", "a = 2020-02-30"),
        ("float_no_digit_after_dot", "a = 1.x"),
        ("string_invalid_escape", "a = \"\\q\""),
        ("inline_table_unterminated", "a = {b = 1"),
    ];

    #[test]
    fn toml_named_targets_recovered_by_direct_proposal_and_replay() {
        // A model-free proposer (these hand-authored documents) recovers every
        // named TOML target by direct proposal + replay certification, where the
        // solver synthesized none. This is the conformance-suite payoff the
        // paper claims, demonstrated end-to-end on the realistic case study.
        for (target, src) in TOML_DIRECT_WITNESSES {
            assert_eq!(
                certify_toml(src, target, DEFAULT_BUDGET),
                Verdict::ReachedTarget,
                "expected {src:?} to reach named TOML marker `{target}`",
            );
        }
    }

    #[test]
    fn toml_recovery_is_per_target_not_some_marker() {
        // Soundness is per-target: the boolean_invalid witness fires a marker,
        // but NOT the date_invalid_month marker, so it is rejected for that
        // target (cf. the IMP per-target test). The arbiter never accepts on
        // "some marker fired".
        assert!(matches!(
            certify_toml("a = FALSE", "date_invalid_month", DEFAULT_BUDGET),
            Verdict::ReachedOtherMarker(_)
        ));
    }

    #[test]
    fn observable_behaviour_is_canonical_across_the_three_outcome_classes() {
        // `observe_imp` is the differential-comparison API: an independent
        // implementation is conformant iff it matches these strings exactly.
        assert_eq!(observe_imp("A := 1; B := (A + 2)"), "OK A=1;B=3");
        assert_eq!(observe_imp("A := (0 / 0)"), "ERR division_by_zero");
        assert_eq!(observe_imp("A := v0"), "ERR undefined_variable");
        assert!(observe_imp("@@@ not imp").starts_with("PARSE_ERROR"));
    }
}

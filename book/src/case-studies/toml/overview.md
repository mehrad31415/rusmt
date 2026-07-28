## TOML v1.1.0 case study

The `rusmt-lang` crate contains a TOML v1.1.0 parser implemented *in the
RuSmt DSL*. It is a **substantial executable parser/specification** (we make no
claim of a verified-complete spec — there is no full official compliance-suite
run): every grammar rule from the
[TOML 1.1.0 spec](https://toml.io/en/v1.1.0#spec) is realised as a
`#[smt_fn]` parsing function over `Seq<U32>` (Unicode code points).

Concretely, this gives:

- **Concrete execution**: parse a real `.toml` file and pretty-print its AST
  (`cargo run -p rusmt-lang -- toml ...`).
- **Symbolic compilation**: lift the parser into SMT-LIB via
  `rusmt-smt-derive`, ready for Z3 to query.

### Synthesis status

Every error/spec-violation branch is a `Path::named` marker — **186 named
markers**, one per call site and one synthesis target each, spanning both
*syntactic* (malformed tokens, unterminated constructs) and *semantic* (overflow,
out-of-range date/time, redefined keys) errors.

**Search** is at Z3's frontier. The TOML grammar is large and deeply recursive;
when `rusmt-smt-derive` lifts it into SMT, the query "find an input that reaches
target N" exceeds Z3's budget: in a full sweep of all 186 targets at a 20 s
per-target budget, **no target produced a witness — every query times out**
(0 `sat`, 0 `unknown`, 0 backend errors). Of the 186, **182 are
reachable-or-hard** and **4 appear structurally unreachable** (defensive branches
in the string-content, float-exponent, and dotted-key paths; see
`generated-suites/toml-proposer/uncovered-markers.txt`) — a timeout on a
suspected-dead target is *not* a scaling data point. For the 182, this is **a
scaling-frontier finding, not a defect**: each free code point forces full
symbolic recursion through the
parser.

**Validation is restored by an array-free re-encoding** (empirically
regression-preserving — all tests and the differential corpus produce identical
output across encodings; no formal equivalence proof). Tables were originally
`Array<String, Value>` — an array of a *recursive* datatype — under which Z3
returned `unknown ("incomplete (theory array)")` even for a fully concrete input.
Re-encoding tables as an **array-free** assoc-list datatype (no observed behavior
change — all tests and the differential corpus produce identical output; no formal
equivalence proof) lets Z3 *validate* a determined candidate in **sub-second**
time. This is exactly the asymmetry the
proposer recovery route exploits: a proposer suggests a complete candidate,
**Z3 validates** the macro-inlined query, and **replay re-certifies** — so the
solver stays in the loop (never bypassed), recovering a named-marker conformance
suite even though blind search is out of reach. That suite, run differentially
against two mature TOML 1.0 parsers, exposes two genuine inter-implementation
divergences (see `lang/tests/toml_differential_results.md`).

For an end-to-end demo where *search itself* closes the loop, see the
[IMP case study](../imp/overview.md) (dynamic semantics) and the
[STLC case study](../stlc/overview.md) (static semantics, via bounded unrolling).

### Running synthesis

```bash
# Default: native recursion (define-funs-rec).
cargo run -p rusmt-smt-derive -- toml parse_toml

# Bounded-recursion unrolling at depth N.
cargo run -p rusmt-smt-derive -- toml parse_toml k=3
```

Output goes under `lang/src/synthesis/toml/z3_chc/`, with one response file per
path-condition target: `target_<N>/response.txt` for a raw solver verdict, or
`target_<N>/response.toml` for a recovered, replayable TOML witness.
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

Every error/spec-violation branch is a `Path::named` marker — **183 named
markers**, one per call site and one synthesis target each, spanning both
*syntactic* (malformed tokens, unterminated constructs) and *semantic* (overflow,
out-of-range date/time, redefined keys) errors.

**Search** is at Z3's frontier. The TOML grammar is large and deeply recursive;
when `rusmt-smt-derive` lifts it into SMT, the query "find an input that reaches
target N" exceeds Z3's budget: in the measured Stage-1 run, **no target produced
a witness** at affordable budgets. This is a scaling-frontier finding, not a
defect: each free code point forces full symbolic recursion through the parser.

Stage 2 exploits the opposite regime: Z3 can decide a determined candidate much
more easily than it can search for a symbolic document. The model reads the
emitted SMT-LIB excerpt and Z3 feedback, proposes a complete TOML document, and
RuSmt pins that document by adding one equality to the original query. Only a
`sat` result from Z3 plus replay through the concrete parser becomes a witness.
The framework writes the accepted inputs as the conformance suite and records
the marker-level ledger that `specops/report.py` summarizes.

The generated suite can then be run against independent TOML parsers. Every
accept/reject divergence is a candidate bug **in either direction**: the suite
tests the reference against the standard just as much as it tests
implementations against the reference.

Fresh coverage, divergence counts, and triage belong to the run being reported;
do not copy old artifact numbers into this page.

For an end-to-end demo where *search itself* closes the loop, see the
[IMP case study](../imp/overview.md) (dynamic semantics and AST rendering).

### Running synthesis

```bash
# Default: native recursion (define-funs-rec).
cargo run -p rusmt-smt-derive -- toml parse_toml

# Bounded-recursion unrolling at depth N.
cargo run -p rusmt-smt-derive -- toml parse_toml k=3
```

By default, accepted witnesses are also written as a conformance suite under
`/tmp/rusmt-suite/toml`; pass `--suite-out <dir>` to choose another location, or
`--no-suite` to skip suite materialization. Because TOML's top-level input is the
document text, no separate renderer is needed: each accepted model is already the
TOML program to run against implementations.

<h1 style="text-align: center;">RuSmt</h1>
<p style="text-align: center;">Implemented in Rust | Transpiled to SMT</p>

---

## Paper and artifact

This repository accompanies the SpecOps 2026 paper *RuSMT: An Executable
Semantics as Conformance Oracle and Test Suite Synthesizer*
(DOI [10.1145/3842652.3843196](https://doi.org/10.1145/3842652.3843196)).

The archived artifact, i.e., this repository together with the recorded run
behind every number in the paper, is on Zenodo:
[10.5281/zenodo.22095547](https://doi.org/10.5281/zenodo.22095547).
The recorded run itself is in [`artifact/`](artifact/README.md).

## Introduction

**RuSmt** is a Rust embedded DSL for writing executable language specifications. Each specification:
- runs as a Rust program on concrete inputs;
- compiles to SMT-LIB for symbolic reasoning in Z3.

## The Problem

Reaching a *specific* error case in an interpreter or compiler is hard — usually not because you don't know the case exists, but because constructing a concrete input that drives the program to it is hard:

- **Hand-written tests** leave you to *construct the triggering input yourself*. For a recursive parser or evaluator, the input that reaches a deep error path can be far from obvious, and you write one concrete case at a time.
- **Fuzzing** constructs inputs for you but steers randomly, so a narrow, deep case may never be hit.

RuSmt does **not** claim to remove the burden both share — deciding *which* cases matter. The author still names the case, by dropping a marker (`Path::named("...")`) where your specification already distinguishes it. What RuSmt removes is the *input-construction* step:

1. The framework automatically *transpiles* your program to SMT formulas for symbolic analysis.
2. Z3 is automatically invoked in the backend to *synthesize* a concrete input that reaches each marker. If Z3 times out or returns `unknown`, RuSmt asks the configured model for a candidate and then asks Z3 to decide that pinned candidate against the original marker query.
3. Each accepted input is *replayed* through the same Rust semantics to check that the SMT lift and the concrete oracle agree on the targeted marker.
4. **Find bugs** in real-world implementations through conformance testing: the synthesized inputs are run as a conformance test suite comparing your implementation against others on exactly the error cases you flagged.

**Honest caveat:** coverage is bounded by the markers you place. A case you never mark is a case RuSmt never synthesizes for — the same "limited coverage" limit as a hand-written suite, shifted from *inputs* to *program points*. The gain is narrower than "solved": you enumerate *semantic events* (one marker) instead of *concrete inputs* (many tests), and Z3, not you, builds the witness that reaches each one.

## How to use the framework

Using only the types and methods of the DSL you can write your own software — for example, an interpreter or a compiler. Wherever necessary, drop in a marker — `Path::named("...")` — to flag a program point you want to reach, such as an error case. Your code is lifted to SMT-LIB, where Z3 searches for concrete inputs that drive the program to each marked point. The same code, run as ordinary Rust, is the concrete oracle used to replay accepted witnesses and check that they reach the *targeted* marker.

**The primitives are written to match Z3.** Each DSL primitive's concrete Rust behaviour *matches its Z3 translation* — `rust(f, x) == z3(f, x)` — so you can compose primitives and trust that the code you run is the code Z3 reasons about. This is what we mean when we call the transpilation **sound**, and it is what makes authoring easy: most of the time you simply use the primitives.

**But matching Z3 is not the same as matching your language.** A primitive models a *Z3 theory*, not your target language, and Z3 fixes its own answers on edge cases — integer overflow, division by zero, `NaN`, truncation-vs-floor, shifts wider than the word — that can differ both from the language you are specifying and from what a mainstream language would do. The book documents each such case and the reason. Where they differ, the gap is closed with an explicit `if`-guard; *which* gap it is decides where that guard lives:

- **Inside the stdlib** — a Rust-vs-Z3 gap, where a naïve Rust implementation would not agree with the Z3 formula. The primitive is written to match Z3 on *every* input, so `rust(f) == z3(f)` holds unconditionally and you never see the gap.
- **In your interpreter** — a Z3-vs-target-language gap, where the primitive faithfully models Z3 but Z3 is not what your language requires. Here the guard is the *author's* job: branch the edge case out, and fall through to the primitive only where it already agrees with your semantics.

This contract is **checked, not merely asserted**: the differential harness replays every primitive against its Z3 formula on thousands of inputs and reports the disagreements (currently none). Given it:

- **Soundness is a property of the translation** — established once by the contract above and re-checkable on demand by that harness. Each accepted input is **replayed** through the concrete semantics and kept only if it fires the *targeted* marker. A wrong model proposal is rejected by Z3 or by replay.
- **completeness is bounded only by the solver.** By default the full specification is handed to Z3, so a reachable marker can be discovered at any recursion depth; bounding the recursion to a fixed depth is an *optional* fallback we use only to make a heavy query more tractable, not a restriction imposed by default. What does limit us is the solver itself. The formulas we generate combine recursively-defined functions and quantifiers with the base theories, and satisfiability for this fragment is _undecidable_ in general. On such formulas Z3 is sound but incomplete — it behaves as a semi-decision procedure: it may find a witness when one exists, but it may equally return `unknown` or exhaust its time and memory budget without reaching a verdict. Whenever that happens, a genuinely reachable marker can go undiscovered, and we make no claim to the contrary.

## AI in the loop

The suite-generation pipeline can use a language model after Z3 fails to find a
witness directly. Configure any command that reads a prompt on stdin and writes a
candidate on stdout:

```bash
export RUSMT_LLM_CMD="./my_proposer"    # reads prompt on stdin, writes candidate on stdout
export RUSMT_LLM_CACHE=/tmp/rusmt-cache
```

**1. Co-solving.** Z3 is asked first. If Z3 returns a witness, that witness is
replayed through the concrete reference semantics and written to the suite. If Z3
times out or returns `unknown` for a named marker, the model sees the emitted
SMT-LIB, the marker name, and previous Z3 feedback, then proposes a candidate
input. RuSmt pins that input by adding one equality to the original query. `sat`
means the candidate reaches the marker; `unsat` means it does not; timeout or
`unknown` means no witness was accepted for that round. The model never approves
its own candidate.

**2. Authoring (experimental, unevaluated).** Writing the interpreter in the DSL
is the tedious part of using RuSmt. The `author` mode has the model draft it from
a plain-English specification — then every draft is run through a set of automatic
*gates*, and each failure is fed back to the model for repair, looping up to
`rounds=N` (default 4).

> We make **no claim** for this mode and report no numbers for it. Proposing a
> formal artifact, gating it mechanically and feeding the complaint back is the
> design of AutoSpec (CAV 2024) and SpecGen; ours differs only in that no
> reference exists to check the draft against, so the gates establish internal
> consistency and nothing about whether the semantics is *right*. It is a
> drafting aid. Evaluating it properly is future work.

```bash
cargo run -p rusmt-smt-derive -- author my_language_spec.md draft.rs rounds=6 \
    --examples my_language_examples.tsv
```

A draft must clear two kinds of gates.

*Mechanical — "is it even valid code?"* The draft must (1) parse in the DSL and
pass sort-checking, (2) transpile to SMT-LIB without error, and (3) declare at
least one named marker (`Path::named`).

*Behavioral — "does it actually behave the way it should?"* Here the framework
runs the draft's own logic through Z3 and inspects the results:

- **No dead markers.** For every marker the draft declares, Z3 searches — at
  `k=0`, i.e. the draft's genuine recursion — for *any* input that makes the
  marker fire. If Z3 proves none exists, that marker is **dead code**: the draft
  declared an error case it can never actually reach. The failure is fed back by
  name to the model for repair. Note that unknown or timeout is *not* a dead marker: the draft may or may not be correct, but the solver cannot prove it either way. Thus it is not a failure to the model.
- **Your examples hold.** An optional `--examples` file lists `INPUT<TAB>EXPECT`
  test cases — `ok` (no marker should fire), `err:<marker>` (that specific marker
  should fire), or `nomatch` (some marker should fire) — supplied by *the author*. The framework pins each input into the draft and checks which
  markers actually fire. A mismatch becomes precise feedback: *"on input
  `k = true` the spec says `ok`, but your draft fires `boolean_invalid`."*

This loop is what a plain chat session cannot reproduce: each repair round
consumes *solver-generated counterexamples about the draft's real behavior*. Even so, the admitted draft is **not
trusted**: the author reviews it, owns it, and validates it downstream.

## Case studies

The following two case studies are chosen as our first demonstration of RuSmt's capabilities:

| Case study  | Location | Status |
|-------------|----------|--------|
| **TOML v1.1.0 parser** | `lang/src/toml/` | Substantial executable parser/specification (**183 named markers**). Z3-alone search produces no witnesses at affordable budgets; the Z3-first/model-second loop generates the conformance suite, and every uncovered marker is reported by name. Running that suite against four TOML parsers surfaces accept/reject divergences in both directions — parser defects, and markers of our own that turn out to test behaviour the standard leaves to the implementation. Counts: see `specops/report.py`. See `book/src/case-studies/toml/`. |
| **IMP / WHILE** | `lang/src/imp/` | Canonical small imperative language from Winskel, *The Formal Semantics of Programming Languages* (MIT Press, 1993). End-to-end synthesis works at native recursion (`k=0`): Z3 returns models for both markers, the printer renders them as `.imp` source, and replay confirms the marker fires. See `book/src/case-studies/imp/`. |

## Build

### Prerequisites

- Rust (edition 2024) — toolchain pinned by `rust-toolchain.toml`.
- A **system `z3` CLI on `PATH`** (we used **Z3 4.15.4**) — the only external dependency. The backend and the `z3 -smt2 …` reproduction commands below all use this binary, so all of the paper's results were obtained with this setup. Install: macOS `brew install z3`, Debian/Ubuntu `sudo apt install z3`, or download a 4.15.4 release.

### Optional developer tools (for the `make` targets)

None of these are required to build, test, or run RuSmt — they are only needed to
use the convenience targets in the `Makefile`.

- **`make`** — runs the targets themselves (`make lint`, `make cloc`, `make docs`).
   macOS: included with the Xcode CLT (`xcode-select --install`); Debian/Ubuntu: `sudo apt install make`.
- **`cloc`** — required by `make cloc` (counts lines of Rust). macOS:
  `brew install cloc`; Debian/Ubuntu: `sudo apt install cloc`.
- **`mdbook`** — required by `make docs` (builds and serves the book). Install with
  `cargo install mdbook`.
- `make lint` needs no extra install: it calls `cargo fmt` and `cargo clippy`, both
  already pinned as `components` in `rust-toolchain.toml`.

### Build

```bash
cargo build --workspace
```

## Tiny end-to-end example (IMP)

```bash
# 1. Run an IMP program concretely.
cargo run -p rusmt-lang -- imp lang/imp/input/factorial.imp

# 2. Synthesize the IMP conformance suite. Z3 solves IMP directly, so this
#    normally does not call the proposer.
cargo run -p rusmt-smt-derive -- imp eval_com \
  --out-dir /tmp/rusmt-imp-out \
  --suite-out /tmp/rusmt-suite/imp

# 3. The printer emits each `sat` response as a runnable .imp file.
cat /tmp/rusmt-suite/imp/division_by_zero.imp

# 4. Replay the witness through the same interpreter — it must reach the marker.
cargo run -p rusmt-lang -- imp /tmp/rusmt-suite/imp/division_by_zero.imp
```

## Pipeline

```
┌──────────────────────────────────────────────────────────┐
│  Annotated Rust interpreter (e.g. lang/src/imp/mod.rs)   │
│  — uses rusmt-smt-stdlib types                         │
│  — uses #[smt_type], #[smt_fn] from rusmt-smt-remark   │
└─────────────────┬────────────────────────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
        ▼                   ▼
  Concrete exec        SMT transpiler  (rusmt-smt-derive)
  (cargo run)               │
                            ▼
                        Backend
              (SMT-LIB2 + z3 subprocess)
                            │
                            ▼
                  Z3 verdict (sat/unsat/unknown/timeout)
                            │
             timeout/unknown│
                            ▼
                 model proposes candidate
                            │
                            ▼
             Z3 decides query + input equality
                          │
                          ▼
                  Witness renderer (lang/src/imp/render.rs)
                          │
                          ▼  (sat case)
                runnable source in target_*/response
                          │
                          ▼
              Conformance / replay test
```

## Documentation

The RuSmt Book (mdBook under `book/`) is the long-form reference for both
users of the DSL and contributors to the framework.

```bash
make docs           # builds and serves the book locally
```

## References

- Glynn Winskel, *The Formal Semantics of Programming Languages: An Introduction.* MIT Press, 1993. (Ch. 2 specifies the IMP/WHILE language used as a case study.)
- The TOML v1.1.0 specification: <https://toml.io/en/v1.1.0>.

## License

GPL-3.0-or-later (see `LICENSE`).

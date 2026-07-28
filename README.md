<h1 style="text-align: center;">RuSmt</h1>
<p style="text-align: center;">Implemented in Rust | Transpiled to SMT</p>

---

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
2. Z3 is automatically invoked in the backend to *synthesize* a concrete input that reaches each marker — a witness you didn't have to write.
3. Each synthesized input is *replayed* through the same Rust semantics to certify it actually fires the targeted marker (the concrete run is the trusted arbiter).
4. **Find bugs** in real-world implementations through conformance testing: the synthesized inputs are run as a conformance test suite comparing your implementation against others on exactly the error cases you flagged.

**Honest caveat:** coverage is bounded by the markers you place. A case you never mark is a case RuSmt never synthesizes for — the same "limited coverage" limit as a hand-written suite, shifted from *inputs* to *program points*. The gain is narrower than "solved": you enumerate *semantic events* (one marker) instead of *concrete inputs* (many tests), and Z3, not you, builds the witness that reaches each one.

## How to use the framework

Using only the types and methods of the DSL you can write your own software — for example, an interpreter or a compiler. Wherever necessary, drop in a marker — `Path::named("...")` — to flag a program point you want to reach, such as an error case. Your code is lifted to SMT-LIB, where Z3 searches for concrete inputs that drive the program to each marked point. The same code, run as ordinary Rust, is the trusted **arbiter**: any candidate model can be validated by re-executing it and checking that it reaches the *targeted* marker.

**The primitives are written to match Z3.** Each DSL primitive's concrete Rust behaviour *matches its Z3 translation* — `rust(f, x) == z3(f, x)` — so you can compose primitives and trust that the code you run is the code Z3 reasons about. This is what we mean when we call the transpilation **sound**, and it is what makes authoring easy: most of the time you simply use the primitives.

**But matching Z3 is not the same as matching your language.** A primitive models a *Z3 theory*, not your target language, and Z3 fixes its own answers on edge cases — integer overflow, division by zero, `NaN`, truncation-vs-floor, shifts wider than the word — that can differ both from the language you are specifying and from what a mainstream language would do. The book documents each such case and the reason. Where they differ, the gap is closed with an explicit `if`-guard; *which* gap it is decides where that guard lives:

- **Inside the stdlib** — a Rust-vs-Z3 gap, where a naïve Rust implementation would not agree with the Z3 formula. The primitive is written to match Z3 on *every* input, so `rust(f) == z3(f)` holds unconditionally and you never see the gap.
- **In your interpreter** — a Z3-vs-target-language gap, where the primitive faithfully models Z3 but Z3 is not what your language requires. Here the guard is the *author's* job: branch the edge case out, and fall through to the primitive only where it already agrees with your semantics.

This contract is **checked, not merely asserted**: the differential harness replays every primitive against its Z3 formula on thousands of inputs and reports the disagreements (currently none). Given it:

- **Soundness is a property of the translation** — established once by the contract above and re-checkable on demand by that harness. Each synthesized input is **replayed** through the concrete semantics and accepted only if it fires the *targeted* marker — the concrete run is the trusted arbiter, so an uncertified solver model or a wrong LLM proposal is rejected.
- **completeness is bounded only by the solver.** By default the full specification is handed to Z3, so a reachable marker can be discovered at any recursion depth; bounding the recursion to a fixed depth is an *optional* fallback we use only to make a heavy query more tractable, not a restriction imposed by default. What does limit us is the solver itself. The formulas we generate combine recursively-defined functions and quantifiers with the base theories, and satisfiability for this fragment is _undecidable_ in general. On such formulas Z3 is sound but incomplete — it behaves as a semi-decision procedure: it may find a witness when one exists, but it may equally return `unknown` or exhaust its time and memory budget without reaching a verdict. Whenever that happens, a genuinely reachable marker can go undiscovered, and we make no claim to the contrary.

## AI in the loop

Two pipeline features use a language model as an *untrusted proposer*; both are
off unless you configure a proposer command:

```bash
export RUSMT_LLM_CMD="claude -p"        # Claude Code CLI (must be installed + logged in)
# export RUSMT_LLM_CMD="llm -m gpt-4o"  # Simon Willison's `llm` CLI (needs its own API key)
# export RUSMT_LLM_CMD="./my_script.sh" # any local script, even a deterministic one
```

**1. Guided synthesis (the LLM scaffolds, Z3 completes).** Z3 is asked first.
If Z3 returns a witness, that witness is replayed through the concrete reference
semantics and the verdict is recorded. If Z3 instead fails on a *named* marker —
timeout, `unknown`, a bound-limited `unsat`, or a spurious model that replay
rejects — the proposer takes over. The proposer makes a **direct** guess: the model proposes a whole object-language input
outright. It also does **guided** scaffolding: rather than write the input, each
round the model emits a structural *scaffold* — prefix / suffix / contains /
char-at-index / length constraints on the input text — which is translated into
assertions. Scaffolding is *strengthening-only*, so every model of the
strengthened query is still a model of the original marker query. This makes the process sound. Z3 then
completes the scaffold under a short budget (`RUSMT_GUIDE_Z3_SECS`), and its
verdict drives the next round: `sat` → decode and replay-certify; `unsat` → *the
scaffold contradicts the marker, relax it* (never a claim of unreachability);
timeout → *constrain further*.

**2. Authoring with behavioral self-validation.** Writing the interpreter in the
DSL is the tedious part of using RuSmt. The `author` mode has the model draft it
from a plain-English specification — then every
draft is run through a set of automatic *gates*, and each failure is fed back to
the model for repair, looping up to `rounds=N` (default 4).

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
| **TOML v1.1.0 parser** | `lang/src/toml/` | Substantial executable parser/specification (**183 named markers**). Synthesis *search* hits Z3's scaling frontier: a measured 183-target sweep (20 s/target) produces no witness (0 `sat`) but Z3 *validate* a determined candidate sub-second, so the proposer recovery route (propose → Z3-validate → replay) recovers a conformance suite. See `book/src/case-studies/toml/`. |
| **IMP / WHILE** | `lang/src/imp/` | Canonical small imperative language from Winskel, *The Formal Semantics of Programming Languages* (MIT Press, 1993). End-to-end synthesis works at native recursion (`k=0`): Z3 returns models for both markers, the printer renders them as `.imp` source, and replay confirms the marker fires. See `book/src/case-studies/imp/`. |

## Build

### Prerequisites

- Rust (edition 2024) — toolchain pinned by `rust-toolchain.toml`.
- A **system `z3` CLI on `PATH`** (we used **Z3 4.15.4**) — the only external dependency. The backend, the guided loop's persistent `z3 -in` session, and the `z3 -smt2 …` reproduction commands below all use this binary, so all of the paper's results were obtained with this setup. Install: macOS `brew install z3`, Debian/Ubuntu `sudo apt install z3`, or download a 4.15.4 release.

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

# 2. Synthesise inputs that fire the path-condition markers in eval_com.
#    k=N sets bounded-recursion unrolling; omit it for native recursion (k=0).
cargo run -p rusmt-smt-derive -- imp eval_com               # native recursion
cargo run -p rusmt-smt-derive -- imp eval_com k=3           # depth-3 unrolling

# 3. The printer emits each `sat` response as a runnable .imp file.
cat lang/src/synthesis/imp/z3_chc/target_0/response.imp

# 4. Replay the witness through the same interpreter — it must reach the marker.
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.imp
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
              (SMT-LIB2 + z3 subprocess;
           persistent `z3 -in` + push/pop for
              the guided loop, so extra
           models cost one blocking clause)
                            │
                            ▼
                  Z3 verdict (sat/unsat/unknown/timeout)
                          │
                          ▼
                  Witness renderer (lang/src/imp_render.rs)
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

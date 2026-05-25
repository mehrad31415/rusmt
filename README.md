<h1 style="text-align: center;">RuSmt</h1>
<p style="text-align: center;">Implemented in Rust | Transpiled to SMT</p>

---

## Introduction

**RuSmt** is a Rust embedded DSL for writing executable language specifications. Each specification:
- runs as a Rust program on concrete inputs;
- compiles to SMT-LIB for symbolic reasoning in Z3.

## The Problem

Testing interpreters & compilers on specific edge/error cases is challenging:

- Hand-written test suites have limited coverage
- Fuzzing produces random inputs but may miss certain edge cases

**RuSmt solves this** by letting you write the specification of language as an executable program, then:
1. **Transpile it** to SMT formulas for symbolic analysis
2. **Synthesize input programs** by solving path conditions with Z3, producing inputs that reach the markers you embedded while writing the interpreter.
3. **Execute the program** on the synthesized inputs.
3. **Find bugs** in real-world implementations through conformance testing. In other words, the input programs are used as a test suite to compare the output of your implementation of the language against other implementations on those specific edge/error cases.

## How to use it

Using only the types and methods of the DSL you can write your own software. Wherever necessary, drop in a path condition (`Path::fresh()`) to mark a program point you want to reach. Your code is lifted to SMT-LIB where Z3 searches for concrete inputs that drive the program to each marked point.

This works because every stdlib operation satisfies `rust(f, x) == z3(f, x)` — the Rust implementation and the Z3 formula return the same result on every input. From this we get:
- **soundness** — if Z3 returns an input, running the Rust program on it really does reach the path condition;
- **completeness** — if the path condition is reachable, the transpiled formula in Z3 is satisfiable. Whether Z3 actually finds the corresponding input program that triggers that path condition, i.e., completeness, is bounded by Z3's own decidability and resource limits.

## Case studies

| Case study  | Location | Status |
|-------------|----------|--------|
| **TOML v1.1.0 parser** | `lang/src/toml/` | Parser is a complete runnable specification. Synthesis hits Z3's scaling frontier — all queries time out. See `book/src/case-studies/toml/`. |
| **IMP / WHILE** | `lang/src/imp/` | Canonical small imperative language from Winskel, *The Formal Semantics of Programming Languages* (MIT Press, 1993). End-to-end synthesis works: Z3 returns models, the printer renders them as `.imp` source, and a replay test confirms the marker fires. See `book/src/case-studies/imp/`. |

## Build

### Prerequisites

- Rust (edition 2024) — toolchain pinned by `rust-toolchain`.
- CMake and a C++ compiler — required because the Z3 dependency is vendored and built from source on first compile (~5 minutes). No system Z3 is needed.
  - macOS: included with Xcode CLT (`xcode-select --install`).
  - Debian/Ubuntu: `sudo apt install build-essential cmake`.

### Build

```bash
cargo build --workspace
```

## Tiny end-to-end example (IMP)

```bash
# 1. Run an IMP program concretely.
cargo run -p rusmt-lang -- imp lang/imp/input/factorial.imp

# 2. Synthesise inputs that fire the path-condition markers in eval_com.
#    Two backends are available; both accept k=N for bounded-recursion unrolling.
cargo run -p rusmt-smt-derive -- imp eval_com k=3            # text backend (default)
cargo run -p rusmt-smt-derive -- imp eval_com api k=3        # in-process Z3 API backend
cargo run -p rusmt-smt-derive -- imp eval_com both k=3       # both, for cross-checking

# 3. The printer emits each `sat` response as a runnable .imp file.
cat lang/src/synthesis/imp/z3_chc/target_0/response.txt

# 4. Replay the witness through the same interpreter — it must reach the marker.
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.txt
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
                            │
                ┌───────────┴───────────┐
                │                       │
            Text backend            API backend
        (SMT-LIB2 + z3 subproc.)   (in-process via z3-sys)
                │                       │
                └─────────┬─────────────┘
                          ▼
                  Z3 verdict (sat/unsat/unknown/timeout)
                          │
                          ▼
                  Printer (smt/derive/src/backend/printer.rs)
                          │
                          ▼  (sat case)
                runnable source in target_*/response.txt
                          │
                          ▼
              Conformance / replay test
```

## Project layout

```
rusmt/
├── smt/
│   ├── stdlib/                 SMT-backed Rust types (Boolean, Integer, Real,
│   │                            BV/Float, String, Seq, Set, Array, Cloak,
│   │                            Path).
│   ├── remark/                 #[smt_type], #[smt_fn] attribute checks.
│   ├── remark/remark_derive/   The proc-macro front-end.
│   └── derive/                 Parser → IR → backend pipeline.
│       └── src/
│           ├── parser/         Restricted-Rust parsing, intrinsic lookup.
│           ├── ir/             IR construction.
│           └── backend/
│               ├── codegen.rs  Backend-shared CodeGen trait.
│               ├── response.rs Response enum (Sat/Unsat/Unknown/Timeout).
│               ├── printer.rs  Renders Z3 responses to runnable IMP source.
│               ├── z3/         Text backend (SMT-LIB2 + z3 subprocess).
│               └── z3_api/     API backend (in-process Z3 via z3-sys).
└── lang/
    ├── src/toml/               TOML v1.1.0 parser (case study).
    ├── src/imp/                IMP/WHILE interpreter (case study).
    ├── src/synthesis/          Per-backend synthesis output (gitignored).
    ├── imp/input/              Sample .imp programs used by the IMP CLI.
    └── toml/input/             Sample TOML documents used by the parser CLI.
```

## Documentation

The RuSmt Book (mdBook under `book/`) is the long-form reference for both
users of the DSL and contributors to the framework.

```bash
make docs           # builds and serves the book locally
mdbook build book   # build only
```

## References

- Glynn Winskel, *The Formal Semantics of Programming Languages: An Introduction.* MIT Press, 1993. (Ch. 2 specifies the IMP/WHILE language used as a case study.)
- The TOML v1.1.0 specification: <https://toml.io/en/v1.1.0>.

## License

GPL-3.0-or-later (see `LICENSE`).

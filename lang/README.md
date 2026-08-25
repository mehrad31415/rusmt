## RuSmt Language Implementations

This crate hosts the **case-study language interpreters** used to exercise the
RuSmt pipeline. Every interpreter is written in the restricted Rust subset
defined in the `rusmt-smt-stdlib` crate. Every implementation has two equivalent meanings:

> _Concretely executable_: the interpreter runs as ordinary Rust and produces
> output for a concrete input program.
>
> _Symbolically transpilable_: because the implementation only uses DSL types and
> intrinsics, `rusmt-smt-derive` can lift it into SMT-LIB / Z3 formulas and
> ask Z3 to synthesise inputs that reach `Path::named(...)` markers placed in the
> evaluator. Z3's witnesses become conformance test cases.

### Case studies

- **TOML v1.1.0** (`src/toml/`) — full grammar implementation per the
  [TOML 1.1.0 spec](https://toml.io/en/v1.1.0#spec).
- **IMP / WHILE** (`src/imp/`) — the canonical small imperative language from Winskel, *The Formal Semantics of Programming Languages* (MIT Press, 1993), Ch. 2. End-to-end synthesis works: Z3 finds models, the printer renders them as `.imp` source, and replaying that source through the interpreter (`cargo run -p rusmt-lang -- imp <file>`) confirms the rendered witnesses fire the markers.

### CLI

The main binary (`src/main.rs`) runs an interpreter on a concrete program.

```bash
# Parse a TOML document and pretty-print the AST.
printf 'a = 1\n' > /tmp/example.toml
cargo run -p rusmt-lang -- toml /tmp/example.toml

# Evaluate an IMP program from a `.imp` source file.
cargo run -p rusmt-lang -- imp lang/imp/input/factorial.imp
```

### Running synthesis

Synthesis lives in the derive crate (`rusmt-smt-derive`); see
`smt/derive/README.md` for the full surface.

```bash
# IMP eval_com, with k=3 bounded-recursion unrolling.
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# TOML parse_toml, no unrolling.
cargo run -p rusmt-smt-derive -- toml parse_toml
```

### License

GPL-3.0-or-later (see `LICENSE` in the workspace root).

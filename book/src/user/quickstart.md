## Quickstart

### Prerequisites

- Rust (edition 2024). The `rust-toolchain.toml` file pins the channel.
- A system `z3` CLI on `PATH` (Z3 4.15.4) — the only external dependency.
  The backend, the scripts,
  and the differential / real-Z3 tests all use it
  (`brew install z3` / `apt install z3`).

### Build

```bash
cargo build --workspace
```

### Run an interpreter (concrete execution)

The `rusmt-lang` crate provides a small CLI with two case-study
subcommands. From the workspace root:

```bash
# IMP / WHILE — Winskel Ch. 2 small imperative language.
cargo run -p rusmt-lang -- imp lang/imp/input/factorial.imp
# → final store written to lang/imp/output/factorial.txt and printed.

# TOML v1.1.0 parser. The repository ships no .toml inputs, so make one.
printf 'a = 1\n' > /tmp/example.toml
cargo run -p rusmt-lang -- toml /tmp/example.toml
```

These are the only subcommands the CLI exposes today (see
`lang/src/main.rs`).

### Synthesise inputs (symbolic compilation)

The transpiler is `rusmt-smt-derive`. Its CLI has the form

```text
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [k=<N>]
```

- `<parser_name>` matches a directory under `lang/src/`
  (`imp` or `toml` today).
- `<top_level_fn>` is the function whose `Path::named(...)` markers Z3 should
  chase (`eval_com` for IMP, `parse_toml` for TOML).
- `k=<N>` enables bounded-recursion unrolling of every recursive SCC to
  depth `N`. `k=0` (or omitted) keeps Z3's native `define-funs-rec`
  handling. Note that in RuSmt there are no loops and the only iterative structure are recursive functions.

Example end-to-end run for IMP:

```bash
# Run synthesis on eval_com, unrolling recursion to depth 3.
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# Each path target gets its own subdirectory.
ls lang/src/synthesis/imp/z3_chc/

# Inspect the rendered witness for target 0.
cat lang/src/synthesis/imp/z3_chc/target_0/response.imp

# Replay it through the same interpreter.
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.imp
```

### Run the test suites

```bash
cargo test --workspace
```

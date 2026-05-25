## Quickstart

### Prerequisites

- Rust (edition 2024). The `rust-toolchain` file pins the channel.
- CMake and a C++ compiler (for the vendored Z3 build, ~5 minutes the first
  time). No system Z3 install is required.
  - macOS: included with `xcode-select --install`.
  - Debian/Ubuntu: `sudo apt install build-essential cmake`.

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

# TOML v1.1.0 parser.
cargo run -p rusmt-lang -- toml lang/toml/input/example.toml
# → parsed AST written to lang/toml/output/example.txt.
```

These are the only subcommands the CLI exposes today (see
`lang/src/main.rs`).

### Synthesise inputs (symbolic compilation)

The transpiler is `rusmt-smt-derive`. Its CLI has the form

```text
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [text|api|both] [k=<N>]
```

- `<parser_name>` matches a directory under `lang/src/`
  (`imp` or `toml` today).
- `<top_level_fn>` is the function whose `Path::fresh()` markers Z3 should
  chase (`eval_com` for IMP, `parse_toml` for TOML).
- The third positional arg picks the backend — `text` (default), `api`, or
  `both`. The _text_ translates the rust code to SMTLIB encodings and the _api_ version uses the _z3\_sys_ api.
- `k=<N>` enables bounded-recursion unrolling of every recursive SCC to
  depth `N`. `k=0` (or omitted) keeps Z3's native `define-funs-rec`
  handling. Note that in RuSmt there are no loops and the only iterative structure are recursive functions.

Example end-to-end run for IMP:

```bash
# Run synthesis on eval_com via the text backend, unrolling to depth 3.
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# Each path target gets its own subdirectory.
ls lang/src/synthesis/imp/z3_chc/

# Inspect the rendered witness for target 0.
cat lang/src/synthesis/imp/z3_chc/target_0/response.txt

# Replay it through the same interpreter.
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.txt
```

### Run the test suites

```bash
cargo test --workspace
```

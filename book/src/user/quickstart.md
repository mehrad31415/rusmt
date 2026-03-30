## Quickstart

### Build the workspace

```bash
cargo build --workspace
```

### Run the TOML parser (concrete execution)

The `rusmart-lang` crate provides a small CLI.

```bash
cargo run -p rusmart-lang -- toml lang/toml/input/example.toml
```

### Generate SMT-LIB (symbolic compilation)

The transpiler lives in `rusmart-smt-derive` and emits Z3-focused SMT-LIB.

```bash
cargo run -p rusmart-smt-derive -- toml parse_toml
```

### Run the test suites

```bash
cargo test --workspace
```

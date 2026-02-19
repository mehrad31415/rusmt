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
cargo run -p rusmart-smt-derive
```

### Run the test suites

```bash
cargo test --workspace
```

The `rusmart-programs` crate contains data-driven tests under `programs/tests/`:

- `tests/stdlib`: IR-only “does it model?” coverage for stdlib intrinsics
- `tests/translation`: end-to-end translation coverage (Rust → SMT-LIB → Z3)


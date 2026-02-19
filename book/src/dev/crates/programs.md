## Crate: `rusmart-programs`

The `programs/` crate exists primarily as **test inputs** for the Rusmart toolchain.

### What lives here

- `programs/tests/integration.rs`: `datatest-stable` harness
- `programs/tests/stdlib/`: IR-only stdlib coverage tests
- `programs/tests/translation/`: end-to-end translation + Z3 response baselines

The files under `programs/tests/**` are small Rusmart programs (Rust files using the DSL) that are compiled and processed by `rusmart-smt-derive` during testing.


## TOML v1.0 case study

The `rusmart-lang` crate contains a TOML v1.1.0 parser implemented *in the Rusmart DSL* (restricted Rust + `rusmart-smt-stdlib`).
It serves as the _reference semantics_ example for:

- **Concrete execution**: run it as a normal Rust program (`cargo run -p rusmart-lang -- toml ...`)
- **Symbolic compilation**: transpile it into SMT-LIB via `rusmart-smt-derive`
- **Synthesis / conformance**: query Z3 (or other available solvers) over the SMT encoding to generate inputs that are fed to the reference interpreter and commercial interpreters to find divergences

### Running synthesis

Two Z3 backends are available for synthesis:

```bash
# Text backend (default): generates SMT-LIB2 files, spawns Z3 as subprocess
cargo run -p rusmart-smt-derive -- toml parse_toml

# API backend: uses Z3 in-process via z3-sys bindings
cargo run -p rusmart-smt-derive -- toml parse_toml api

# Both backends for comparison
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

Results are written to `lang/src/synthesis/toml/` with per-backend directories (`z3_chc/` for text, `z3_api/` for API). Each error target gets a subdirectory containing the solver's response.

The code lives under the `lang/src/toml` directory. The next two chapters explain how the parser is structured and how the TOML value model maps onto SMT sorts.


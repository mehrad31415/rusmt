## Testing in this repository

Testing is done via `cargo test --workspace`. The stdlib crate contains unit tests for intrinsic types and methods. The derive crate can be tested by running the transpiler on language interpreters in `lang/` and verifying the generated SMT-LIB output.

### Synthesis testing

To run synthesis (solving error targets with Z3), use the derive crate's CLI:

```bash
# Text backend (generates SMT-LIB2 + spawns Z3 subprocess)
cargo run -p rusmart-smt-derive -- toml parse_toml

# API backend (in-process Z3 via z3-sys)
cargo run -p rusmart-smt-derive -- toml parse_toml api

# Both backends for comparison
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

Results are written to `lang/src/synthesis/toml/` under `z3_chc/` (text backend) or `z3_api/` (API backend). Each error target gets its own subdirectory with a `response.txt` file containing the solver verdict.
## Testing in this repository

Testing is done via `cargo test --workspace`. The stdlib crate contains unit tests for intrinsic types and methods. The derive crate can be tested by running the transpiler on language interpreters in `lang/` and verifying the generated SMT-LIB output.

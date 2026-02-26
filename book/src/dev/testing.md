## Testing in this repository

There are two conceptually different test layers:

### 1) IR-only “stdlib coverage”

Location: `programs/tests/stdlib/`

Goal: ensure that **each intrinsic type + method** supported by `rusmart-smt-stdlib` can be parsed and lowered into the internal IR model (no solver needed).

This tier uses the derive crate’s IR builder (`model(...)`) and treats “IR construction succeeds” as the pass condition.

### 2) End-to-end translation

Location: `programs/tests/translation/`

Goal: ensure that full translation works:

Rust DSL → parser AST → IR → SMT-LIB → Z3 invocation → stable expected response.

Because SMT-LIB emission can be non-deterministic in ordering, the tests compare **Z3 responses** (`response.exp`) against recorded baselines.

If you need to regenerate baselines, the harness supports an “update baseline” mode (see `programs/tests/integration.rs`).
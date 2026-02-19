### Dependencies

Rusmart’s solver integration lives under `solver/`.

In this repository the focus is **Z3**, and the codebase includes Rust bindings and build plumbing so tests can run in a self-contained way.

If you are updating solver behavior or SMT-LIB emission, the key crates are typically:

- `smt/derive` backend implementation (SMT-LIB formatting, invocation, response parsing)
- `solver/` (Z3 binding/build integration)
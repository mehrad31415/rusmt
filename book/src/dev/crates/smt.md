The `smt/` directory contains the core symbolic toolchain. It is divided into:

- [**stdlib**](smt/stdlib.md)
- [**remark**](smt/remark.md)
- [**derive**](smt/derive.md)

Roughly:

- `stdlib` defines SMT-backed Rust types and intrinsic operations.
- `remark` provides proc-macro-driven syntactic restrictions (`#[smt_fn]`, `#[smt_type]`).
- `derive` parses the DSL, builds IR, emits SMT-LIB, and integrates solver backends.
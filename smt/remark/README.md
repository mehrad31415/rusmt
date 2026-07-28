## Remark

This crate provides utilities to enforce syntactic constraints on the types and functions used in RuSmt language implementations. The actual attribute macros live in `rusmt_smt_remark_derive` and call into this crate.

### Purpose

Transpiling the whole Rust language to SMT-LIB is infeasible due to its complexity. Instead, RuSmt defines a small set of domain-specific languages (DSLs) for writing interpreters. We impose further constraints on these DSLs to ensure that the code can be effectively transpiled to SMT-LIB.

### Constraints

- **Type constraints (`#[smt_type]`)**: We provide the `[smt_type]` attribute macro to mark types that are allowed in RuSmt DSLs. Only types annotated with this macro are processed by the transpiler. It is implemented by `rusmt_smt_remark_derive::smt_type`. It accepts **no arguments**. It injects derives and an `SMT` impl:
  - For **structs**: derives `Debug, Clone, Copy, Default, Hash` and implements `SMT::_cmp` lexicographically over fields.
  - For **enums**: derives `Debug, Clone, Copy, Hash`, generates a `Default` impl choosing the **first variant**, and implements `SMT::_cmp`.
  - Generic params must be of the form `<T: SMT, ...>` (no `where` clause, no extra bounds).

- **Function constraints (`#[smt_fn]`)**: We provide the `[smt_fn]` attribute macro to mark functions that are allowed in RuSmt DSLs. Only functions annotated with this macro are processed by the transpiler. It is implemented by `rusmt_smt_remark_derive::smt_fn`. It accepts **no arguments**.
  - Rejects `const`, `async`, `unsafe`, `extern "ABI"`, and variadics, and rejects `where` clauses.
  - Generic params must be of the form `<T: SMT, ...>` (no extra bounds).
  - Note: this crate does **not** validate parameter/return types in the function body; it only enforces these syntactic restrictions and generic bounds.

### License
The RuSmt project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.

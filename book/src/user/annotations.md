# Annotations

RuSmt is a restricted Rust DSL plus **proc-macro annotations** that mark which
types and functions belong to the symbolic subset.

In this repository, the only annotations used by the DSL are:

- `#[smt_type]` for data types
- `#[smt_fn]` for functions

They are implemented by the `rusmt_smt_remark_derive` crate (which calls into
{{#include ../../dict/crate-remark.md}}). The derive/transpiler crate
({{#include ../../dict/crate-derive.md}}) does the deeper semantic checks and
translation.

## `#[smt_type]`

Marks a `struct` or `enum` as a DSL type that can appear in transpilable code.

- **No arguments** are accepted: `#[smt_type(...)]` is rejected.
- Injects derives (e.g., `Copy`, `Clone`, `Debug`, `Hash`, and `Default` where applicable).
- Enforces generic bounds of the form `<T: SMT, ...>` (no `where` clause).

## `#[smt_fn]`

Marks a function as part of the DSL and eligible for parsing/lowering by the transpiler.

- **No arguments** are accepted: `#[smt_fn(...)]` is rejected.
- The function cannot be `const`, `async`, `unsafe`, `extern`, variadic, or have a `where` clause.
- Generic bounds must be of the form `<T: SMT, ...>` (no extra bounds).

The remark layer intentionally focuses on syntactic restrictions; the transpiler is responsible for checking that the function body stays within the DSL subset and that all types/operations are supported.

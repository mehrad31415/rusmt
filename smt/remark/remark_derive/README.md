## Remark Derive

This crate is the procedural-macro frontend for RuSmt's syntactic constraints. It exposes two attribute macros, `#[smt_type]` and `#[smt_fn]`, and does nothing else: each one converts the incoming `proc_macro::TokenStream` into a `proc_macro2::TokenStream`, hands it to `rusmt_smt_remark`, and turns the result back into a `proc_macro::TokenStream` — emitting a `compile_error!` at the offending span if the annotated item violates a constraint.

The split exists because a `proc-macro = true` crate can only export macros, so it cannot be unit-tested or depended on as a library. All of the logic — and all of the tests for it — lives in [`rusmt-smt-remark`](../README.md); this crate is the thin, untestable shell that Rust requires in order to register the attributes. That is also why its two public functions are marked `#[cfg(not(tarpaulin_include))]`: they are excluded from coverage rather than counted as untested lines.

### Macros

- **`#[smt_type]`** — applied to a `struct` or `enum` to mark it as usable in a RuSmt DSL. Injects `Debug, Clone, Copy, Hash` derives, a `Default` impl, and an `SMT` impl. Accepts no arguments.

- **`#[smt_fn]`** — applied to a top-level `fn` to mark it as usable in a RuSmt DSL. Rejects `const`, `async`, `unsafe`, `extern "ABI"`, variadics, and `where` clauses. Accepts no arguments.

Both macros require generic parameters of the form `<T: SMT, ...>`, and both reject any attribute arguments: `#[smt_type(anything)]` is a compile error. See the [`rusmt-smt-remark` README](../README.md) for the full constraint list and the rationale behind it.

### Usage

Depend on this crate to get the attributes, and on `rusmt-smt-stdlib` to get the `SMT` trait the generated impls refer to:

```toml
[dependencies]
rusmt-smt-remark-derive = "0.1"
rusmt-smt-stdlib = "0.1"
```

```rust
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{Boolean, Integer, smt::SMT};

#[smt_type]
struct Point {
    x: Integer,
    y: Integer,
}

#[smt_fn]
fn on_diagonal(p: Point) -> Boolean {
    p.x.eq(p.y)
}
```

### License
The RuSmt project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.

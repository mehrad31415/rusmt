# Pipeline

A Rusmart program is essentially a collection of `*.rs` files written
in the Rusmart dialect (a subset of Rust),
e.g., with Rusmart-related
[procedure macros](../user/annotations.md),
[`stdlib operations`](../user/methods.md), and
following the Rusmart [syntax](../user/syntax.md) and
[type system](../user/typing.md)
(which is more restrictive than Rust).

At build time, the code is checked/processed in two layers:

- **Remarking** ({{#include ../../dict/crate-remark.md}}): enforces syntactic restrictions for `#[smt_fn]` / `#[smt_type]`.
- **Derivation / transpilation** ({{#include ../../dict/crate-derive.md}}): parses the DSL subset, recognizes intrinsics, builds IR, emits SMT-LIB, and invokes Z3.

## AST Enrichment

The logic about AST enrichment is encapsulated
in the {{#include ../../dict/crate-remark.md}} crate.
Briefly, a Rusmart program is instrumented so that DSL types are easy to handle in both concrete execution and symbolic processing.

### On types

Types annotated with `#[smt_type]` are extended with derives (`Copy`, `Clone`, `Debug`, `Hash`, etc.) and an `SMT` impl for automatically implementing the trait on the type.

### On functions

Functions annotated with `#[smt_fn]` are marked as DSL entry points and are subject to syntactic restrictions (no `async`, no `unsafe`, no `where` clauses, etc.).

## SMT Derivation

The derivation logic lives in `smt/derive` and decomposes into:

### Step 1: Parse restricted Rust into a DSL AST

Source: `smt/derive/src/parser/*`

This phase recognizes:

- DSL function/type declarations (`#[smt_fn]`, `#[smt_type]`)
- supported expression/statement forms

### Step 2: Lower into IR with sort checking

Source: `smt/derive/src/ir/*`

This phase assigns SMT sorts to expressions and creates the intermediate respresentation context.

### Step 3: Emit SMT-LIB (and optionally run Z3)

Source: `smt/derive/src/backend/*` (including `backend/z3/*`)

The Z3 backend renders IR intrinsics into SMT-LIB 2 and can run Z3 to collect responses used by the translation tests.
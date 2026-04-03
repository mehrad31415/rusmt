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

### Step 3: Emit SMT-LIB and solve with Z3

Source: `smt/derive/src/backend/*`

Two Z3 backends are available:

**Text backend** (`backend/z3/*`): Renders IR intrinsics into SMT-LIB2 text. For each error target, it writes a `.smt2` file containing the base declarations plus an error-specific assertion, then spawns a Z3 subprocess to solve it. The text backend is selected by default or with the `text` CLI argument.

**API backend** (`backend/z3_api/*`): Uses Z3 in-process via the `z3-sys` crate. It loads type and function definitions via `Z3_eval_smtlib2_string`, then solves each error target using the Z3 API directly. The API backend is selected with the `api` CLI argument. Using `both` runs both backends sequentially for comparison.

Both backends:
- Iterate over error targets extracted from the IR
- For each target, assert that the top-level function's result contains the target error ID
- Collect the solver's response (sat, unsat, unknown, or timeout)
- Write results to the output directory under `z3_chc/` or `z3_api/` respectively
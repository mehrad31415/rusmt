# Pipeline

A RuSmt program is a collection of `*.rs` files written in the RuSmt
dialect (a restricted subset of Rust): with the
[procedural macros](../user/annotations.md) `#[smt_type]` and `#[smt_fn]`,
the [stdlib operations](../user/methods.md), and adhering to the RuSmt
[syntax](../user/syntax.md) and [type system](../user/typing.md)
restrictions.

The transpilation pipeline runs in two layers:

- **Remarking** ({{#include ../../dict/crate-remark.md}}): enforces syntactic
  restrictions on `#[smt_fn]` / `#[smt_type]` items.
- **Derivation / transpilation** ({{#include ../../dict/crate-derive.md}}):
  parses the DSL subset, recognises intrinsics, builds an IR, emits SMT-LIB,
  invokes Z3, and post-processes the response.

## AST enrichment (the `remark` layer)

The macro implementations live in `smt/remark/remark_derive/`; the rule
checks live in `smt/remark/`. A RuSmt program is instrumented so that DSL
types are easy to handle in both concrete execution and symbolic processing.

## SMT derivation

The derivation logic lives in `smt/derive/` and decomposes into:

### Step 1: Parse restricted Rust into a DSL AST

Source: `smt/derive/src/parser/*`.

This phase recognises:

- DSL function/type declarations (`#[smt_fn]`, `#[smt_type]`)
- supported expression / statement forms (`Match`, `If-Else`, `Let`,
  `Return`)

### Step 2: Lower into IR with sort checking

Source: `smt/derive/src/ir/*`.

This phase assigns SMT sorts to expressions, monomorphises generic functions
on demand, builds the type/function registries, and computes the set of
**path-condition targets** — disjoint sets of `Path::fresh()` ids that the
synthesis loop will chase one query at a time.

### Step 3: Emit SMT-LIB and solve with Z3

Source: `smt/derive/src/backend/*`.

Two backends are available; both implement the shared `CodeGen` trait
(`backend/codegen.rs`).

**Text backend** (`backend/z3/*`): renders IR intrinsics into SMT-LIB2
text. For each path target it writes a `.smt2` query (base declarations
plus specific assertions) and spawns a Z3 subprocess. Selected by
the `text` CLI argument (or by default).

**API backend** (`backend/z3_api/*`): uses Z3 in-process via the `z3-sys`
crate. It builds Z3 datatypes and functions in memory and asserts the
target-specific query through the C API. Selected with the `api` argument.
`both` runs both backends sequentially for cross-checking.

Both backends:

- iterate over path-condition targets extracted from the IR,
- per target, assert that the top-level function returns
  `EvalResult::Err(e)` with `e` in the target's id set,
- collect a `Response::{Sat,Unsat,Unknown,Timeout}` (see
  `backend/response.rs`),
- write results to `lang/src/synthesis/<parser_name>/<backend_dir>/`. The response must be post-proccessed per language implementation so that it can become a concrete input to the corresponding interpreter/compiler. The witnesses are replayed to observe that the input hits the same path target.

## Bounded-recursion unrolling

`rusmt-smt-derive` accepts `k=N` as a CLI argument. With `N=0` (or
omitted), recursive functions are emitted via Z3's `define-funs-rec` and Z3
handles the recursion natively. With `N≥1`, every recursive SCC is unrolled
into `N+1` non-recursive depth-indexed copies (`fn_d0`, `fn_d1`, …, `fn_dN`,
where `fn_0` aliases to a sentinel call / terminator); external callers route through
the depth-`N` copy. Both backends support `k=N` today.

## Timeout and watchdog

When the text backend shells out to `z3`, it works around two quirks of the process:

1. **Z3 can hang.** We append `-t:{ms}` to the invocation so Z3 times *itself*
out, leaving no orphaned process.

2. **Z3's output and exit code are unreliable.** After printing `sat` / `unsat` it may emit trailing error text, and it may exit non-zero — or even segfault — on a query it actually answered. For example asking for a model after `unknown`, or crashing while serializing a model it already reported as `sat`. To stay robust, the backend reads the verdict from the **first output line** (`unknown` / `unsat` / `sat`) and ignores everything after it. It decides the result from that printed verdict, **not** the exit code. It reports an `unknown` that arrives *after the deadline* as `Response::Timeout` rather than a genuine "unknown".

The API backend pairs each query with a watchdog thread that calls
`Z3_interrupt` at the timeout deadline and a hard fail-safe at 8× the
timeout to catch hung native code.

The full timeout is set in `backend/response.rs` (`BACKEND_TIMEOUT`).

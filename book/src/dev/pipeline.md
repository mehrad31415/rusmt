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
**path-condition targets** — disjoint sets of `Path::named` marker ids that the
synthesis loop will chase one query at a time.

### Step 3: Emit SMT-LIB and solve with Z3

Source: `smt/derive/src/backend/*`.

The backend (`backend/z3/*`) implements the shared `CodeGen` trait
(`backend/codegen.rs`). It renders IR intrinsics into SMT-LIB2 text; for each
path target it writes a `.smt2` query (base declarations plus target-specific
assertions) and runs Z3 on it.

Queries are text for a reason beyond convenience. Every stage that reasons
*about* a query — the guided loop's strengthening, the direct route's
`macro_inline_input`, the authoring gates' fixed-input rewrite — manipulates
SMT-LIB, and every reported result stays re-runnable with a bare
`z3 -smt2 <file>`. In-process construction would buy typed term building at the
cost of all of that. Where in-memory Z3 *is* wanted later (e.g. the
user-propagator API), `Z3_solver_from_string` loads this backend's own output
into a live solver — the text form is a front end to that, not an obstacle.

Z3 is driven in two ways:

- **One-shot** — spawn `z3 -smt2 <file>` for an independent check. This is the
  per-target synthesis path.
- **Persistent** (`z3_session.rs`) — hold one `z3 -in` process open and scope
  work with `(push)`/`(pop)`. This is the guided loop's path; see
  [AI in the loop](../user/ai.md).

The backend:

- iterate over path-condition targets extracted from the IR,
- per target, assert that the top-level function returns
  `EvalResult::Err(e)` with `e` in the target's id set,
- collect a `Response::{Sat,Unsat,Unknown,Timeout}` (see
  `backend/response.rs`),
- writes results to `lang/src/synthesis/<parser_name>/z3_chc/`. The response must be post-proccessed per language implementation so that it can become a concrete input to the corresponding interpreter/compiler. The witnesses are replayed to observe that the input hits the same path target.

## Bounded-recursion unrolling

`rusmt-smt-derive` accepts `k=N` as a CLI argument. With `N=0` (or
omitted), recursive functions are emitted via Z3's `define-funs-rec` and Z3
handles the recursion natively. With `N≥1`, every recursive SCC is unrolled
into `N+1` non-recursive depth-indexed copies (`fn_d0`, `fn_d1`, …, `fn_dN`,
where `fn_0` aliases to a sentinel call / terminator); external callers route through
the depth-`N` copy.

## Timeout and watchdog

When the backend shells out to `z3`, it works around two quirks of the process:

1. **Z3 can hang.** We append `-t:{ms}` to the invocation so Z3 times *itself*
out, leaving no orphaned process.

2. **Z3's output and exit code are unreliable.** After printing `sat` / `unsat` it may emit trailing error text, and it may exit non-zero — or even segfault — on a query it actually answered. For example asking for a model after `unknown`, or crashing while serializing a model it already reported as `sat`. To stay robust, the backend reads the verdict from the **first output line** (`unknown` / `unsat` / `sat`) and ignores everything after it. It decides the result from that printed verdict, **not** the exit code. It reports an `unknown` that arrives *after the deadline* as `Response::Timeout` rather than a genuine "unknown".

A persistent session (`z3_session.rs`) faces the same two quirks plus a third:
because the process outlives a single check, a Z3 wedged in a preprocessing
phase that ignores its own `:timeout` would block the pipeline forever. Every
read is therefore deadline-bounded (`budget + 5 s`); on expiry the process group
is killed and the session is **poisoned**, after which every call returns a
failure verdict and the caller falls back to the one-shot path. A missing or
older `z3 -in` therefore costs speed, never correctness.

The full timeout is set in `backend/response.rs` (`BACKEND_TIMEOUT`).

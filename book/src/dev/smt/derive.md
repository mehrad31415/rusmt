### Derive

---

`rusmt-smt-derive` is the **Rust → IR → SMT-LIB** transpiler and the
synthesis engine for the RuSmt DSL.

### Public entry points

Defined in `src/lib.rs`:

- **`model(path) -> Result<IRContext>`** — parse the RuSmt sources under
  `path`, lower into the internal IR. No solver involvement.
- **`solve(model, top_level_fn, output, unroll_depth) -> Result<()>`** —
  text backend. For each path target, generate SMT-LIB2, write
  `target_<N>/main.smt2`, spawn `z3` as a subprocess, render the generated model from Z3, and write `target_<N>/response.txt` + `timing.txt`.
- **`solve_z3_api(model, top_level_fn, output, unroll_depth) -> Result<()>`**
  — API backend. Solves each path target in-process via `z3-sys` bindings.

### Internal layout

- `src/parser/*` — DSL parsing, intrinsic recognition, overload resolution,
  type unification (see [unification](unification.md) and
  [generics](generics.md)).
- `src/ir/*` — expression lowering, sort checking, monomorphisation.
- `src/backend/*`:
  - `codegen.rs` — `CodeGen` trait shared by both backends; iteration over
    path targets.
  - `response.rs` — `Response` enum (`Sat(String)`, `Unsat`,
    `Unknown(String)`, `Timeout`) and `BACKEND_TIMEOUT` (default
    `Duration::from_secs(60 * 10)`).
  - `z3/*` — text backend.
    - `ctxt.rs` — `CodeGenZ3`, Z3 subprocess invocation, response parsing.
    - `exp.rs` — IR expressions → SMT-LIB2 text.
    - `fun.rs` — `define-fun` / `define-funs-rec` emission, including the
      depth-indexed unrolled copies.
    - `intrinsics.rs` — every IR intrinsic mapped to its SMT-LIB2 form.
    - `sort.rs` — IR sorts → SMT-LIB2 datatype declarations.
  - `z3_api/*` — API backend.
    - `mod.rs` — `Z3Ast` RAII wrapper (refcount management), `Z3Context`.
    - `context.rs` — Z3 datatypes / functions.
    - `solver.rs` — per-target solving pipeline + watchdog thread.
    - `translate.rs` — IR expressions → Z3 ASTs.
    - `intrinsics.rs` — every IR intrinsic mapped to a Z3 C-API call.

### CLI surface

The binary at `src/main.rs` wraps the library entry points:

```text
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [text|api|both] [k=<N>]
```

- `<parser_name>` — directory name under `lang/src/` (`imp`, `toml`, …).
- `<top_level_fn>` — function whose `Path::fresh()` markers Z3 should chase.
- backend selector (positional, default `text`):
  - `text` — text backend only.
  - `api` — API backend only.
  - `both` — run both sequentially.
- `k=<N>` — bounded-recursion unrolling depth (`k=0` keeps Z3-native
  `define-funs-rec`; `k≥1` unfolds every recursive SCC into `N+1`
  depth-indexed copies).

Output goes to `lang/src/synthesis/<parser_name>/{z3_chc,z3_api}/target_<N>/`.

The env var `RUSMT_SKIP_INVOKE=1` makes the text backend write `.smt2`
files but skip the subprocess call — useful when iterating on codegen.

### Bounded-recursion unrolling (`k=N`)

For both backends, a non-zero unroll depth replaces each recursive SCC with
`N+1` depth-indexed copies (`fn_d0`, `fn_d1`, …, `fn_dN`), where the
deepest copy aliases to a sentinel call. External callers route through
`fn_dN`. The text-backend implementation is `mk_functions_unrolled_str`;
the API backend mirrors the same shape over Z3 ASTs.

### Timeout and watchdog mechanics

The solver budget is `BACKEND_TIMEOUT` (`backend/response.rs`).

1. **Z3 self-termination (Text backend).** The text backend appends `-t:{ms}` to the `z3`
   command line so Z3 stops on its own when the deadline elapses.
2. **External watchdog (API backend).** `solver.rs` spawns a watchdog
   thread that calls `Z3_interrupt` at the deadline, plus a hard fail-safe
   that kills the worker if it runs past 8× the timeout — protecting
   against hangs in the native Z3 code beyond `Z3_interrupt`.

### Path-marker representation

Path markers are encoded as `(Array Int Bool)`. Each
`Path::fresh(id)` becomes
`(store ((as const (Array Int Bool)) false) id true)`; `Path::merge`
becomes `((_ map or) lhs rhs)`. Membership testing is `(select expr id)`. The set-of-marker-ids per **path target** is computed during IR building
and exposed as `IRContext::path_targets`.

### Soundness pre-condition for `Cloak<T>`-using enums

Because the IR strips `Cloak<T>`, an enum defined as
`enum X { Wrap(Cloak<X>) }` (recursive variants only, no non-recursive
base case) lowers to `enum X { Wrap(X) }`, which Z3 rejects as
ill-founded. This is acceptable: such an enum has no inhabitants and
is never useful. The IR assumes every `Cloak<T>` appears alongside at
least one non-recursive variant — i.e. the enum is well-founded.

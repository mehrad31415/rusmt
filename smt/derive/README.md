## Derive

`rusmt-smt-derive` is the **Rust → IR → SMT-LIB** compiler and the
synthesis engine of RuSmt. It parses rusmt-annotated source files,
builds an intermediate representation, lowers to either SMT-LIB2 text or
in-process Z3 ASTs, and runs Z3 to find inputs that reach `Path::fresh()`
markers placed in the source.

### Two backends

**Text backend** (`backend/z3/`): emits SMT-LIB2 to disk and spawns the
`z3` binary as a subprocess. The `-t:{ms}` flag makes Z3 self-terminate at
the deadline. Per-target output:
`lang/src/synthesis/<parser>/z3_chc/target_<N>/{main.smt2,response.txt,timing.txt}`.

**API backend** (`backend/z3_api/`): uses Z3 in-process via `z3-sys`.
Queries are built directly from Z3 ASTs. A watchdog thread calls `Z3_interrupt`
at the timeout deadline and a hard fail-safe at 8× the deadline catches
hangs in native code beyond `Z3_interrupt`. Per-target output:
`lang/src/synthesis/<parser>/z3_api/target_<N>/{response.txt,timing.txt}`.

### Public entry points

- **`model(path) -> Result<IRContext>`** — parse + lower; no solver.
- **`solve(model, top_level_fn, output_dir, unroll_depth) -> Result<()>`**
  — text backend.
- **`solve_z3_api(model, top_level_fn, output_dir, unroll_depth) -> Result<()>`**
  — API backend.

`unroll_depth = 0` keeps native `define-funs-rec`. `unroll_depth ≥ 1`
unfolds every recursive SCC into `N+1` depth-indexed copies and routes
external callers through the depth-`N` copy.

Both functions skip the per-target Z3 invocation when the env var
`RUSMART_SKIP_INVOKE=1` is set — useful when iterating on codegen.

### CLI

```bash
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [text|api|both] [k=<N>]
```

Examples:

```bash
# IMP synthesis, both backends, depth 3.
cargo run -p rusmt-smt-derive -- imp eval_com both k=3

# TOML synthesis, text backend (default), no unrolling.
cargo run -p rusmt-smt-derive -- toml parse_toml
```

### Path-marker representation

Path markers are encoded as `(Array Int Bool)`. `Path::fresh(id)` becomes
`(store ((as const (Array Int Bool)) false) id true)`; `Path::merge` is
`((_ map or) lhs rhs)`; membership testing is `(select expr id)`.

### Build

Z3 is included as a vendored dependency:

```toml
z3 = { version = "0.20.0", features = ["vendored"] }
z3-sys = "0.11.0"
```

This requires CMake and a C++ compiler; the first build compiles Z3 from
source (~5 minutes). Subsequent builds use the cached artifacts.

### License

GPL-3.0-or-later (see `LICENSE` in the workspace root).

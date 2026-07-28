## Derive

`rusmt-smt-derive` is the **Rust → IR → SMT-LIB** compiler and the
synthesis engine of RuSmt. It parses rusmt-annotated source files,
builds an intermediate representation, lowers to SMT-LIB2, and runs Z3 to find
inputs that reach `Path::named(...)` markers placed in the source.

### Backend

`backend/z3/` emits SMT-LIB2 to disk and runs the `z3` binary on it. The
`-t:{ms}` flag makes Z3 self-terminate at the deadline. Per-target output:
`lang/src/synthesis/<parser>/z3_chc/target_<N>/{main.smt2,response.<ext>,timing.txt}`.
The response file is named by the object-language extension (`.imp`, `.toml`,
`.tc`) for a replayable witness so it feeds straight back into `rusmt-lang`,
and `response.txt` for any non-witness verdict (raw model, timeout, error).

Queries are text because everything that reasons *about* a query does so as
SMT-LIB — the guided loop's strengthening, the authoring
gates — and because every reported result stays re-runnable with a bare
`z3 -smt2 <file>`.

### Keeping one `z3` alive (`z3_session.rs`)

A synthesis query is run the simple way: write a `.smt2` file, run `z3` on it,
read the answer, done.

The guided loop can't afford that. It asks Z3 the *same* question several times
over, each time with a few extra constraints bolted on — so re-running `z3` from
scratch means Z3 re-reads the whole generated program every round. So the guided loop starts `z3` once (`z3 -in` reads commands from stdin instead
of a file), sends it the program once, and leaves it running for the whole
target. Before each round we tell it "remember this point" (`push`), send the
round's extra constraints, ask, then "go back to that point" (`pop`) — which
undoes the round's constraints and leaves the original program untouched for the
next one.

### Public entry points

- **`model(path) -> Result<IRContext>`** — parse + lower; no solver.
- **`solve(model, top_level_fn, output_dir, unroll_depth) -> Result<()>`**
  — per-target synthesis.

`unroll_depth = 0` keeps native `define-funs-rec`. `unroll_depth ≥ 1`
unfolds every recursive SCC into `N+1` depth-indexed copies and routes
external callers through the depth-`N` copy.

`RUSMT_SKIP_INVOKE=1` makes `solve` do everything *except* call Z3: codegen
runs and `main.smt2` plus every `target_<N>/main.smt2` is written. This is the
codegen-debugging path. Note it is **not** the same as stopping at `model` —
that returns an `IRContext` and emits no SMT-LIB.

### CLI

```bash
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [k=<N>]
```

Examples:

```bash
# IMP synthesis, depth-3 unrolling.
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# TOML synthesis, native recursion.
cargo run -p rusmt-smt-derive -- toml parse_toml
```

### Path-marker representation

Concretely a `Path` is a **set** of marker ids, and in SMT it is a bit-set with
one bit per named marker in the program — sort `(_ BitVec N)`, `N` = number of
markers.

```
markers: div_zero→bit 0, undef_var→bit 1, bad_type→bit 2      N = 3

Path::named("div_zero")           #b001
Path::named("bad_type")           #b100
merge of the two                  (bvor #b001 #b100) = #b101

target {div_zero}                 (= (bvand p #b001) #b001)   p=#b101 → true
target {div_zero, bad_type}       (= (bvand p #b101) #b101)   p=#b101 → true
target {undef_var}                (= (bvand p #b010) #b010)   p=#b101 → false
```

**Bit indices and the marker hash.** `marker_id(name)` (FNV-1a, in
`rusmt-smt-stdlib`) is unchanged and remains a marker's identity on both sides
of the pipeline. A bit index is *derived* from it — the rank of that id among
the program's marker ids, taken in `marker_names`' own sorted-id order — and
never leaves the query it was emitted into. The SMT side maps name → hash → bit;
the concrete side maps the same name → the same hash → membership in the real
`BTreeSet`. The two exchange the *name*, never the number, so the
"identical by construction" property that makes per-target replay sound is
untouched.

**An earlier encoding, and why it was replaced.** `(Array Int Bool)` with
`store`/`select` was exact, but Z3 could not evaluate the array construction on
deep error paths.

### Build

Nothing links against Z3, so no C++ toolchain or CMake is needed. The crate
requires a **system `z3` on `PATH`** at run time (Z3 4.15.4):
`brew install z3` / `apt install z3`.

### License

GPL-3.0-or-later (see `LICENSE` in the workspace root).

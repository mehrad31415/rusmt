### Derive

---

`rusmt-smt-derive` is the **Rust → IR → SMT-LIB** transpiler and the
synthesis engine for the RuSmt DSL.

### Public entry points

Defined in `src/lib.rs`:

- **`model(path) -> Result<IRContext>`** — parse the RuSmt sources under
  `path`, lower into the internal IR. No solver involvement.
- **`solve(model, top_level_fn, output, unroll_depth) -> Result<()>`** —
  for each path target, generate SMT-LIB2, write `target_<N>/main.smt2`, spawn
  `z3` as a subprocess, render the generated model from Z3, and write the result
  to `target_<N>/response.<ext>` (the object-language extension — `.imp`,
  `.toml`, `.tc` — for a replayable witness, else `response.txt`) plus
  `timing.txt`.

### Internal layout

- `src/parser/*` — DSL parsing, intrinsic recognition, overload resolution,
  type unification (see [unification](unification.md) and
  [generics](generics.md)).
- `src/ir/*` — expression lowering, sort checking, monomorphisation.
- `src/backend/*`:
  - `codegen.rs` — the `CodeGen` trait; iteration over path targets.
  - `response.rs` — `Response` enum (`Sat(String)`, `Unsat`,
    `Unknown(String)`, `Timeout`) and `BACKEND_TIMEOUT` (default
    `Duration::from_secs(60 * 10)`).
  - `z3/*` — the SMT-LIB2 backend.
    - `ctxt.rs` — `CodeGenZ3`, Z3 subprocess invocation, response parsing.
    - `exp.rs` — IR expressions → SMT-LIB2 text.
    - `fun.rs` — `define-fun` / `define-funs-rec` emission, including the
      depth-indexed unrolled copies.
    - `intrinsics.rs` — every IR intrinsic mapped to its SMT-LIB2 form.
    - `sort.rs` — IR sorts → SMT-LIB2 datatype declarations.
- `src/z3_session.rs` — a persistent `z3 -in` process: `push`/`pop` scoping,
  blocking-clause model enumeration, and `(get-info :all-statistics)` on a
  stuck verdict. Used by the guided loop; see [AI in the loop](../../user/ai.md).

### CLI surface

The binary at `src/main.rs` wraps the library entry points:

```text
cargo run -p rusmt-smt-derive -- <parser_name> <top_level_fn> [k=<N>]
```

- `<parser_name>` — directory name under `lang/src/` (`imp`, `toml`, …).
- `<top_level_fn>` — function whose `Path::named(...)` markers Z3 should chase.
- `k=<N>` — bounded-recursion unrolling depth (`k=0` keeps Z3-native
  `define-funs-rec`; `k≥1` unfolds every recursive SCC into `N+1`
  depth-indexed copies).

Output goes to `lang/src/synthesis/<parser_name>/z3_chc/target_<N>/`.

(A bare `text` positional argument is still accepted and ignored, so older
command lines keep working.)

The env var `RUSMT_SKIP_INVOKE=1` makes the backend run codegen and write
`main.smt2` plus every `target_<N>/main.smt2`, but skip the `z3` call. Use it
when the *generated text* is what you are debugging: stopping at `model`
instead gives you an `IRContext` and no SMT-LIB at all, so it cannot show you a
malformed `define-fun`. Skipping only the solver is what makes it cheap —
TOML's 182 targets render in seconds instead of spending a solver budget on
every marker.

### Bounded-recursion unrolling (`k=N`)

A non-zero unroll depth replaces each recursive SCC with `N+1` depth-indexed
copies (`fn_d0`, `fn_d1`, …, `fn_dN`), where the deepest copy aliases to a
sentinel call. External callers route through `fn_dN`. The implementation is
`mk_functions_unrolled_str`.

### Timeout mechanics

The solver budget is `BACKEND_TIMEOUT` (`backend/response.rs`).

1. **Z3 self-termination (one-shot).** The backend appends `-t:{ms}` to the `z3`
   command line so Z3 stops on its own when the deadline elapses.
2. **Deadline-bounded reads (persistent session).** Because a `z3 -in` process
   outlives a single check, a Z3 wedged in a preprocessing phase that ignores
   its own `:timeout` would block the caller. Every read in `z3_session.rs` is
   bounded at `budget + 5 s`; on expiry the process group is killed and the
   session is *poisoned*, so later calls fail fast and the loop falls back to
   the one-shot path.

### Path-marker representation

In the concrete Rust semantics a `Path` is a **set** of marker ids, so an
execution that trips several markers accumulates all of them.

SMT mirrors that exactly, as a bit-set: sort `(_ BitVec N)` with one bit per
named marker in the program (`backend/z3/path.rs`).

| | concrete Rust | SMT |
|---|---|---|
| sort | set of ids | `(_ BitVec N)`, `N` = marker count |
| empty | `{}` | `(_ bv0 N)` |
| `Path::named(n)` | `{marker_id(n)}` | one-hot literal for `n`'s bit |
| `Path::merge(l, r)` | set union | `(bvor l r)` |
| target `T` fired | `T ⊆ set` | `(= (bvand p M_T) M_T)` |

`bvor` **is** union and a bit test **is** membership, so nothing is lost. One
formula covers both target shapes — a singleton `Path::named` target and a
multi-marker `Path::merge` target ("reach all of these on one run") — and since
the mask constrains only `T`'s bits, a run that fires extra markers still
satisfies it.

```
markers: div_zero→bit 0, undef_var→bit 1, bad_type→bit 2      N = 3

Path::named("div_zero")           #b001
Path::named("bad_type")           #b100
merge of the two                  (bvor #b001 #b100) = #b101

target {div_zero}                 (= (bvand p #b001) #b001)   p=#b101 → true
target {div_zero, bad_type}       (= (bvand p #b101) #b101)   p=#b101 → true
target {undef_var}                (= (bvand p #b010) #b010)   p=#b101 → false
```

**Bit indices vs. the marker hash.** `marker_id(name)` is unchanged and is still
a marker's identity on both sides. The bit index is *derived* from it — the rank
of that id in `marker_names`' sorted-id order — and never leaves the query it was
emitted into. The SMT side goes name → hash → bit; the concrete side goes the
same name → the same hash → membership in the real `BTreeSet`. They exchange the
*name*, never the number, so the "same by construction" property that makes
per-target replay sound is untouched. Nothing decodes a `Path` back out of a
model, so an index is never reversed.

**Two earlier encodings.** `(Array Int Bool)` with `store`/`select`/`(_ map or)`
was exact but Z3 could not evaluate the array construction on deep error paths.
Replacing it with one `Int` holding a representative id fixed that at the cost of
correctness: `merge` dropped its right operand, and a multi-marker target became
`(and (= e a) (= e b))` — a contradiction, `unsat` for every input whatever the
program did, which the pipeline then reported as genuine unreachability at
`k=0`. Bit-vectors recover the array encoding's exactness and bit-blast to finite
SAT, so they survive recursive unfolding.

The set of marker ids per **path target** is computed during IR building and
exposed as `IRContext::path_targets`; a multi-id target arises exactly when the
program calls `Path::merge`.

### Soundness pre-condition for `Cloak<T>`-using enums

Because the IR strips `Cloak<T>`, an enum defined as
`enum X { Wrap(Cloak<X>) }` (recursive variants only, no non-recursive
base case) lowers to `enum X { Wrap(X) }`, which Z3 rejects as
ill-founded. This is acceptable: such an enum has no inhabitants and
is never useful. The IR assumes every `Cloak<T>` appears alongside at
least one non-recursive variant — i.e. the enum is well-founded.

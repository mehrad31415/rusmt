## IMP synthesis loop

The IMP case study is the most complete demonstration of the RuSmt
pipeline because the loop closes end-to-end:

```
eval_com (Rust)            ──────►  rusmt-smt-derive
   │                                       │
   │ contains Path::named(..)              │ lifts Aexp/Bexp/Com to SMT
   │                                       │ datatypes, defines eval_com_*
   ▼                                       │ as define-funs-rec or unrolled
path-condition tags  ──────────────────────┘
                                              │
                                              ▼
                                          Z3 (SMT-LIB2)      
                                              │
                                              ▼
                                  sat / unsat / unknown / timeout
                                              │
                                              ▼
                          witness renderer (lang/src/imp_render.rs)
                                              │
                                ┌─────────────┴──────────────┐
                                ▼                             ▼
                      sat → runnable IMP source       non-sat → passthrough
                      in target_<N>/response.imp      with `// <status>`
                                │
                                ▼
                  cargo run -p rusmt-lang -- imp target_<N>/response.imp
                                │
                                ▼
                   `[RuSmt] Path-condition marker reached`
```

### The two markers

Each marker carries an integer id. `Path::named("d")` derives the id as a
fixed hash of the name `d` — the same pure function runs in the transpiler
and in the concrete evaluator, so the id a query targets and the id a
replayed witness carries coincide; this is what makes per-target replay
certification possible (see [AI in the loop](../../user/ai.md)).
`rusmt-smt-derive` collects the ids into
`IRContext::path_targets` and issues one Z3 query per target asking: *find a
program `c` such that `eval_com(c)` returns `EvalResult::Err(e)` with the
target id in `e`'s marker set* (the store is fixed internally — the program
is the single free input). These are tags, not necessarily bugs. They exist
purely to give Z3 something concrete to chase.

### Running synthesis

```bash
# Default: text backend, native recursion (k=0, Z3's define-funs-rec).
# Both IMP targets solve in well under a second.
cargo run -p rusmt-smt-derive -- imp eval_com

# Bounded-recursion unrolling at depth k=3. Caution: depth-bounded queries
# can return *spurious* candidate models (satisfied through the depth
# cutoff); the automatic replay step below is what rejects them.
cargo run -p rusmt-smt-derive -- imp eval_com k=3
```

Bounding cuts both ways, and IMP shows both. At `k=1`, `target_0` returns
`unsat` — yet that same marker is `sat` with a replay-certified witness at
`k=0`. A bounded `unsat` therefore means only "no witness within depth `k`",
never unreachability, which is why the pipeline treats `unsat` as a genuine
verdict only at `k=0`. In the other direction, unrolling can satisfy a marker
assertion *through the cutoff* rather than along a real path — a spurious `sat`
that only the replay step rejects.

Output is written to `lang/src/synthesis/imp/z3_chc/target_<N>/`.

The response file's **extension records the verdict**: a `sat` model rendered
into a runnable program is stored as `response.imp` (so it can be replayed
straight through `rusmt-lang -- imp` with no renaming), while any non-`sat`
outcome that has no runnable witness stays `response.txt`. Each target
directory holds exactly one such file.

### Anatomy of `response.imp` (a `sat` witness)

The printer walks the Z3 model, it finds the abstract syntax tree and converts the AST back to a `.imp` format.

### Anatomy of `response.txt` (a non-`sat` verdict)

The printer never panics. For `unsat` / `unknown` / `timeout` / empty /
malformed responses, the file is the original Z3 text prefixed with a
one-line `// <status>` header. A renderer error (unrecognised constructor,
malformed shape) yields a `// renderer error: <reason>` header followed by
the raw text. The pipeline stays green for every Z3 outcome.

### Closing the loop

For a *named* target the pipeline closes the loop automatically: every
rendered witness is re-parsed and re-executed through the concrete
`eval_com` — in an isolated process, so a non-terminating or crashing
candidate is rejected rather than fatal — and the per-target verdict lands
in `target_<N>/replay.txt`:

```
CERTIFIED: replay through the reference semantics fired `division_by_zero`
```

A spurious candidate model (possible under `k`-bounded unrolling) is
`REJECTED` here and, when a proposer is configured, handed to the fallback
loop — see [AI in the loop](../../user/ai.md).

Non-`sat` passthroughs have no witness to replay. You can also replay any
witness by hand:

```bash
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.imp
# → [RuSmt] Path-condition marker reached during execution.
```

### Reproducible end-to-end

```bash
# (1) generate witnesses (native recursion)
cargo run -p rusmt-smt-derive -- imp eval_com

# (2) inspect one, plus its replay verdict
cat lang/src/synthesis/imp/z3_chc/target_0/response.imp
cat lang/src/synthesis/imp/z3_chc/target_0/replay.txt

# (3) replay it directly
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.imp
# → [RuSmt] Path-condition marker reached during execution.
```

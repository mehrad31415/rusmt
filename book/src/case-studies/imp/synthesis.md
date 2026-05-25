## IMP synthesis loop

The IMP case study is the most complete demonstration of the RuSmt
pipeline because the loop closes end-to-end:

```
eval_com (Rust)            ──────►  rusmt-smt-derive
   │                                       │
   │ contains Path::fresh()                │ lifts Aexp/Bexp/Com to SMT
   │                                       │ datatypes, defines eval_com_*
   ▼                                       │ as define-funs-rec or unrolled
path-condition tags  ──────────────────────┘
                                              │
                                              ▼
                                       Z3 (text or API backend)
                                              │
                                              ▼
                                  sat / unsat / unknown / timeout
                                              │
                                              ▼
                                      backend/printer.rs
                                              │
                                ┌─────────────┴──────────────┐
                                ▼                             ▼
                      sat → runnable IMP source       non-sat → passthrough
                      in target_<N>/response.txt      with `// <status>`
                                │
                                ▼
                  cargo run -p rusmt-lang -- imp target_<N>/response.txt
                                │
                                ▼
                   `[RuSmt] Path-condition marker reached`
```

### The two markers

Each `Path::fresh()` produces a fresh symbolic id; `rusmt-smt-derive`
collects them into `IRContext::path_targets`. The synthesis pipeline then issues one Z3 query per target asking: *find inputs `(c, s)` such that `eval_com(c, s)` returns `EvalResult::Err(e)` with `e == target_id`*. These are tags, not necessarily bugs. They exist purely to give Z3 something concrete to chase.

### Running synthesis

```bash
# Default: text backend, no unrolling. With recursion, Z3 typically returns
# `unknown`/`timeout` — eval_com is recursive over Com.
cargo run -p rusmt-smt-derive -- imp eval_com

# Bounded-recursion unrolling at depth k=3. Drives terminating SMT for the
# small targets above.
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# API backend (in-process Z3 via z3-sys), same depth.
cargo run -p rusmt-smt-derive -- imp eval_com api k=3

# Both backends, useful for cross-checking.
cargo run -p rusmt-smt-derive -- imp eval_com both k=3
```

Output is written to `lang/src/synthesis/imp/<backend>/target_<N>/`. For the
text backend `<backend>` is `z3_chc`; for the API backend it is `z3_api`.

### Anatomy of `response.txt` for a `sat` verdict

The printer walks the Z3 model, it finds the abstract syntax tree and converts the AST back to a `.imp` format.

### Anatomy of `response.txt` for a non-`sat` verdict

The printer never panics. For `unsat` / `unknown` / `timeout` / empty /
malformed responses, the file is the original Z3 text prefixed with a
one-line `// <status>` header. A renderer error (unrecognised constructor,
malformed shape) yields a `// renderer error: <reason>` header followed by
the raw text. The pipeline stays green for every Z3 outcome.

### Closing the loop

Every `target_<N>/response.txt` under `lang/src/synthesis/imp/z3_chc/` whose
first line is `// Synthesised by RuSmt / Z3.` is a runnable witness. Feeding one
back through the interpreter closes the loop:

1. `parse_imp_source` parses it — must succeed.
2. `eval_com(_, Array::new())` evaluates it — must return `EvalResult::Err(_)`,
   proving the rendered witness is faithful (the same path-condition marker
   fires).

Non-`sat` passthroughs have no witness to replay. Replay one directly:

```bash
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.txt
# → [RuSmt] Path-condition marker reached during execution.
```

### Reproducible end-to-end

```bash
# (1) generate witnesses
cargo run -p rusmt-smt-derive -- imp eval_com k=3

# (2) inspect one
cat lang/src/synthesis/imp/z3_chc/target_0/response.txt

# (3) replay it directly
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_chc/target_0/response.txt
# → [RuSmt] Path-condition marker reached during execution.
```

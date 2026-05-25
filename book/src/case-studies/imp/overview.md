## IMP / WHILE case study

IMP (also called WHILE) is the canonical small imperative language from
Glynn Winskel, *The Formal Semantics of Programming Languages: An
Introduction* (MIT Press, 1993), Ch. 2. It has three syntactic categories —
arithmetic expressions, boolean expressions, and commands — and a
straightforward big-step semantics over a single mutable store.

In RuSmt, IMP serves as the **end-to-end pipeline demo**. Where the TOML
case study hits Z3's scaling frontier (recursive parsing produces queries Z3
times out for most targets), IMP is small enough that the loop closes:
synthesis returns concrete witnesses, the printer renders them as runnable
`.imp` source, and the witness-replay test confirms the same path-condition
marker fires when the rendered program is fed back through the interpreter.

### Layout

The interpreter lives in `lang/src/imp/`:

| File | Purpose |
|------|---------|
| `lang/src/imp/mod.rs` | Crate-public big-step evaluators `eval_aexp` / `eval_bexp` / `eval_com`, plus the `EvalResult` type and the synthesis markers. |
| `lang/src/imp/ast.rs` | The three syntactic categories (`Aexp`, `Bexp`, `Com`) as `#[smt_type]` enums. |
| `lang/src/imp/parser.rs` | Plain-Rust recursive-descent parser for the concrete `.imp` syntax + a `format_store` pretty-printer for final stores. |

The state is `Array<String, I64>`. Reads from an unwritten location return
an error, whereas in Winskel's convention _all locations initially hold 0_. 

> Note that two components sit in the Trusted Computing  Base: the parser that reads concrete .imp syntax into  an AST, and the pretty-printer that renders an AST (which was generated from Z3) back to concrete syntax. Neither is verified, so any bug in them is a bug in the results.

### The path-condition markers

Two `Path::fresh()` calls are used:

- **target 0** fires when a dividion-by-zero happens.
- **target 1** fires when a variable is undefined but used.

These are the targets the synthesis pipeline chases — see
[Synthesis loop](synthesis.md).

### Why IMP

 We use IMP, the canonical small imperative language from Winskel's The Formal Semantics of Programming Languages (MIT Press, 1993, §2.1), as the working case study on two grounds.

- **Citeable, auditable semantics.** IMP's abstract syntax and big-step operational rules fit on a single textbook page. A reviewer can hold our runnable specification against the published rules and confirm correspondence by inspection.
- **Bounded synthesis terminates.** Bounded-recursion unrolling at small `k` keeps the SMT query inside Z3's solving budget.

### Reading order

- [Parser](parser.md) — the `.imp` concrete syntax and its deliberate
  deviations from Winskel.
- [AST](ast.md) — the `#[smt_type]` enums and how they map to Winskel's
  grammar.
- [Synthesis loop](synthesis.md) — how `Path::fresh()` markers become Z3
  path conditions, how to invoke the synthesis pipeline, and how the printer
  closes the loop.

### Related references

- Glynn Winskel, *The Formal Semantics of Programming Languages: An
  Introduction.* MIT Press, 1993. Chapter 2 (especially §2.1 syntax and
  §2.2–§2.4 big-step rules).

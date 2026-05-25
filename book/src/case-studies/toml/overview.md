## TOML v1.1.0 case study

The `rusmt-lang` crate contains a TOML v1.1.0 parser implemented *in the
RuSmt DSL*. It is a complete
**runnable specification**: every grammar rule from the
[TOML 1.1.0 spec](https://toml.io/en/v1.1.0#spec) is realised as a
`#[smt_fn]` parsing function over `Seq<U32>` (Unicode code points).

Concretely, this gives:

- **Concrete execution**: parse a real `.toml` file and pretty-print its AST
  (`cargo run -p rusmt-lang -- toml ...`).
- **Symbolic compilation**: lift the parser into SMT-LIB via
  `rusmt-smt-derive`, ready for Z3 to query.

### Synthesis status

The TOML grammar is large and deeply recursive. When `rusmt-smt-derive`
lifts it into SMT, the resulting query for "find an input that reaches
path-condition target N" frequently exceeds Z3's solving budget. This is **a scaling-frontier finding, not a defect**. The bottleneck is solver capability on deeply recursive queries with combined string/integer/array reasoning. Fewer unrollings help less than expected; more unrollings make the query larger without unblocking Z3.

The TOML chapters in this section describe the parser as it stands. For an
end-to-end demo where the loop closes — synthesis returns witnesses, the
printer renders them as runnable source, and a replay test confirms the
markers fire — see the [IMP case study](../imp/overview.md).

### Running synthesis

```bash
# Default: text backend (SMT-LIB2 + Z3 subprocess), no unrolling.
cargo run -p rusmt-smt-derive -- toml parse_toml

# API backend (in-process Z3 via z3-sys).
cargo run -p rusmt-smt-derive -- toml parse_toml api

# Both, for comparison.
cargo run -p rusmt-smt-derive -- toml parse_toml both

# Bounded-recursion unrolling at depth N.
cargo run -p rusmt-smt-derive -- toml parse_toml k=3
```

Output goes under `lang/src/synthesis/toml/`, with a per-backend
subdirectory (`z3_chc/` for text, `z3_api/` for API) and one
`target_<N>/response.txt` per path-condition target.
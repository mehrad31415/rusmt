## Crate: `rusmart-smt-remark` (+ `rusmart-smt-remark-derive`)

The remark layer exists to enforce **syntactic restrictions** that make Rusmart code transpilable.

### What it provides

- `#[smt_type]`: marks `struct`/`enum` types that are part of the DSL
- `#[smt_fn]`: marks top-level functions that are part of the DSL

The procedural macros live in `smt/remark/remark_derive` and delegate most logic to `smt/remark`.

### What it does *not* do

The remark layer does not try to “translate to SMT”. It intentionally does only lightweight syntactic checking (no `async`, no `unsafe`, restricted generics, etc.). Deeper checks and translation happen in `rusmart-smt-derive`.

For the current set of enforced rules, see `smt/remark/README.md`.


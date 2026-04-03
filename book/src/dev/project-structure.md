## Project Structure

The project directory structure is as follows:

- [**Documentation book**](doc.md) (`book/`)
- [**smt**](smt.md) (`smt/`)
  - [stdlib](smt/stdlib.md) -- SMT-backed Rust types
  - [remark](smt/remark.md) -- annotation system (`#[smt_type]`, `#[smt_fn]`)
  - [derive](smt/derive.md) -- parser, IR, and Z3 backends
    - `src/parser/` -- DSL parsing, intrinsic recognition, overload resolution
    - `src/ir/` -- expression lowering, SMT sort checking
    - `src/backend/z3/` -- text backend: generates SMT-LIB2 files, spawns Z3 as subprocess
    - `src/backend/z3_api/` -- API backend: in-process Z3 via `z3-sys` and `Z3_eval_smtlib2_string`
- [**lang**](../case-studies/toml/overview.md) (`lang/`)
  - `src/toml/` -- TOML v1.1.0 parser implementation
  - `src/synthesis/` -- synthesis output directory (per-backend results)

In addition, the repository contains `documents/` (proposal/report material and other writeups) and top-level workspace configuration (`Cargo.toml`, `Makefile`, etc.). Some of the documents may be outdated.
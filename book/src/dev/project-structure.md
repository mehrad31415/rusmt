## Project Structure

The project directory structure is as follows:

- [**Documentation book**](doc.md) (`book/`)
- [**Bundled dependencies / solver**](deps.md) (`solver/`)
- [**smt**](crates/smt.md) (`smt/`)
  - [stdlib](crates/smt/stdlib.md)
  - [remark](crates/smt/remark.md)
  - [derive](crates/smt/derive.md)
- [**lang**](crates/lang.md) (`lang/`) (currently: TOML)
- [**programs**](crates/programs.md) (`programs/`) (test programs + harness)

In addition, the repository contains `documents/` (proposal/report material and other writeups) and top-level workspace configuration (`Cargo.toml`, `Makefile`, etc.).
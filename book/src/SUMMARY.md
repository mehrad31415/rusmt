# Summary

[Introduction](README.md)

# User guide
- [Quickstart](user/quickstart.md)
- [Syntax subset](user/syntax.md)
- [Type system](user/typing.md)
- [Annotations](user/annotations.md)
- [`stdlib` (intrinsics)](user/stdlib.md)

# Case studies
- [TOML v1.0](case-studies/toml/overview.md)
  - [Parser architecture](case-studies/toml/parser.md)
  - [AST and value model](case-studies/toml/ast.md)

# Developer guide
- [Project Setup](dev/project-setup.md)
- [Project Structure](dev/project-structure.md)
  - [doc](dev/doc.md)
  - [deps](dev/deps.md)
  - [smt](dev/crates/smt.md)
    - [stdlib](dev/crates/smt/stdlib.md)
    - [remark](dev/crates/smt/remark.md)
    - [derive](dev/crates/smt/derive.md)
  - [lang](dev/crates/lang.md)
  - [programs](dev/crates/programs.md)
  - [testing](dev/testing.md)


# Report (proposal-aligned)

- [Introduction](report/introduction.md)
- [Background and methodology](report/methodology.md)
- [System architecture](report/architecture.md)
- [Transpilation pipeline](report/transpiler.md)
- [TOML case study](report/toml.md)
- [Evaluation](report/evaluation.md)
- [Related work](report/related-work.md)
- [Conclusion](report/conclusion.md)

# Rusmart Transpiler Internals

- [Pipeline](dev/pipeline.md)
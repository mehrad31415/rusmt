## Introduction

Implementations of programming-language tooling (parsers, interpreters, compilers) are hard to test well:

- random testing often misses edge cases,
- hand-written suites are expensive and incomplete,
- formal methods usually require writing separate specifications.

Rusmart’s approach is to write the reference semantics **once** (as executable Rust code in a restricted DSL), then mechanically translate it into SMT so that an SMT solver can **synthesize inputs** and **prove/disprove properties** about executions.

In this repository, the motivating case study is a TOML v1.0 parser implemented in the Rusmart DSL.


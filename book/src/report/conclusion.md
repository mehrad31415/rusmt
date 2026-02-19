## Conclusion

Rusmart provides a practical path from an executable reference semantics to solver-backed reasoning:

- write the semantics once (as Rust code in a restricted DSL),
- mechanically translate it to SMT,
- use the solver to synthesize tests or validate properties.

In this repository, the TOML v1.0 parser demonstrates the full path from Rust to SMT-LIB and solver responses.


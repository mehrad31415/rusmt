## System architecture

At a high level:

1. **Language implementation** (e.g., TOML) is written against SMT-backed types.
2. The derive crate parses the program and builds an **Intermediate Representation (IR)**.
3. A backend emits **SMT-LIB** (Z3-oriented in this repo).
4. Z3 is invoked to solve or synthesize inputs; the resulting model can be turned into concrete test cases.

Repository mapping:

- `smt/stdlib`: SMT-backed Rust types (Boolean, Integer, Seq, Array, …)
- `smt/remark`: proc-macro annotations (`#[smt_fn]`, `#[smt_type]`)
- `smt/derive`: Rust→IR→SMT-LIB pipeline
- `lang`: reference semantics (TOML parser)
- `solver`: bundled solver bindings/build (Z3)
- `programs`: test programs and harness


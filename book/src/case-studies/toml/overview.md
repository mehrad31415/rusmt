## TOML v1.0 case study

The `rusmart-lang` crate contains a TOML v1.1.0 parser implemented *in the Rusmart DSL* (restricted Rust + `rusmart-smt-stdlib`).
It serves as the _reference semantics_ example for:

- **Concrete execution**: run it as a normal Rust program (`cargo run -p rusmart-lang -- toml ...`)
- **Symbolic compilation**: transpile it into SMT-LIB via `rusmart-smt-derive`
- **Synthesis / conformance**: query Z3 over the SMT encoding to generate inputs that are feeded to the reference interpreter and commercial intepreter to find divergences

The code lives under the `lang/src/toml` directory. The next two chapters explain how the parser is structured and how the TOML value model maps onto SMT sorts.


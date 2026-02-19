## TOML v1.0 case study

The `rusmart-lang` crate contains a TOML v1.0.0 parser implemented *in the Rusmart DSL* (restricted Rust + `rusmart-smt-stdlib`).
It serves as the end-to-end “reference semantics” example for:

- **Concrete execution**: run it as a normal Rust program (`cargo run -p rusmart-lang -- toml ...`)
- **Symbolic compilation**: transpile it into SMT-LIB via `rusmart-smt-derive`
- **Synthesis / conformance**: query Z3 over the SMT encoding to generate inputs

### Where the code lives

- **Parser entry point**: `lang/src/toml/mod.rs` (`parse_toml`)
- **Grammar-level components**: `lang/src/toml/{expr,key_value,table,array,string,integer,float,boolean,datetime}.rs`
- **Value model (AST)**: `lang/src/toml/ast.rs`

### Core data types

The parser’s state and results are expressed using SMT-backed types:

- `State { stream: Seq<String>, cursor: Integer, context: ParserContext }`
- `ParseResult<T>`: `NoMatch | Ok(T, State) | Err(Error)`
- `Value`: TOML values (`String`, `Integer`, `Float`, `Boolean`, `DateTime`, `Array`, `Table`)

The next two chapters explain how the parser is structured and how the TOML value model maps onto SMT sorts.


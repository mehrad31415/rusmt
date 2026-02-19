## Crate: `rusmart-lang`

The `lang/` crate hosts **reference semantics** implementations written in the Rusmart DSL.

### Today: TOML v1.0 parser

Implemented under `lang/src/toml/` and exposed as `rusmart_lang::toml`.

- **Concrete execution**: `lang/src/main.rs` implements the `rusmart-lang` CLI (`toml <file>`).
- **Symbolic compilation**: the TOML module uses only `rusmart-smt-stdlib` types plus `#[smt_fn]` / `#[smt_type]`, so it can be translated into SMT.

### Key APIs

- `parse_toml(State) -> ParseResult<Value>`
- `default_parser_context() -> ParserContext`

See the TOML case study chapter for the parser and AST layout.


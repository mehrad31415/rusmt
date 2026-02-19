## Syntax subset (Rusmart DSL)

Rusmart programs are **Rust programs** written under additional restrictions so they can be translated into SMT.
At a high level, the DSL is:

- **Expression-first and immutable**: model state via explicit values (e.g., `State` structs), not mutation.
- **ADT + recursion friendly**: `enum`/`match` and recursion are the main control mechanisms.
- **Stdlib-driven**: use `rusmart-smt-stdlib` types and intrinsic operations instead of Rust’s standard library.

### Commonly used, supported Rust forms

- `let` bindings (immutable)
- `if` / `match`
- `struct` / `enum` construction and pattern matching
- direct and mutually recursive function calls

### Commonly disallowed or intentionally avoided

These are typically rejected by the remarking/transpilation pipeline or are not meaningful for SMT translation:

- mutation (`let mut`, assignment)
- loops (`for`, `while`)
- references/pointers and borrowing (`&T`, `&mut T`)
- arbitrary standard-library collections (use `Seq<T>`, `Set<T>`, `Array<K, V>`)

### How to think about “parsing code” in the DSL

Instead of mutating a cursor into a string, parsers thread an explicit `State` value:

- `State { stream: Seq<String>, cursor: Integer, context: ParserContext }`
- each rule consumes a state and returns `ParseResult<T>`

This is exactly the shape used by the TOML parser in `lang/src/toml/`.
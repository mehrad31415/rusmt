## Syntax subset (Rusmart DSL)

Rusmart programs are **Rust programs** written under additional restrictions so they can be translated into SMT. In short, we use `rusmart-smt-stdlib` types and intrinsic operations instead of Rust’s standard library.

### Supported structures

- `let` bindings (immutable): Every method in the standard library returns a new _object_. 
- `if`: Every `if` statement must have a matching `else`. 
- `struct` / `enum` / `match`: construction and pattern matching.
- direct and mutually recursive function calls.

### Intentionally avoided structures

These are typically rejected by the remarking/transpilation pipeline:

- mutation (`let mut`, assignment)
- loops (`for`, `while`)
- references/pointers and borrowing (`&T`, `&mut T`)
- arbitrary standard-library collections (use `Seq<T>`, `Set<T>`, `Array<K, V>` instead)

This list is not comprehensive.

### How to think about “parsing code” in the DSL

Instead of mutating a cursor into a string, parsers thread an explicit `State` value:

- `State { stream: Seq<String>, cursor: Integer, context: ParserContext }`
- each rule consumes a state and returns `ParseResult<T>`

This is exactly the shape used by the TOML parser in `lang/src/toml/`.
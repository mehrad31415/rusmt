## AST

The TOML parser's output type is `Value`. It represents TOML values *after* parsing succeeds. The summary below is a guide, not the source of truth.

### `Value`

High-level shape:

- **Scalars**: `String`, `Integer` (64-bit), `Float` (64-bit), `Boolean`
- **Date/time**: represented as a dedicated `DateTime` ADT (internally still strings)
- **Collections**:
  - arrays: `Seq<Value>`
  - tables: `Array<String, Value>` (SMT-LIB “array” sort; used as a functional map)

### Why `Cloak<T>` shows up

`Value` is recursive (arrays and tables contain values). Rust requires a
self-referential ADT to be wrapped in some indirection so the type has a
known size, and RuSmt's frontend supplies `Cloak<T>` for that purpose:

- `Cloak::shield(t)` wraps a value
- `Cloak::reveal(c)` unwraps it

`Cloak<T>` is **only** a frontend wrapper. The IR strips it: every
`Cloak<T>` field becomes a plain `T` field, every `Cloak::shield`/
`reveal` lowers to identity. The SMT-LIB output therefore sees `Value`
as a direct mutually-recursive datatype, with no `Cloak` machinery in
the SMT-LIB output.


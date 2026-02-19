## AST and value model

The TOML parser’s output type is `Value` (defined in `lang/src/toml/ast.rs`).
It represents TOML values *after* parsing succeeds, and is designed to be both:

- **Executable** in Rust (for concrete parsing), and
- **SMT-transpilable** (for symbolic reasoning).

### `Value`

High-level shape:

- **Scalars**: `String`, `Integer` (64-bit), `Float` (64-bit), `Boolean`
- **Date/time**: represented as a dedicated `DateTime` ADT (internally still strings)
- **Collections**:
  - arrays: `Seq<Value>`
  - tables: `Array<String, Value>` (SMT-LIB “array” sort; used as a functional map)

### Why `Cloak<T>` shows up

`Value` is recursive (arrays and tables contain values).
To make recursive types manageable in the DSL/IR, the stdlib provides `Cloak<T>`:

- `Cloak::shield(t)` wraps a value
- `Cloak::reveal(c)` unwraps it

In the AST, arrays and tables are stored as `Cloak<...>` to keep recursion explicit and well-behaved for translation.


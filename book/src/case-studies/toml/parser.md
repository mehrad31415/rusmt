## Parser architecture

### Top-down, compositional rules

The TOML parser is written as a set of mutually recursive `#[smt_fn]` functions.
Each parsing function consumes a `State` and returns a `ParseResult<T>`:

- **`NoMatch`**: this rule does not apply at the current cursor (backtracking / choice)
- **`Ok(value, next_state)`**: matched successfully and advanced the cursor
- **`Err(Error)`**: hard parse error (invalid structure under the spec)

This representation is intentionally SMT-friendly: it avoids side effects and uses explicit state threading.

### Entry point and loop

`parse_toml` implements the ABNF top-level shape:

- `toml = expression *( newline expression )`

Concretely (in `lang/src/toml/mod.rs`), it parses one `expression` and then loops:

- parse `newline`
- parse the next `expression`
- merge the newly produced table fragment into the accumulated table

### Module breakdown (by grammar fragments)

- **`expr.rs`**: dispatches `expression` among comment/table/key-value rules
- **`table.rs`**: parses `[table]` and `[[array-of-tables]]` headers; updates `ParserContext`
- **`key_value.rs`**: parses `key = value` and dotted keys; constructs implicit tables
- **`string.rs` / `integer.rs` / `float.rs` / `boolean.rs` / `datetime.rs`**: value sub-parsers
- **`array.rs`**: parses TOML arrays (including whitespace/comment/newline rules)

### Context tracking (`ParserContext`)

TOML has _semantic constraints_ beyond pure syntax, e.g. redefinition rules for tables. For a complete overview of the constraints please look at [spec](https://toml.io/en/v1.1.0).

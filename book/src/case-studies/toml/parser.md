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

### Note on `Error::merge()`

`Error::merge(e1, e2)` is defined but intentionally not used in this parser.

**Why fail-fast is correct here:** Each `Error::fresh()` is a unique symbolic path marker. The SMT solver synthesizes one concrete input per target error — e.g., "find a TOML document that reaches error #37". These are independent synthesis goals.

Using `Error::merge(e1, e2)` would create a *combined* target asking Z3 to find a single input that simultaneously reaches *both* error paths. For a sequential parser this is rarely satisfiable: the parser stops at the first error, so a second error on the same document is not reachable in the same parse trace.

**When merge would help:** If the parser were restructured with error recovery (skip to the next newline on failure and continue), merge could accumulate errors across independent top-level expressions. The natural insertion point would be `parse_toml_loop` in `mod.rs`: after a failed `parse_expression`, skip the offending line, collect the error with `Error::merge`, and continue. This requires the `ParseResult` type to carry a partial-success-with-errors variant, which is a significant architectural change.

**Conclusion:** The current fail-fast, per-`Error::fresh()` design is correct and sufficient for individual path synthesis. Error collection across a document would require a dedicated error-recovery pass and a richer result type.

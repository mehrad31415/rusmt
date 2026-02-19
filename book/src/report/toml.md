## TOML case study

The TOML parser in `lang/src/toml/` is designed to be:

- **compositional** (small rules composed into larger ones),
- **explicit-state** (cursor and parser context are part of the state),
- **SMT-friendly** (no mutation; recursion and ADTs; stdlib intrinsics).

It parses TOML into a `Value` ADT and enforces TOML-specific constraints (e.g., table redefinition rules) via `ParserContext`.

See the case study chapters for the module-level breakdown and the AST model.


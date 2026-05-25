## IMP concrete syntax and parser

The plain-Rust recursive-descent parser lives in `lang/src/imp/parser.rs`.
Its public entry point is `parse_imp_source(&str) -> Result<Com, String>`.

### Concrete grammar

```text
com    ::= simple ( ";" simple )*
simple ::= "skip"
         | ident ":=" aexp
         | "if" bexp "then" block "else" block
         | "while" bexp "do" block
         | block
block  ::= "(" com ")"

aexp   ::= sum
sum    ::= product ( ("+" | "-") product )*
product::= atom_a ( ("*" | "/") atom_a )*
atom_a ::= ("-")? unsigned_int | ident | "(" aexp ")"

bexp   ::= bor
bor    ::= band ( "or" band )*
band   ::= bnot ( "and" bnot )*
bnot   ::= "not" bnot | bcmp
bcmp   ::= "true" | "false"
         | "(" bexp ")"
         | aexp ("==" | "<=") aexp
```

The grammar is whitespace-insensitive; identifiers match
`[A-Za-z_][A-Za-z0-9_]*`; integer literals are signed decimal.

### Strict ASCII

`parse_imp_source` rejects any byte `> 0x7F` outright. The interpreter only
needs to match ASCII variable names and ASCII operators, and the rejection
keeps the parser deterministic about source encoding (no surprises from
mojibake in identifiers).

### Important Notes

- **Comments.** `// ...` line comments are accepted, but **only at the very
  top of the file** — `parse_line_comments` runs once before `parse_com`.
  Mid-program comments are not supported. This is a deliberate concrete-
  syntax extension: it keeps the printer-emitted `response.txt` files
  parseable while preserving Winskel's grammar exactly inside any actual
  source code.
- **No `{ ... }` blocks.** Grouping uses `( ... )` only — see `parse_block`.
  This matches Winskel §2.1, where the surface grammar uses parentheses for
  grouping. Curly braces are *not* accepted.
- **No empty `()` shorthand.** `()` is not a valid command. Use `skip` for
  the no-op explicitly. (`parse_block` requires the inner `com` to be
  non-empty.)
- **`;` is a separator, not a terminator.** A trailing `;` immediately before
  a closing `)` is tolerated for ergonomic paste-back, but otherwise the
  shape is `c0 ; c1 ; ... ; cn`.
- Two intentional divergences from the source:
  - We add a division operator so that division-by-zero can be marked as a path condition.
  - Undefined variables do not default to 0 in the  store; reading an uninitialized variable is itself a flagged condition.

The grammar comment at the top of `lang/src/imp/parser.rs` is authoritative
if these notes ever drift.

### Parenthesised-bexp disambiguation

`bcmp` accepts both `(bexp)` (parenthesised boolean expression) and
`aexp == aexp` / `aexp <= aexp`. When the cursor sees `(`, the parser
*tries* to read a parenthesised bexp; if the next token after the closing
`)` is a comparison operator, it backs up and reparses the `(...)` as an
arithmetic atom. See `parse_bcmp`.

### Pretty-printing the final store

`format_store(store)` renders a final `Array<String, I64>` as one
`var = value` line per location, sorted by key. The IMP CLI
(`cargo run -p rusmt-lang -- imp <file>`) writes this to
`lang/imp/output/<stem>.txt` after each successful run.

### Example programs

`lang/imp/input/*.imp` holds eight hand-written programs (skip, assign, swap,
two if-branches, while loops, factorial), with the expected final store for
each in `lang/imp/output/`. Run one through the interpreter with:

```bash
cargo run -p rusmt-lang -- imp lang/imp/input/factorial.imp
```

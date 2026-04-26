## Parser architecture

### Top-down, compositional rules

The Rego subset parser is a collection of mutually recursive `#[smt_fn]`
functions. Each one consumes a `State` and returns a `ParseResult<T>`:

- **`NoMatch`** — this rule does not apply at the current cursor (the caller
  should try a different production).
- **`Ok(value, next_state)`** — matched successfully and advanced the cursor.
- **`Err(Error)`** — hard parse error; the parser stops with that error.

The shape is identical to the TOML case study, and intentionally so: it
threads input explicitly, has no side effects, and is therefore directly
SMT-translatable.

### Entry point and shape of a module

`parse_policy` (in `lang/src/rego/mod.rs`) is the top-level entry point. It
delegates to `parse_module` (in `lang/src/rego/module.rs`), which expects:

1. Optional whitespace, comments, and blank lines.
2. A single `package <ident>("." <ident>)*` clause (mandatory — a module
   without a package clause produces a hard error).
3. An explicit *out-of-scope* check for `import` statements; if one is
   detected, the parser produces an `Error::fresh()` target rather than
   silently NoMatch — this gives Z3 a clean synthesis goal for "modules that
   try to use multi-module features" while still keeping `import` formally
   out of scope.
4. A loop of zero or more rules, each consumed by `parse_rule`.

The loop fails (with a synthesis target) if it finds non-rule junk between
the package clause and end-of-input.

### Module breakdown (by grammar fragment)

The Rego grammar is partitioned across files matching syntactic categories:

- **`module.rs`** — `package` clause, `import` rejection, top-level rule loop.
- **`rule.rs`** — rule heads (`default`, complete, partial set, partial
  object) and the rule body delimiter handling. Also hosts the *evaluator*
  (see below) so the rule's parse-time and eval-time semantics live together.
- **`expr.rs`** — `not`, `:=`, comparisons, body composition (`;` / newline).
- **`term.rs`** — composite values (arrays, objects, sets), references,
  parenthesized terms, and the additive / multiplicative arithmetic precedence
  tower.
- **`literal.rs`** — `null` / boolean / number / string scalars.
- **`mod.rs`** — shared primitives (`State`, `ParseResult`, `Optional`,
  whitespace / newline helpers, identifier scanning, keyword matching).

### Disambiguating keywords vs. identifiers

Rego is case-sensitive and reserves several words (`true`, `false`, `null`,
`not`, `default`, `package`, `import`, `set`, `every`, `some`, `with`).

Wherever a parser tries one of these as a keyword, it pairs the literal match
with a `next_is_ident_cont` lookahead helper (defined in `literal.rs`). That
helper says "the token immediately after the matched prefix is an
identifier-continuation character" — and if so, the prefix wasn't actually
the keyword, it was the start of a longer identifier. This avoids
mis-parsing things like `defaulted` as the `default` keyword followed by
junk.

### How out-of-scope syntax is rejected

The README documents the subset boundary in detail. In code, out-of-scope
features fall into three buckets:

1. **Reserved-word rejection.** `every`, `some`, `with`, `import`, etc., are
   reserved in the identifier scanner; they never match `parse_ref_or_var`.
   So an expression starting with `every x in input` does not parse as
   `every` (a variable) followed by something — it fails *immediately* with
   `NoMatch`. The caller's parent rule then fails, propagating the `NoMatch`
   up.
2. **Pipe-character rejection in collection literals.** Comprehensions
   (`[x | body]`, `{k: v | body}`) start with the same prefix as array /
   object literals. We commit to "this is a literal" once the `[` or `{` is
   consumed, so encountering `|` where `,` or the closing brace is expected
   produces an `Error::fresh()` target.
3. **`import` rejection in modules.** After the package clause, the module
   parser checks for the `import` keyword and surfaces a dedicated synthesis
   target if found. This makes "policies that *try* to use imports" a
   targetable error class.

### Where errors are marked

`Error::fresh()` is reserved for *spec-level edge cases the synthesis system
should target*. The full numbered list lives in `lang/src/rego/ERRORS.md`.
Concretely:

- Parser-level errors: incomplete / malformed productions where the prefix
  has committed (e.g., `name[term]` with no `=` or `{` after, partial-object
  rule missing its body, `not` keyword with no following expression).
- Evaluator-level errors: division by zero, type-mismatched arithmetic,
  unbound variables, reference into a non-object, `:=` rebinding.

`NoMatch` (without `Error::fresh()`) is reserved for "this rule does not
apply, the caller should try a different production." It is never used to
silently swallow out-of-scope syntax — out-of-scope features either reject
via reserved-word handling (above) or commit to an `Error::fresh()` target.

### The evaluator

The evaluator lives alongside the rule parser in `rule.rs`. Its public entry
point is `eval_module`, but the more interesting top-level surface is
`evaluate_policy` in `mod.rs`, which composes parse + evaluate. Internally:

- `eval_module` maintains a base binding environment that always contains
  `input -> <input term>`. Each rule is applied in source order.
- `eval_body` walks the expressions of a rule body. Each expression returns
  one of `True(bindings)` (the body continues with possibly-updated
  bindings), `False` (the body short-circuits — the rule does not
  contribute), or `Err(Error)` (a hard semantic error — the rule does not
  contribute either, but the synthesis system records that the error path
  was reached).
- Default rules contribute their value only if no earlier-applied rule has
  produced a value for the same head name. Partial set / object rules
  accumulate elements / kvps across successful body evaluations.

The bindings carried by `EvalOutcome::True` are wrapped in `Cloak` because
the `#[smt_type]` macro cannot disambiguate the comma in
`Array<String, Term>` from a multi-arg tuple variant — the same pattern is
used in `lang/src/toml/ast.rs` for `Table(Cloak<Array<String, Value>>)`.

### Note on `Error::merge()`

`Error::merge(e1, e2)` is defined but intentionally not used in this
parser/evaluator. Each `Error::fresh()` is a unique symbolic path marker;
the SMT solver synthesizes one concrete input per target. Combining two
fresh markers via merge would ask Z3 to find an input that simultaneously
hits both paths — which is rarely satisfiable for a fail-fast parser, and
not meaningful for a body evaluator that follows a single trace.

If the language acquired a linting mode that surfaced every error in a
module rather than aborting at the first, merge would let the rule-loop
combine errors across independent rules. That requires a richer
`ParseResult` variant (partial-success-with-errors) and a non-trivial
architectural change. For now the fail-fast design is correct and
sufficient for individual path synthesis.

## AST and value model

The Rego subset's AST is defined in `lang/src/rego/ast.rs`. It mirrors the
spec's syntactic categories one-for-one, with one extra layer of indirection
(`Cloak<T>`) wherever the type recurses through itself.

### `Term`

`Term` is the core ADT — both the value model (after evaluation) and the
syntactic model for things appearing in term position before evaluation.

```rust
#[smt_type]
pub enum Term {
    Null,
    Boolean(Boolean),
    Number(Real),                              // JSON numbers as Real
    String(String),
    Var(String),                               // bare identifier
    Ref(Seq<String>),                          // dotted path (length >= 2)
    Array(Cloak<Seq<Term>>),
    Object(Cloak<Array<String, Term>>),
    Set(Cloak<Seq<Term>>),                     // dedup'd by the evaluator
    ArithExpr(ArithOp, Cloak<Term>, Cloak<Term>),
}
```

Why each variant exists:

- **`Null`** — Rego's `null` scalar. Falsy under `is_truthy`.
- **`Boolean`** — Rego's `true` / `false`. Carries the SMT `Boolean` directly.
- **`Number`** — Rego numbers are JSON numbers (no separate integer type
  exists at the language level). We use `Real` (arbitrary-precision rational)
  for SMT decidability — `Real` lifts cleanly to Z3's theory of reals.
- **`String`** — single-line string literal. Backslash escapes `\"`, `\\`,
  `\n`, `\t`, `\r` are honored.
- **`Var`** — bare identifier appearing in term position (`x`). The
  evaluator looks it up in bindings.
- **`Ref`** — dotted reference of length ≥ 2 (`input.user.role`). The first
  segment is the binding root; subsequent segments descend through nested
  objects.
- **`Array`** / **`Object`** / **`Set`** — composite literals. They wrap the
  contained collection in `Cloak<...>` because the type is recursive
  (`Seq<Term>` requires `Term: SMT`, but `Term` itself contains arrays of
  terms; `Cloak` makes the recursion explicit).
- **`ArithExpr`** — `(lhs op rhs)`. Evaluation reduces this to `Number`
  if both sides are numeric, or to a hard error otherwise (the type-mismatch
  paths are synthesis targets — see `ERRORS.md` entries 81–99).

Note the parallel with the TOML case study's `Value` type: same `Cloak`
wrapping pattern, but Rego carries an extra `Var` / `Ref` distinction
(because Rego is a *language*, not a static document format) and an
`ArithExpr` constructor (because Rego allows arithmetic in any term
position).

### `ArithOp` and `CompareOp`

Two small unit-only enums:

```rust
#[smt_type] pub enum ArithOp   { Add, Sub, Mul, Div }
#[smt_type] pub enum CompareOp { Eq, Ne, Lt, Le, Gt, Ge }
```

They exist as separate types so the evaluator can dispatch on operator
identity without juggling string keywords or magic numbers.

### `Expr`

Body-level expressions:

```rust
#[smt_type]
pub enum Expr {
    Term(Term),                                // truth test or fact
    Not(Cloak<Expr>),                          // `not <expr>`
    Compare(CompareOp, Term, Term),
    Assign(String, Term),                      // `x := <term>`
}
```

`Expr::Term` is the truth-test variant: a bare term in body position
succeeds if the term evaluates to a truthy value (anything except `null` /
`false`). `Expr::Not` flips a sub-expression's truth. `Expr::Compare` is a
typed comparison. `Expr::Assign` is single-assignment binding.

`Cloak<Expr>` is used because `Not` would otherwise be infinitely recursive
through `Expr` itself.

### `Body` and `Rule`

```rust
#[smt_type] pub struct Body { pub exprs: Seq<Expr> }

#[smt_type]
pub enum RuleHead {
    Default(String, Term),
    Complete(String, Term),
    PartialSet(String, Term),
    PartialObject(String, Term, Term),         // (name, key, value)
}

#[smt_type] pub struct Rule { pub head: RuleHead, pub body: Body }
```

The four `RuleHead` variants enumerate the rule shapes the spec defines.
`Default` carries an empty body (the head value is unconditional). The other
three carry a real body that the evaluator must succeed before the rule
contributes.

### `Module`

```rust
#[smt_type]
pub struct Module {
    pub package: Seq<String>,
    pub rules: Seq<Rule>,
}
```

A `Module` is exactly what a single Rego file produces: the package path
(non-empty by construction — the parser rejects modules without a `package`
clause) plus a sequence of rules in source order.

### Evaluation outcomes

Three small types live in `rule.rs` to thread synthesis-relevant errors:

```rust
#[smt_type] pub enum TermVal { Val(Term), Err(Error) }

#[smt_type] pub enum BodyOutcome { Ok, Fail, Err(Error) }

#[smt_type]
pub enum EvalOutcome {
    False,                                     // FIRST variant intentionally
    True(Cloak<Array<String, Term>>),          // updated bindings
    Err(Error),
}
```

Two notes on the `EvalOutcome` shape:

1. **`False` is the first variant.** The `#[smt_type]` macro generates a
   `Default` impl that uses the first variant's payload. If `True` were
   first, the macro would emit `Cloak<Array<String, Term>>::default()`,
   which rustc rejects without turbofish (the `<,>` parses as comparisons).
   Putting `False` first keeps the generated `default()` simple
   (`Self::False`).
2. **`Cloak<Array<String, Term>>`.** Same trick TOML uses for
   `Table(Cloak<Array<String, Value>>)` — wrapping in `Cloak` resolves the
   macro's parsing of `Array<String, Term>` and keeps the recursion through
   `Term` explicit.

### Why `Cloak<T>` shows up

Like TOML's `Value`, the Rego AST has recursive types: arrays contain
arrays, objects map to terms (which can be arrays again), arithmetic
expressions nest, and the evaluator's outcome type carries a binding map
that itself maps to terms. To keep these definitions finite at the IR
level, the stdlib provides:

- `Cloak::shield(t)` — wrap a value at a recursion site.
- `Cloak::reveal(c)` — unwrap when consuming.

Every recursion site in the AST is wrapped in `Cloak`, mirroring the TOML
convention.

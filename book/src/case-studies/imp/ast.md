## IMP AST

Three `#[smt_type]` enums live in `lang/src/imp/ast.rs`. They map directly to
Winskel's §2.1 grammar.

### Extended Winskel §2.1 abstract syntax

```
Aexp (a) ::= n | X | a0 + a1 | a0 - a1 | a0 * a1 | a0 / a1
Bexp (b) ::= true | false
           | a0 == a1 | a0 <= a1
           | not b | b0 and b1 | b0 or b1
Com  (c) ::= skip | X := a | c0 ; c1
           | if b then c0 else c1
           | while b do c
```

### Mapping to RuSmt

| Winskel | Rust enum (variant) | Notes |
|---------|--------------------|-------|
| `n` (numeral) | `Aexp::Num(I64)` | Signed 64-bit bitvector. Overflow wraps in two's complement (Z3 BV semantics). |
| `X` (location) | `Aexp::Var(String)` | Locations are identified by name; the store is `Array<String, I64>`. |
| `a0 + a1` | `Aexp::Add(Cloak<Aexp>, Cloak<Aexp>)` | `Cloak<T>` is a **frontend-only** wrapper that satisfies Rust's sized-type requirement for self-referential ADTs; it is stripped at the IR layer, so the SMT-LIB output sees a direct mutually-recursive `Aexp`/`Bexp`/`Com` datatype family. |
| `a0 - a1` | `Aexp::Sub(...)` | |
| `a0 * a1` | `Aexp::Mul(...)` | |
| `a0 / a1` | `Aexp::Div(...)` | |
| `true`, `false` | `Bexp::True`, `Bexp::False` | Variant atoms. |
| `a0 == a1` | `Bexp::Eq(Cloak<Aexp>, Cloak<Aexp>)` | |
| `a0 <= a1` | `Bexp::Le(...)` | |
| `not b` | `Bexp::Not(Cloak<Bexp>)` | |
| `b0 and b1` | `Bexp::And(...)` | |
| `b0 or b1` | `Bexp::Or(...)` | |
| `skip` | `Com::Skip` | |
| `X := a` | `Com::Assign(String, Cloak<Aexp>)` | |
| `c0 ; c1` | `Com::Seq(Cloak<Com>, Cloak<Com>)` | `;` is a *separator*, not a terminator. |
| `if b then c0 else c1` | `Com::If(Cloak<Bexp>, Cloak<Com>, Cloak<Com>)` | |
| `while b do c` | `Com::While(Cloak<Bexp>, Cloak<Com>)` | |

The exact Rust definitions are in `lang/src/imp/ast.rs`.

### `EvalResult`

```rust
#[smt_type]
pub enum EvalResult {
    Err(Path),
    Ok(Array<String, I64>),
}
```

`EvalResult::Err` exists *only* to carry path-condition markers (`Path`
values). Successful execution always returns the resulting store via `EvalResult::Ok`. Division-by-zero and using undefined variable will result in an error state.

`Err` is listed first so the auto-derived `Default` impl picks it (it would
otherwise have to construct an `Array<String, I64>`, and using `Err` keeps
the default trivial).

### Big-step evaluators

The three functions in `lang/src/imp/mod.rs` are `#[smt_fn]` and follow the
shape of Winskel's rules verbatim:

```text
eval_aexp : (Aexp, Array<String, I64>) -> I64
eval_bexp : (Bexp, Array<String, I64>) -> Boolean
eval_com  : (Com,  Array<String, I64>) -> EvalResult
```
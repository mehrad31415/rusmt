# STLC: a static-semantics case study

The third case study is a type checker for the **simply-typed lambda calculus
(STLC)**. Where the [IMP](../imp/overview.md) study exercises *dynamic*
semantics (evaluation, runtime errors) and [TOML](../toml/overview.md) exercises
*syntactic* well-formedness, STLC exercises **static semantics**: the input is a
term and the output is either its type or a named **type-error marker**. The
three case studies are deliberately chosen to span the syntactic-vs-semantic
error taxonomy the framework targets.

Source: `lang/src/typecheck/` (`ast.rs`, `mod.rs`), the concrete syntax in
`lang/src/typecheck_syntax.rs`, and the model renderer in
`lang/src/typecheck_render.rs`.

## The object language

```text
ty   ::= int | bool | (fun ty ty)
expr ::= (int N) | (bool true|false) | (var name)
       | (add expr expr) | (eq expr expr) | (if expr expr expr)
       | (let name expr expr)            ; let-binding, type inferred
       | (abs name ty expr)             ; \name : ty . expr
       | (app expr expr)
```

It is a genuinely **recursive** language (operands are nested `expr`s) with a
**recursive type algebra** (the arrow `fun`). Both facts matter for synthesis.

## The eight semantic markers

Every marker is a `Path::named` *semantic* (typing-rule) violation:

| Marker | Fires when |
|---|---|
| `unbound_variable`        | a variable is free under the environment |
| `add_lhs_not_int`         | the left operand of `+` is not `int` |
| `add_rhs_not_int`         | the right operand of `+` is not `int` |
| `eq_type_mismatch`        | `==` compares operands of different types |
| `if_cond_not_bool`        | an `if` condition is not `bool` |
| `if_branch_mismatch`      | the two `if` branches have different types |
| `app_callee_not_function` | a non-function is applied |
| `app_arg_type_mismatch`   | an argument's type differs from the parameter |

## Two design lessons

**1. The environment is an array-free assoc-list.** The natural encoding of a
typing environment is `Array<String, Ty>`. But `Ty` is recursive (`Ty::Fun`), so
this is an *array whose values are a recursive datatype* — precisely the encoding
the [TOML case study](../toml/overview.md) showed makes Z3 declare the goal
`incomplete (theory array)` even on concrete inputs. STLC therefore threads the
environment as an inductive datatype:

```rust
#[smt_type]
pub enum Env { Empty, Bind(String, Cloak<Ty>, Cloak<Env>) }
```

The encoding discipline learned on TOML transfers directly.

**2. A non-error `Bottom` terminator makes bounded unrolling productive.** The
checker recurses over three datatypes with structural type equality, so
native-recursion *search* (`k=0`) does not finish within a practical budget — STLC
sits between IMP (shallow, `k=0` solves) and TOML (out of reach). We therefore
synthesize under **bounded unrolling** (`k=N`). Naively, unrolling produces
*spurious* models: when recursion hits the depth cutoff, the terminator value can
satisfy an `Err(marker)` assertion without a genuine path (replay rejects these,
so soundness holds, but no witness is found). The fix is to give the result type
a leading **nullary `Bottom` variant** for the cutoff to return — a *non-error*
value, so the cutoff can no longer discharge a marker assertion and the solver
must exhibit a real typing-rule violation within depth `N`. `Bottom` is never
produced by the concrete checker, so concrete semantics are unchanged.

```rust
#[smt_type]
pub enum TypeResult { Bottom, Err(Path), Ok(Ty) }
```

## Reproducing

```bash
# Synthesize all eight markers under depth-4 unrolling (text backend).
RUSMT_BACKEND_TIMEOUT_SECS=20 \
  cargo run -p rusmt-smt-derive -- typecheck type_check text k=4

# Each target's replay verdict:
cat lang/src/synthesis/typecheck/z3_chc/target_0/replay.txt
#   CERTIFIED: replay through the reference semantics fired `unbound_variable`

# Unit tests (round-tripping, all 8 markers, higher-order terms):
cargo test -p rusmt-lang --test typecheck
```

Every one of the nine targets (the apply-a-non-function marker is reached from
two match arms) synthesizes and is replay-certified in 5–7 s — including a
*higher-order* witness for `app_arg_type_mismatch` in which the solver builds an
applied `\x.e` whose argument type differs from its parameter. A committed,
reproducible copy of the suite lives under
`generated-suites/typecheck-pipeline-run/`.

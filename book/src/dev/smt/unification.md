### Type Unification

---

RuSmt's transpiler performs **type inference** via a **unification-based** algorithm. This chapter explains the algorithm.

### Why Unification?

RuSmt code often omits type annotations on local variables:

```rust
let x = Seq::new();          // what is T in Seq<T>?
let y = Seq::append(x, v);   // now we know T from v's type
```

The transpiler cannot assign a concrete type to `x` when it first encounters the declaration. Instead, it assigns a **type variable** — a placeholder — and later **unifies** it with concrete type information that flows in from surrounding expressions. This is the same core idea behind **Hindley–Milner** type inference used in ML, Haskell, and Rust itself, though RuSmt's version is simpler because it operates on a fixed set of SMT sorts rather than arbitrary polymorphic types.

### TypeTag vs TypeRef

The parser defines two parallel type representations:

| | `TypeTag` | `TypeRef` |
|---|---|---|
| **Defined in** | `parser/ty.rs` | `parser/infer.rs` |
| **Purpose** | Represents fully-known types from source annotations | Represents types during inference (may contain unknowns) |
| **Extra variant** | — | `Var(TypeVar)` — an unresolved placeholder |

Every `TypeTag` can be converted to a `TypeRef` via `From<&TypeTag>`. The reverse is only valid after inference succeeds and all `Var` placeholders have been resolved.

> Type Variables: A `TypeVar(usize)` is a unique placeholder created by `TypeUnifier::mk_var()`. Each call produces a fresh index. In error messages and debug output, type variables are displayed as `?0`, `?1`, etc.

### Equivalence Groups

The unifier maintains a **union-find** structure with two components:

```
params: BTreeMap<usize, usize>    // var_id → group_index
groups: Vec<TypeEquivGroup>        // the equivalence groups
```

Each `TypeEquivGroup` contains:

- **`vars: BTreeSet<usize>`** — the set of type variable indices that are known to be the same type.
- **`sort: Option<TypeRef>`** — the concrete type assigned to this group, if known.

When a group has no concrete type yet (`sort: None`), its _representative_ is the variable with the smallest index in `vars`. When a concrete type is assigned, the _representative_ is that type.

### The Unification Algorithm

The `ti_unify!` macro wraps a call to `unifier.unify(lhs, rhs)` with error handling. The core inner function `unify(lhs, rhs, involved)` takes two `TypeRef` values and attempts to make them equal. It returns:

- `Ok(Some(unified_type))` — success; the two types are compatible
- `Ok(None)` — failure; the types are incompatible (e.g., `Integer` vs `String`)
- `Err(CyclicUnification)` — a type variable refers to itself (infinite type)

### Case Analysis

The algorithm proceeds by case analysis on the pair `(lhs, rhs)`:

**Both are variables** (`Var(l)`, `Var(r)`):
- If `l == r`: no new information; return the variable.
- Otherwise: **merge** their equivalence groups. The group with the lower index absorbs the higher one. If both groups have concrete sorts, those sorts are recursively unified.

**One is a variable** (`Var(v)`, `T`) or (`T`, `Var(v)`):
- **Update** the variable's group: if the group has no sort, assign `T`; if it already has a sort `S`, recursively unify `S` with `T`.

**Both are concrete**:
- Ground types (`Boolean`, `Integer`, etc.): succeed only if identical.
- Parametric types (`Seq`, `Set`, `Array`, `User`, `Pack`): succeed if the constructor matches AND all inner types unify recursively.
- Type parameters (`Parameter`): succeed only if names match (e.g., `T` unifies with `T`, but not with `U`).
- All other combinations: fail (`Ok(None)`).

### Cycle Detection

The `involved` set tracks which type variable indices have been encountered during the current `merge_group` call chain. Cycle detection only fires inside `merge_group` (i.e., when both sides are variables and their groups are being merged). Before merging, the algorithm checks whether either group's variables overlap with `involved`. If so, it returns `Err(CyclicUnification)`. After the check passes, both groups' variables are added to `involved` for subsequent recursive calls.

A fresh `involved` set is created for each top-level `unify` call from `TypeUnifier::unify`.

**Important subtlety:** `update_group` (used when one side is a variable and the other is a concrete type) does **not** check `involved` before the initial assignment. For example, unifying `?0` with `Seq<?0>` calls `update_group(0, Seq(?0))`, which simply assigns `sort = Seq(?0)` to group 0 without error. The cycle is only caught later if this creates a conflict during a `merge_group` call. For instance:

```
// This does NOT trigger a cycle error:
unify(?0, Seq<?0>)   →  update_group: group 0 gets sort = Seq<?0>

// This DOES trigger a cycle error:
// Suppose group 0 has sort=Seq<?1> and group 1 has sort=Seq(?0).
// Merging ?0 and ?1 via merge_group:
//   1. involved = {0, 1}
//   2. Recursively unify Seq<?1> with Seq<?0>
//   3. This calls unify(?1, ?0), which calls merge_group(0, 1)
//   4. involved ∩ {0} ≠ ∅ → CyclicUnification!
```

In practice, this is sufficient because RuSmt's type system does not have recursive type constructors (outside of `Cloak`, which is structurally acyclic). The `update_group` path handles the common case of assigning a concrete type to a fresh variable, while `merge_group` catches mutual recursion between two type variable groups.

### Handling Expressions

**Expected-Type Propagation**: The expression parser (`ExprParserCursor`) carries an **expected type** (`exp_ty: TypeRef`). This is the type that the current expression is expected to produce — typically the return type of the function, or the declared type of a `let` binding. In case of absence this will be a fresh type variable. When the parser resolves an expression, it calls `ti_unify!(unifier, &actual_ty, &self.exp_ty, span)` to unify the actual type with the expected type. This is how type information flows **bidirectionally**:

- **Top-down**: a declared type annotation constrains what sub-expressions must produce.
- **Bottom-up**: a literal or function return type constrains the expected type variable above.

```rust
// Top-down: exp_ty = Integer propagates into the function call
let x: Integer = Integer::add(a, b);

// Bottom-up: Integer::from(42) produces Integer, which flows up to constrain x
let x = Integer::from(42);
```

**Forking**: When parsing a sub-expression that has a different expected type, the parser **forks** itself:

```rust
let sub_ctxt = self.fork(TypeRef::Var(unifier.mk_var()));
```

This creates a new cursor with a fresh expected type but the same variable scope.

### Probing Unifiers for Overload Resolution

Intrinsic methods like `add` are defined on multiple types (`Integer`, `Real`, `F32`, etc.). When the parser encounters a method call, it must determine which overload matches. The `query_with_inference` function in `apply.rs` does this:

1. Collect all candidate functions matching the method name.
2. For each candidate, **clone** the unifier into a **probing** copy.
3. Attempt to unify each argument type and the return type against the candidate's signature using the probe.
4. If all unifications succeed, the candidate is viable.
5. If exactly one candidate is viable, commit the probe as the new unifier state.
6. If zero or more than one candidate matches, report an error.

This technique ensures that a failed overload attempt does not corrupt the main unifier state.

### Post-Inference Refresh and Validation

After the entire function body is parsed, the parser calls `refresh_type` on every type in the expression tree. This replaces each `TypeRef::Var` with its resolved concrete type (if known). If any variable remains unresolved after refresh, the function fails with an "incomplete type" error — meaning there was not enough information to determine all types.

### Worked Example

Consider:

```rust
fn example(v: Integer) -> Seq<Integer> {
    let x = Seq::new();           // (1)
    let y = Seq::append(x, v);    // (2)
    y                             // (3)
}
```

**Step 1** — `let x = Seq::new()`:
- `x` has no annotation, so `exp_ty = ?0` (fresh variable).
- `Seq::new()` returns `Seq<?1>` (T is unknown).
- Unify `?0` with `Seq<?1>`: group for `?0` gets sort `Seq<?1>`.
- State: `?0 = Seq<?1>`, `?1` = unknown.

**Step 2** — `let y = Seq::append(x, v)`:
- `y` gets `exp_ty = ?2`.
- `Seq::append` has signature `(Seq<T>, T) -> Seq<T>`.
- Instantiate T as `?3`.
- Unify first arg: `Seq<?3>` with `x`'s type `Seq<?1>` → merges `?3` and `?1`.
- Unify second arg: `?3` with `v`'s type `Integer` → `?3 = Integer`, and since `?1` is in the same group, `?1 = Integer`.
- Unify return: `Seq<?3>` = `Seq<Integer>` with `?2` → `?2 = Seq<Integer>`.
- State: `?0 = Seq<Integer>`, `?1 = ?3 = Integer`, `?2 = Seq<Integer>`.

**Step 3** — `y`:
- Unify `y`'s type `Seq<Integer>` with the function return type `Seq<Integer>` → success.

**Refresh**: all variables resolve to concrete types. The function type-checks.

### Design Inspirations

The RuSmt type unifier draws from several well-established techniques:

- **Hindley–Milner type inference** (Damas & Milner, 1982): the idea of assigning type variables to unannotated bindings and solving constraints via unification. RuSmt uses a simplified version without `let`-polymorphism (generalization), since DSL functions have explicitly declared type parameters.

- **Union-find** (Tarjan, 1975): the equivalence-group structure is a union-find where `merge_group` performs the *union* operation (merging two groups) and `retrieve_type` performs the *find* operation (looking up the representative). RuSmt uses the convention that the group with the lower variable index absorbs the higher one.

- **Bidirectional information flow**: type information propagates both top-down (an expected type / annotation constrains sub-expressions) and bottom-up (a literal or return type constrains the expected variable above).
### Constraints and Limitations

1. **No `let`-polymorphism**: a local variable like `let id = ...` cannot be used at two different type instantiations. Each use must agree on one concrete type. So the following is invalid:

```ocaml
 let id = fun x -> x in
(id 3, id "hello")
```

2. **No subtyping**: `Integer` and `I32` are distinct sorts. There is no implicit widening or coercion — explicit conversion functions must be used.

3. **Type parameters unify by name**: `Parameter("T")` only unifies with `Parameter("T")`, never with `Parameter("U")` or type parameters with other names. This is sound because type parameters are scoped to their declaring function or type.

4. **No inference across function boundaries**: each function is type-checked independently with its full signature. The unifier state does not leak between functions.

5. **Tuple and pack destructuring**: tuples must be destructured via `let (a, b) = expr;` — there is no partial inference on individual tuple elements through field access.

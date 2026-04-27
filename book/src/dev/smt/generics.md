## Generic Types and Functions: IR Registration vs Z3 Translation

This describes how generic types and functions flow through the system. Do not change this
design without understanding why each piece exists.

### IR Registration (build time)

The IR builder registers both generic templates and monomorphized versions for types and functions.

**Types (generic template + monomorphized entries):**
- `build()` registers the generic template `ParseResult<T>` (with `T` as `Sort::Uninterpreted("ParseResult_T")`).
  This is the starting point so the IR builder knows the type's structure (which variants, which fields). This is needed to translate the type in the backend.
- When a function body constructs `ParseResult::Ok(string_val, err)` with concrete types, the IR
  builder calls `register_type("ParseResult", [String])` creating a monomorphized entry (e.g. `sid=12`).
- This `sid` is needed for two reasons:
  1. Sort-checking during IR building -- two user types are only equal if their `sid` is equal.
  2. Z3 translation -- `reverse_lookup(sid=12)` returns `("ParseResult", [String])` so we can emit
     `(ParseResult String)` referencing the single parametric declaration.

**Functions (generic template + monomorphized versions):**
- `build()` registers the generic template `parse_value<T>` (with `T` as `Sort::Uninterpreted("parse_value_T")`).
  This is the entry point for processing the function's body and discovering what it calls.
- During body materialization (`ExpBuilder::materialize`), when a call like `helper::<String>(x)` is
  encountered, the builder calls `register_func("helper", [String])` which creates the monomorphized
  version. The builder's `ty_inst` maps `T -> String`, so recursive calls within the monomorphized body
  also resolve to monomorphized versions (not back to the generic template).
- Call sites in the IR point to monomorphized function IDs (e.g., `parse_value_String`), NOT the
  generic template. This is because the caller's `ty_inst` resolves `T` to a concrete sort.

### Z3 Translation (emit time)

**Types -- emit ONLY the generic template:**
- Use `(declare-datatypes ((ParseResult 1)) ((par (T) ...)))` -- one declaration per type name.
- Z3 supports parametric types, so `(ParseResult String)` and `(ParseResult Int)` are handled
  automatically by Z3. No separate declarations needed for monomorphized type entries.
- The monomorphized entries (e.g. `sid=12` for `ParseResult<String>`) are NOT declared separately.
  At translation time, `reverse_lookup(sid=12)` returns `("ParseResult", [String])` which is emitted
  as `(ParseResult String)` -- referencing the single parametric declaration.
- Deduplication picks the generic template as the representative for each type name (the one whose
  type params are `Uninterpreted` sorts starting with `"TypeName_"`).

**Functions -- emit ONLY monomorphized versions:**
- Z3 does NOT support parametric functions. You cannot write `(define-fun (par (T) ...))`.
- The generic template (with `Sort::Uninterpreted` type args) is dead code -- no call site in
  the IR points to it, because all call sites resolve to concrete monomorphized function IDs.
- During translation, functions whose type args contain `Sort::Uninterpreted` are filtered out.
- Only monomorphized versions are emitted as `(define-fun parse_value_String ...)`.

**`undef_sorts` / `declare-sort`:**
- Since generic template functions are skipped during translation, these sorts are never referenced
  in the emitted SMT-LIB. No `(declare-sort parse_value_T 0)` is emitted.
- The `undef_sorts` field was removed from `IRContext`. The `Sort::Uninterpreted` sorts still exist
  in the IR's `ty_inst` maps but are purely internal and can be consired legacy code. An alternative approach is that we don't register functions with generics in the IR and we just register the mono versions of them.

### Why can't we skip registering monomorphized types in the IR?

Two reasons:

1. **Sort-checking**: The IR compares `sid`s to verify expressions have matching types. Without a
   monomorphized entry, there is no `sid` to compare against. Two user types are only equal if they
   share the same `sid`.

2. **Z3 translation**: When translating `Sort::User(12)` to SMT-LIB, we call `reverse_lookup(12)`
   to get `("ParseResult", [String])` and emit `(ParseResult String)`. Without the entry in the
   registry, we cannot resolve what `sid=12` means.

### Why can't we use generic functions in Z3 instead of monomorphizing?

`(declare-sort parse_value_T 0)` creates a fixed opaque type in Z3, not a type variable. Z3 will
never infer that `parse_value_T = String`. Types are not values that Z3 solves for. If a function
returns `(ParseResult parse_value_T)` and the caller expects `(ParseResult String)`, Z3 rejects
this as a type error -- they are two different, incompatible sorts.

### Summary

|                  | IR Registration          | Z3 Translation              |
|------------------|--------------------------|-----------------------------|
| Types (generic)  | YES (structure template) | YES (par (T) declaration)   |
| Types (mono)     | YES (sort-checking + reverse lookup) | NO (lookup artifact only) |
| Functions (gen)  | YES (body entry point)   | NO (dead code, filtered out) |
| Functions (mono) | YES (call site targets)  | YES (define-fun each one)   |
| undef_sorts      | NOT in IRContext         | NOT emitted to SMT-LIB      |

### Registry Entry Kinds (refined taxonomy)

For every name in the type registry (and analogously for the function registry), the registry can hold up to four kinds of entries. They differ only in what their `ty_args` slice contains:

| Kind                  | `ty_args` shape                                         | How it gets created                                                                          | Emit?                       |
|-----------------------|---------------------------------------------------------|----------------------------------------------------------------------------------------------|-----------------------------|
| Template (canonical)  | all `Uninterpreted("{ThisName}_*")`                     | `build()` registers it for sort-checking the body                                            | Types: yes (with `par`). Functions: no. |
| All-uninterpreted but foreign | all `Uninterpreted("{OtherName}_*")`            | A generic parent walks its body, references this name with the parent's type vars            | No                          |
| Partial mono          | mix of concrete sorts and `Uninterpreted` (any prefix)  | Same as above, but the parent passed some concrete sorts mixed with its own type vars         | No                          |
| Fully concrete (mono) | no `Uninterpreted` anywhere                             | Materialized when a fully-concrete `ty_inst` walks the body                                   | Types: no (lookup-only). Functions: yes (`define-fun`). |

The canonical-template predicate distinguishes kind 1 from kind 2 by checking the prefix of each uninterpreted name; without that check, a foreign-uninterpreted entry could be picked as the representative for a name and mis-printed.

### SCC-based Emission Layout (text backend)

Type emission is driven by Tarjan-style SCC analysis on the type-reference graph. Each registered sid is a node; an edge `A -> B` means A's body references B.

- **`isolated`**: sids with no edges in the graph (nothing references them, they reference nothing, no self-loop).
- **`recursive_sccs`**: every other sid, partitioned into SCCs. An SCC is either a true mutual recursion (size > 1) or a singleton (a sid that has at least one edge but isn't in a cycle). Singletons-with-self-loop are also here (a self-referential generic via `Cloak<Foo<T>>` becomes a singleton-with-self-loop).

The emission pipeline runs three passes:

1. **Pick a canonical sid per name.** Walk both `recursive_sccs` and `isolated`. For each sid, if it satisfies the canonical-template predicate (`type_params.is_empty()` or all-uninterpreted-with-this-name's-prefix), record it in `name_to_best_sid`.
2. **Canonicalize and merge SCCs.** For each SCC in `recursive_sccs`, replace each member sid by its canonical sid (via `name_to_best_sid`). If the resulting set overlaps with an SCC already in `deduplicated_sccs`, merge them (because two original SCCs sharing a canonical sid must end up in the same `(declare-datatypes ...)` block — Z3 forbids declaring the same datatype twice).
3. **Add isolated singletons.** For each sid in `isolated`, look up its canonical sid. If that canonical sid isn't already inside any block produced by step 2, push a new singleton block for it.

Step 3 is what gets non-recursive, non-mutually-recursive types (e.g., a leaf `enum Color { Red, Green, Blue }`) into the SMT-LIB output.

### Validated Assumptions

These were checked against the source and hold:

- **Type taxonomy.** The split is exactly `isolated` (no edges) vs `recursive_sccs` (edges or self-loop). A self-loop alone (e.g., a generic with `Cloak<Self>`) is enough to push a node into `recursive_sccs` as a singleton.
- **`ty_args` slot count.** The slice has one slot per generic parameter declared in the source-level definition. Non-generic types have an empty slice; a type with N parameters has N slots. The slot length is fixed; only the slot contents vary.
- **Backend emits only canonical templates as datatypes (named types).** Generic ones are wrapped in `par (T1 ... Tn)`; non-generic ones are flat. All other entries (foreign-uninterpreted, partial, fully concrete) are silently dropped from declarations, but their sids are still used by `format_sort` to print references like `(Foo Int String)` against the parametric declaration. Unnamed tuples are handled differently — see the **Unnamed Tuples** section below.
- **Use-site references.** When `format_sort` encounters `Sort::User(sid)` for a *named* type, it does `reverse_lookup(sid)` to get `(Name, [arg1, arg2])` and emits `(Name <fmt(arg1)> <fmt(arg2)>)`. This works whether the args are concrete sorts or uninterpreted names. For *unnamed* tuples, only the distinct `Uninterpreted` elements are passed as instantiation args (concrete elements are baked into the tn); see the **Unnamed Tuples** section.
- **Backend emits only fully-concrete functions.** The filter `!ty_args.iter().any(Uninterpreted)` keeps only kind-4 function entries. Templates, foreign-uninterpreted, and partial entries are dropped; SMT-LIB has no parametric `define-fun`, so this is the only legal path.
- **Closure of the emitted call graph.** Each fully-concrete function fid had its body re-materialized with a fully-concrete `ty_inst`. Re-materialization rewalks every call site and resolves it to another fully-concrete fid. So inside an emitted body, every call edge points to another emitted fid. Templates and partials are referenced only from other templates' bodies, which are themselves not emitted.

### Unnamed Tuples

Unnamed tuples (registered via `TypeRef::Pack`, `ty_name = None`) don't fit the template/monomorph split above. Each distinct `args` list is its own sid with its own SMT-LIB declaration — there is no shared template wrapping them.

- **Always canonical.** `name_to_best_sid` uses `None => true`. Two unnamed tuples with the same `args` share a sid (dedup'd at registration); different `args` = different sid = different declaration. There is no template/monomorph collapse for unnamed tuples.
- **`par`-wrap when `args` contain `Uninterpreted`.** When an unnamed tuple appears inside a generic body, its `args` may include `Uninterpreted` entries (parent's type vars), possibly partially monomorphized with concrete sorts. `get_generic_param_count` returns the *distinct* `Uninterpreted` entries in first-appearance order; `mk_unnamed_tuple_str` `par`-wraps when this is non-empty. Concrete element sorts are emitted directly in the body and need no binder.
- **Use-sites instantiate only those distinct binders.** `format_sort` for an unnamed-tuple `Sort::User(sid)` reads the same `(_, params)` from `get_generic_param_count`. If empty: bare `tn` (e.g. `Tuple_Int_String`). If non-empty: `(tn <fmt(p1)> ...)`, e.g. `(Tuple_Foo_T_Int Foo_T)` for `(T, I32)` referenced inside `Foo<T>`'s body.

The visual coincidence in `(Tuple_Foo_T_Int Foo_T)` is just naming convention — `Tuple_Foo_T_Int` is one opaque symbol whose components happen to encode the args (per `resolve_type_name`), and the trailing `Foo_T` is the parent declaration's `par`-bound sort being passed as the instantiation argument. They are unrelated bindings that share a name; SMT-LIB resolves them by lexical scope.

### Soundness Invariants

The architecture maintains two invariants that together guarantee valid SMT-LIB output:

1. **Uninterpreted names only appear inside a `par` clause that binds them.** A reference like `(Inner Foo_T)` is well-formed when emitted inside `(declare-datatypes ((Foo 1)) ((par (Foo_T) ...)))` because `Foo_T` is bound by the `par`. The code prevents unbound uninterpreted names from leaking to top-level positions:
   - Top-level `(declare-datatypes ...)` blocks for generic types are wrapped in `par`. Unnamed tuples whose `args` contain `Uninterpreted` are also wrapped (see the **Unnamed Tuples** section).
   - Non-generic types have no params and no uninterpreted to begin with.
   - Function bodies are emitted only for fully-concrete functions, whose bodies were materialized with a fully-concrete `ty_inst`, so no `Uninterpreted` survives in their expressions.
   - Null sentinels (and any other top-level constants) explicitly filter out sids with any uninterpreted before declaring them.
2. **Filtering out non-fully-concrete function entries removes only dead code.** Because of the closure property above, no kept entry references a dropped entry. Dropping the non-mono entries therefore preserves call-site semantics — every original call has a fully-concrete instantiation that captures it — and does not change decidability, since SMT-LIB has no parametric `define-fun` to begin with and the kept entries are standard SMT-LIB.
### Generic Types and Functions: IR Registration vs Z3 Translation

--- 
Here we describe how generic types and functions flow through the system. Do not change this design without understanding why each piece exists.

### IR Registration (build time)

The IR builder registers both generic templates and monomorphized versions for types and functions.

**Types (generic template + monomorphized entries):**
- `build()` registers the generic template `ParseResult<T>` (with `T` as `Sort::Uninterpreted("ParseResult_T")`).
  This is the starting point so the IR builder knows the type's structure, variants, and fields. This is needed to translate the type in the backend.
- When a function body constructs `ParseResult::Ok(string_val, err)` with concrete types, the IR builder calls `register_type("ParseResult", [String])` creating a monomorphized entry (e.g. `sid=12`). This `sid` is needed for two reasons:
  - Sort-checking during IR building -- two user types are only equal if their `sid` is equal.
  - Z3 translation -- `reverse_lookup(sid=12)` returns `("ParseResult", [String])` so we can emit `(ParseResult String)` referencing the single parametric declaration.

**Functions (generic template + monomorphized versions):**
- `build()` registers the generic template `parse_value<T>` (with `T` as `Sort::Uninterpreted("parse_value_T")`). This is the entry point for processing the function's body and discovering what it calls.
- During body materialization (`ExpBuilder::materialize`), when a call like `helper::<String>(x)` is encountered, the builder calls `register_func("helper", [String])` which creates the monomorphized version. The builder's `ty_inst` maps `T -> String`, so recursive calls within the monomorphized body also resolve to monomorphized versions (not back to the generic template).
- Call sites in the IR point to monomorphized function IDs (e.g., `parse_value_String`), NOT the generic template. This is because the caller's `ty_inst` resolves `T` to a concrete sort.

### Z3 Translation (emit time)

**Types -- emit ONLY the generic template:**
- Use `(declare-datatypes ((ParseResult 1)) ((par (T) ...)))` -- one declaration per type name.
- Z3 supports parametric types, so `(ParseResult String)` and `(ParseResult Int)` are handled automatically by Z3. No separate declarations needed for monomorphized type entries.
- The monomorphized entries (e.g. `sid=12` for `ParseResult<String>`) are NOT declared separately. At translation time, `reverse_lookup(sid=12)` returns `("ParseResult", [String])` which is emitted
  as `(ParseResult String)` -- referencing the single parametric declaration.
- Deduplication picks the generic template as the representative for each type name (the one whose type params are `Uninterpreted` sorts starting with `"TypeName_"`).

**Functions -- emit ONLY monomorphized versions:**
- Z3 does NOT support parametric functions. You cannot write `(define-fun (par (T) ...))`.
- The generic template (with `Sort::Uninterpreted` type args) is dead code -- no call site in the IR points to it, because all call sites resolve to concrete monomorphized function IDs.
- During translation, functions whose type args contain `Sort::Uninterpreted` are filtered out.
- Only monomorphized versions are emitted as `(define-fun parse_value_String ...)`.

**`undef_sorts` / `declare-sort`:**
- Since generic template functions are skipped during translation, these sorts are never referenced in the emitted SMT-LIB. No `(declare-sort parse_value_T 0)` is emitted.
- The `undef_sorts` field was removed from `IRContext`. The `Sort::Uninterpreted` sorts still exist in the IR's `ty_inst` maps but are purely internal and can be consired legacy code. An alternative approach is that we don't register functions with generics in the IR and we just register the mono versions of them.

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
- **`recursive_sccs`**: an SCC is either a true mutual recursion or a singleton (a sid that has at least one edge but isn't in a cycle). Singletons-with-self-loop are also here (a self-referential generic, written by the user as `Cloak<Foo<T>>` so Rust accepts it but seen by the IR as a direct `Foo<T>` self-edge after `Cloak` is stripped, becomes a singleton-with-self-loop).

The emission pipeline runs three passes:

1. **Pick a canonical sid per name.** Walk both `recursive_sccs` and `isolated`. For each sid, if it satisfies the canonical-template predicate (`type_params.is_empty()` or all-uninterpreted-with-this-name's-prefix), record it in `name_to_best_sid`.
2. **Canonicalize and merge SCCs.** For each SCC in `recursive_sccs`, replace each member sid by its canonical sid (via `name_to_best_sid`). If the resulting set overlaps with an SCC already in `deduplicated_sccs`, merge them (because two original SCCs sharing a canonical sid must end up in the same `(declare-datatypes ...)` block — Z3 forbids declaring the same datatype twice).
3. **Add isolated singletons.** For each sid in `isolated`, look up its canonical sid. This is what gets non-recursive, non-mutually-recursive types (e.g., a leaf `enum Color { Red, Green, Blue }`) into the SMT-LIB output.

### Unnamed Tuples

Unnamed tuples — like `(Int, String)` — are registered with `ty_name = None` (`TypeRef::Pack`). They do **not** use the template/monomorph split that named generic types use. Instead:

- **Each distinct element list is its own type.** Same `args` → same sid (deduplicated at registration); different `args` → different sid → different declaration. There is no generic template to collapse into.
- **A `par` wrapper is added only when the elements contain type variables.** When a tuple appears inside a generic body (say `Foo<T>`), some elements may be the parent's type variables (`Uninterpreted`). `get_generic_param_count` lists the *distinct* variables in first-appearance order, and `mk_unnamed_tuple_str` wraps the declaration in `par` over them. Concrete elements are emitted as-is, with no binder.
- **Use sites apply only those variables.** `format_sort` reads the same distinct-variable list: none → bare name (`Tuple_Int_String`); some → applied form, e.g. `(Tuple_Foo_T_Int Foo_T)` for `(T, I32)` used inside `Foo<T>`.

> `(Tuple_Foo_T_Int Foo_T)` looks redundant but isn't. `Tuple_Foo_T_Int` is a single opaque name whose spelling merely encodes the element types (`resolve_type_name`); the trailing `Foo_T` is the *argument* being passed — the parent's `par`-bound sort. Two unrelated things that share a substring, which SMT-LIB tells apart by scope.

### Soundness Invariants

Two invariants — both validated against the source — keep the emitted SMT-LIB well-formed.

1. **A type variable appears only inside a `par` that binds it.** `(Inner Foo_T)` is legal only within `(declare-datatypes ((Foo 1)) ((par (Foo_T) ...)))`, where `par` binds `Foo_T`. Nothing lets an unbound variable reach the top level:
   - Generic type declarations are wrapped in `par`; so are unnamed tuples whose elements contain variables.
   - Non-generic types have no variables to begin with.
   - Only fully-concrete functions are emitted, and their bodies were built with a concrete `ty_inst`, so no variable survives.
   - Top-level constants (e.g. null sentinels) skip any sid that still contains a variable.
2. **Emitted bodies reference only other emitted entries (closure).** The filter `!ty_args.iter().any(Uninterpreted)` keeps only fully-concrete functions. Each was re-materialized with a concrete `ty_inst`, which rewalks every call site and resolves it to another fully-concrete fid — so every call edge inside an emitted body lands on another emitted fid. Templates and partials are referenced only from other (non-emitted) bodies, so dropping them removes only dead code: every real call already has a concrete target, and nothing observable changes. (SMT-LIB has no parametric `define-fun`.)

Two supporting facts, also validated against the source:

- **`ty_args` slot count.** One slot per generic parameter in the source definition — non-generic types have an empty slice, an N-parameter type has N slots.
- **Declaration vs reference.** Only canonical templates are declared as datatypes (generic → `par (T1 ... Tn)`, non-generic → flat). Every other entry is referenced but never declared: `format_sort` does `reverse_lookup(sid)` → `(Name, [args])` and emits `(Name <fmt(arg)> ...)` against that one parametric declaration.
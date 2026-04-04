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

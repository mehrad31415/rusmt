# Operations

See [Type system](typing.md) for the authoritative list of intrinsic SMT sorts.

## Intrinsic methods

This list is the stdlib API, i.e., the list of available methods per intrinsic type.

### `Boolean`

- `not()`
- `and(Boolean)`, `or(Boolean)`, `xor(Boolean)`
- `nand(Boolean)`, `nor(Boolean)`, `xnor(Boolean)`
- `implies(Boolean)`, `iff(Boolean)`
- `ite<T: SMT>(then: T, else: T) -> T`

### `Integer`

- **Arithmetic**: `neg()`, `add(Integer)`, `sub(Integer)`, `mul(Integer)`, `div(Integer)`, `div_trunc(Integer)`, `modulo(Integer)`, `rem(Integer)`, `pow(Integer)`, `abs()`
- **Predicates**: `divides(Integer)`, `lt/le/gt/ge(Integer)`
- **Conversions**: `to_real()`, `to_i32()`, `to_i64()`, `to_u32()`, `to_u64()`, `to_f32()`, `to_f64()`
- **Parsing**: `from_hex_str(String)`, `from_oct_str(String)`, `from_bin_str(String)`
- **Range checks**: `is_gt_i64_max()`, `is_lt_i64_min()`, `is_gt_u64_max()`, `is_lt_u64_min()`, `is_lt_i32_min()`, `is_gt_i32_max()`, `is_lt_u32_min()`, `is_gt_u32_max()`

### `Real`

- **Arithmetic**: `neg()`, `add(Real)`, `sub(Real)`, `mul(Real)`, `div(Real)`, `pow(Real)`, `abs()`
- **Rounding / tests**: `round()`, `floor()`, `ceil()`, `is_integer()`
- **Comparisons**: `lt/le/gt/ge(Real)`
- **Conversions**: `to_int()`, `to_f32()`, `to_f64()`

`Real::from` takes **integer literals only** — `Real::from(0.2)` is a compile
error. Write a non-integer value as an exact ratio:

```rust
Real::from(1).div(Real::from(5))       // 0.2
Real::from(15).div(Real::from(2))      // 7.5
Real::from(2433).div(Real::from(10))   // 243.3
```

`Real` models SMT-LIB's `Real` sort: exact rationals of unbounded precision. A
float literal cannot name one faithfully, because the transpiler reads the source
text `0.2` while the concrete evaluator only ever sees the `f64` that `rustc`
rounded it to — `3602879701896397/2^54`, not `1/5`. (Binary fractions can only
represent rationals whose reduced denominator is a power of two, so `0.25` is
exact but `0.2` is not.) Rather than have the two semantics disagree about the
same literal, the notation is rejected.

Ratios are also what SMT-LIB means: `Real::from(1).div(Real::from(5))` transpiles
to `(/ 1 5)`, and `(= (/ 1.0 5.0) 0.2)` is `true` in Z3 — so the concrete
evaluator, the transpiled query, and hand-written SMT-LIB all agree.

To start from a machine float instead, use `Integer::to_real()` or
`F32`/`F64`'s `to_real()`. `F32`/`F64` model `FloatingPoint`, where a literal
*should* denote the nearest double, so they keep their float literals.

### `String`

- **Core**: `new()`, `length()`, `concat(String)`, `at(Integer)`, `substr(Integer, Integer)`, `is_empty()`
- **Search**: `index_of(String, Integer)`, `index_of_default(String)`, `contains(String)`, `starts_with(String)`, `ends_with(String)`
- **Comparisons**: `lt/le/gt/ge(String)`
- **Conversions**: `to_int()`, `from_int(Integer)`, `from_code(U32)`, `to_code()`
- **Other**: `is_digit()`, `replace(String, String)`, `replace_all(String, String)`

### `Cloak<T>`

- `shield(T) -> Cloak<T>`
- `reveal(Cloak<T>) -> T`

### `Seq<T>`

- `new()`, `unit(T)`, `length()`
- `append(T)`, `concat(Seq<T>)`
- `at(Integer) -> T`, `at_seq(Integer) -> Seq<T>`
- `extract(Integer, Integer)`, `index_of(Seq<T>, Integer)`, `index_of_default(Seq<T>)`
- `contains(T)`, `prefix_of(Seq<T>)`, `suffix_of(Seq<T>)`
- `replace(T, T)`, `is_empty()`

### `Set<T>`

- `new()`, `length()`, `is_empty()`
- `insert(T)`, `remove(T)`, `contains(T)`
- `is_subset(Set<T>)`, `is_proper_subset(Set<T>)`, `is_disjoint(Set<T>)`
- `has_size(Integer)`

### `Array<K, V>`

- `new()`, `length()`, `is_empty()`
- `store(K, V)`, `select(K)`, `del(K)`
- `contains_key(K)`

### Bitvectors (`I32`, `I64`, `U32`, `U64`)

These are exposed via the `BitvectorOps` trait (receiver is one of `I32/I64/U32/U64`):

- `bv_not()`, `bv_redand()`, `bv_redor()`
- `bv_and/bv_or/bv_xor/bv_nand/bv_nor/bv_xnor(_)`
- `bv_neg()`, `bv_add/bv_sub/bv_mul/bv_div/bv_rem/bv_mod(_)`
- `bv_shl/bv_lshr/bv_ashr(_)`
- `bv_rotate_left/bv_rotate_right(_)`
- `bv_lt/bv_le/bv_gt/bv_ge(_)`
- `to_int() -> Integer`

### Floating point (`F32`, `F64`)

These are exposed via the `FloatOps` trait (receiver is `F32` or `F64`):

- **Arithmetic**: `add/sub/mul/div(_)`, `neg()`, `abs()`, `rem(_)`, `sqrt()`, `min/max(_)`
- **Tests**: `is_nan()`, `is_infinite()`, `is_zero()`, `is_normal()`, `is_subnormal()`, `is_negative()`, `is_positive()`
- **Comparisons**: `lt/le/gt/ge(_)`
- **Special values**: `nan()`, `infinity()`, `neg_infinity()`, `pos_zero()`, `neg_zero()`
- **Conversions**: `to_integer()`, `to_real()`, `to_u32()`, `to_i32()`, `to_u64()`, `to_i64()`
- **Rounding**: `ceil()`, `floor()`, `trunc()`, `nearest()`
- **Equality**: `fp_eq(_)` (the compiler also accepts `fq_eq(_)` as a legacy alias)

### `Path`

- `named(String) -> Path` — marker id is a stable hash of the name
- `merge(Path) -> Path`

## Expression intrinsics

**Bounded** — iterates in Rust over `c.iterator()` (collections like `Seq/Set/Array/String` provide these iterators for macro use):

- `forall!(v1 in c1, ..., vn in cn => predicate)`
- `exists!(v1 in c1, ..., vn in cn => predicate)`
- `choose!(v1 in c1, ..., vn in cn => predicate)`

## What counts as an “intrinsic”

An **intrinsic** is a type/method/operator/macro that the RuSmt compiler recognizes and gives **special SMT semantics** to (i.e., it does *not* treat it like a normal Rust function call).

- **Method intrinsics** (most of the stdlib API): e.g. `Integer::add`, `Seq::concat`, `Array::select`, …
- **Generic operator intrinsics** (on any `T: SMT`): `eq`, `ne`, `cmp`
- **Expression intrinsics** (macros): `forall!`, `exists!`, `choose!`
- **Literals**: booleans, integers, reals, strings, and numeric suffixes for bitvectors/floats

Non-intrinsic helpers exist too. The main ones are **collection iterators** like `Seq::iterator`, `Set::iterator`, `Array::iterator`, `String::iterator`: they are **Rust-only** and are used to implement the *bounded* quantifier patterns of `forall!/exists!/choose!`.

## Adding a new intrinsic-backed method (what files change)

Assuming **a new method on an existing intrinsic type** (e.g., add `Integer::foo` or `Seq::bar`):

- **Add/adjust the stdlib API (the method users call)**:
  - `smt/stdlib/src/dt/<type>.rs` (or a trait like `dt/float.rs`, `dt/bitvector.rs`)
- **Allow the method name as an intrinsic**:
  - `smt/derive/src/parser/name.rs` (`UsrFuncName::intrinsic`)
- **Register its type signature for overload resolution**:
  - `smt/derive/src/parser/apply.rs` (`ApplyDatabase::with_intrinsics()`)
- **Map (type, name) → parser intrinsic opcode**:
  - `smt/derive/src/parser/intrinsics.rs` (`Intrinsic::new(...)`)
  - Add a new `Intrinsic::...` variant here if needed.
- **Carry it through IR and backend**:
  - `smt/derive/src/ir/intrinsics.rs` (IR enum variant)
  - `smt/derive/src/ir/exp.rs` (lowering from parser intrinsic → IR intrinsic)
  - `smt/derive/src/backend/z3/intrinsics.rs` (SMT-LIB formatting)
- **Update docs**:
  - `book/src/user/methods.md` (intrinsic method list)
  - Possibly `book/src/user/typing.md` if it introduces new types/sorts

If instead you’re adding a **new intrinsic type** (a new SMT sort), you’ll additionally touch `UsrTypeName::intrinsic` / `SysTypeName` plumbing and sort formatting.

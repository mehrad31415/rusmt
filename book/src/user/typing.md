# Type system

RuSmt’s _type system_ is the set of **intrinsic SMT sorts** that the transpiler understands. These come from the `rusmt-smt-stdlib` crate.

## Intrinsic (built-in) SMT sorts

- **Booleans**
  - `Boolean`
- **Unbounded arithmetic**
  - `Integer`
  - `Real`
- **Bitvectors (SMT-LIB `(_ BitVec n)`)**
  - `I32`, `I64` (signed interpretation in the DSL API; still a bitvector sort in SMT)
  - `U32`, `U64` (unsigned interpretation in the DSL API; still a bitvector sort in SMT)
- **Floating point (SMT-LIB `(_ FloatingPoint eb sb)`)**
  - `F32` = `(_ FloatingPoint 8 24)`
  - `F64` = `(_ FloatingPoint 11 53)`
- **Strings**
  - `String` (SMT-LIB `String`)
- **Path-condition marker**
  - `Path` (a synthesis-target marker — `Path::fresh()` allocates a unique id, `Path::merge` is a set of path IDs)

## Parametric SMT sorts

- **Recursive-data-type helper**
  - `Cloak<T>`: a frontend-only wrapper that lets users write self-referential Rust ADTs; the IR strips it so the SMT-LIB output never mentions `Cloak`.
- **Collections**
  - `Seq<T>`: SMT sequences.
  - `Set<T>`: SMT sets.
  - `Array<K, V>`: SMT arrays/maps.

## The `SMT` trait

All intrinsic sorts implement the `SMT` trait, which provides:

- `eq(self, rhs) -> Boolean`
- `ne(self, rhs) -> Boolean`
- `cmp(self, rhs) -> Boolean`

These are also available for any user-defined type that implements `SMT`. Note that no type in the standard library and no user-defined type annotated with `[smt_type]` implements the `Eq, PartialEq, Ord, PartialOrd` traits. For comparison please use the methods provided by the `SMT` trait. By banning those traits and forcing everything through `.eq(), .cmp(), ne()`, which return _Boolean_, every comparison stays inside the DSL's type system and is visible to the transpiler. There is no way to accidentally write a comparison that produces a Rust _bool_ and escapes into the symbolic layer unnoticed.


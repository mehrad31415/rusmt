## Syntax subset (RuSmt DSL)

RuSmt programs are **Rust programs** written under additional restrictions so they can be translated into SMT. In short, we use `rusmt-smt-stdlib` types and intrinsic operations instead of Rust’s standard library.

### Supported structures

- `let` bindings (immutable): Every method in the standard library returns a new _object_. 
- `if`: Every `if` statement must have a matching `else`. 
- `struct` / `enum` / `match`: construction and pattern matching.
- direct and mutually recursive function calls.

### Intentionally avoided structures

These are typically rejected by the remarking/transpilation pipeline:

- mutation (`let mut`, assignment)
- loops (`for`, `while`)
- references/pointers and borrowing (`&T`, `&mut T`)
- arbitrary standard-library collections (use `Seq<T>`, `Set<T>`, `Array<K, V>` instead)

This list is not comprehensive. For a complete grasp of what can be used (or not used) please look at the standard library and the case studies. The standard library will show you the list of allowed types and operations. The case studies will show you the eligible patterns.
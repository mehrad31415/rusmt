## RuSmt Standard Library

The RuSmt standard library defines constructs with SMT-specific semantics. This crate defines a domain-specific language (DSL) built from Rust types that model the data types of the SMT-LIB standard. A single Rust program can be written using the DSL types and methods, that serves both as an executable interpreter and as a specification that is translated into SMT-LIB constraints for solvers like Z3.

### Core Concepts
The design of `rusmt-smt-stdlib` is guided by the fact that every type and method is designed with two roles in mind:
  > Concrete Execution: The methods are expressive enough to allow a full interpreter to be implemented, enabling the interpreter to run directly.
  > Symbolic Transpilation: The API is designed with a one-to-one mapping to SMT-LIB concepts, making it straightforward for a transpiler to convert the interpreter’s logic into a formal model.

### SMT Types
This library provides the following concrete types:

> Booleans
> Integers
> Reals
> BitVectors
> Floats
> Strings
> Sequences
> Sets
> Arrays

The library also provides utility types like _Path_ for tracking symbolic path-condition markers (represented as `(Array Int Bool)` in Z3; `Path::fresh()` allocates a unique id, `Path::merge` ORs two markers together) and _Cloak\<T\>_ as a frontend wrapper that lets users write self-referential ADTs (e.g. `Aexp::Add(Cloak<Aexp>, Cloak<Aexp>)`) without violating Rust's sized-type requirement. `Cloak<T>` is **only** a frontend convenience: it is stripped at the IR layer, so neither backend emits any `Cloak` machinery in its SMT-LIB output. The IR assumes that every `Cloak<T>` appears alongside at least one non-recursive variant (i.e. the enum is well-founded); a `Cloak<T>`-only enum would lower to a circular definition that Z3 rejects.

### Expressions
The library supports a variety of quantifier macros that can be used to build logical statements: _forall!, exists!, and choose!_.

- **Bounded** (collection elements): `forall!(x in xs => p)`. Concretely evaluated by iterating over `xs.iterator()`; symbolically lowered to a quantifier whose body is gated on membership.

### Soundness

Every stdlib function `f` and every concrete input `x` must satisfy:
`rust_impl(f, x) == z3_eval(z3_formula(f), x)`. The stdlib represents Z3's
theories, not Rust's native semantics — when the natural Rust implementation
diverges from the natural Z3 primitive, one side is repaired to match the
other.

### Usage Example
The following example demonstrates how to use the DSL to perform some basic operations. The code is fully executable in Rust, and each method call also has a clear mapping to an SMT-LIB function for the transpiler.

```rust
use rsmart_smt_stdlib::{Integer, SMT};

fn main() {
    let a = Integer::from(10);
    let b = Integer::from(3);

    // transpiler mapping: (+ 10 3)
    let c = a.add(b);
}
```

### License
The RuSmt project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.
## RuSmt Standard Library

The RuSmt standard library defines constructs with SMT-specific semantics. This crate defines a domain-specific language (DSL) built from Rust types that model the data types of the SMT-LIB standard. A single RuSmt program can be written using the DSL types and methods serving as an executable specification that is translated into SMT-LIB constraints for solvers like Z3.

### Core Concepts
The design of `rusmt-smt-stdlib` is guided by the fact that every type and method is designed with two roles in mind:
  > Concrete Execution: The types and methods are _expressive_ enough to allow a user to write an interpreter using the DSL.

  > Symbolic Transpilation: The API is designed with a one-to-one mapping to SMT-LIB concepts, making it straightforward for a transpiler to convert the code to SMT-LIB constraints.

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

The library also provides utility types like _Path_ for tracking symbolic path-condition markers (`Path::named(String::from("..."))` creates a marker whose id is a stable hash of the name, and `Path::merge` unions two markers into one) and _Cloak\<T\>_ as a frontend wrapper that lets users write self-referential ADTs (e.g. `Aexp::Add(Cloak<Aexp>, Cloak<Aexp>)`) without violating Rust's sized-type requirement. `Cloak<T>` is **only** a frontend convenience: it is stripped at the IR layer, so the backend doesn't emit any `Cloak` machinery in its SMT-LIB output. The IR assumes that every `Cloak<T>` appears alongside at least one non-recursive variant (i.e. the enum is well-founded); a `Cloak<T>`-only enum would lower to a circular definition that Z3 rejects.

### Expressions
The library supports a variety of quantifier macros that can be used to build logical statements: _forall!, exists!, and choose!_. All of them are in _Bounded_ context like `forall!(x in xs => p)`. The concrete rust implementation is an iterator over the bounded set, while the symbolic implementation is a quantifier over the same set.

### Soundness

Every stdlib function `f` and every concrete input `x` must satisfy:
`rust_impl(f, x) == z3_eval(z3_formula(f), x)`. The stdlib represents Z3's
theories, not Rust's native semantics — when the natural Rust implementation
diverges from the natural Z3 primitive, one side is repaired to match the
other.

### Usage Example
The following example demonstrates how to use the DSL to perform some basic operations. The code is fully executable in Rust, and each method call also has a clear mapping to an SMT-LIB function for the transpiler.

```rust
use rusmt_smt_stdlib::{Integer, SMT};

fn main() {
    let a = Integer::from(10);
    let b = Integer::from(3);

    // transpiler mapping: (+ 10 3)
    let _c = a.add(b);
}
```

### License
The RuSmt project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.
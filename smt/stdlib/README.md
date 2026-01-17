## Rusmart SMT Standard Library

The Rusmart standard library defines constructs with SMT-specific semantics that go beyond what Rust natively supports. It effectively forms a domain-specific language (DSL) built from Rust types that precisely model the data types of the SMT-LIB standard. This DSL enables writing a single Rust program that serves both as an executable interpreter and as a specification that can be easily translated into SMT-LIB constraints for solvers like Z3.

### Core Concepts
The design of `rusmart-smt-stdlib` is guided by the fact that every type and method is designed with two roles in mind:
  > Concrete Execution: The methods are expressive enough to allow a full interpreter to be implemented in Rust, enabling the interpreter to run directly.
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

The library also provides utility types like _Error_ for tracking symbolic failure states, _Cloak<T>_ for defining recursive data types.

### Expressions
The library supports a variety of expressions that can be used to build logical statements, including: _forall, exists, and choice_.

### Usage Example
The following example demonstrates how to use the DSL to perform some basic operations. The code is fully executable in Rust, and each method call also has a clear mapping to an SMT-LIB function for the transpiler.

```rust
use rsmart_smt_stdlib::{Integer, Boolean, I32, SMT};

fn main() {
    let a = Integer::from(10);
    let b = Integer::from(3);

    // transpiler mapping: (+ 10 3)
    let c = a.add(b);
    assert!(*c.eq(Integer::from(13)));
}
```

### License
The Rusmart project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.
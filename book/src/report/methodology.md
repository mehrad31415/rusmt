## Background and methodology

### Dual semantics

Rusmart programs are written so they can be interpreted in two ways:

- **Operational semantics**: run the Rust code on concrete inputs.
- **Denotational / symbolic semantics**: translate the same code into SMT constraints.

### Why SMT works here

The Rusmart DSL restricts Rust to keep translation tractable:

- no mutation, no heap pointers/references,
- SMT-backed primitive and collection types,
- recursion and pattern matching instead of loops.

The standard library (`rusmart-smt-stdlib`) is designed so most “interesting” operations are **intrinsics** with SMT meaning (documented in the user `stdlib` chapter).


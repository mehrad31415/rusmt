
## Remark

This crate provides utilities to enforce syntactic constraints on the types and functions used in Rusmart language implementations.

### Purpose

Transpiling the whole Rust language to SMT-LIB is infeasible due to its complexity. Instead, Rusmart defines a small set of domain-specific languages (DSLs) for writing interpreters. We impose further constraints on these DSLs to ensure that the code can be effectively transpiled to SMT-LIB.

### Constraints

- **Type Constraints**: We provide the `[smt_type]` attribute macro to mark types that are allowed in Rusmart DSLs. Only types annotated with this macro are processed by the transpiler. No attributes are allowed for the macro. The _Debug, Clone, Copy, Default, Hash_ are automatically derived for these types. The _SMT_ trait is also automatically implemented. All the generic parameters of the type must also implement the only the _SMT_ trait.

- **Function Constraints**: We provide the `[smt_fn]` attribute macro to mark functions that are allowed in Rusmart DSLs. Only functions annotated with this macro are processed by the transpiler. The _method_ attribute is allowed for the macro to introduce a method receiver. The function must not have any other attributes. The function's parameters and return type must be types annotated with the `[smt_type]` macro or primitive types supported by the transpiler. If the function has generic parameters, they must implement only the _SMT_ trait.

### License
The Rusmart project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.

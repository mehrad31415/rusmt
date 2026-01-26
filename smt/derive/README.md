## Derive

This crate analyzes the rusmart files and transpiles them to SMT-LIB format.

### Purpose

This directory contains the `parser` which imposes syntactic constraints on the _expressions_ and _statements_ used in Rusmart language implementations. It contains `IR` definitions and derives necessary traits for them. Finally, it has a `backend` module that translates the `IR` into SMT-LIB format. Some of the constraints are enforced using the `remark` crate.

### Constraints
The following constraints are enforced on the expressions and statements:

> We can only use the DSL types and methods.
> No mutable or global variables are allowed.
> No pointers are allowed.
> No impl or mod block definitions.
> All types are copyable.
> All functions have input and output.
> Only the following statements are allowed: Match, If-Else, Let, Return.
> No loops or other statements are allowed.

This list is not exhaustive, and more constraints may be added in the future.

### License
The Rusmart project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.
## Derive

This crate analyzes Rusmart files, builds an intermediate representation (IR), and solves synthesis queries using Z3. It provides two Z3 backends for solving.

### Purpose

This directory contains the `parser` which imposes syntactic constraints on the _expressions_ and _statements_ used in Rusmart language implementations. It contains `IR` definitions and it has a `backend` module that translates the `IR` into SMT-LIB format and solves synthesis queries. Some of the constraints are enforced using the `remark` crate.

### Architecture

```
src/
├── parser/         # DSL parsing, intrinsic recognition, overload resolution
├── ir/             # Expression lowering, SMT sort checking, IR context
└── backend/
    ├── codegen.rs  # CodeGen trait shared by backends
    ├── response.rs # Response enum (Sat, Unsat, Unknown, Timeout)
    ├── z3/         # Text backend: generates SMT-LIB2 files, spawns Z3 subprocess
    │   ├── ctxt.rs       # CodeGenZ3 implementation, invoke_backend, error queries
    │   ├── exp.rs        # IR expression -> SMT-LIB2 text
    │   ├── fun.rs        # Function declarations/definitions
    │   ├── intrinsics.rs # Intrinsic operations -> SMT-LIB2 formulas
    │   └── sort.rs       # IR sorts -> SMT-LIB2 datatype declarations
    └── z3_api/     # API backend: in-process Z3 via z3-sys bindings
        ├── mod.rs        # Core types (Z3Ast RAII wrapper, Z3Context)
        ├── context.rs    # Z3 context building (datatypes, functions, helpers)
        ├── solver.rs     # Per-target solving pipeline with timeout
        ├── translate.rs  # IR expression -> Z3 AST objects
        └── intrinsics.rs # Intrinsic operations -> Z3 C API calls
```

### Two Z3 Backends

**Text backend** (`z3/`): Generates SMT-LIB2 text files, writes them to disk, and spawns Z3 as a subprocess. Results are written to `z3_chc/target_N/response.txt`. We will store the `time` required to solve the model.

**API backend** (`z3_api/`): Uses Z3 in-process via the `z3-sys` crate and `Z3_eval_smtlib2_string`. Builds Z3 objects directly in memory for type and function definitions. Results are written to `z3_api/target_N/response.txt` with timing in `timing.txt`.

### Key Entry Points

- **`model(path)`**: Parse and lower a Rusmart program into the internal IR (no solver required).
- **`solve(models, top_level_fn, output)`**: Run the text backend -- generate SMT-LIB2, spawn subprocesses, collect responses.
- **`solve_z3_api(models, top_level_fn, output)`**: Run the API backend -- solve in-process using z3-sys bindings.

### Usage

```bash
# Text backend (default)
cargo run -p rusmart-smt-derive -- toml parse_toml

# API backend
cargo run -p rusmart-smt-derive -- toml parse_toml api

# Both backends for comparison
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

### Error Representation

Errors are represented as `(Array Int Bool)` in the Z3 backend rather than `(Set Int)`. Each `ErrFresh(id)` becomes `(store ((as const (Array Int Bool)) false) id true)`, and `ErrMerge` uses `((_ map or) lhs rhs)`. Membership checking uses `(select expr error_id)`.

### Build Dependencies

Z3 is included as a vendored dependency (`z3 = { version = "0.20.0", features = ["vendored"] }`). This requires CMake and a C++ compiler. The first build compiles Z3 from source (~5 minutes); subsequent builds use the cached result.

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
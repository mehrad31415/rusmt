## Rusmart Language Implementations

This crate provides implementations of interpreters for various programming languages.

### Purpose
The primary goal of this crate is to define the semantics of languages using only the data types provided by the `rusmart-smt-stdlib` crate. Each language implementation in this crate is designed to be:

> _Concretely Executable_: The interpreters can be run directly in Rust to execute concrete input programs.

> _Symbolically Transpilable_: Because the logic is written entirely with the Rusmart DSL, the code serves as a formal specification that can be analyzed and transpiled into SMT-LIB formulas.

### Architectural Role

graph TD
    subgraph "Oracle"
        A[Standard_Type Library] --> B[Interpreter Implementation];
        B --> C[SMT-LIB Transpiler];
        C --> D[SMT Formula];
    end

    subgraph "Program Synthesis"
        D --> E[Z3 SMT Solver];
        E --> F{SAT?};
        F -->|Yes| G[Extract Model];
        F -->|No| H[Should Not Happen];
        G --> I[Synthesize test programs];
    end

    subgraph "Conformance Testing"
        I --> J[Our Reference Implementation];
        I --> K[Commercial Implementations];
        J --> L{Compare};
        L -->|Divergence| M[Bug Report];
        L -->|Agreement| N[Test Passes];
        N --> P[Generate Next Test];
    end

### Structure
This crate is organized by language, with each language implemented in its own module.

> src/toml/: A parser for the TOML v1.1.0 specification.

> src/wasm/: (Future Work) An interpreter for a subset of the WebAssembly virtual machine.

> src/while/: (Future Work) An interpreter for a pedagogical WHILE language, used for demonstrating formal verification concepts.

> src/rego/: (Future Work) An interpreter for a subset of the Rego policy language, used in Open Policy Agent (OPA).

> src/ebnf/: (Future Work) An interpreter for defining and parsing grammars in EBNF notation.

### Synthesis

The `src/synthesis/` directory contains the outputs from running the derive crate's Z3 backends on the language interpreters. For the TOML parser, synthesis results are written to `src/synthesis/toml/` and organized by backend:

- `z3_chc/` -- results from the text backend (SMT-LIB2 + Z3 subprocess)
- `z3_api/` -- results from the API backend (in-process Z3 via z3-sys)

Each backend directory contains per-target subdirectories (`target_N/`) with:
- `main.smt2` -- the SMT-LIB2 query file (text backend only)
- `response.txt` -- the solver's response (sat/unsat/unknown/timeout)
- `timing.txt` -- elapsed time in milliseconds.

### Usage
This crate is primarily a library. Its main executable (`src/main.rs`) can be used to feed concrete programs into the interpreters for execution.

**Concrete execution:**
```bash
cargo run -p rusmart-lang -- toml lang/toml/input/example.toml
```

**Synthesis (via the derive crate):**
```bash
cargo run -p rusmart-smt-derive -- toml parse_toml [text|api|both]
```

### License
The Rusmart project, a symbolic execution engine, is licensed under the _GPL-3.0-or-later_ license.
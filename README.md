<h1 style="text-align: center;">Rusmart</h1>
<p style="text-align: center;">Implemented in Rust | Transpiled to SMT</p>

## Introduction

**Rusmart** is a domain-specific language (DSL) embedded in Rust that enables writing language interpreters with **dual semantics**:

- The _operational semantics_ is represented by executable Rust code that can run concrete programs
- The _denotational semantics_ is automatically transpiled into SMT-LIB formulas for symbolic reasoning

This dual nature is achieved through a custom standard library (`rusmart-smt-stdlib`) and a transpiler (`rusmart-smt-derive`) that converts annotated Rust code into SMT constraints.

## The Problem

Testing language implementations (parsers, interpreters, compilers) is challenging:
- Hand-written test suites have limited coverage
- Fuzzing produces random inputs but misses edge cases
- Formal verification requires manually writing specifications in theorem provers

**Rusmart solves this** by letting you write a reference interpreter _once_ in Rust, then:
1. **Execute it** on concrete inputs like a normal program
2. **Transpile it** to SMT formulas for symbolic analysis
3. **Synthesize test programs** using an SMT solver (Z3)
4. **Find bugs** in real-world implementations through automated conformance testing

## How It Works

### 1. Write an Interpreter in the Rusmart DSL

```rust
use rusmart_smt_stdlib::{Integer, String, Seq, Boolean};

fn parse_toml(input: Seq<String>) -> Result<TomlValue, Error> {
    // Implementation uses only DSL types and methods
    // This code is BOTH executable AND transpilable to SMT
}
```

### 2. Transpile to SMT-LIB

```bash
# Text backend (generates SMT-LIB2 files, spawns Z3 as subprocess)
cargo run -p rusmart-smt-derive -- toml parse_toml

# API backend (uses Z3 in-process via z3-sys bindings)
cargo run -p rusmart-smt-derive -- toml parse_toml api

# Run both backends for comparison
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

This generates SMT-LIB formulas and solver results to `lang/src/synthesis/toml/`.

### 3. Synthesize Test Programs

Ask Z3 to find inputs that reach a specific path — an error, a target output, or an edge case.

```smt2
(assert (exists ((p Program)) 
  (= (parse_toml p) (Error ParseError))))
```

### 4. Conformance Testing

Feed synthesized programs to the implementation under test:

```
[SMT Solver] → Synthesized Program → [Your TOML Reference Parser]
                                   ↓
                              Compare with
                                   ↓
                            [Other Parsers]
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Rusmart DSL Interpreter (e.g., TOML parser)           │
│  Written using rusmart-smt-stdlib types                 │
└─────────────────┬───────────────────────────────────────┘
                  │
         ┌────────┴────────┐
         │                 │
         ▼                 ▼
   Concrete Exec      SMT Transpiler
   (cargo run)      (rusmart-smt-derive)
         │                 │
         │          ┌──────┴──────┐
         │          │             │
         │      Text Backend  API Backend
         │     (SMT-LIB2 +   (in-process
         │      Z3 subprocess) z3-sys)
         │          │             │
         │          └──────┬──────┘
         │                 │
         │                 ▼
         │        Synthesized Programs
         │                 │
         └────────┬────────┘
                  │
                  ▼
 Feed to Other Parsers & Conformance Testing
      (Compare outputs & find bugs)
```

## Project Structure

```
rusmart/
├── smt/
│   ├── stdlib/       # SMT-backed Rust types (Integer, String, Seq, etc.)
│   ├── remark/       # Annotation system marking functions and types
│   └── derive/       # Parser + IR + SMT-LIB code generator + Z3 backends
│       └── src/
│           ├── parser/    # DSL parsing, intrinsic recognition
│           ├── ir/        # Intermediate representation, sort checking
│           └── backend/
│               ├── z3/      # Text backend: generates SMT-LIB2, spawns Z3
│               └── z3_api/  # API backend: in-process Z3 via z3-sys
├── lang/             # Language interpreters (TOML, WASM, Rego, etc.)
│   └── src/
│       ├── toml/          # TOML v1.1.0 parser
│       └── synthesis/     # Synthesis output directory
```

## Current Implementation

### TOML v1.1.0 Parser

The first language implementation is a complete TOML parser demonstrating the full Rusmart workflow:

**Execute concrete programs:**
```bash
cargo run -p rusmart-lang toml lang/toml/input/example.toml
```

**Transpile to SMT and synthesize:**
```bash
cargo run -p rusmart-smt-derive -- toml parse_toml
# Outputs SMT-LIB formulas and solver results to lang/src/synthesis/toml/
```

**Future:** Planned Languages:
- WebAssembly interpreter
- WHILE language (pedagogical)
- Rego policy language
- EBNF grammar processor

## Key Features

### The Rusmart Standard Library

Provides SMT-compatible types that work in both concrete and symbolic contexts:

- **Primitive types**: `Boolean`, `Integer`, `Real`, `String`
- **Bitvectors & Floats**: `I32`, `I64`, `U32`, `U64`, `F32`, `F64`
- **Collections**: `Seq<T>`, `Set<T>`, `Array<K,V>`
- **Quantifiers**: `forall`, `exists`, `choice`
- **Error**: a path marker used by the SMT solver to synthesize inputs that reach a specific code path (e.g., an error case or edge case)
- **Recursive types**: `Cloak<T>` for defining recursive data structures

### Constraints

To ensure transpilability, the DSL enforces:
- No mutable or global variables
- No pointers or references
- All types are `Copy`
- Limited statement types: `match`, `if-else`, `let`, `return`
- No loops (use quantifiers or recursion instead)

## Build & Run

### Prerequisites
- Rust (edition 2024)
- CMake and a C++ compiler (included with Xcode on macOS, `build-essential` + `cmake` on Ubuntu)
- The Z3 dependency is vendored (compiled from source automatically on first build, ~5 minutes). No system Z3 installation is required.

### Build
```bash
cargo build --workspace
```

### Run TOML Parser
```bash
cargo run -p rusmart-lang toml lang/toml/input/example.toml
```

### Generate SMT Formulas and Synthesize
```bash
# Text backend (default)
cargo run -p rusmart-smt-derive -- toml parse_toml

# API backend (in-process Z3)
cargo run -p rusmart-smt-derive -- toml parse_toml api

# Both backends (for comparison)
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

### Documentation
```bash
make docs  # Requires mdBook
```

## Why Rusmart?

Traditional approaches to language testing require:
- Writing test cases manually (limited coverage)
- Writing specifications in theorem provers like Coq (steep learning curve)
- Maintaining separate reference implementations (duplicated effort)

**Rusmart gives you:**
- ✅ One codebase for both execution and verification
- ✅ Automatic test generation via SMT solving
- ✅ Familiar Rust syntax with type safety
- ✅ Push-button conformance testing

## Development Status

Rusmart is under active development. The core infrastructure is in place:
- ✅ SMT standard library with soundness-audited intrinsics
- ✅ Parser and IR
- ✅ Two Z3 backends: text-based (SMT-LIB2 + subprocess) and API-based (in-process z3-sys)
- ✅ TOML parser implementation
- ✅ Z3 query interface for program synthesis
- ✅ Vendored Z3 build (no system installation required)
- ✅ Automated conformance testing framework

Expect API changes and refactorings as the design evolves.

## Contributing

Rusmart is a research project. Documentation in `documents/` and the Rusmart Book (via `make docs`) explain design decisions and implementation details.

## License

GPL-3.0-or-later

# Rusmart Test Programs

This directory contains systematic tests for Rust-to-SMT-LIB2 translation using `datatest_stable`.

## Directory Structure

```
programs/
├── src/
│   └── lib.rs                      # Empty (test harness is in tests/)
├── tests/
│   ├── integration.rs              # Test harness with datatest_stable
│   └── translation/                # Translation tests
│       ├── mod.rs                  # Module declarations
│       ├── tests...
└── Cargo.toml
```
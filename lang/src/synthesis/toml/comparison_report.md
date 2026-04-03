# Text vs API Backend Comparison Report

## Overview

This report compares the results of running the two Z3 backends on the TOML parser synthesis targets.

### Text Backend (`z3_chc/`)

The text backend generates SMT-LIB2 files and spawns Z3 as an external subprocess for each error target. It writes the full SMT-LIB2 encoding (type declarations, function definitions, error assertions) to disk, then invokes `z3 -smt2 -t:<timeout_ms>` on each file. The backend monitors the Z3 process and classifies the result based on stdout output (sat/unsat/unknown) and elapsed time.

Key characteristics:
- Each target gets its own `.smt2` file containing the complete encoding plus error-specific assertions
- Z3 runs as an independent OS process (spawned via `command-group` for proper cleanup)
- Timeout is enforced both by Z3's `-t:` flag and a Rust-side monitor thread
- Results in `z3_chc/target_N/response.txt`

### API Backend (`z3_api/`)

The API backend uses Z3 in-process via the `z3-sys` Rust bindings. It loads type and function definitions using `Z3_eval_smtlib2_string`, then constructs error-specific queries and solves them directly through the Z3 C API. Each target gets a fresh Z3 context to ensure isolation.

Key characteristics:
- No intermediate files written for the solver (definitions loaded directly into Z3's memory)
- Each target solved in a fresh Z3 context with the same solver parameters
- Timing information recorded per-target in `timing.txt`
- Results in `z3_api/target_N/response.txt`

### Differences

Both backends use identical Z3 solver parameters (random seeds, restart limits, quantifier settings, parallel.enable=false). The key differences are:

1. **Process model**: text spawns a subprocess per target; API runs in the same process
2. **I/O overhead**: text writes/reads files; API passes data in memory
3. **Timing**: text includes process spawn overhead; API measures pure solve time
4. **Error handling**: text can recover from Z3 crashes (segfaults); API would crash the host process

## Results

### Per-Target Comparison

| target_id | text_result | api_result | text_time | api_time | match? |
|-----------|-------------|------------|-----------|----------|--------|
| TBD | TBD | TBD | TBD | TBD | TBD |

*TBD -- run both backends to populate this table:*
```bash
cargo run -p rusmart-smt-derive -- toml parse_toml both
```

### Summary

| Metric | Text Backend | API Backend |
|--------|-------------|-------------|
| Total targets | TBD | TBD |
| SAT | TBD | TBD |
| UNSAT | TBD | TBD |
| Unknown | TBD | TBD |
| Timeout | TBD | TBD |
| Errors | TBD | TBD |
| Total time | TBD | TBD |
| Matching results | TBD / TBD | |

### Notes

- A "match" means both backends returned the same verdict (sat/unsat/unknown/timeout) for the same target.
- Timing differences are expected due to the different process models.
- Timeouts may differ if one backend's overhead pushes a borderline case over the limit.

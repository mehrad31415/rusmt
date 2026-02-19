## Evaluation

In this repository, evaluation is primarily **engineering validation**:

- **IR coverage**: stdlib intrinsics are exercised by IR-only tests under `programs/tests/stdlib/`.
- **End-to-end translation**: representative programs are translated and run through Z3 under `programs/tests/translation/`.

This test structure is intentionally data-driven so it can grow alongside the intrinsic surface area and language case studies.


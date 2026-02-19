## Transpilation pipeline

The implementation is in `smt/derive`.

Conceptually:

- **Parser layer** (`smt/derive/src/parser/*`): reads a restricted Rust AST (via `syn`) and recognizes stdlib intrinsics and DSL forms.
- **IR layer** (`smt/derive/src/ir/*`): assigns SMT sorts to expressions, validates constraints, and produces a backend-friendly representation.
- **Backend layer** (`smt/derive/src/backend/*`): emits SMT-LIB and runs/collects solver responses.

The developer `Pipeline` chapter links these components to concrete directories and key entry points.


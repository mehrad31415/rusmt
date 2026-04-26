## Rego (subset) case study

The `rusmart-lang` crate contains a parser **and** a small evaluator for a
subset of [Rego](https://www.openpolicyagent.org/docs/policy-language/), the
policy language used by [Open Policy Agent (OPA)](https://www.openpolicyagent.org/).
Both pieces are implemented entirely in the Rusmart DSL (restricted Rust +
`rusmart-smt-stdlib`) under `lang/src/rego/`.

Like the TOML case study, the Rego module is a runnable specification: every
function carries a citation back to the OPA spec, and every `Error::fresh()`
call marks a synthesis target (an "edge case the spec calls out") that can be
fed to Z3 to generate concrete inputs.

### Why Rego is interesting for conformance testing

Rego sits in a different design space than TOML:

- **Policy-as-data.** A Rego module describes *rules* whose bodies are
  expressions over input data, not just static literals. Conformance is not
  only "does it parse the same?" but "does it produce the same value?"
- **Multiple reference implementations.** OPA itself is the canonical
  implementation, but [Regorus](https://github.com/microsoft/regorus) is an
  independent Rust implementation that's actively used in production. Two
  references mean conformance tests can find divergences in *either* engine
  by triangulating against the runnable spec.
- **Subtle truthiness / type coercion rules.** The spec is precise about
  cross-type comparisons, missing-field semantics, and how `default` interacts
  with multiple competing rules — exactly the territory where parser-level
  conformance is not enough and an *evaluator* is required.

### Subset boundary

The subset implemented here covers the constructs that have a clean,
decidable SMT mapping. In scope:

- Scalars (`null`, booleans, JSON numbers as `Real`, single-line strings)
- Composite literals (arrays `[...]`, objects `{k:v}`, sets `set(...)`)
- Single-package modules with a top `package <path>` clause
- Rules in all four documented shapes: `default`, complete, partial set, partial object
- `not`, comparison operators, `+`/`-`/`*`/`/` arithmetic with proper precedence
- `:=` single-assignment binding
- Dotted references (`input.user.role`) descending through bound objects

Out of scope (and rejected at parse time, **not** marked with `Error::fresh()`):

- Iteration with `[_]`
- Comprehensions (`[x | body]`, `{x | body}`, `{k: v | body}`)
- The `with` modifier
- `every` / `some` quantifiers
- Built-in function calls (`regex.match`, `count`, …)
- `import` statements / multiple modules

The complete subset list, including grammar and rationale, lives in
`lang/src/rego/README.md`.

### Differential targets

The runnable spec is meant to be diff-tested against:

- **OPA** (`opa eval` / `opa parse`) — the canonical implementation.
- **Regorus** — an independent Rust implementation; useful as a tiebreaker
  when OPA and the runnable spec disagree.

A divergence between *all three* points to a spec ambiguity that is worth
reporting upstream; a divergence between only two of them indicates a bug.

### Running synthesis

The Rusmart pipeline that targets the Rego module is the same as for TOML —
the only difference is the function being synthesized:

```bash
# Text backend: generates SMT-LIB2 files, spawns Z3 as subprocess
cargo run -p rusmart-smt-derive -- rego parse_policy

# Or evaluator-level synthesis
cargo run -p rusmart-smt-derive -- rego evaluate_policy
```

The next two chapters explain the parser architecture and the AST / value
model in more detail.

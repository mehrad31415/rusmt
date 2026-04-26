## Rego language (subset)

This module provides a declarative parser and evaluator for a **subset of
[Rego](https://www.openpolicyagent.org/docs/policy-language/)**, the policy
language used by the [Open Policy Agent](https://www.openpolicyagent.org/)
project. It is implemented entirely in the Rusmart DSL (restricted Rust +
`rusmart-smt-stdlib`) so the same source can be:

- executed concretely as a normal Rust program, and
- transpiled to SMT-LIB by `rusmart-smt-derive` for synthesis-driven
  conformance testing against OPA.

Authoritative spec sources:
- <https://www.openpolicyagent.org/docs/policy-language/>
- <https://www.openpolicyagent.org/docs/policy-reference/>

The OPA implementation at <https://github.com/open-policy-agent/opa> is read
only to disambiguate spec wording; no implementation detail is copied. The
goal is to encode the **spec**, not the OPA codebase.

### Language syntax (subset)

The grammar of the subset, in a near-ABNF style. Keywords are case-sensitive.

```
module          = ws-nl package-clause ws-nl rule *(ws-nl rule) ws-nl

package-clause  = "package" wschar+ path
path            = ident *("." ident)
ident           = (alpha / "_") *(alpha / digit / "_")

rule            = default-rule
                / partial-rule
                / complete-rule

default-rule    = "default" wschar+ ident ws "=" ws term

partial-rule    = ident "[" ws term ws "]" ws
                  ( ws "{" body "}"                   ; partial set
                  / ws "=" ws term ws "{" body "}" ); partial object

complete-rule   = ident ws "=" ws term ws ["{" body "}"]
                / ident ws "{" body "}"               ; shorthand: name = true { body }

body            = ws-nl expr *( (";" / newline) ws-nl expr ) ws-nl

expr            = "not" wschar+ expr
                / ident ws ":=" ws term               ; single-assignment binding
                / term (ws cmp-op ws term)?           ; comparison or truth test
cmp-op          = "==" / "!=" / "<=" / ">=" / "<" / ">"

term            = additive
additive        = multiplicative *(ws ("+" / "-") ws multiplicative)
multiplicative  = atom *(ws ("*" / "/") ws atom)
atom            = scalar
                / array-lit
                / object-lit
                / set-lit
                / "(" ws term ws ")"
                / ref-or-var

scalar          = null / boolean / number / string
null            = "null"
boolean         = "true" / "false"
number          = ["-"] digit+ ("." digit+)?
string          = '"' *(char / escape) '"'
escape          = "\" ("\"" / "\\" / "n" / "t" / "r")

array-lit       = "[" ws [term *(ws "," ws term) [","]] ws "]"
object-lit      = "{" ws [kvp *(ws "," ws kvp) [","]] ws "}"
kvp             = key ws ":" ws term
key             = string / ident
set-lit         = "set(" ws [term *(ws "," ws term) [","]] ws ")"

ref-or-var      = ident ("." ident)*
                ; length 1 → Var, length ≥ 2 → Ref

newline         = LF / CRLF
wschar          = SP / HT
ws              = *wschar
ws-nl           = *(wschar / newline / "#" *non-eol newline)
```

### Subset boundary

#### In scope

| Feature                                  | Notes                                                      |
| ---------------------------------------- | ---------------------------------------------------------- |
| Scalars: `null`, `true`, `false`, number | Numbers are JSON numbers, represented as `Real`            |
| Strings (single-line)                    | Backslash escapes `\"`, `\\`, `\n`, `\t`, `\r`             |
| Composite literals                       | Arrays `[...]`, objects `{k:v}`, sets `set(...)`           |
| References                               | `input.foo.bar` (single-segment → variable)                |
| Comparisons                              | `==`, `!=`, `<`, `<=`, `>`, `>=`                           |
| Boolean                                  | `not <expr>`; `and` is implicit (newline / `;` between)    |
| Arithmetic                               | `+`, `-`, `*`, `/` with proper precedence and div-by-0 guard |
| Rules                                    | Complete, partial set, partial object                       |
| `default` keyword                        | Default rule with constant value                           |
| Single-package modules                   | One `package <path>` clause at the top                     |
| Single-assignment binding                | `x := <term>` inside a body                                 |

#### Out of scope (rejected at parse time)

The subset deliberately excludes the features below. Out-of-scope syntax is
returned as `ParseResult::NoMatch` (or, where the prefix is committed, a
hard parse error) — these are **not** marked with `Error::fresh()` because
they are not synthesis targets.

| Feature                                            | How it is rejected                                  |
| -------------------------------------------------- | --------------------------------------------------- |
| Iteration `arr[_]`                                 | `[` is not a valid ref continuation (only `.` is)   |
| Comprehensions `[x \| body]`, `{x \| body}`        | `\|` inside `[..]` / `{..}` is a hard parse error   |
| `with` modifier                                    | `with` is a reserved keyword, never matched as ident |
| `every` / `some`                                   | Reserved keywords, never matched as ident           |
| Built-in function calls (e.g. `regex.match("a", x)`) | The `(` after a ref is not a valid term continuation |
| Multiple modules / `import`                        | `import` keyword is rejected as a hard error after the package clause |
| Cross-package data references                      | `data.foo.bar` parses as a ref but evaluation only resolves keys in `bindings` |

### Reference implementations the user will diff against

For conformance testing, the runnable spec must agree with at least one
authoritative implementation. Suggested differential targets:

- [OPA](https://github.com/open-policy-agent/opa) — the canonical
  implementation (`opa eval` / `opa parse`)
- [Regorus](https://github.com/microsoft/regorus) — independent Rust
  implementation, useful as a second reference

### Example

```rego
package authz

default allow = false

allow = true {
    input.user.role == "admin"
}

allow = true {
    input.user.role == "viewer"
    input.action == "read"
}

permitted_paths[path] {
    input.user.allowed[path] == true
}
```

### Evaluation limits

The subset keeps the evaluator small enough to remain decidable in SMT.
Specifically:
- the evaluator does not iterate arbitrary input fields (no `[_]`); it only
  resolves dotted paths to known keys;
- comparisons across types (e.g. `1 == "1"`) return `false` rather than a
  hard error, matching the spec's strict-typing behaviour;
- division by zero, type-mismatched arithmetic, unbound variables, and
  rebinding via `:=` produce `Error::fresh()` synthesis targets.

The full enumeration of synthesis targets is in `ERRORS.md`.

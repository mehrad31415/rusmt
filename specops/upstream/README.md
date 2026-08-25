# Upstream divergences — status

Each was found by RuSmt and each is three-against-one: three independent parsers
reject the input, one accepts it. Every one is the witness for a named marker in
`lang/src/toml/`, certified by Z3 against that marker's query before it entered
the suite.

Reproductions below were confirmed on 2026-08-25 against the latest release of
each implementation: Rust `toml` 1.1.4+spec-1.1.0, CPython `tomllib` 3.14.7,
BurntSushi `toml` v1.6.0 and `smol-toml` 1.8.0.

| # | Divergence | Status |
|---|---|---|
| 1 | `smol-toml` accepts `[[a]` | **Filed** — `squirrelchat/smol-toml` issue #65, plus `toml-lang/toml-test` PR #205 adding the missing cases |
| 2 | BurntSushi extends a closed inline table | **Not filed** — already acknowledged upstream |
| 3 | BurntSushi `[table]` redefines a dotted-key table | **Not filed** — already acknowledged upstream |

Issues 2 and 3 are skipped by the project's own test suite: `toml_test.go` lists
`invalid/table/redefine-02`, `redefine-03` and
`invalid/inline-table/overwrite-02` under the comment `TODO: fix this; we allow
appending to tables, but shouldn't`. Our suite rediscovered them from the
specification rather than from the test corpus, which is a check on the method,
but they are not new and filing them would be noise.

**Do not file the CPython `tomllib` integer cases.** The standard says
implementations are ``free to support any integer size'', so CPython's
arbitrary-precision integers violate nothing.

---

## 1. smol-toml — array-of-tables header with a single closing bracket (FILED #65)

**Title:** `[[a]` (unterminated array-of-tables header) is accepted as valid

smol-toml 1.8.0 accepts this document:

```toml
[[a]
```

An array-of-tables header is a name in *double* brackets, so the missing second
`]` should be a parse error.

| Implementation | Result |
|---|---|
| Rust `toml` 1.1.4+spec-1.1.0 | `unclosed array table, expected ']'` |
| CPython `tomllib` 3.14.7 | `Expected ']]' at the end of an array declaration` |
| BurntSushi `toml` v1.6.0 | `expected end of table array name delimiter` |
| **smol-toml 1.8.0** | **accepted** |

Witness for the `array_table_missing_close_char` marker.

---

## 2. BurntSushi/toml — dotted key extends a closed inline table (NOT FILED, known)

**Title:** A dotted key can add a key to an already-closed inline table

toml v1.6.0 accepts:

```toml
a = { b = 1 }
a.c = 2
```

TOML states that inline tables are fully self-contained and that keys and
sub-tables cannot be added outside the braces, so `a.c = 2` should be an error.

```go
var v map[string]any
_, err := toml.Decode("a = { b = 1 }\na.c = 2", &v)
fmt.Println(v, err)
```

| Implementation | Result |
|---|---|
| Rust `toml` | `cannot extend value of type inline table with a dotted key` |
| CPython `tomllib` | `Cannot mutate immutable namespace` |
| Node `smol-toml` | `trying to redefine an already defined value` |
| **BurntSushi `toml` v1.6.0** | **accepted** |

Witness for the `dotted_key_redefines_inline_table` marker.

---

## 3. BurntSushi/toml — `[table]` redefines a dotted-key table (NOT FILED, known)

**Title:** `[table]` header accepted for a table already created by a dotted key

toml v1.6.0 accepts:

```toml
a.b = 1
[a]
```

The dotted key `a.b` defines `a` implicitly, and a later `[table]` header may not
redefine it. (`[table]` *can* define sub-tables within tables created by dotted
keys, but `[a]` here targets `a` itself.)

```go
var v map[string]any
_, err := toml.Decode("a.b = 1\n[a]", &v)
fmt.Println(v, err)
```

| Implementation | Result |
|---|---|
| Rust `toml` | `duplicate key` |
| CPython `tomllib` | `Cannot declare ... twice` |
| Node `smol-toml` | `trying to redefine an already defined value` |
| **BurntSushi `toml` v1.6.0** | **accepted** |

Witness for the `std_table_redefines_implicit_table` marker.

*(2 and 3 may be filed together — different rules, same maintainer.)*

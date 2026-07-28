# Differential conformance harness

Takes a synthesized suite and reports where independent TOML implementations
disagree with the RuSmt reference oracle about whether a document is accepted.

## What is compared, and what is not

Only **accept/reject**. Error *messages* are recorded for triage but never
diffed: implementations word them differently and no part of the TOML spec
requires otherwise, so a wording mismatch is not a conformance bug.

An accept/reject mismatch is a *candidate* bug. Each one still has to be read
against the spec by hand — the reference semantics is a formalisation written by
the authors of this artifact, not a normative document, so a disagreement can
equally be a bug in the reference. `diff.py` flags; it does not adjudicate.

## Implementations

| name | what it is | how it is built |
|---|---|---|
| `rust-toml` | the Rust `toml` crate | `runners/rust_toml`, own lockfile |
| `py-tomllib` | CPython's stdlib `tomllib` | no build |
| `go-burntsushi` | `github.com/BurntSushi/toml` | `runners/go_bs`, own `go.mod` |
| `node-smol-toml` | `smol-toml` | `runners/node_smol`, own `package.json` |

Each runner reads one file and prints exactly one line: `OK`, or `ERR <class>`
where `<class>` is a truncated implementation message. Adding an implementation
means adding a runner with that contract plus one entry in `IMPLS` in `diff.py`.

The oracle side is `rusmt-lang observe toml <file>`, which prints the canonical
observable behaviour — `OK`, `ERR <marker>`, `NOMATCH`, or `TIMEOUT`. The marker
name is what makes a disagreement diagnosable: it says which spec rule the
reference thinks the input violates.

## Running it

```sh
# 0. build the oracle and the runners
cargo build --workspace
(cd runners/rust_toml && cargo build)
(cd runners/go_bs     && go build -o go_bs_runner .)
(cd runners/node_smol && npm install)

# 1. gather the pipeline's witnesses into a suite directory
python3 collect.py ../lang/src/synthesis/toml/z3_chc /tmp/suite

# 2. diff the suite across implementations
python3 diff.py /tmp/suite --json /tmp/diff.json
```

`collect.py` writes one `.toml` per accepted witness, named after its marker;
additional witnesses for the same marker get a `__2`, `__3` suffix so the diff
exercises all of them. One witness per marker is thin — the solver may return an
input every implementation already handles.

Versions are printed in the `diff.py` header, so a reported table always carries
the implementations it was measured against.

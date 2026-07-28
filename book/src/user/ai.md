# AI in the loop

RuSmt uses a model only after Z3 fails to synthesize a witness directly. The
order is fixed:

1. RuSmt emits the ordinary marker query.
2. Z3 tries the query with the input unconstrained.
3. `sat` becomes a decoded witness; native-recursion `unsat` means the marker is
   unreachable under the encoding and bound.
4. Only `unknown` or timeout invokes the model.
5. The model reads the emitted SMT-LIB excerpt, the marker name, and previous Z3
   feedback, then proposes one candidate input between `<<<` and `>>>`.
6. RuSmt adds one equality, `input_0 == candidate`, to the original query and
   asks Z3 to decide it.
7. Only Z3 `sat` plus replay through the concrete oracle becomes a suite entry.

The model is therefore a search heuristic, not a source of truth. A wrong
candidate is rejected by Z3 as `unsat`, times out, or fails replay.

## Configuration

The proposer is any command that reads a prompt on stdin and writes its answer on
stdout:

```bash
mkdir -p /tmp/rusmt-sandbox

export RUSMT_LLM_CMD='cd /tmp/rusmt-sandbox && codex exec -m gpt-5.4-mini \
  -c model_reasoning_effort="low" \
  -s read-only \
  --skip-git-repo-check \
  --ephemeral \
  --ignore-rules \
  -'

export RUSMT_LLM_CACHE=/tmp/rusmt-cache
export RUSMT_ROUNDS=9
export RUSMT_WITNESSES=1
export RUSMT_Z3_SECS=15
```

The sandbox matters. In the reported TOML run, the proposer ran from an empty
directory with file and shell tools disabled, so it could not inspect
`lang/src/toml/` and recover answers from the Rust source. The prompt cache is
also part of the result: it lets the reported run replay without calling the
model again. Re-running the model from scratch may produce a different suite.

## Running Suite Generation

Use the normal derive command. The framework automatically tries Z3 first and
falls back to the model when needed:

```bash
cargo run -p rusmt-smt-derive -- toml parse_toml \
  --suite-out /tmp/rusmt-suite/toml

cargo run -p rusmt-smt-derive -- imp eval_com \
  --suite-out /tmp/rusmt-suite/imp
```

For large runs, use the resumable sweep driver:

```bash
python3 experiments/sweep.py toml parse_toml /tmp/toml.jsonl --jobs 6 --timeout 120
python3 conformance/collect.py /tmp/toml.jsonl /tmp/rusmt-suite/toml
python3 conformance/diff.py /tmp/rusmt-suite/toml --json /tmp/toml-diff.json
```

## Reported TOML Result

The TOML v1.1.0 parser has 182 named markers. Z3 alone produced 0 witnesses at
affordable budgets. With the model-to-Z3 loop, the reported run produced 131
accepted TOML documents and left 51 markers uncovered.

The proposal audit is the main reason Z3 remains in the loop. Per-round histories
were recorded for 156 of the 182 markers; the other 26 were covered by an earlier
sequential `cargo run` pass that preserved the accepted witnesses but not the
per-round outcomes, so they count towards coverage and are excluded from the
audit below.

| Outcome | Count |
|---|---:|
| Accepted by Z3 and replay | 105 |
| Rejected by Z3 as not reaching the marker | 537 |
| Undecided within budget | 7 |
| **Total proposals that reached Z3** | **649** |
| Duplicate candidate skipped before Z3 | 64 |
| No parsable candidate returned | 5 |

So roughly five of every six proposals did not reach the marker they were
labelled with. Adding the 26 witnesses from the sequential pass gives the
131-input conformance suite. Running it against the Rust `toml` crate, CPython
`tomllib`, BurntSushi `toml`, and Node `smol-toml` produced 15 accept/reject
divergences.

## TOML vs IMP Rendering

TOML's top-level input is already the document text, so an accepted model is
already a TOML program. No separate renderer is needed.

IMP is different: `eval_com` takes a `Com` AST. When Z3 solves an IMP marker, the
framework decodes the AST model and renders it back to `.imp` source before
replay and suite materialization.

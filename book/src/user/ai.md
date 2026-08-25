# AI in the loop

RuSmt uses a model only after Z3 fails to synthesize a witness directly. The
order is fixed:

1. RuSmt emits the ordinary marker query.
2. Z3 tries the query with the input unconstrained.
3. `sat` becomes a decoded witness; native-recursion `unsat` means the marker is
   unreachable under the encoding and bound.
4. Anything else — `unknown`, a timeout, or no verdict at all — invokes the
   model. There is no flag that turns this off: a missing `RUSMT_LLM_CMD` is a
   run failure the driver reports, not a quieter run.
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
export RUSMT_LLM_CMD="./my_proposer"
export RUSMT_LLM_CACHE=/tmp/rusmt-cache
export RUSMT_ROUNDS=9        # a resource guard; see "When the loop stops"
export RUSMT_WITNESSES=1
export RUSMT_STAGE1_SECS=2   # Z3 alone on the unmodified query
export RUSMT_Z3_SECS=15      # each Stage-2 acceptance
```

Any command works, including a local model or a deterministic script, as long as
it reads exactly one prompt on stdin and writes exactly one answer on stdout.

The sandbox matters. The framework runs every proposer invocation from a fresh
temporary directory, then deletes it. The prompt cache is also part of the
result: it lets the reported run replay without calling the model again.
Re-running the model from scratch may produce a different suite.

## Running Suite Generation

Use the normal derive command. The framework automatically tries Z3 first and
falls back to the model when needed:

```bash
cargo run -p rusmt-smt-derive -- toml parse_toml \
  --suite-out /tmp/rusmt-suite/toml

cargo run -p rusmt-smt-derive -- imp eval_com \
  --suite-out /tmp/rusmt-suite/imp
```

```bash
cargo run -p rusmt-smt-derive -- toml parse_toml \
  --jobs 6 \
  --out-dir /tmp/rusmt-toml-out \
  --suite-out /tmp/rusmt-suite/toml

cargo run -p rusmt-smt-derive -- imp eval_com \
  --out-dir /tmp/rusmt-imp-out \
  --suite-out /tmp/rusmt-suite/imp

python3 conformance/diff.py /tmp/rusmt-suite/toml --json /tmp/toml-diff.json
python3 specops/report.py /tmp/rusmt-toml-out/ledger.jsonl /tmp/toml-diff.json
```

`--out-dir` keeps the per-marker queries, transcripts, and `ledger.jsonl`
outside the repository. `--jobs` runs independent markers in parallel; each
marker still has its own output directory and each proposer call still gets a
fresh temporary directory.

## When the loop stops

Three reasons, and only one of them is a conclusion:

| Stop | What it licenses |
|---|---|
| `witness` | Z3 accepted the candidate. A **conclusion**: the lifted semantics reach the marker on that input. |
| `budget` | `RUSMT_ROUNDS` ran out. **A spending limit, not a verdict.** The proposer is stochastic, so another sample or a larger budget may still find a witness. |
| `proposer-error` | The transport failed. A run to fix, not a result. |

The only statement of the form "no input reaches this marker" the pipeline makes
comes from Z3 answering `unsat` in Stage 1 on the unmodified, unbounded query.
Everything else leaves the marker open, so **a coverage figure is always a lower
bound**. The two spending stops are recorded to help you diagnose a run, not so
either can be reported as exhaustion.

## Reported TOML Result

The TOML v1.1.0 parser has 183 named markers, and Z3 alone produces 0 witnesses
at any affordable budget, so every marker that gets covered is covered by the
loop.

> **Numbers here are not pinned.** They belong to whichever run you report, and a
> run with a different proposer will differ. Regenerate them with
> `specops/report.py`, which prints the coverage, the proposal audit, the
> round curve and the miss list, and also emits the macro block for the paper.

The proposal audit is the main reason Z3 remains in the loop: it records how many
proposals Z3 *rejected*. A rejected proposal is one whose document does not
exercise the rule it was labelled with, which is precisely what an unaudited
model-written suite would ship without noticing.

## TOML vs IMP Rendering

TOML's top-level input is already the document text, so an accepted model is
already a TOML program. No separate renderer is needed.

IMP is different: `eval_com` takes a `Com` AST. When Z3 solves an IMP marker, the
framework decodes the AST model and renders it back to `.imp` source before
replay and suite materialization.

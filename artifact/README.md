# The recorded run behind the paper's numbers

`run/` is the output of `specops/run-experiment-1.sh` on 2026-08-24 (Apple M1, Z3 4.15.4,
Rust 1.88.0, proposer gpt-5.4-mini via codex, RUSMT_ROUNDS=9, RUSMT_Z3_SECS=20,
RUSMT_STAGE1_SECS=2), followed by `conformance/diff.py` against the latest parser
releases on 2026-08-25:

- `suite/imp/`, `suite/toml/` — the generated conformance suites (146 TOML inputs)
- `imp/ledger.jsonl`, `toml/ledger.jsonl` — one row per marker: rounds, outcome, witnesses
- `toml/z3_chc/target_*/` — per-marker transcripts (cosolve.txt, replay.txt, response.toml)
- `toml-diff.json`, `toml-diff.log` — the differential result over four implementations
- `report.txt` — every number in the paper, as printed by `specops/report.py`

`llm-cache/` holds every proposer exchange of the run, keyed by a content hash of the
prompt, so the run replays without model access (`RUSMT_LLM_CACHE=artifact/llm-cache`).

The emitted `.smt2` queries are not archived: they are a deterministic function of the
semantics and regenerate in seconds (`cargo run -p rusmt-smt-derive -- toml parse_toml`).

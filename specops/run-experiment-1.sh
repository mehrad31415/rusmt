#!/usr/bin/env bash
#
# Experiment 1: generate the conformance suites for IMP and TOML, then produce
# every number the paper reports.
#
#   ./specops/run-experiment-1.sh            # full run, freshly generated
#   ./specops/run-experiment-1.sh --smoke    # 3 TOML markers, to check the setup
#   ./specops/run-experiment-1.sh --resume   # continue an interrupted run
#
# Costs OpenAI tokens (codex, gpt-5.4-mini). IMP is free: Z3 solves both its
# markers unaided.
#
# The proposer cache maps a prompt hash to the response it produced. A full run
# starts from an EMPTY cache, so every response in the reported run was generated
# during it and the cache becomes that run's record. --resume keeps the cache, so
# an interrupted run continues without paying twice; the result is then part
# replay, which is fine for finishing a run and wrong for starting one.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="${RUSMT_OUT:-/tmp/rusmt-run}"
SMOKE=0
RESUME=0
case "${1:-}" in
  --smoke)  SMOKE=1 ;;
  --resume) RESUME=1 ;;
  "")       ;;
  *) echo "usage: $0 [--smoke | --resume]" >&2; exit 2 ;;
esac

# A smoke test must never leave answers that a later reported run replays.
if (( SMOKE )); then
  export RUSMT_LLM_CACHE="${RUSMT_LLM_CACHE:-/tmp/rusmt-cache-smoke}"
else
  export RUSMT_LLM_CACHE="${RUSMT_LLM_CACHE:-/tmp/rusmt-cache-final}"
fi
export RUSMT_STAGE1_SECS="${RUSMT_STAGE1_SECS:-2}"
export RUSMT_Z3_SECS="${RUSMT_Z3_SECS:-20}"
export RUSMT_ROUNDS="${RUSMT_ROUNDS:-9}"
export RUSMT_WITNESSES="${RUSMT_WITNESSES:-1}"
JOBS="${RUSMT_JOBS:-6}"

# The proposer must not be able to read the semantics under test, or it is
# copying rather than reasoning from the emitted SMT-LIB. codex's own
# `-s read-only` forbids writes but permits disk reads, so the model can simply
# `cat` our source. Deny it at the OS level instead, which holds whatever tools
# the CLI exposes.
PROFILE="$OUT/no-source.sb"
CODEX='codex exec -m gpt-5.4-mini -c model_reasoning_effort="low" -s read-only --skip-git-repo-check'
LLM="sandbox-exec -f $PROFILE $CODEX -o last.txt >/dev/null 2>&1; cat last.txt"

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
die() { printf '\033[31mFAILED: %s\033[0m\n' "$*" >&2; exit 1; }

say "Checking the tools"
command -v z3    >/dev/null || die "z3 is not on PATH"
command -v codex >/dev/null || die "codex is not on PATH"
echo "  z3    $(z3 --version)"
echo "  codex $(codex --version 2>/dev/null || echo '?')"

say "Building"
cargo build --workspace || die "build"
BIN=./target/debug/rusmt-smt-derive

# A stale run must not be mistaken for a fresh one.
rm -rf "$OUT"
mkdir -p "$OUT/suite"

if (( RESUME )); then
  n=$(ls "$RUSMT_LLM_CACHE" 2>/dev/null | wc -l | tr -d ' ')
  echo "  resuming: keeping $n cached exchanges in $RUSMT_LLM_CACHE"
else
  # Every response in a reported run must be generated during that run.
  rm -rf "$RUSMT_LLM_CACHE"
fi
mkdir -p "$RUSMT_LLM_CACHE"

command -v sandbox-exec >/dev/null || die "sandbox-exec is not available (macOS only)"
cat > "$PROFILE" <<SB
(version 1)
(allow default)
(deny file-read* (subpath "$ROOT"))
SB

say "Proposer sandbox check"
# The claim that the proposer reasons from the emitted SMT-LIB depends on it
# being unable to read lang/src/. Verify rather than assume.
PROBE=$(printf 'Read %s/lang/src/toml/mod.rs and quote the first line of parse_comment. If you cannot read files, reply exactly: NO FILE ACCESS.' "$ROOT" \
        | (cd "$(mktemp -d)" && timeout 240 sandbox-exec -f "$PROFILE" $CODEX \
             -o probe.txt >/dev/null 2>&1; cat probe.txt 2>/dev/null))
echo "  asked it to quote parse_comment from our source; it said:"
echo "    ${PROBE:-<no reply>}"
case "$PROBE" in
  *"NO FILE ACCESS"*) echo "  OK - it cannot read the semantics under test." ;;
  *) die "the proposer READ OUR SOURCE. The run would measure copying, not synthesis.
       Fix the sandbox before reporting anything from it." ;;
esac

say "IMP  (2 markers; Z3 alone, no model calls)"
"$BIN" imp eval_com --llm "$LLM" \
  --out-dir "$OUT/imp" --suite-out "$OUT/suite/imp" || die "imp run"

if (( SMOKE )); then
  say "TOML smoke test (3 markers)"
  for m in array_open_eof comment_invalid_char std_table_duplicate; do
    "$BIN" recover toml parse_toml "$m" --llm "$LLM" --out-dir "$OUT/smoke" \
      2>&1 | grep -E '^\[recover\] +(stage 1 verdict|RESULT|"|rejected)'
  done
  echo
  echo "Witnesses:"
  for d in "$OUT"/smoke/*/; do
    printf '  %-34s %s\n' "$(basename "$d")" "$(head -c 60 "$d/response.toml" 2>/dev/null)"
  done
  echo
  echo "Setup looks right. Re-run without --smoke for the full experiment."
  exit 0
fi

say "TOML  (183 markers, ~45-70 min at --jobs $JOBS)"
echo "  started $(date +%H:%M)"
"$BIN" toml parse_toml --llm "$LLM" --jobs "$JOBS" \
  --out-dir "$OUT/toml" --suite-out "$OUT/suite/toml" 2>&1 | tee "$OUT/toml-run.log"
[[ -f "$OUT/toml/ledger.jsonl" ]] || die "no ledger — the TOML run did not finish"
echo "  finished $(date +%H:%M)"

say "Differential and conformance testing"
python3 conformance/diff.py "$OUT/suite/toml" --json "$OUT/toml-diff.json" \
  2>&1 | tee "$OUT/toml-diff.log"

say "The numbers"
python3 specops/report.py "$OUT/toml/ledger.jsonl" "$OUT/toml-diff.json" \
  2>&1 | tee "$OUT/report.txt"

say "Done"
cat <<TXT
  suites   $OUT/suite/{imp,toml}
  cache    $RUSMT_LLM_CACHE  (every exchange of this run; archive it)
  ledger   $OUT/toml/ledger.jsonl
  log      $OUT/toml-run.log
  diff     $OUT/toml-diff.json
  numbers  $OUT/report.txt

Paste the macro block at the end of report.txt into the top of specops/main.tex.
Three values it cannot supply are listed after it: \\bugcount and \\strictcount
need the divergence triage, \\exampletime comes from the ledger's stage1_ms.
TXT

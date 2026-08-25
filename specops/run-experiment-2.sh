#!/usr/bin/env bash
#
# Experiment 2: authoring, names withheld.
#
#   ./specops/run-experiment-2.sh            # full run
#   ./specops/run-experiment-2.sh --smoke    # 1 round, to check the setup
#   ./specops/run-experiment-2.sh --resume   # keep the proposer cache
#
# THE QUESTION
#   Given only the prose specification for TOML integers, can a proposer that has
#   never seen our parser draft a semantics that draws the valid/invalid line
#   where the standard draws it — and at what resolution?
#
#   The draft invents its own marker names. Ours are not supplied: 56 of the 57
#   contain their own rule word, so supplying them would supply the rules. Both
#   metrics below are therefore name-free.
#
# WHAT THE PROPOSER IS GIVEN
#   1. the specification   specops/exp2/spec-integer.md
#                          the TOML 1.1.0 Integer section, verbatim from toml.io
#   2. the prompt          smt/derive/src/authoring.rs (DSL_GUIDE, build_prompt):
#                          the task's requirements, each citing the code that
#                          enforces it
#   3. the DSL itself      READ FROM SOURCE, not summarised — smt/stdlib,
#                          smt/remark, smt/derive. A curated summary is a second
#                          artifact that drifts from the crate it describes.
#   4. 10 valid documents  specops/exp2/examples-integer.tsv, from the spec's own
#                          code blocks, as the spec-example gate. Disjoint from
#                          the scoring set. No example names a marker.
#
#   Denied: lang/ (the reference semantics), specops/ (the paper), conformance/,
#   and the generated suites. Asserted by direct file reads before any model
#   call, then confirmed in both directions with the model itself.
#
# HOW IT IS SCORED   (see specops/evaluate_draft.rs)
#   1. accept/reject agreement over the invalid witnesses AND 43 valid documents.
#      Invalid-only would be degenerate: a parser that rejects everything would
#      score 100%. The 43 valid ones are accepted unanimously by all four
#      independent parsers, and every value is inside 2^53 so that no conforming
#      parser may legitimately reject one for lossless-representation reasons.
#   2. granularity: how many DISTINCT markers the draft fires across the invalid
#      documents. The reference distinguishes one per document. A draft can score
#      100% on (1) with a single marker; this is what separates detecting an
#      error from distinguishing which rule it broke.
#
# Costs OpenAI tokens (codex, gpt-5.4-mini) — the same proposer as experiment 1.
# Requires experiment 1 to have run: its suite supplies the invalid documents.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

EXP1="${RUSMT_OUT:-/tmp/rusmt-run}"
OUT="${RUSMT_OUT2:-/tmp/rusmt-author}"
# macOS resolves /tmp to /private/tmp; a sandbox subpath rule on the unresolved
# form silently matches nothing, so both spellings are denied below.
EXP1_REAL="$(cd "$EXP1" 2>/dev/null && pwd -P || echo "$EXP1")"

SMOKE=0
RESUME=0
case "${1:-}" in
  --smoke)  SMOKE=1 ;;
  --resume) RESUME=1 ;;
  "")       ;;
  *) echo "usage: $0 [--smoke | --resume]" >&2; exit 2 ;;
esac

if (( SMOKE )); then
  export RUSMT_LLM_CACHE="${RUSMT_LLM_CACHE:-/tmp/rusmt-cache-author-smoke}"
  ROUNDS="${RUSMT_AUTHOR_ROUNDS:-1}"
else
  export RUSMT_LLM_CACHE="${RUSMT_LLM_CACHE:-/tmp/rusmt-cache-author}"
  ROUNDS="${RUSMT_AUTHOR_ROUNDS:-4}"
fi
export RUSMT_Z3_SECS="${RUSMT_Z3_SECS:-20}"
export RUSMT_AUTHOR_Z3_SECS="${RUSMT_AUTHOR_Z3_SECS:-20}"
K="${RUSMT_K:-0}"

SPEC=specops/exp2/spec-integer.md
EXAMPLES=specops/exp2/examples-integer.tsv
VALID=specops/exp2/valid-score

PROFILE="$OUT/no-source.sb"
# codex must not sandbox itself: its `-s read-only` permits reading any file on
# disk, and nesting its profile inside ours disables its file tools entirely.
# One layer -- ours -- is what gives selective access.
CODEX='codex exec -m gpt-5.4-mini -c model_reasoning_effort="low" --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check'
# -o must be absolute; a relative path is not written under sandbox-exec.
LAST=/tmp/rusmt-proposer-last.txt
LLM="sandbox-exec -f $PROFILE $CODEX -o $LAST >/dev/null 2>&1; cat $LAST"

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
die() { printf '\033[31mFAILED: %s\033[0m\n' "$*" >&2; exit 1; }

say "Checking the tools"
command -v z3    >/dev/null || die "z3 is not on PATH"
command -v codex >/dev/null || die "codex is not on PATH"
command -v sandbox-exec >/dev/null || die "sandbox-exec is not available (macOS only)"
echo "  z3    $(z3 --version)"
echo "  codex $(codex --version 2>/dev/null || echo '?')"
[ -f "$SPEC" ]     || die "missing $SPEC"
[ -f "$EXAMPLES" ] || die "missing $EXAMPLES"
[ -d "$VALID" ]    || die "missing $VALID"
[ -d "$EXP1/suite/toml" ] || die "no reference suite at $EXP1/suite/toml — run experiment 1 first"
echo "  spec      $SPEC ($(wc -l < "$SPEC" | tr -d ' ') lines)"
echo "  gate docs $EXAMPLES ($(grep -vc '^#' "$EXAMPLES" | tr -d ' ') valid documents)"
echo "  valid set $VALID ($(ls "$VALID"/*.toml | wc -l | tr -d ' ') documents)"

# The scoring set must be disjoint from the gate examples.
for f in "$VALID"/*.toml; do
  if grep -v '^#' "$EXAMPLES" | cut -f1 | grep -qxF "$(cat "$f")"; then
    die "$f is also a gate example; the scoring set must be disjoint"
  fi
done
# An example naming a marker would leak the taxonomy being measured.
if grep -v '^#' "$EXAMPLES" | grep -q 'err:'; then
  die "$EXAMPLES names a marker; the names are withheld in this design"
fi
echo "  disjointness and no-marker-names: OK"

say "Building"
cargo build --workspace || die "build"
BIN=./target/debug/rusmt-smt-derive

rm -rf "$OUT"; mkdir -p "$OUT"
OUT_REAL="$(cd "$OUT" && pwd -P)"

if (( RESUME )); then
  n=$(ls "$RUSMT_LLM_CACHE" 2>/dev/null | wc -l | tr -d ' ')
  echo "  resuming: keeping $n cached exchanges in $RUSMT_LLM_CACHE"
else
  rm -rf "$RUSMT_LLM_CACHE"
fi
mkdir -p "$RUSMT_LLM_CACHE"

# The DSL is readable, the TOML semantics is not. SBPL applies later rules over
# earlier ones.
cat > "$PROFILE" <<SB
(version 1)
(allow default)
(deny file-read* (subpath "$ROOT"))
(allow file-read* (subpath "$ROOT/smt"))
(deny file-read* (subpath "$EXP1"))
(deny file-read* (subpath "$EXP1_REAL"))
(deny file-read* (subpath "$OUT"))
(deny file-read* (subpath "$OUT_REAL"))
SB

say "Boundary self-test (no model calls)"
must_read() { sandbox-exec -f "$PROFILE" head -1 "$1" >/dev/null 2>&1 \
                && echo "  readable  $2" || die "the proposer cannot read $1 — it needs the DSL"; }
must_deny() { sandbox-exec -f "$PROFILE" head -1 "$1" >/dev/null 2>&1 \
                && die "LEAK: $1 is readable under $PROFILE" || echo "  denied    $2"; }
must_read "$ROOT/smt/stdlib/src/exp.rs"             "stdlib quantified expressions"
must_read "$ROOT/smt/stdlib/src/dt/seq.rs"          "stdlib sequences"
must_read "$ROOT/smt/remark/src/marker.rs"          "remark marker collection"
must_read "$ROOT/smt/derive/src/parser/dsl.rs"      "derive front end"
must_read "$ROOT/smt/derive/src/ir/sort.rs"         "derive sorts"
must_read "$ROOT/smt/derive/src/backend/codegen.rs" "derive backend"
must_deny "$ROOT/lang/src/toml/integer.rs"          "the reference integer parser"
must_deny "$ROOT/lang/src/certify.rs"               "the marker catalog"
must_deny "$ROOT/lang/src/toml/MARKERS.md"          "the marker index"
must_deny "$ROOT/specops/main.tex"                  "the paper"
must_deny "$EXP1/suite/toml/integer_bin_overflow.toml"      "the scoring set (via /tmp)"
must_deny "$EXP1_REAL/suite/toml/integer_bin_overflow.toml" "the scoring set (resolved)"

say "Proposer boundary check"
# Both directions matter: if the DSL is unreadable the proposer is guessing and a
# failure means nothing; if the reference is readable it is copying and a success
# means nothing.
probe() { printf '%s' "$1" | (cd "$(mktemp -d)" && timeout 240 sandbox-exec -f "$PROFILE" \
            $CODEX -o /tmp/rusmt-probe.txt >/dev/null 2>&1); cat /tmp/rusmt-probe.txt 2>/dev/null; }

POS=$(probe "List the names of the pub fn items in $ROOT/smt/stdlib/src/dt/seq.rs, comma separated. If you cannot read the file, reply exactly: NO FILE ACCESS.")
echo "  CAN it read the DSL? (pub fns of stdlib seq.rs)"
echo "    $(printf '%s' "${POS:-<no reply>}" | head -c 150)"
case "$POS" in
  *extract*|*concat*) echo "  OK - the DSL definition is readable." ;;
  *) die "the proposer CANNOT read the DSL it must write in. A failed draft would
       measure our sandbox, not the proposer. Check the allow rules in $PROFILE." ;;
esac

NEG=$(probe "Read $ROOT/lang/src/toml/integer.rs and quote the first line of parse_integer. If you cannot read files, reply exactly: NO FILE ACCESS.")
echo "  CAN it read the answer? (quote parse_integer)"
echo "    $(printf '%s' "${NEG:-<no reply>}" | head -c 150)"
case "$NEG" in
  *"NO FILE ACCESS"*|*"not"*"read"*|*"cannot"*|*"denied"*) echo "  OK - the reference is not readable." ;;
  *) die "the proposer READ THE REFERENCE SEMANTICS. This run would measure
       copying, not authoring. Fix the deny rules before reporting anything." ;;
esac

say "Scoring set"
mkdir -p "$OUT/invalid"
n=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  cp "$f" "$OUT/invalid/" && n=$((n+1))
done < <(find "$EXP1/suite/toml" -maxdepth 1 -type f -name 'integer_*.toml' 2>/dev/null)
echo "  invalid: $n documents (the integer witnesses from experiment 1)"
echo "  valid  : $(ls "$VALID"/*.toml | wc -l | tr -d ' ') documents"
[ "$n" -gt 0 ] || die "no integer witnesses found in $EXP1/suite/toml"

say "Authoring (up to $ROUNDS rounds)"
DRAFT="$OUT/draft.rs"
"$BIN" author "$SPEC" "$DRAFT" \
  --llm "$LLM" --examples "$EXAMPLES" "rounds=$ROUNDS" \
  2>&1 | tee "$OUT/authoring.log"

if [ ! -s "$DRAFT" ]; then
  say "No draft was admitted"
  cat <<'MSG'
  Every round was rejected by the gates. That is a reportable outcome, not a
  crash: it says the proposer could not produce a semantics that parses,
  transpiles, declares markers, has no marker it can never reach, and accepts
  the spec's own valid documents — within the round budget.

  Which gate stopped each round:
MSG
  echo "    $OUT/authoring.log"
  echo "    $DRAFT.authoring.txt   (every draft and every gate complaint)"
  exit 0
fi

say "Scoring"
ENTRY=$(grep -o 'entry function: `[a-zA-Z0-9_]*`' "$OUT/authoring.log" | tail -1 | sed 's/.*`\(.*\)`/\1/')
[ -n "$ENTRY" ] || die "could not read the entry function name from $OUT/authoring.log"
echo "  entry function: $ENTRY   k = $K"
mkdir -p "$OUT/draftdir" && cp "$DRAFT" "$OUT/draftdir/"
cargo run -q -p rusmt-smt-derive --example evaluate_draft -- \
  "$OUT/draftdir" "$ENTRY" "$OUT/invalid" "$VALID" "$K" \
  2>&1 | tee "$OUT/score.txt"

say "Done"
cat <<MSG
  draft        $DRAFT
  transcript   $DRAFT.authoring.txt   (every draft, every gate complaint)
  authoring    $OUT/authoring.log
  score        $OUT/score.txt
  cache        $RUSMT_LLM_CACHE   (every exchange of this run; archive it)

  Two numbers for the paper: accept/reject agreement, and the resolution ratio.
  The gate transcript is worth reading even if the draft scored well — a marker
  the solver PROVED unreachable in the draft's own semantics is a failure mode
  no unaided chat session can detect.
MSG

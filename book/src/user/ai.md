# AI in the loop

RuSmt has two optional AI integrations. Both follow one architectural rule:

> **The model is a proposer, never a certifier.** Whatever an LLM produces is
> accepted only after the framework's own machinery validates it — concrete
> replay through the reference semantics for candidate inputs, the DSL front
> end and SMT backend for drafted specifications. A wrong proposal is rejected;
> it can never become a false result.

Both features are disabled unless a proposer command is configured:

```bash
# Any command that reads a prompt on stdin and writes its answer on stdout.
export RUSMT_LLM_CMD="claude -p"          # or `llm -m <model>`, a local script…
export RUSMT_LLM_MODE=guided              # direct (default) | guided | both
export RUSMT_LLM_MAX_GUESSES=4            # optional; direct-mode rounds per target
export RUSMT_GUIDE_ROUNDS=4               # optional; guided-mode rounds per target
export RUSMT_GUIDE_Z3_SECS=30             # optional; per-round Z3 budget (guided)
export RUSMT_GUIDE_MODELS=3               # optional; distinct models mined per round
export RUSMT_AUTHOR_Z3_SECS=20            # optional; per-gate Z3 budget (authoring)
```

The pipeline is deliberately vendor-agnostic: it never embeds a model name, an
API key, or an HTTP client. Anything that can run as a shell command can be the
proposer — including a deterministic script, which is how the test suite
exercises the loop.

## Recovering targets the solver fails on

Synthesis (`cargo run -p rusmt-smt-derive -- <lang> <entry_fn> [k=N]`) asks Z3
first, for every target. The proposer concerns what happens
next, per target — in one of two modes selected by `RUSMT_LLM_MODE`.

### Direct mode (default): whole-input proposals

1. **Z3 returns a model** that decodes to object-language source → if the
   target is a *named* marker of a language registered for replay, the
   witness is *replayed* through the concrete reference semantics; the
   verdict is written to `target_*/replay.txt`. A genuine witness is `CERTIFIED`; a
   spurious candidate model (possible under `k`-bounded unrolling, where the
   query can be satisfied through the depth cutoff) is `REJECTED` — and treated
   like a solver failure below.
2. **Z3 fails to *search*** — timeout, `unknown`, an `unsat` under `k`-bounding
   (which only means "no witness within depth k"), or a replay-rejected model —
   and the target is a **named** marker (`Path::named`): the proposer is asked
   for a candidate input directly. The candidate is **double-gated, and Z3 is
   never bypassed**: it is first pinned as a `define-fun` macro and *validated by
   Z3* (`macro_inline_input`; sub-second `sat` on the array-free encoding), and
   only a Z3-validated candidate is then certified by replay **in an isolated
   process** (so a non-terminating candidate like `while true do skip` or a
   stack-overflowing one costs one rejected attempt instead of crashing the
   run). A candidate Z3 refuses (`unsat`/`unknown`/timeout) is rejected, never
   accepted on replay alone. The verdict — Z3's, plus *which other marker* fired
   by name — is fed back into the next prompt. The loop stops at the first
   Z3-validated, replay-certified witness or after `RUSMT_LLM_MAX_GUESSES`
   rounds.
3. A certified fallback witness replaces the target's response file — named by
   the object-language extension so it replays directly (`target_*/response.imp`
   for IMP, `.toml`, or `.tc`); the complete attempt transcript (every
   candidate, every verdict, provenance) is written to `target_*/fallback.txt`.

### Guided mode: the LLM scaffolds, Z3 completes

`RUSMT_LLM_MODE=guided` replaces the whole-input guess with an iterative
LLM⇄Z3 collaboration (`both` runs one direct round first — the degenerate
empty-scaffold case — then the guided loop). Per round:

1. The proposer emits a **scaffold**: declarative constraints on the input
   text, one per line, in a deliberately small language —

   ```text
   prefix "…"   suffix "…"   contains "…"   forbid "…"
   at <i> "<c>"   range <i> '<lo>' '<hi>'   len_min <n>   len_max <n>
   ```

   Unparseable lines are ignored and reported back; the proposer constrains
   the *skeleton* of an input and leaves the rest open.
2. Each line translates to one sequence-theory assertion over the entry input
   (`seq.prefixof`, `seq.contains`, `seq.nth`, `seq.len`, …), and the block is
   conjoined to the **unmodified** per-target query — function definitions and
   the marker assertion are never touched. Strengthening-only is what keeps
   this sound: every model of the strengthened query is a model of the
   original.

   > **Two experimental, non-structural forms.** The grammar also accepts
   > `exact "…"` and `assume <smt-predicate>`. `exact` pins the whole input via
   > an *equality* assertion `(assert (= input …))` — this is **not** the direct
   > route's macro-inline, so on a deeply recursive parser it can still return
   > `unknown`. `assume` injects a raw predicate the solver uses **without
   > proving it**; it is *not* a meaning-preserving strengthening, so a wrong
   > assumption can make the query `sat` spuriously. Any `assume`-assisted witness
   > is therefore admitted by **replay alone**, not as a Z3 validation of the
   > original query — treat such results as replay-certified-only, not
   > solver-validated. (Neither form is used in any reported artifact result.)
3. Z3 runs on the strengthened query under a short budget
   (`RUSMT_GUIDE_Z3_SECS`, default 30 s), and its verdict drives the next
   round:
   * **sat** → up to `RUSMT_GUIDE_MODELS` *distinct* inputs are mined from the
     round (see [the persistent solver](#the-persistent-solver-more-models-and-knowing-where-z3-stalled)),
     each Z3-validated and replay-certified in an isolated process exactly as in
     direct mode; the first that clears both gates ends the loop, and every
     rejection — including which marker mis-fired, by name — is fed back.
   * **unsat** → "your scaffold contradicts reaching the marker — relax".
     An unsat of the *strengthened* query proves nothing about the original
     and is never reported as unreachability.
   * **timeout/unknown** → "still too hard — constrain further", together with
     Z3's own account of where it stalled.

The loop runs for `RUSMT_GUIDE_ROUNDS` rounds (default 4); the transcript goes
to `target_*/guidance.txt` and the per-round queries to
`target_*/guided_round_*.smt2`; a certified witness replaces the target's
response file as in direct mode.

The division of labor is the point: the model contributes *structural insight*
(what kind of input reaches the marker), the solver contributes *exactness*
(an input that actually satisfies the full executable semantics down to the
marker assertion), and replay remains the only acceptance criterion. Scaffolds
speak the theory of sequences, so guided mode applies to entry inputs of sort
`Seq<U32>` (e.g. TOML's code-point sequence); for ADT inputs (IMP's `Com`) the
pipeline transparently degrades to direct mode.

Boundaries respected deliberately, in both modes:

* `unsat` under native recursion (`k=0`) is a genuine unreachability verdict —
  the fallback does **not** run, because no witness exists to find.
* Markers are addressed by name. A `Path::named(...)` marker's id is a fixed
  hash of its name — identical in the SMT query and in the concrete evaluator —
  so the id a query targets and the id a replayed witness carries coincide by
  construction, which is what makes per-target certification sound. Name the
  markers you want recoverable.

Why this helps: on deeply recursive, multi-theory queries (see the TOML case
study) *searching* for an input is beyond the solver's budget, but *validating*
a determined candidate is a sub-second Z3 decision (plus a millisecond concrete
replay). The fallback exploits exactly that asymmetry — reach is delegated to an
untrusted generator, while trust stays with Z3 *and* the executable
specification.

Validation in `direct` mode keeps Z3 **in the loop** rather than bypassing it.
The candidate is pinned as a `define-fun` **macro** (`macro_inline_input`), which
Z3 inlines and constant-folds along the taken branch; on the array-free TOML
encoding (tables are an assoc-list datatype, not `Array<String,Value>`) a fully
determined candidate decides `sat` in ~0.4 s — where the array encoding returned
`unknown ("incomplete (theory array)")`, and where an `(assert (= input …))`
equality (leaving the input a symbolic constant) still returns `unknown`.
Acceptance is then double-gated: Z3 must *validate* the macro-inlined candidate
with `sat` (an `unknown` or timeout is **rejected**, not accepted) **and**
isolated replay must certify it reaches the same marker. The artifact-included
all-marker TOML run names all 186 targets, supplies concrete proposer candidates
for 182 of them, and stores 151 Z3-validated plus replay-certified witnesses under
`generated-suites/toml-pipeline-run-rerun/`.

A caveat on which mode helps where. Scaffold-completion (`guided`) only pays off
when the *residual search* after strengthening is itself within the solver's
frontier; for TOML the solver completes no free code point, so `direct`
validation is the effective route. Use `guided` where the residual is tractable
(shallow targets); use `direct`/`both` otherwise.

Languages are registered for replay in `lang/src/certify.rs`
(`LanguageOracle`): a name, a prompt brief, and a `certify` function from
candidate source + target marker to a verdict. Adding a language to the
fallback means adding one entry there.

## Authoring with behavioral self-validation

Writing the interpreter in the DSL is the expensive part of using RuSmt. The
`author` mode lowers that cost without trusting the model:

```bash
export RUSMT_LLM_CMD="claude -p"
cargo run -p rusmt-smt-derive -- author my_language_spec.md draft.rs rounds=6 \
    --examples my_language_examples.tsv
```

`rounds=` defaults to 4. The proposer receives a condensed DSL reference plus
your prose specification (and your examples, if given) and must produce one
self-contained Rust source file. Every draft passes through three **mechanical**
gates:

1. **Front end** — the real DSL parser and sort checker (`model()`), the same
   code that processes hand-written specifications; rejections carry the
   parser's error messages.
2. **Back end** — the draft must transpile: the text backend must emit SMT-LIB
   for it without error.
3. **Markers** — the draft must declare at least one `Path::named(...)` marker,
   because named markers are the test intent the rest of the pipeline operates
   on.

A draft that clears those is then checked **behaviorally**, by turning the
framework's own synthesis machinery against it. Both gates need the draft to
declare exactly one top-level entry function (one that no other function
calls — the shape the DSL guide already demands); each gate query runs Z3
under `RUSMT_AUTHOR_Z3_SECS` (default 20 s):

4. **Marker reachability** — for each named marker the draft declares, the
   per-target synthesis query is built from the draft's own IR (the same query
   the pipeline would later run) and solved at `k=0`, i.e. against the draft's
   genuine recursive semantics. `unsat` is then a solver-*proved* fact about
   the draft: the marker is dead code — the draft declared a test intent it
   cannot meet — and that counterexample is fed back ("marker `time_invalid`
   is unreachable dead code in YOUR draft"). `sat` records the witness in the
   transcript; a timeout is a warning, never a failure.
5. **Spec examples** — `--examples` takes a file of `INPUT<TAB>EXPECT` lines
   (blank lines and `#` comments skipped; `\n \t \r \\` escapes in INPUT),
   where `EXPECT` is the input's observable class, phrased in the draft's
   named markers:

   ```text
   k = true<TAB>ok                  # accepted: no named marker fires
   k = tru<TAB>err:boolean_invalid  # this specific marker fires
   = true<TAB>nomatch               # rejected: some named marker fires
   ```

   For each example the input is fixed concretely in the query
   (`(assert (= input_0 …))`) — strengthening only, exactly like a guided-mode
   scaffold — and marker membership of the draft's result is checked per named
   marker. Because the input is concrete, these queries are cheap. A mismatch
   is precise gate feedback: *"on input `k = true` the spec says ok, but your
   draft fires marker `boolean_invalid`"*. Examples come from the prose spec
   or from you — never from the model — and require the entry to take exactly
   one parameter, of type `Seq<U32>` (the input text).

This loop is what a chat session cannot reproduce: each repair round consumes
*solver-generated behavioral counterexamples* against the framework's actual
semantics of the draft — not the model's opinion of its own output. Both gate
failures *and* what the solver positively established (which markers are
reachable and by what concrete input, which examples were reproduced) are fed
back, so a repair knows what already works as well as what broke; the loop
repeats up to the round budget. The full session — every draft, gate failure,
and solver observation — is written to `<out_file>.authoring.txt`, so a
rejected session can be inspected the same way `fallback.txt` / `guidance.txt`
record the synthesis loop.

The admitted draft is written out for **human review**: the gates establish
mechanical admissibility plus the checked behavioral facts (every declared
marker reachable, every supplied example reproduced), not full semantic
fidelity to your prose specification. Fidelity is established the same way as
for a hand-written spec — review, synthesis against its markers, and
differential testing against independent implementations. The human owns the
artifact; the model only saved keystrokes.

## The persistent solver: more models, and knowing where Z3 stalled

A guided round is not one query and one answer. Every round conjoins a
*strengthening* block to the **same** base theory, so re-spawning `z3` per round
would make Z3 re-read and re-typecheck the whole base each time — measured at
~0.9 s for the 206 KB TOML base — and throw away everything the previous round's
search learned.

Instead the loop opens one `z3 -in` process per target (`z3_session.rs`), loads
the base once, and scopes each round with `(push)` / `(pop)`. Two capabilities
follow, and both are steering signal for the proposer:

**More models per round.** Once a scaffold is `sat`, asking for the *next*
distinct input costs only a blocking clause (`input ≠ <previous>`) and a
re-check. A round therefore mines up to `RUSMT_GUIDE_MODELS` (default 3)
candidates, and **each faces both acceptance gates independently** — Z3
validation of the macro-inlined candidate *and* isolated replay. This buys extra
chances, never a cheaper acceptance: the gates are unchanged, there are just
more candidates arriving before the loop spends another proposer round (a model
call plus a full solve). When no candidate passes, *every* rejection is fed back
by name, so the proposer can see the shape of what it keeps hitting:

```text
the solver completed your scaffold with 3 distinct model(s), none accepted — …:
  "k = tru" (Z3 validated the macro-inlined candidate (sat); REJECTED: fired `boolean_invalid`) |
  "k = fals" (…) | "k = ture" (…)
```

If the enumeration returns `unsat` before reaching the limit, the scaffold
admits no further inputs — the round was exhaustive, not truncated.

**Why the search stalled.** On a stuck verdict the process is still alive, so the
loop asks it for its own counters (`(get-info :all-statistics)`) and folds the
profile into the round's feedback. A real line from an IMP query forced to time
out in search:

```text
Where Z3 stalled (its own counters): [z3-stats] conflicts=480 decisions=22401
 propagations=23257 added_eqs=51560 bv_bit2core=115776 datatype_splits=22
 recfun_macro_expansion=796 recfun_case_expansion=654 recfun_body_expansion=2390
 max_memory=23.49 memory=17.59 …
```

That localizes the cost — here recursive-function unfolding (`recfun_*`) and
bit-vector reasoning (`bv_bit2core`), not surface parsing — which is a far better
prompt than the single bit "timeout".

Nothing becomes less inspectable: each round's equivalent standalone query is
still written to `target_*/guided_round_N.smt2` and still re-runnable with a bare
`z3 -smt2`.

**Robustness.** A Z3 wedged in a preprocessing phase that ignores its own
`:timeout` would otherwise hang the pipeline, so every read is deadline-bounded.
On expiry the process group is killed and the session is *poisoned*: subsequent
calls return a failure verdict and the loop falls back to the one-shot path
(`round_on_file`), which spawns a fresh `z3` per round. So a missing, older, or
wedged `z3 -in` costs speed, never correctness — and on the hardest TOML queries,
where the hard kill leaves nothing to read, the statistics are simply absent
rather than wrong.

**Limit.** The counters are only readable once the check returns. Receiving
control from *inside* the search would need Z3's user-propagator API
(`Z3_solver_propagate_*`) — a documented facility, not a patch to Z3 — which is
future work. That does not require abandoning SMT-LIB text:
`Z3_solver_from_string` loads the backend's own emitted query into an in-memory
solver, which a propagator attaches to directly.

# Design rationale

Why each part of the pipeline is the way it is. Every choice below is either
forced by a soundness argument or is a spending decision reported as such;
where an earlier justification turned out to be wrong it is recorded rather than
quietly replaced.

## 1. One command per language

```
rusmt-smt-derive <lang> <entry_fn> [--jobs N] [--out-dir D] [--suite-out S]
```

Emits one query per marker, runs Z3 first, escalates when Z3 returns no model,
writes `ledger.jsonl` and materializes the suite. `specops/report.py` reads
the ledger. There is no second driver; `sweep.py` is gone.

## 2. Stage 1 — Z3 alone

### Why Stage 1 fails on TOML

Lowering the whole semantics into one query and asking Z3 to invert it is
**bounded model checking**. The formula-size blowup that follows from unrolling is
a known cost of that choice, not a discovery — see Clarke et al. (CBMC) and the
CPAchecker tutorial.

Our encoding takes the sharpest form of it: transpiled parsers are `define-fun`,
which SMT-LIB eliminates by macro expansion during preprocessing, before the
solver knows which branch is taken. A concrete document folds as it expands; a
symbolic one keeps both arms of every conditional alive.

**Measured cliff** (marker `array_comment_missing_newline`, 16-char document):

| free chars | result |
|---|---|
| 0 | `sat` 798 ms |
| 1, 2, 3, 4, 6, 8 | `unknown` @ 30 s |

One free character is enough. That is ~55k values, so it is not a search-space
effect — any free variable defeats the folding. There is therefore **no
partially-symbolic middle** to exploit: the concretization move that makes
concolic testing work has no foothold here.

### What `unsat` is and is not for

Unreachability produces no test — the suite is built from `sat` answers. **It is
not a reason to keep the solver in the generation loop.** Its only bearing is on
the miss list: a miss means no input was *found*, not that none exists. It did not
fire in either run, so it is a stated limitation, not a capability we exercised.

### The fix, if you want one

Per-path encoding (KLEE/SymCC-style): walk one path, collect the branch conditions
actually taken, hand the solver a short conjunction that mentions no parser. For
RuSmt that is a symbolic walker over our own DSL AST carrying a path condition and
a cursor. Recursion stops being a problem because a path fixes the data's shape —
a table built by two bindings unrolls `table_get` exactly twice.

The trade: per-path can never prove unreachability (it only ever explores paths it
walked). Since our whole-program query never returns `unsat` at TOML scale anyway,
that trades a global answer we cannot compute for a bounded one we could.

### The stages


The unmodified query, input free, budget `RUSMT_STAGE1_SECS`.

* `sat` → decode, render with the language's registered renderer, replay through
  the concrete semantics, report any mismatch rather than hide it.
* `unsat` → **only at k=0** is this unreachability. Under k-unrolling it means
  "no witness within depth k".
* anything else → Stage 2, unconditionally. A missing proposer is a reported run
  failure, never a quieter run.

## 3. Stage 2 — CEGIS with a language model as generator

The method is counterexample-guided inductive synthesis
(Solar-Lezama et al., ASPLOS 2006) with the generator position filled by a model,
as in Jha et al. (MILCOM 2023). Three parts, each load-bearing:

**Generation.** The proposer sees the emitted SMT-LIB excerpt, the marker name,
the function that raises it, and every earlier candidate with Z3's verdict. It
never sees Rust. The prompt separates a *channel contract* (how to spell a
control character) from a *domain prior* explicitly labelled as hints; the prior
affects convergence speed and nothing about what is accepted.

**Acceptance.** A candidate enters the UNMODIFIED query as one equality,
`input_0 = c`. By the strengthening lemma every model of `Q ∧ A` is a model of
`Q`, so `sat` is a statement about the original marker query however the
candidate was produced. This is why nothing about the model is trusted.

**The counterexample.** A bare `unsat` says only "not this marker", which leaves
the generator guessing — that is generate-and-test, not CEGIS. On a rejection the
pipeline emits a second query that pins the same input, drops the marker
assertion, and binds the returned `Path` to a constant; Z3's model of that
constant decodes to the marker that actually fired. **The counterexample is
derived from Z3, not from the concrete semantics**, so the loop stays inside the
solver and a transpiler bug cannot make the feedback disagree with the query.

## 4. When the loop stops — and what that does NOT mean

Three reasons. Only one is a conclusion:

| Stop | Licenses |
|---|---|
| `witness` | Z3 accepted the candidate. A conclusion. |
| `budget` | `RUSMT_ROUNDS` ran out. **A spending decision.** |
| `proposer error` | The proposer failed. **Not a finding.** |

The proposer is stochastic, so `budget` says only that we stopped paying. It is
not evidence that no input reaches the marker.

The only "no input reaches this marker" the pipeline ever states comes from Z3
answering `unsat` on the unmodified, unbounded query in Stage 1. Everything else
leaves the marker open, so **every coverage figure is a lower bound.**

> Two earlier designs were wrong and are recorded so they are not reinvented.
> First, the cap was justified as "the smallest budget that does not truncate" —
> false, inferred from a weak proposer. Second, a stop-on-repeat rule read a
> repeated proposal as the generator having nothing left to say — false for a
> stochastic generator, and it gave different markers different budgets on a coin
> flip. The rule was deleted; every marker now gets the same budget.

## 5. Independence per marker

Each proposer invocation runs in a fresh temporary directory that is deleted
afterwards, so nothing one marker's proposal writes — or that a model runner
keeps keyed by working directory — can reach another. Verified: the command sees
a cwd containing only `.` and `..`. Prompts contain only their own marker, and
the cache is keyed by prompt hash. `--jobs` runs markers concurrently; they share
no state, so it changes wall clock and nothing else.

## 6. Two kinds of divergence, never conflated

* **Conformance**: an implementation disagrees with the *oracle*. The executable
  specification says the document violates a named rule and the implementation
  accepts it — a candidate bug against the spec, only as good as our
  formalisation, hence triage against the standard.
* **Differential**: implementations disagree with *each other*. Weaker but more
  robust: it shows the suite discriminates between production parsers whether or
  not our reference is right. A suite with no oracle divergence would still be
  doing work if it produced these.

`conformance/diff.py` reports both separately.

## 7. Authoring — kept, unclaimed, with an experiment designed

The mode stays in the artifact. The paper reports no numbers and makes no claim:
proposing a formal artifact, gating it mechanically and feeding the complaint back
is AutoSpec (CAV 2024) and SpecGen, and ours differs only in having no reference
to check the draft against, so the gates show internal consistency and nothing
about whether the semantics is right.

### The experiment (designed, not yet run)

**Subset: the TOML integer literal.** Chosen on evidence — 57 markers, the
largest single fragment, and the only large one where *all 57 already have a
witness* (datetime has 32 markers but only 14 witnesses). Self-contained grammar,
a one-page spec section, and many independent failure modes per radix, so a draft
can be partly right and the result is a distribution rather than a pass/fail.

**Given to the model:** the 57 marker names as a file, the TOML v1.1.0 Integer
spec section by URL, the DSL description, and the `remark` / `derive` / `stdlib`
constraints. **Never our source.**

**It returns:** `parse_toml(Seq<U32>)` — the *same entry signature as the real
reference* — restricted to documents of the form `key = integer`.

**No wrapper is needed, and this is why the subset was chosen that way.** Every
one of the 57 witnesses is already a whole document (`a = 1a`, `x = 1__0`,
`a=0_1`), so they feed the draft unchanged. An earlier design pointed the
framework at `parse_integer` directly and needed a wrapper, because that function
takes a `State { stream, cursor, context }` rather than text: Z3 would have been
free to choose the cursor and the parser context and could return a "witness" no
real document reaches. That is the free/fixed agreement principle — what the query
leaves free must be exactly what the real run leaves free — and the fix is to pick
a fragment whose natural entry is already text, not to bolt a wrapper on.

**Evaluation:** fire the existing 57-input suite at the draft. Per marker:

* **correct** — fires the same named marker;
* **wrong** — fires a different marker, or accepts what the reference rejects;
* **never fired** — declared but unreachable, or not declared.

Report the three percentages. If the draft fails the gates instead, that is also
a result: which gate, after how many repair rounds. That is evidence about DSL
ergonomics, which is what reviewer 5A asked for, and it is the likelier outcome.

## What the framework claims

A conformance suite nobody wrote, from an executable specification the author
writes once. IMP: 1,084 non-blank lines, 2 markers. TOML 1.1.0: 5,878 lines,
183 markers. From that one source the framework derives the oracle, one
reachability query per marker, and the suite.

## Status

Design pinned; build clean; 335 tests pass. The reported TOML run covers 146 of
183 markers,
with the 37 misses named. Paper numbers all live in one macro block that
`report.py` prints verbatim.

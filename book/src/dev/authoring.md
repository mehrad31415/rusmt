# Authoring a semantics with a model

Writing the interpreter in the DSL is the tedious part of using RuSmt. The
`author` mode drafts one for you. It is **experimental and unevaluated**: we
report no numbers for it and make no claim. What follows is what it does and
what is known about it.

## The loop

1. Build a prompt: the prose specification + the author's examples + whatever
   failed last time.
2. Ask the model. It returns a whole `.rs` file, as text.
3. Run that file through a stack of checks.
4. All checks pass → hand the draft to the author. Any check fails → put the
   failure messages in the next prompt and go to 1.

After `rounds=N` (default 4) with no clean draft it gives up and hands you the
transcript, which records every draft and every gate failure.

## The checks, cheapest first

A draft that will not parse never reaches the solver.

| # | Check | Cost |
|---|---|---|
| 1 | Does it parse and sort-check as DSL? | instant |
| 2 | Does it transpile to SMT-LIB? | instant |
| 3 | Does it declare at least one named marker? | instant |
| 4 | Is every marker it declares actually reachable? | one Z3 call per marker |
| 5 | Does it behave as the author's examples say? | one Z3 call per example |

Checks 1–3 are "does it compile". Check 5 is "does it pass the tests".

**Check 4 is the only one that needs this framework.** A draft says "I detect 57
kinds of error". Check 4 builds the reachability query from the draft's *own*
transpiled semantics and asks Z3 whether any input can reach each branch. An
`unsat` means the model wrote a branch that can never fire — it claimed to detect
something it does not. That is fed back as a specific complaint. You cannot do
this by asking a model, and you cannot do it without the DSL-to-SMT lift.

## What admission means

The draft is in the DSL subset, it transpiles, it declares named markers, none of
those markers is dead under its own semantics, and it reproduces the author's
examples. That is *internal consistency*.

It does **not** mean the semantics is correct, faithful to the prose
specification, or complete. No reference exists to check the draft against — that
is the whole difficulty — so an admitted draft is a starting point the author
reviews and owns.

## Where it stands in the literature

Every piece is established. None of it is new.

* **The loop** — propose a formal artifact, gate it mechanically, feed the
  complaint back, repeat — is AutoSpec (CAV 2024) and SpecGen. Same shape, and
  they measured theirs.
* **Checks 1–3** are "use the compiler as the filter", universal in model-driven
  code generation.
* **Check 5** is test-driven acceptance.
* **Check 4** is a *sanity check* in the model-checking sense: the family
  containing vacuity detection (Beer, Ben-David, Eisner and Rodeh) and coverage,
  where the point is that a property can pass *trivially*. A declared but
  unreachable marker is exactly that: coverage intent satisfied vacuously.
  Solver-based reachability for dead-code detection is standard.

So: a known loop with known checks, one of them a known sanity check applied to a
new kind of artifact. The composition is an engineering choice, not a discovery,
and the difference from AutoSpec cuts against us — its verifier checks the draft
against real code, ours can only check the draft against itself.

## The scalability limit

Check 4 costs one solver call per declared marker per round, so a 57-marker draft
over four rounds is up to 228 calls. Two things keep that tolerable and one does
not.

The per-gate budget is capped (`RUSMT_AUTHOR_Z3_SECS`, default 20 s) and **a
timeout is a warning, not a rejection**, so a slow marker cannot stall the loop.
A drafted subset is also small — a few hundred lines rather than the 5,878 of the
full TOML reference — so its queries are far smaller than the ones Z3 cannot
finish on the real thing.

What does not scale: a draft the size of the full TOML reference would make check
4 impractical, for the same reason Stage 1 of synthesis is impractical there. The
mode is suited to drafting a fragment, not a whole large language.

## What would make it publishable

Measurement, not design. Draft a TOML subset from its marker names and the spec,
then fire the generated conformance suite at the draft and report, per marker,
whether it fires correctly, fires the wrong marker, or never fires. If the drafts
do not compile, the distribution of gate failures is itself a result about the
DSL's ergonomics. That experiment is designed in
[Design rationale](design.md) and has not been run.

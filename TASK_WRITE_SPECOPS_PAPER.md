# TASK — Write the RuSMT scientific paper for SPECops 2026 (background, autonomous)

You are an autonomous engineer-author working inside the RuSMT repository
(`/Users/mehradhq/WATERLOO/rusmart_project/rusmart`). Your job is to turn the
existing draft into a **submission-ready, fully scientific paper** for the
**SPECops 2026** workshop (1st Intl. Workshop on Specification-Driven Development
Life Cycle, co-located with SPLASH/ISSTA 2026), and to back every claim with
**real, reproduced measurements** and **real, verified citations**.

You have **broad authority**: read/run/modify anything in the repo, run the build
and the solver, run experiments, install or fetch LaTeX classes, edit `.tex`,
`.bib`, and figures, and rebuild the PDF. You do **not** need to ask permission to
proceed through the phases below. Do not push to a remote, do not make the GitHub
repo public, and do not delete the existing `documents/paper/main.tex` (the long
non-anonymized draft) — treat it as source material and keep it intact.

> **Deadline reality:** SPECops submission is **June 15, 2026** (today is
> 2026-05-28, ~18 days out). Notification July 15. Format: **ACM SIGPLAN `acmart`
> (`sigplan` subformat), DOUBLE-BLIND.** Full paper = **10 pages body + 2 pages
> references**; Short/Tool/Vision = **4 pages + 1 page references**; Extended
> Abstract = 4 pages (no APC).

---

## 0. The single most important rules (read first)

1. **HONEST BUT AMBITIOUS.** Claim everything the evidence supports, and frame it
   for maximum impact — but never claim what you cannot reproduce or cite. If a
   number is not measured, you either measure it (preferred) or you do not state
   it. **No `[TODO]`, no placeholder numbers, no "we expect", no hand-waving in
   place of data.** The current draft has two literal `[TODO: measure]` tokens
   (`documents/paper/main.tex:630` and `:704`) — these MUST be gone from the
   final paper, replaced by real measurements or honestly scoped out.
2. **NEVER fabricate a citation.** Every reference must be a real, locatable
   paper. The existing `documents/paper/refs.bib` is real — reuse it. For any
   **new** citation, verify it exists (title, authors, venue, year, DOI/URL) via
   web search before adding it; if you cannot verify it, do not cite it. A single
   hallucinated reference is a desk-reject and an integrity violation.
3. **Reproduce, don't trust.** Re-run the experiments yourself (Phase 1). Report
   the numbers you actually observe on this machine, with the exact solver
   version, per-target budget, and machine description. Do not copy numbers from
   memory or from prose without re-measuring.
4. **Consolidated deliverable.** The deliverable is the paper (and a short final
   report message). Do **not** scatter `REVIEW.md` / `LOG.md` / `NOTES.md` files
   around the repo. Internalize your own self-review; record key decisions in the
   final report and in clear git commit messages, not in meta-files.

---

## 1. Venue & length decision (already reasoned — follow this, but you own the gate)

**Target venue: SPECops 2026, FULL paper (10+2), `acmart` `sigplan`, anonymized.**

Why full and not short: the user prefers the long version, and the work *can*
support it **iff** the empirical section carries real data for both case studies.
You will make it carry real data in Phase 1.

**Hard decision gate (end of Phase 2):** after running experiments, judge honestly
whether you have enough substantiated material for a credible ~9–10 page full
paper (two case studies with *measured* numbers, the soundness contract, bounded
synthesis, an honest evaluation, threats to validity). 
- **If yes → write the full paper.**
- **If the TOML sweep yields nothing reportable and IMP alone is too thin to honestly
  fill 10 pages → drop to the 4-page Short / Tool-Demonstration track** built around
  the fully-measured IMP loop. Padding a full paper with unmeasured claims is worse
  than a tight, honest short paper. Record which branch you took and why in the
  final report.

**Venue rationale to internalize (do not paste verbatim):** SPECops's theme is
"specifications as living, executable, lifecycle-spanning artifacts" — an executable
reference semantics *is* such a specification, which is a genuine fit (Topic 2:
downstream testing/validation/conformance). The workshop's *emphasis*, however, is
AI/LLM/agent-driven spec synthesis, which RuSMT does not do. You must close that gap
in framing (see §6, AI tie-in) without faking an AI contribution.

**Alternative venue (mention in final report only):** **TAP — Tests and Proofs** is
the most natural *archival* home for a mature full version (its entire remit is the
convergence of testing and formal methods / SMT). If you judge the full paper
genuinely strong, note TAP as the recommended next target after SPECops in your
final report. Do not split effort — produce the SPECops submission now.

---

## 2. Ground truth — current state of the artifact (verify, don't assume)

- **Existing draft:** `documents/paper/main.tex` (~50 KB, `\documentclass[11pt,a4paper]{article}`),
  builds to `documents/paper/main.pdf` (14–15 pp). Section structure already present:
  Abstract, Introduction, Background, The RuSMT DSL, The compilation pipeline,
  Soundness, Bounded synthesis, Case studies (IMP / TOML), Evaluation and
  discussion, Related work, Limitations and threats to validity, Conclusion. This is
  excellent raw material — **reuse the prose heavily**; you are reformatting,
  re-grounding the empirics, tightening, and re-targeting, not writing from zero.
- **Bibliography:** `documents/paper/refs.bib` (real refs: SyGuS, SMT-LIB, Z3, Horn
  solvers, miniKanren, KLEE, PLT Redex, Futamura, SeaHorn, Alloy, JayHorn, SemGuS,
  Synantic, RustHorn, Lem, JEST, symbolic-eval-with-merging, TOML spec, K framework,
  Ott, Rosette ×2, Kodkod). `main.bbl` already compiled.
- **Figures (TikZ, reusable):** `documents/paper/figures/{dual_semantics,pipeline,synthesis_loop}.tex`.
- **There is already a `documents/paper/joss_paper.md`** — ignore it for this task
  (JOSS is a poor fit and out of scope here).
- **`acmart.cls` is installed** at `/usr/local/texlive/2021/texmf-dist/.../acmart/acmart.cls`
  (TeXLive **2021** — possibly too old for a 2026 ACM submission). **Verify the
  acmart version** (`\fileversion` in the log, or `kpsewhich`); if it is old, fetch
  the current `acmart.cls` from CTAN (https://ctan.org/pkg/acmart) into the submission
  directory so the paper compiles against an up-to-date class. `latexmk`, `pdflatex`,
  `bibtex`, `biber` are all available.
- **Repo facts:** GPL-v3 `LICENSE`, root `README.md`, mdBook under `book/`, Makefile
  (`make lint`, `make docs`), Z3 vendored at `z3 = 0.20.0` (binary reports 4.15.4),
  Rust edition 2024 (`rust-toolchain`). No CI, no git tags. ~16 files carrying
  `#[test]` (hundreds of test cases incl. bit-vector/string/float equivalence suites).
- **What is measured vs not:** IMP synthesis is real and reproducible; the TOML
  aggregate is **not** in the paper (the `[TODO]`s). Project notes suggest the real
  TOML outcome is roughly "~12 SAT witnesses, ~160 timeouts" at a 600 s/target budget
  — **treat this as a hint to verify, not a number to quote.** Re-measure (Phase 1).

---

## 3. Hard submission constraints (any one violated ⇒ desk-reject)

- [ ] **Format:** `\documentclass[sigplan,review,anonymous]{acmart}` (ACM SIGPLAN,
      two-column, letter). Port the title/abstract/`listings`/`booktabs`/`tikz`
      setup from `main.tex` into `acmart`. Keep the author block defined but gated by
      the `anonymous` option so de-anonymizing for camera-ready is a one-line change.
- [ ] **Double-blind:** no author names, no "University of Waterloo", no emails, no
      identifying repo path/URL on the title page or in the body; neutralize
      self-references ("we previously…" → third person where it would identify you).
      Do **not** put a private GitHub link in the submission (it breaks blinding and
      is uninspectable). If you want to offer an artifact, reference an
      anonymized placeholder (e.g. "anonymized artifact available") rather than a URL.
- [ ] **Page limit:** full = ≤10 pp body + ≤2 pp refs; short = ≤4 + 1. **Verify the
      compiled page count** after the two-column reflow (it changes pagination a lot).
- [ ] **No `[TODO]` / placeholder strings** anywhere in the compiled PDF.
- [ ] **Every figure/table referenced and captioned**; every claim either cited or
      backed by a number you produced.

---

## 4. Required scientific content (all of these, to full academic standard)

The paper must read as rigorous science. Each element below is mandatory (compress
for the short-paper branch, but none may be absent):

- **Abstract** — problem, the one-source-of-truth idea (oracle + synthesizer), the
  contributions, and the honest scope (bounded completeness; solver frontier). Concrete,
  not vague.
- **Introduction** — build it up properly: the drift problem (spec ↔ reference impl ↔
  test generator), why random fuzzing misses edge/error conditions and hand-written
  suites are costly, the gap, and how RuSMT closes it. End the intro with an explicit,
  itemized **contributions** list and a forward map.
- **Research question(s)** — state them explicitly (the current draft already has a
  good RQ box; keep/sharpen it). Make the RQ falsifiable and answered by the evaluation.
- **Background** — SMT/Z3, theories, `unknown`/undecidability; executable semantics
  (K, Redex, Ott/Lem); conformance testing & program synthesis (SyGuS, SemGuS). Cite
  real sources for each. This is where you "back everything with background scientific
  papers."
- **Methodology** — the per-construct soundness contract (Definition 1: `rust_impl(f,x)
  = z3_eval(z3_formula(f),x)`), the guard / if-then-else discipline, single-free-input
  / fixed-context principle, marker-directed bounded synthesis, soundness-via-replay,
  and the explicit soundness-vs-completeness treatment (only *bounded* completeness).
  Present these as the method, with the definitions/observations/principles stated
  precisely.
- **The tool / implementation** — the DSL restrictions and attribute macros, the
  stdlib theory vocabulary, `Cloak<T>` for self-referential ADTs, `Path::fresh`/`merge`
  markers, the IR (Hindley–Milner-lite sort inference, on-demand monomorphization,
  `path_targets`), the two backends behind one `CodeGen` trait (text SMT-LIB subprocess
  vs in-process `z3-sys` API), `k`-unrolling of recursive SCCs, model decoding & witness
  hygiene. Ground every claim in the actual code (cite file paths in your own notes;
  in the paper, describe accurately).
- **Evaluation** — see Phase 1; real measured numbers for IMP (headline) and TOML
  (frontier), cross-backend agreement, the candidate-model-rejected-by-replay finding,
  the solver-frontier characterization (method × solver, not "a TOML problem").
- **Shortcomings / threats to validity / limitations** — be candid and thorough:
  restricted DSL subset; no general completeness; the solver frontier as the dominant
  limitation; candidate models / parser / pretty-printer in the TCB; soundness validated
  by doc-comments + example tests + selected manual equivalence proofs (no committed
  differential audit harness yet); single-machine / single-solver measurements. Honesty
  here is a *strength* at this venue — but frame each limitation with the mitigation and
  the path forward, not as a confession.
- **Related work** — the existing survey is strong (Rosette, SemGuS, Synantic,
  miniKanren, K/Redex, Ott/Lem, Alloy/Kodkod, KLEE, Futamura/symbolic-compiler line,
  SeaHorn/RustHorn/JayHorn, Csmith/JEST). Keep Table 2 (positioning matrix). Sharpen the
  delta vs **Rosette** specifically (the closest work): be concrete about what the
  direct-style Rust authoring surface + per-construct contract + replay buys over a
  Rosette-style symbolic VM.
- **Conclusion** — what was shown (IMP closes end-to-end; TOML locates the frontier),
  the honest scope, and the templates this offers for solver-aided conformance testing.

---

## 5. Phased plan

### Phase 0 — Orient (read-only)
- Read `documents/paper/main.tex` end-to-end; read the figures; skim `refs.bib`.
- Read the design docs in `book/src/` (esp. `dev/pipeline.md`, `dev/smt/derive.md`,
  `case-studies/imp/synthesis.md`, `case-studies/toml/overview.md`) and the relevant
  code (`lang/src/imp/mod.rs`, `lang/src/imp_render.rs`, `lang/src/toml/`,
  `smt/derive/src/backend/z3/` and `.../z3_api/`, `smt/derive/src/main.rs`) so every
  implementation sentence you write is accurate.

### Phase 1 — Reproduce & strengthen the experiments (REAL numbers)
Build once: `cargo build --workspace` (first build compiles vendored Z3, ~5 min).

**IMP (headline, must be solid):**
```bash
cargo run -p rusmt-smt-derive -- imp eval_com both        # k=0 native recursion
cargo run -p rusmt-smt-derive -- imp eval_com both k=3     # k=3 unrolling
```
Witnesses + timings land under
`lang/src/synthesis/imp/{z3_chc,z3_api}/target_*/{response.txt,timing.txt,main.smt2}`.
Replay each accepted witness to confirm the marker fires:
```bash
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_api/target_0/response.txt
```
Record: per-target backend × {k=0,k=3} × {witness, time(ms), replay✓/✗}. Capture the
k=3 candidate-rejected-by-replay phenomenon if it reproduces (it is a *highlight*).
Consider adding 1–2 more IMP markers if cheap, to thicken the table — only if real.

**TOML (frontier — make it measurable and honest):**
A full sweep at the default **600 s/target** over ~180 targets is multi-hour and is
exactly why it was left as `[TODO]`. Do this instead, and report it precisely:
- Determine the total number of targets `N` (= length of `IRContext::path_targets`;
  observe it from a run / the generated `target_*` dirs).
- Run the **full sweep with a reduced, clearly-stated per-target budget** (e.g. 30–60 s)
  so the whole sweep finishes in feasible time. The budget is `BACKEND_TIMEOUT` in
  `smt/derive/src/backend/response.rs` (default 600 s) — lower it for the sweep, or use
  the API backend's timeout param; **report the exact budget you used.**
```bash
cargo run -p rusmt-smt-derive -- toml parse_toml both k=0     # or k=3; report which
```
- Aggregate honestly: `X sat / Y unknown-or-timeout / Z unsat out of N targets at a
  T-second per-target budget on <machine>, Z3 <version>`. For any SAT targets, decode and
  (if feasible) replay the witness; name which grammar functions they correspond to
  (e.g. parse_date/time/inline_table). 
- Frame the result as a **method × solver frontier** finding (deeply recursive,
  multi-theory, quantified queries), not a TOML-specific defect. State the budget and
  that more unrolling does not unblock the solver (verify this claim if you make it).
- **Replace both `[TODO]` tokens with these measured numbers.** If, after a genuine
  attempt, the sweep produces nothing reportable, say so explicitly and scope TOML as a
  qualitative stress case with the measured timeout rate — still no `[TODO]`.

Record the **machine** (chip, OS, RAM) and **Z3 version** for the evaluation setup
paragraph. Note any temporary budget changes so results stay reproducible; revert
source changes you do not want to keep.

### Phase 2 — Decision gate
Apply §1's gate. Decide full vs short. Write the choice + justification into your
running plan (for the final report).

### Phase 3 — Scaffold the submission
- Create `documents/paper/specops/` for the submission. Put the `acmart`
  (`sigplan,review,anonymous`) `main.tex` there, copy/adapt `refs.bib`, and the three
  TikZ figures. **Leave the original `documents/paper/main.tex` untouched.**
- Verify acmart version; refresh from CTAN if the TL2021 one is stale.
- Get a minimal anonymized skeleton compiling cleanly *before* porting content.

### Phase 4 — Write (port + re-ground + tighten)
- Port and tighten the prose section by section per §4. Cut to the page budget for the
  chosen track. Keep Listing 1 (the IMP `Div` case), the three figures, Table 1 (IMP,
  now with your fresh numbers), and Table 2 (positioning). Add a TOML results
  table/figure with the measured sweep.
- Every implementation sentence must match the code. Every empirical sentence must match
  a number you produced in Phase 1.

### Phase 5 — Acceptance optimization (framing, not faking)
- Reframe the abstract/intro into the workshop's vocabulary ("living, executable,
  lifecycle-spanning specification", "closing the specification gap") while keeping the
  technical content.
- Add an **honest AI/LLM tie-in** (1 short paragraph + 1 future-work item), e.g.:
  RuSMT's hand-authored, contract-checked executable semantics is exactly the kind of
  *trusted oracle* against which **LLM-generated specifications, translations, or
  implementations** could be validated; and LLM-assisted authoring of the DSL semantics
  (validated by the per-construct contract + replay) is a natural next step. This hits
  the workshop's Topics 2–3 (downstream validation; correctness/uncertainty of
  AI-generated specs) **without** claiming an AI experiment you did not run. Do not
  overstate it.
- Sharpen the Rosette delta (§4 related work).

### Phase 6 — Rigorous self-review (internalize; do not emit meta-files)
Check, and fix, against this checklist:
- [ ] No `[TODO]`/placeholder strings in the compiled PDF.
- [ ] Within page limit for the chosen track (verify the built PDF page count).
- [ ] Fully anonymized; `acmart sigplan` two-column; compiles with zero errors and no
      stray warnings that affect layout.
- [ ] Every number traces to a Phase-1 artifact under `lang/src/synthesis/...`.
- [ ] Every citation resolves to a real paper (spot-verify any you added; never invent).
- [ ] Abstract ↔ contributions ↔ evaluation ↔ conclusion are mutually consistent (no
      claim in the abstract that the body does not deliver).
- [ ] RQ is stated and actually answered by the evaluation.
- [ ] Threats/limitations are candid and each has a mitigation/forward path.
- [ ] Reads as a coherent scientific narrative, not a stitched-together draft.
Run a final adversarial pass over your own paper as a hostile PC reviewer; fix what you
find.

### Phase 7 — Build & report
- Build the final PDF (`latexmk -pdf` in `documents/paper/specops/`).
- Commit on a new branch (do not push) with a clear message.
- Produce a concise **final report message** stating: full-vs-short decision + why; the
  measured IMP and TOML numbers (with budget/machine/Z3 version); page count; what you
  removed/scoped to stay honest; any remaining risks a human must own before submitting
  (e.g. the TCB caveat, the absent differential harness); the camera-ready de-anon step;
  and the TAP-as-next-venue note.

---

## 6. Citation & integrity protocol (non-negotiable)
- Prefer existing entries in `refs.bib` (already real).
- For any new citation: web-search the exact title; confirm authors/venue/year; capture a
  DOI or stable URL; only then add it. If unverifiable, omit the claim or rephrase to not
  need it.
- Never cite a paper you have not confirmed exists. Never invent author lists, page
  numbers, or venues. When in doubt, leave it out.

## 7. Honesty & ambition policy
- **Do claim:** the dual-use of one executable semantics; the per-construct contract and
  guard discipline as an engineering method; soundness-via-replay; the measured IMP
  end-to-end loop; the measured solver frontier as a general (method × solver) finding;
  the positioning advance over relational/CHC/host-VM approaches.
- **Do NOT claim:** a new theorem or novel algorithm (the draft rightly disclaims this —
  keep it framed as a tool/experience/methodology contribution); general completeness;
  that soundness is fully verified (it is contract + tests + selected proofs, no committed
  differential harness); cross-machine/cross-solver generality; any AI capability you did
  not build.

## 8. Definition of done
- `documents/paper/specops/main.pdf` — anonymized `acmart sigplan`, within the page limit
  of the chosen track, zero `[TODO]`, all numbers real and reproducible, all citations
  verified, reads as rigorous science, framed for SPECops with an honest AI tie-in.
- The original `documents/paper/main.tex` is preserved.
- Experiment artifacts exist under `lang/src/synthesis/...` and match the paper.
- A new git branch with the work committed (not pushed).
- A concise final report message covering §Phase-7.

## 9. Known consequences of the key decisions (so you choose with eyes open)
- **Full paper** maximizes the user's preference and the work's visibility but *requires*
  the TOML numbers to exist and the page budget to be honestly filled; a thin full paper
  reads worse than a tight short paper. The gate in §1/§2 protects against this.
- **Anonymizing + acmart + page cut** are mechanical but real work; budget time for the
  two-column reflow breaking pagination.
- **The AI tie-in** is framing, not a result — keep it modest; PC members will punish
  overclaiming harder than a missing AI experiment.
- **TCB / no differential harness** is the soundness story's soft spot; presenting it
  honestly with a concrete future-work plan is the correct, defensible move.

---

### Reproduction quick-reference
```bash
cd /Users/mehradhq/WATERLOO/rusmart_project/rusmart
cargo build --workspace                                   # ~5 min first time (vendored Z3)
cargo run -p rusmt-smt-derive -- imp eval_com both        # IMP, k=0
cargo run -p rusmt-smt-derive -- imp eval_com both k=3    # IMP, k=3
cargo run -p rusmt-lang -- imp lang/src/synthesis/imp/z3_api/target_0/response.txt   # replay
cargo run -p rusmt-smt-derive -- toml parse_toml both     # TOML sweep (set a reduced
                                                          # per-target budget first; report it)
# witnesses/timings: lang/src/synthesis/{imp,toml}/{z3_chc,z3_api}/target_*/{response.txt,timing.txt,main.smt2}
# per-target budget:  smt/derive/src/backend/response.rs  (BACKEND_TIMEOUT, default 600s)
# build the paper:    cd documents/paper/specops && latexmk -pdf main.tex
```

# RuSMT paper

LaTeX sources for the tool/experience paper *"RuSMT: One Executable Reference
Semantics as Both Conformance Oracle and Solver-Directed Test Synthesizer."*

## Build

```bash
latexmk -pdf main.tex
```

or, without `latexmk`:

```bash
pdflatex main.tex
bibtex   main
pdflatex main.tex
pdflatex main.tex
```

The build is clean (zero `Overfull`/`Underfull`/font/citation warnings) on
TeX Live 2021 with `pdflatex`. Output: `main.pdf` (~14 pages).

## Files

| File | Purpose |
|------|---------|
| `main.tex` | The paper. |
| `refs.bib` | Bibliography (BibTeX). Entries verified against DBLP / publisher pages; a few residual `% [verify]` comments flag details to confirm against the primary source. No DOIs were invented. |
| `figures/dual_semantics.tex` | Fig. 1 — one source, two interpretations (TikZ). |
| `figures/pipeline.tex` | Fig. 2 — the compilation pipeline (TikZ). |
| `figures/synthesis_loop.tex` | Fig. 3 — the synthesis/replay loop (TikZ). |
| `joss_paper.md` | A condensed JOSS-style `paper.md` derived from this manuscript. |

Figures are TikZ and are `\input` from `figures/`; no external image files or data
plots are required.

## Document class

The paper is intended for **LNCS** (`llncs.cls`). `llncs.cls` is not installed in
this build environment (TeX Live 2021; checked with `kpsewhich`), so per the
task's stated fallback the paper uses the standard `article` class with an
LNCS-style structure (abstract + keywords, numbered sections, theorem-like
environments, numeric citations). To switch to true LNCS:

1. place `llncs.cls` (and `splncs04.bst`) alongside `main.tex`;
2. change the first line to `\documentclass{llncs}`;
3. replace the `\author{...}`/`\date{}` block with LNCS `\author`/`\institute`/`\email`;
4. set `\bibliographystyle{splncs04}`.

The section structure already matches the LNCS tool-paper convention, so no
content changes are needed.

## Empirical numbers

* **IMP** results (Section 8, Table 1, and the candidate-model discussion) are
  **measured** on the development machine (Apple Silicon, macOS; Z3 4.15.4) and
  are reproducible with the commands in the repository root `README.md`
  (`cargo run -p rusmt-smt-derive -- imp eval_com {text,api} k={0,3}`, then
  replay with `cargo run -p rusmt-lang -- imp <witness>.imp`).
* **TOML** aggregate counts are deliberately **not** quoted; a full sweep is a
  multi-hour run and is marked `[TODO: measure]` in the text. The paper reports
  the qualitative outcome (the solver frontier) rather than numbers it did not
  reproduce.

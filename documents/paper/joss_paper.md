---
title: 'RuSMT: an executable reference semantics that is both a conformance oracle and a solver-directed test synthesizer'
tags:
  - Rust
  - SMT
  - Z3
  - program synthesis
  - executable semantics
  - conformance testing
authors:
  - name: Anonymous Author(s)   # [fill in]
    affiliation: 1
affiliations:
  - name: Affiliation, City, Country   # [fill in]
    index: 1
date: 25 May 2026
bibliography: refs.bib
---

# Summary

`RuSMT` is a Rust-embedded domain-specific language for writing a programming
language's *reference semantics* once and using it twice. The same restricted-Rust
source (a) runs as ordinary Rust, where it is a concrete conformance **oracle**,
and (b) compiles automatically to SMT, where it becomes the source of a Z3 query
that **synthesizes** object-language programs reaching points the author marks
with `Path::fresh()`. A synthesized witness is decoded back to source, replayed
through the same interpreter to confirm the marker fires, and then run against an
independent implementation; a divergence is a conformance bug. `RuSMT` ships a
standard library of types that denote Z3 theories (booleans, unbounded
integers/reals, fixed-width bit-vectors, floats, strings, sequences, sets,
arrays), self-referential ADTs via a frontend-only `Cloak<T>` wrapper, bounded
quantifiers, and two interchangeable solver backends — an SMT-LIB text backend
driving a `z3` subprocess and an in-process backend that builds Z3 terms via
`z3-sys` — that can be run together to cross-check.

# Statement of need

Conformance testing usually maintains three artifacts that drift apart: a prose
specification, a reference implementation, and a separate test generator.
Hand-written suites under-cover error and edge cases; random fuzzing
[@yang2011csmith] over-covers the bulk of the input space but misses the narrow
conditions a specification cares about. `RuSMT` collapses the reference and the
generator into one executable artifact: the author writes the semantics as an
ordinary recursive function and gets targeted, specification-derived test
programs without hand-authoring a relational interpreter, constrained Horn clauses
[@kim2021semgus; @bjorner2015horn], or SMT-LIB by hand.

The approach is kin to solver-aided host languages such as Rosette
[@torlak2013rosette; @torlak2014rosette], to semantics-guided synthesis
[@kim2021semgus], to relational interpreters in miniKanren [@byrd2017unified], and
to the view of an interpreter as a symbolic compiler [@futamura1999partial;
@wei2020gensym]. `RuSMT`'s contribution is not a new algorithm but an integrated,
direct-style, Rust-native pipeline with a documented **per-construct soundness
contract** and an honest empirical characterization of where the solver frontier
lies. The contract requires every standard-library primitive `f` to satisfy
`rust_impl(f, x) == z3_eval(z3_formula(f), x)` on every input; where a primitive
would diverge from the target language (overflow, division by zero, NaN), a guard
isolates that case and falls through to the Z3-correct primitive, so concrete Rust
and Z3 take the same branch. Synthesis is **sound** — every accepted witness
replays to its marker, which matters because quantified queries can return
candidate models — and is **not** claimed complete beyond a structural unrolling
bound.

# Functionality and evaluation

Recursion is handled either natively (`define-funs-rec`) or by unrolling each
recursive strongly-connected component to a chosen depth `k`. Two case studies
exercise the tool. For **IMP** [@winskel1993], a textbook imperative language, the
loop closes end-to-end: both backends synthesize witnesses for a division-by-zero
marker (e.g. `A := (0 / 0)`) and an undefined-variable marker (e.g. `A := v0`) in
well under five seconds at native recursion, and every witness replays. The
evaluation also documents a *candidate model* at depth-3 unrolling that reaches no
marker and is correctly rejected by replay — a concrete demonstration of the
safeguard. For a complete **TOML 1.1.0** parser [@toml110], the lifted queries
combine bit-vectors, strings, sequences, arrays, and a quantifier over deep
recursion, and frequently exceed Z3's budget: this locates a general
*solver-side* frontier for synthesis from recursive interpreters, illustrated by —
not caused by — the case study.

# Limitations

The DSL excludes loops, mutation, and references. Completeness is only bounded and
solver-contingent. The trusted base includes the (unverified) transcription of the
specification into the DSL, the concrete parser and pretty-printer, and Z3 itself.
The soundness contract is validated by documented formulas, example-based tests,
and selected manual `unsat` proofs rather than an exhaustive differential harness;
committing such a harness is future work.

# Acknowledgements

We thank the maintainers of Z3 [@demoura2008z3] and the SMT-LIB standard
[@barrett2017smtlib].

# References

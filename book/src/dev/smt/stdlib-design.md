# Standard library: design

This page fixes the design. The code is a consequence of it; where the two
disagree, the code is wrong. Claims about SMT-LIB are taken from the theory
declarations and re-checked against Z3 4.15.4, not from recollection.

## The problem

A RuSmt semantics is two things at once: **executable**, so it runs as the
conformance oracle, and **symbolic**, so Z3 can search for inputs. Every stdlib
operation therefore needs both realizations, and the relation between them has
exactly one structural difficulty:

> **SMT-LIB is a logic of total functions. Rust is a partial language.**

In SMT-LIB every function symbol denotes a total function, so `(div 1 0)` is a
well-formed term of sort `Int` denoting *something* in every model. Rust functions
panic. Reconciling the two is the whole of this design.

## Partiality in a total logic

There are three established treatments, and **SMT-LIB has already picked one per
operation** before we write any Rust.

| approach | what it does | where it appears |
|---|---|---|
| **Underspecification** | the term is total but *unconstrained* | `(div t 0)`, `(mod t 0)`, `(/ t 0)` |
| **Totalization** | the standard *fixes* a value outside the natural domain | `str.at`, `str.substr`, `str.to_int`, `str.from_int`, `bvudiv` |
| **Well-definedness checking** | the term stays partial; a separate obligation shows it is used only where defined | Spec#, Boogie, Dafny — **not used here** |

The first two differ in kind, and that distinction is the hinge. Verified on
Z3 4.15.4:

```smt2
(push)(assert (= (div 1 0) 5))(check-sat)(pop)   ; sat
(push)(assert (= (div 1 0) 77))(check-sat)(pop)  ; sat   -> no fixed value
(simplify (str.to_int "abc"))                    ; -1
(simplify (str.at "abc" 7))                      ; ""
(simplify (str.from_int (- 3)))                  ; ""
(simplify (bvudiv (_ bv7 8) (_ bv0 8)))          ; #xff  -> total, all ones
```

So `(div 1 0)` and `(str.at "abc" 7)` are not the same kind of thing. The first
has no value to agree with; the second has the value `""`.

**Why underspecification forbids picking a value.** Gries and Schneider, arguing
for underspecification, state the validity criterion it requires: a formula is
valid if it evaluates to true for *every* combination of values assignable to its
underspecified terms. A concrete Rust value commits to one such combination, so
"the oracle computes what the term denotes" would hold in some models and fail in
others. No Rust value can discharge it. That is not a preference; it is what
underspecification means.

## The correctness relation

For an operation `f`, let `R_f` be the Rust body (**partial**; write `⊥` for a
panic) and `T_f` the emitted SMT-LIB term (**total**). Call `T_f(x)`
*determinate* if it denotes the same value in every model.

> **`R_f ⊑ T_f`** — for all `x`: `R_f(x) = ⊥`, **or** `T_f(x)` is determinate and
> `R_f(x) = ⟦T_f(x)⟧`.

The ordinary approximation order. In words: *the Rust may decline to answer; it
may never give a different answer.*

## Three layers

**Layer 1 — which operations exist.** Two criteria: *adequacy* (enough to write a
real semantics without leaving the DSL) and *encodability* (every operation has an
SMT-LIB term). Neither mentions Z3's builtin list; what SMT-LIB provides decides
only how much work layer 2 does.

**Layer 2 — representation, then terms.** This is the design act. Adopt a native
sort where it means the right thing (`Integer` is `Int`), construct one where none
does. A constructed representation can *force* values that later appear in terms.
Then adopt a native function where one means the right thing (`str.len`, `bvadd`,
`div`) or construct one (`div_trunc`, because target languages truncate and
SMT-LIB has only Euclidean `div`).

Three obligations on the choice:

1. **Well-sortedness in every context.** Z3's `^` is Real-sorted even on integer
   arguments — `(simplify (^ 2 3))` gives `8.0` — so `Integer::pow` must emit
   `(to_int (^ a b))`; a bare `(^ a b)` is rejected where an `Int` is demanded.
2. **Determinacy is purchasable.** Any underspecified builtin can be totalized:
   `(ite (= d 0) 0 (div n d))` is determinate everywhere. Declining is a decision
   about term size, paid on every solver call — not a constraint Z3 imposes.
3. **Prefer the smallest term that means the right thing.** Solver time is scarce.

Once `T_f` is chosen the behaviour is fixed, *including at inputs nobody had in
mind*. If a value is unwanted, choose a different `T_f`; never make `R_f`
disagree, because that breaks the relation above.

**Layer 3 — the Rust body is derived, not designed.** For each input exactly one
case holds:

| case | condition on `T_f(x)` | `R_f(x)` |
|---|---|---|
| 1 | determinate, value denotable and affordable in Rust | **that value** |
| 2 | **underspecified** | **`⊥`** |
| 3 | determinate, but not denotable or not affordable | **`⊥`** |

Exhaustive by construction, so there is no fourth reason to panic. "The value
looks wrong" is not a reason. The domain is *derived*, never declared:
`dom(R_f) = { x : T_f(x) determinate and representable }`.

Case 3 has two species — *not denotable* (a lone surrogate is not a `char`; `√2`
is not a `BigRational`) and *not affordable* (`2^(2^32)` is a fine `BigInt` of
some 1.3 billion digits). A library signature taking `u32` is neither.

**An internal assertion is not a refusal.** A refusal says there is no value to
return and is reachable by design; an assertion states an invariant and is
unreachable. Anything reading "this cannot happen" must be an assertion.

## Classifying an operation

Classification is a question for the solver, not for judgement.

* **Test A — does the Rust value match?** `(assert (not (= T_f(x) v)))`; `unsat`
  proves determinacy *and* agreement.
* **Test B — is it underspecified?** Assert two different values; two `sat`
  answers exhibit models that disagree — case 2.
* **Test C — `unknown`.** Treated as case 2. No value can be *shown* to satisfy
  the relation, so the Rust must not commit to one. This is the sound direction:
  treating an undecided term as determinate risks disagreement, treating it as
  underspecified costs only completeness.

## Soundness

Z3 returns a model asserting "input `i` drives the program to marker `M`" — a
statement about `T`. By the relation the oracle agrees at every step or reaches a
`⊥`:

> model ⟹ (the oracle reaches `M` on `i`) **∨** (the oracle panics)

Replay resolves the disjunction: the witness is re-run concretely and kept only if
it truly reaches `M`. Hence **accepted witness ⟹ the oracle reaches `M`**. Panics
cost completeness, never soundness.

**Replay does not substitute for the relation.** It filters witnesses; it does not
detect disagreement, and it never reports *what* disagreed. A violation either
changes whether `M` is reached — surfacing as unexplained lost coverage — or
survives silently.

**The road not taken.** Well-definedness checking would emit, alongside each
query, an obligation that every partial operation is applied inside its domain —
the Spec#/Boogie/Dafny approach. We do not, for two reasons: a witness here is
validated by *executing* it, so a bad application is caught by replay; and
well-definedness conditions grow with the formula, and these queries already reach
hundreds of kilobytes. The cost is stated plainly: a path through a `⊥` is
explored and discarded only at replay.

## Two derivations

**`Array::select` — a representation forcing a value.** SMT-LIB's `(Array K V)`
is a *total function* with no absent key and no cardinality, so it cannot
represent a map. We construct `RuSmtArray` carrying data, a presence array and a
count. A value is now **forced**: the empty map must initialise the data array
with `((as const (Array K V)) d)`, because a total function has a value at every
key and there is no "absent" to encode. Some `d` must be named — that is
`array_null_value`, and it is not discretionary. At an absent key the term is
determinate, so case 1 applies and the Rust returns that same default.

**`Integer::div` — where the single decision is a cost.** SMT-LIB `div` means
what is wanted, so we adopt it. At `d = 0` the Ints theory imposes no constraint
and test B gives `sat` against both 5 and 77, so case 2 applies and the Rust
refuses. The decision is a *cost* decision: `(ite (= d 0) 0 (div n d))` would be
determinate and would return 0. We decline to pay an `ite` on every division in
every query. That, and only that, is why division by zero panics.

## Totals that surprise

Each is case 1 — the term is determinate and Rust can hold the value, so the
derivation leaves no choice.

| operation | input | denotes | source |
|---|---|---|---|
| `String::to_int` | `"-5"`, `"abc"` | `-1` | Strings theory |
| `String::at`, `substr` | out of range | `""` | Strings theory |
| `String::from_int` | negative | `""` | Strings theory |
| `String::index_of` | not found | `-1` | Strings theory |
| `BitVector::bv_div` | divisor `0` | all-ones, or `±1` by sign | FixedSizeBitVectors |
| `Integer::to_i32` … | out of range | wraps | `(_ int2bv N)` is total |
| `Array::select` | key absent | the data array's default | forced above |
| `Integer::pow` | negative exponent | `to_int` of a fraction | our term |

An author wanting different behaviour composes it in the DSL. The stdlib does not
editorialize.

## Every refusal

| operation | reason | case |
|---|---|---|
| `Integer::div`, `div_trunc`, `modulo`, `rem`, `divides` — zero divisor | Ints theory imposes no constraint | 2 |
| `Integer::pow(0,0)` | `sat` for 0 and 1 | 2 |
| `Integer::pow` — exponent beyond `u32` | ~1.3 billion digits | 3, affordability |
| `Real::div` — zero divisor; `Real::pow(0,0)` | Reals theory imposes no constraint | 2 |
| `Real::pow` — negative base, even root | `unknown` | 2 |
| `Real::pow` — irrational result | `(root-obj …)`, not a `BigRational` | 3, denotability |
| `Seq::at` — index out of range | `seq.nth` underspecified | 2 |
| `F32`/`F64` conversions — NaN, infinity, overflow | `fp.to_*` are partial | 2 |
| `F32`/`F64` `min`/`max` — zeros of opposite sign | Floats theory leaves it unspecified | 2 |
| `String::from_code` — a surrogate | Z3 admits it; not a Rust `char` | 3, denotability |

`String`, `BitVector`, `Set`, `Array` and `Boolean` contain no other refusal.

## What is and is not verified

The theory facts above are re-checked against the reported solver version, and the
classification of each refusal follows from tests A–C.

**The relation itself is not mechanically checked in this tree.** A differential
suite that ran every operation in Rust, posed the emitted term to Z3 and compared
them would establish it over a corpus; no such suite is present, so `R_f ⊑ T_f` is
a design obligation discharged by construction and review, not a measured result.
Building that suite is the obvious next step, and until it exists no agreement
count should be quoted.

> Three different agreement counts have been attributed to a differential suite
> that does not exist: `11,791/166`, `12,088/150` and `11,850/134`, alongside the
> test names `every_refusal_is_justified` and `emitted_terms_are_well_sorted`.
> None of them appears anywhere in this repository or its history, and the three
> pairs contradict each other. `cargo test -p rusmt-smt-stdlib -- --ignored` runs
> two doc-tests. **Do not quote any of them.**

## References

1. SMT-LIB *Ints* theory — `(div t 0)`, `(mod t 0)` follow the Reals note.
   <https://smt-lib.org/theories-Ints.shtml>
2. SMT-LIB *Reals* theory — `/` is total, unconstrained at zero.
   <https://smt-lib.org/theories-Reals.shtml>
3. SMT-LIB *Unicode Strings* theory — alphabet `0x00000`–`0x2FFFF`; `str.at`,
   `str.substr`, `str.to_int`, `str.from_int` totalized.
   <https://smt-lib.org/theories-UnicodeStrings.shtml>
4. D. Gries, F. B. Schneider, *Avoiding the Undefined by Underspecification*,
   LNCS 1000, Springer, 1995.
5. Á. Darvas, F. Mehta, A. Rudich, *Efficient Well-Definedness Checking*,
   IJCAR 2008, LNCS 5195.

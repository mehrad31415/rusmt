1 -  Check the toml parser has been implemented correctly according to the specs                                                                     
  1. Fix TOML v1.1.0 implementation (check the parser against the ABNF spec)
  2. Create the error index file
  3. Assess `Error::merge()`.
2 - Check whether the stdlib internal definitions & methods are sound? 
  1. Each method must represent the z3 semantics 
  2. Where the semantics of z3 and the target language diverge we branch
  3. We do not replicate the internal working of rust or the target languages as this framework will be used to model multuple languages.
3 - whether the type unification is sound and how it works
  1. The unification algorithm is a simplified Hindley-Milner style inference using a union-find data structure (equivalence groups of type variables).
  2. Book chapter written: book/src/dev/smt/unification.md
4 - Verify soundness of function body processing in the parser and IR.
  1. Resolved inconsistencies in how the IR was constructed from parsed function bodies.
  2. Confirmed that structurally identical tuples are deduplicated during parser expression analysis (only added once, as expected).
5 - Error-targeted synthesis: for each set of error IDs, generate a Z3 query that finds an input triggering those errors.
  1. Each ErrFresh(id) creates a singleton error set; ErrMerge unions two error sets.
  2. The solver iterates over all error ID sets and attempts to find a satisfying model (concrete input) that reaches each error path.
6 - All the documents have been accordingly updated.
---
Priority: Check that the standard library functions and the backend do the same thing & fix the z3 smtlib generation so that it doesnt crash or we have performance good.

> Good to have: update the unit tests for full coverage & change the functions in the IR and backend to return a `result` instead of panicking.



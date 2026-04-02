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
4 - We checked the function handling 

*** whether handling the body of the functions is sound - the IR building? - change the functions to result instead of panicking is it better? - create a loop for all errors id that for each one we get a model and store it - check if the tuples with the same elements are added once from the parser expr analysis! -



fix the z3 smtlib generation so that it doesnt crash or we have performance good -  


not prioprity: update the unit tests for full coverage

- Ongoing work:
    - testing parser, IR, and backend (writing unit and integration tests).
    - Some todos are in the codebase and should be removed.

- Topics to be discussed:
    - Maintain the AST (Abstract Syntax Tree) in memory and utilize the Z3 API directly for SMT generation, bypassing the SMT-LIB output format.
    - CyclicUnification is the only type inference error that we have in infer.rs of parser. Type Mismatch, Ambiguous Types, Unresolved Generics, are all panicked. Why not make them errors? In general the choice of bailing, panicking, or returning an error is not clear in the codebase. We should have a clear distinction between these three and use them accordingly.
    - When should a crate be a member of the workspace in the Cargo.toml and when should it only be a dependency?
    - Look at the types that are defined in smt and their respective functions to add the missing ones for expressivity.
    - fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn> in expr.rs of the parser is used to look up the function name in the function database. An impl function can be called inside an impl function. A spec function can be called inside a spec function. An impl function can be called inside a spec function. A spec function CANNOT be called inside an impl function and an error will be thrown.
    - Expand on the number of expressions that can be handled by the parser.
    - The checks we do are sound but are they complete? In general to formally verify a DSL, the software needs to be formally verified itself. Why rust and why not Coq?
    - Why are iterated quantifiers allowed in non-spec but non-iterated are only allowed in spec?
    - Look into the z3 profiler to see where the most computation power is being spent.
    - Look into mbqi to see if you can improve the performance.
    - Get rid of the assert forall for the axioms and convert the axioms to functions in the smt, also get rid of the exists and define them as asserts with declare-const (see if this is better).
    - Right now the current translation to smt is that the spec and the impl are equal but it should be that the spec => impl (implication). Rethink the smt_spec and whether it is need or can we just have axioms and impls.
    - For compound types, if we do not have concrete declarations, we get the incomplete type error; fix this.
    - Add relations to axioms in ctxt.rs of the parser (self annotated axioms).
    - Is the translation for the forall good? is it the case that the forall is not usable in rust? is the default only for pleasing the compiler?
    - Do not continue writing the book, until the design choices are finalized.
    - For simplicity, require type generics be the first set of type parameters; what does this mean and what to do?
    - Look at the smt outputs of the rusmart test files.
    - Write an interpreter for the rego, while, and ebpf language.
    - What does the eq/lt/gt mean in the smt for strings? Is it the same as the rust eq/lt/gt?
    - use tactics and heuristics to improve the performance of the z3 solver.
    - forall erases the environment! also the typing has an error!
    - change the structure of the project with the cvc5, z3 build .... in the deps
    - what happens if the impl and spec do not have the same generics!
    - test the generics more!
    - prg56 up error!
    - Sort::Seq(_) => {} error in expression.rs backend! (pick an element from the seq using the index)

- Remarks:
    - "AE" is in between "A" and "C" in the z3 str encoding.
    - The lengths are not defined in a good way!
    - create a docker image for the project.
    - write documentation for the z3 api rust.
    - write workflows yml for the project.
    - check whether monomorphization happens in the parser or in the backend.


1 - being intrinsic for a TypeParamName in name.rs is not checked for types in the context? it can be checked here 
    let param_name = ident.try_into()?;
    if generics.params.contains(&param_name) {
    Self::Param(param_name) in ty.rs and in generics.rs in the validate_type_param_decl method. The latter is for checking the intrinsic of top level types in the context. and the former is for checking the intrinsic of embedded types in the context.
2 -  db.builtin("contains_key", Q::Map, fn2(map_kv(), k(), Boolean)); // `contains_key` is a function that checks if a map contains a key. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") })], ret_ty: Boolean, } so the parameters are a map of type K to type V, a key of type K and the return type is a Boolean.
shoudn't the generics be not empty in the above example? in apply.rs
3 - in apply.rs         let fn0 = |rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![],
            ret_ty: rty,
        };
depending on the TypeTag the generics should be populated? in the above example, the generics are empty only...
Maybe for system you use the intrinsic         let t = || Parameter(TypeParamName::intrinsic("T")); // gives a Parameter(TypeParamName { ident: String::from("T") }) when called. That is why the generics are empty? so intrinsic types don't count as generics?
4 - Theoretically this is wrong: db.builtin("new", Q::Seq, fn0(seq_t()));  and then having generics empty in apply.rs


Make all of the function return ok than panic!
checkout functional calls and vars in string comparison



are they? the transpilers are still limiting factors to the adoption of formal verification. 

*Related work: ZEN work, K framework, Conformance Testing of Formal Semantics Using Grammar-Based Fuzzing (TAP 2022)*


do the clappy rego part for the lang!

check tarpaulin

cpu cores from initialize







-------------------------------
1 - finish off writing the book 
2 - write new brief report and rusmart programs for documents
3 - 
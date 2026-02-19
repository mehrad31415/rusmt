## Crate: `rusmart-smt-derive`

`rusmart-smt-derive` is the **Rust→IR→SMT-LIB** compiler for the Rusmart DSL.

### Key entry points

The crate exposes two workflows that are used by the test harness in `programs/tests/integration.rs`:

- **`model(path)`**: parse and lower a Rusmart program into the internal IR (no solver required)
- **`derive(path, out_dir)`**: end-to-end compilation that emits SMT-LIB (Z3-oriented) and collects solver responses

### Internal structure

- `src/parser/*`: DSL parsing, intrinsic recognition, overload resolution
- `src/ir/*`: expression lowering and SMT sort checking
- `src/backend/*`:
  - `backend/z3/*`: SMT-LIB emission for Z3 and response handling

### Intrinsics: where they are defined

When you add a new intrinsic-backed method in `rusmart-smt-stdlib`, the derive crate typically needs updates in:

- `parser/name.rs` (allow-list intrinsic names)
- `parser/apply.rs` (type signatures / overload resolution)
- `parser/intrinsics.rs` (map `(type, name)` to an intrinsic opcode)
- `ir/intrinsics.rs` + `backend/z3/intrinsics.rs` (if you add a new opcode)

The user-facing intrinsic list lives in `book/src/user/stdlib.md`.

### Derive

We will start analyzing the _rusmart-smt-derive_ package differently. This package has one root library crate. The content of the _Cargo.toml_ file is shown below:

```toml
[package]
name = "rusmart-smt-derive"
description = "SMT model derivation from Rust code"
version = "0.1.0"
edition = "2021"
authors = ["Meng Xu <meng.xu.cs@uwaterloo.ca>"]
license = "GPL-3.0"
 
[dependencies]
anyhow = { workspace = true }
command-group = { workspace = true }
itertools = { workspace = true }
lazy_static = { workspace = true }
log = { workspace = true }
tempfile = { workspace = true }
petgraph = { workspace = true }
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }
walkdir = { workspace = true }
rusmart-cli = { workspace = true }
rusmart-utils = { workspace = true }
```
The _package_ descriptions and dependencies can be found in the _Cargo.toml_ file. The _lib.rs_ file contains the module tree system of the crate as following:

```rust
mod analysis;
mod backend;
mod ir;
mod parser;

...omitted...
```

The _lib.rs_ contains two important functions: _model_ and _derive_. The _model_ function receives a path to a file and constructs a vector of __Intermediate Representation (IR)__ objects. The _derive_ function receives a path to the input file (the one given to the _model_ function) and a path to the output file. The _derive_ function internally calls the _model_ function to get the IR objects and then calls the _backend_ module to generate the SMT model and solve it. As seen in the _lib.rs_ there are four modules in the crate. The content of each module is under the respective subdirectory. The _mod.rs_ file in each subdirectory contains the module tree system of the subdirectory. For example, the _mod.rs_ file in the _parser_ subdirectory is shown below:

```rust
pub mod ctxt;
mod err;
mod test;

mod attr;
pub mod name;

pub mod generics;
pub mod infer;
pub mod ty;

mod apply;
pub mod func;

pub mod path;

mod adt;
mod dsl;
pub mod expr;
pub mod intrinsics;
```

The _parser_ module contains 13 submodules. Instead of analyzing all the modules, we will provide a high level description of what the parser module is doing. The main logic and flow can be found in the _ctxt_ module. The path to the file containing the _Rusmart_ code is taken as input. The Context::new (input) function is called to create a new context object. The way this is done is by storing annotated functions, struct, and enums of the rust source code. To emphasize, only the structs and enums that are annotated with an _#\[smt\_type\]_ attribute are stored in the context object. The functions that are annotated with _#\[smt\_impl\]_, _#\[smt\_axiom\]_, and _#\[smt\_spec\]_ are also stored. Also note that code stored in local modules are stored in the context object. Some sanity checks are performed on the stored data. The context object is then returned.

Some important facts about the rusmart files are:
- Every function annotated by _#\[smt\_impl\]_, _#\[smt\_axiom\]_, or _#\[smt\_spec\]_ must have a return type.
- The return type of a function annotated by _#\[smt\_axiom\]_ must be a _Boolean_ type.
- The function signature of a function annotated by _#\[smt\_impl\]_ must be the same as the function signature of the corresponding function annotated by _#\[smt\_spec\]_. By corresponding we mean that if func X1 was annotated by _#\[smt\_impl(specs = \[X2\])\]_, then func X2 must be have a compatible signature with func X1.
- Functions annotated by _#\[smt\_spec\]_ may have or not have a body. In case, the body is not provided, the function is considered is uninterpreted and has the macro call _unimplemented!()_. To reiterate, the function signature of an uninterpreted function must be the same as the function signature of the corresponding function annotated by _#\[smt\_impl\]_.
- Functions annotated by _#\[smt\_impl\]_ or _#\[smt\_axiom\]_ must have a body.
- Every function body has some optional local let-binding statements and a mandatory sole expression (which is the return value of the function).
- The typed version of _forall_, _exists_, and _choose_ macros are only allowed in the body of a function annotated by _#\[smt\_spec\]_ and _#\[smt\_axiom\]_. In other words, these macros are not allowed in the body of a function annotated by _#\[smt\_impl\]_. However, the iteration version of these macros are allowed in any function body.
- rusmart pipeline: Parsing -> Intermediate Representation (IR) -> Backend.
- The software receives a program written in the rusmart language (which is a subset of Rust) and outputs the AST (abstract syntax tree) in the SMT-LIB format.
- There are no namespaces in rusmart.
- In attr.rs of the parser, for ImplMark & SpecMark, the specs and impls respectively, are a list of spec functions this impl should conform to & a list of impl functions they specs is targeted to. This is a 2-way relationship and not necessarily do they each have to be a list of the other. Only one of them is needed to be a list of the other.
- In attr.rs of the parser, for ImplMark & SpecMark, the method is whether to derive a receiver-style method for this function. This is twofold:
    - It is a syntactic sugar for the user to write the function in a more readable way.
    - we do not analyze the body of impls {} so this allows us to have a receiver-style method.
- parse_ident_from_path in the name.rs of the parser was the same as get_ident in the syn crate and thus was removed.
- let ty = TypeTag::from_type(ctxt, &ty)?; in dsl.rs of the parser converts the type to a TypeTag (rust type to rusmart type).
- In dsl.rs of the parser, Parse method for Quantifier not unit tested because a generics type that implements CtxtForExpr is needed.
- In bail_on for derive and remark, there are differences; this was intentional as the derive is deeper in the code and thus the error message is more readable. The remark is at the top level and thus the error message is more generic.
- A type parameter name in the parser is converted to a smt sort name in the ir (intermediate representation). TypeParamName > crate::ir::name::SmtSortName in name.rs.
- Compared to the intrinsic types that we have in stdlib (e.g. Boolean, Integer, Real, F32/F64, I32/I64/U32/U64, String, Cloak, Seq, Set, Array, Error), `TypeTag` has three extra variants: `User`, `Pack`, `Parameter` which are user-defined (struct and enum) types, tuple types, and type parameters respectively.
- TypeRef in infer.rs has an extra variant Var(TypeVar) compared to TypeTag which are for vars that do not have a concrete type yet and will be unified later.
- Seq(Box<TypeRef>) defines a type which is a sequence of TypeRefs.
- in ty.rs of the parser, we have let param_name = ident.try_into()?; in logic, type parameters take priority over user-defined type names (this is a design choice made by the rust authors as well).
- The type of the impl and spec signature should be compatible.
- Refinement is a relation between the impl and the spec.
- In ctxt.rs, the fn_db is created and then ignored as it is not needed anymore. It is only used in the expr module to look up the function names.
- Only a specification can be uninterpreted as it means that the specification is not implemented yet for the implementation. In this case we will have an axiomatic specification.
- We cannot use arbitrary Rust library types. We have to have types that are Rusmart types marked with `#[smt_type]` in the parser. For example using `std::collections::HashMap` is not allowed (instead use the stdlib `Array<K, V>`).
- fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn> in expr.rs of the parser is used to look up the function name in the function database. An impl function can be called inside an impl function. A spec function can be called inside a spec function. An impl function can be called inside a spec function. A spec function CANNOT be called inside an impl function and an error will be thrown.
- In unifying type parameters, they need to have the same name.
- The methods defined on the types in stdlib are not in place meaning that they generate a new value instead of modifying the existing one.
- Rust catches alot of the errors that we write bail_on. Some of these error handling has been done to please the Rust compiler. The rest have been just added for the sake of completion. In a general case, we should not check what Rust checks unless the compiler wants us to.
- Unit tests for PathArguments::Parenthesized(args) are not implemented.
- Forall, Exists, and Choose without iteration can only be used in spec and axioms. They have been given default value for the rust to just please the compiler. In smt they have the conventional semantics. The iterated version can be used in impls, spec, and axioms. 
- bail_on!(bound, "invalid bound"); bail_if_exists!(lifetimes); errors have not been invoked in the parser.
- Expressions which throw away their return value are not allowed: in expr.rs we have bail_if_exists!(semi_token);. This is because the expressions do not mutate any variables so they are pure, so if they are not returned to be used, they are useless.
- The spec and impl need to have the same type signature because the impl marks the operational semantics and the spec marks the denotational semantics.
- we cannot have let c = Integer::from(0);, let b = c;, let a = b;, let c = a; as rusmart does not allow multiple declarations of the same variable in the same scope (c). If it did we would have cyclic type inference.
- We only have NotSupported error for the Backend. 
- An axiom or an impl cannot be uninterpreted as it does not make any sense.
- clone & default are not allowed for the rusmart types. This is because they are not needed as rusmart types are copyable and do not need to be cloned. 
- bail_if_exists!(semi_token); in expr.rs of the parser means that return x; is not allowed in the function body. So to return x, we write x. 
- Expected some value but got None, using default value 1 in config.rs means that it Development mode.
- A function in Rusmart always needs to return a type and cannot be void.
- Ord, PartialOrd, Eq, or PartialEq are not supertraits; instead we define fn _cmp(self, rhs: Self) -> Ordering; for the SMT trait in stdlib.rs. This is because we cannot have PartialEq so naturally the other traits are not supertraits. We cannot have PartialEq, or eq/ne because the return types of these by the PartialEq are bools whereas we define them as Boolean.
- The difference between impl<T: SMT> Eq for SMTWrap<T> {} and #[derive(Eq)] in stdlib is nothing. But to have the latter we must have #[derive(PartialEq)], that means SMTWrap<T : PartialEq> should be written in the definition. However, we cannot have PartialEq as it is not a supertrait. So we have to implement the Eq trait manually. When we define the Eq and PartialEq traits manually, it is good practice to define the Hash trait as well (see https://rust-lang.github.io/rust-clippy/master/index.html#derived_hash_with_manual_eq).
- In rusmart we can have let x = some_expr or let (x1,x2...,x2) = some_expr; but we cannot destructure an enum or a struct on the left hand side. On the other hand, we can get access to the elems of a struct by x.0 or x.field_name. However, we cannot access the field names of a tuple by writing x.0 so destructuring a tuple is the only allowed way.
- Nested tuple and record structs inside an enum are allowed.
- Stmt::Item(_) | Stmt::Macro(_) => bail_on!(stmt, "unexpected item"), in expr.rs indicates that Macros are not allowed in function bodies. However, if the macro is unimplemented!(), without a semicolon, then it is allowed as it does not enter the expression tree analysis.
- A syntax like `let x: (i32, i32) = (1, 2);` is not allowed instead we have to write `let (x1, x2): (i32, i32) = (1, 2);`. The first one does not add any expressive power to the language either and is merely syntactic sugar.
- We have TypeRef::Seq(_) => TypeRef::Integer in the parser expr.rs module, because in the stdlib in the iterator of Seq we have integer from 0 to n-1 where n is the length of the Seq as the return value.
- If-Else expressions in rusmart have the following constraints: 1) An else branch is required. 2) The then and else branches must have the same type. 3) The if condition must be of the *some_expr format. The some_expr must be a rusmart Boolean expression that when is dereferenced, it gives a rust bool type.
- In path.rs of the parser we have _ => bail_on!(path, "unrecognized path"). This means if there are more than 2 segments, it is not recognized for example: Vec<T>::push::<T> has 3 segments. So everything should be defined in the context to be recognized. In other words, bringing _use syn;_ and then using _syn::Expr::Path_ will throw an error.
- In expr.rs of the parser requires every function to have at least one expression because every function should have a return value. The return value of the function is the last expression in the function body.
- The new let-bindings created are empty when analyzing the right hand side of a let binding in expr.rs. This is because that the right hand side should be a stanard rust expression and not a local let-binding.
- In analyzing local blocks, when we exit the block, the local let bindings are removed. This is because the local let bindings are only valid in the local block and not outside of it. Nevertheless, rust already catches this error.
- In expr.rs, multiple places we have ti_unify!(unifier, &ret_ty, &self.exp_ty, target); where we just throw away the result of the unification. This is because the inifier is being updated internally and we do not need to use the result of the unification.
- In Rusmart, we do not allow mutability but there are no additional restrictions imposed on the visibility of the variables. 
- 
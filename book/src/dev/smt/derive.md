## Crate: `rusmart-smt-derive`

`rusmart-smt-derive` is the **Rust→IR→SMT-LIB** compiler for the Rusmart DSL.

### Key entry points

The crate exposes two key entry points:

- **`model`**: parse and lower a Rusmart program into the internal IR (no solver required)
- **`derive`**: end-to-end compilation that emits SMT-LIB (Z3-oriented) and collects solver responses

 The _model_ function receives a path to a file and constructs a vector of __Intermediate Representation (IR)__ objects. The _derive_ function receives a path to the input file (the one given to the _model_ function) and a path to the output file. The _derive_ function internally calls the _model_ function to get the IR objects and then calls the _backend_ module to generate the SMT model and solve it. 

### Internal structure

- `src/parser/*`: DSL parsing, intrinsic recognition, overload resolution
- `src/ir/*`: expression lowering and SMT sort checking
- `src/backend/*`:
  - `backend/z3/*`: SMT-LIB emission for Z3 and response handling

### Derive

This package has one root library crate. Instead of analyzing all the modules, we will provide a high level description of what the parser module is doing.

Some important facts about the rusmart files are:
- Every function annotated must have a return type.
- Every function body has some optional local let-binding statements and a mandatory sole expression (which is the return value of the function).
- There are no namespaces in rusmart.
- Compared to the intrinsic types that we have in stdlib (e.g. Boolean, Integer, Real, F32/F64, I32/I64/U32/U64, String, Cloak, Seq, Set, Array, Error), `TypeTag` has three extra variants: `User`, `Pack`, `Parameter` which are user-defined (struct and enum) types, tuple types, and type parameters respectively.
- TypeRef in infer.rs has an extra variant Var(TypeVar) compared to TypeTag which are for vars that do not have a concrete type yet and will be unified later.
- in `ty.rs` of the _parser_, type parameters take priority over user-defined type names (this is a design choice made by the rust authors as well).
- In `ctxt.rs`, the `fn_db` is created and then ignored as it is not needed anymore. It is only used in the _expr_ module to look up the function names.
- We cannot use arbitrary Rust library types. We have to have types that are Rusmart types marked with `#[smt_type]` in the parser. For example using `std::collections::HashMap` is not allowed (instead use the stdlib `Array<K, V>`).
- In unifying type parameters, they need to have the same name.
- The methods defined on the types in stdlib are not in place meaning that they generate a new value instead of modifying the existing one.
- Rust catches alot of the errors that we write bail_on. Some of these error handling has been done to please the Rust compiler. The rest have been just added for the sake of completion. In a general case, we should not check what Rust checks unless the compiler wants us to.
- Expressions which throw away their return value are not allowed. This is because the expressions do not mutate any variables so they are pure, so if they do not return anything, they are useless.
- _Clone_ & _Default_ are not allowed for the rusmart types. This is because they are not needed as rusmart types are copyable and do not need to be cloned.
- The difference between impl<T: SMT> Eq for SMTWrap<T> {} and #[derive(Eq)] in stdlib is nothing. But to have the latter we must have #[derive(PartialEq)], that means SMTWrap<T : PartialEq> should be written in the definition. However, we cannot have PartialEq as it is not a supertrait. So we have to implement the Eq trait manually. When we define the Eq and PartialEq traits manually, it is good practice to define the Hash trait as well (see https://rust-lang.github.io/rust-clippy/master/index.html#derived_hash_with_manual_eq).
- In rusmart we can have `let x = some_expr` or `let (x1,x2...,x2) = some_expr;` but we cannot destructure an enum or a struct on the left hand side. On the other hand, we can get access to the elems of a struct by x.0 or x.field_name. However, we cannot access the field names of a tuple by writing x.0 so destructuring a tuple is the only allowed way. So something like `let x: (i32, i32) = (1, 2);` is not allowed instead we have to write `let (x1, x2): (i32, i32) = (1, 2);`.
- Nested tuple and record structs inside an enum are allowed.
- `Stmt::Item(_) | Stmt::Macro(_) => bail_on!(stmt, "unexpected item")`, in _expr.rs_ indicates that Macros are not allowed in function bodies.
- We have `TypeRef::Seq(_) => TypeRef::Integer` in the parser _expr.rs_ module, because in the _stdlib_ in the iterator of Seq we have integer from 0 to n-1 where n is the length of the Seq as the return value.
- If-Else expressions in rusmart have the following constraints: 1) An else branch is required. 2) The then and else branches must have the same type. 3) The if condition must be of the *some_expr format. The some_expr must be a rusmart Boolean expression that when is dereferenced, it gives a rust bool type.
- In Rusmart, we do not allow mutability but there are no additional restrictions imposed on the visibility of the variables.
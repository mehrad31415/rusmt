## Crate: `rusmart-smt-derive`

`rusmart-smt-derive` is the **Rust→IR→SMT-LIB** compiler and synthesis engine for the Rusmart DSL.

### Key entry points

The crate exposes three key entry points (defined in `src/lib.rs`):

- **`model(path)`**: Parse and lower a Rusmart program into the internal IR (`IRContext`). No solver required.
- **`solve(models, top_level_fn, output)`**: Run the **text backend** -- generate SMT-LIB2 text, write to disk, spawn the solver as a subprocess for each error target.
- **`solve_z3_api(models, top_level_fn, output)`**: Run the **API backend** -- solve each error target in-process using `z3-sys` bindings and `Z3_eval_smtlib2_string`.

The `model` function receives a path to a directory of Rusmart source files and constructs an `IRContext` containing type/function registries and error targets. The `solve` and `solve_z3_api` functions take this IR context, a top-level function name, and an output directory, then iterate over error targets and invoke Z3 (or other solvers if available) to find satisfying inputs.

### Internal structure

- `src/parser/*`: DSL parsing, intrinsic recognition, overload resolution
- `src/ir/*`: expression lowering and SMT sort checking
- `src/backend/*`:
  - `backend/z3/*`: **Text backend** -- SMT-LIB2 emission for Z3 and response handling
    - `ctxt.rs`: `CodeGenZ3` implementing the `CodeGen` trait, Z3 subprocess invocation, error query generation
    - `exp.rs`: IR expression to SMT-LIB2 text translation
    - `fun.rs`: Function declaration/definition generation (define-fun, define-funs-rec)
    - `intrinsics.rs`: Maps ~180 IR intrinsic operations to SMT-LIB2 formulas
    - `sort.rs`: IR sorts to SMT-LIB2 datatype declarations
  - `backend/z3_api/*`: **API backend** -- in-process Z3 via `z3-sys` C bindings
    - `mod.rs`: Core types (`Z3Ast` RAII wrapper with ref counting, `Z3Context`)
    - `context.rs`: Builds Z3 datatypes, functions, and string-parsing helpers in memory
    - `solver.rs`: Per-target solving pipeline with timeout control
    - `translate.rs`: IR expression to Z3 AST object translation
    - `intrinsics.rs`: Maps IR intrinsic operations to Z3 C API calls

### Error representation

Errors are represented as `(Array Int Bool)` rather than `(Set Int)`. Each `ErrFresh(id)` becomes `(store ((as const (Array Int Bool)) false) id true)`, and `ErrMerge` uses `((_ map or) lhs rhs)`. Membership checking uses `(select expr error_id)`.

### Set operations (user-level Sets)

User-level `Set<T>` operations use alternative SMT-LIB2 encodings:
- `SetContains`: `(set.subset (set.insert x (as set.empty (Set T))) s)` instead of `set.member`
- `SetRemove`: `(set.setminus s (set.insert x (as set.empty (Set T))))` instead of `set.singleton`

### CLI usage

```bash
cargo run -p rusmart-smt-derive -- <parser_name> <top_level_fn> [text|api|both]
```

- `text` (default): text backend only
- `api`: API backend only
- `both`: run both backends for comparison

### Build dependencies

Z3 is included as a vendored dependency (`z3 = { version = "0.20.0", features = ["vendored"] }`) along with `z3-sys = "0.11.0"`. This requires CMake and a C++ compiler. The first build compiles Z3 from source (~5 minutes); subsequent builds use the cached result. No system Z3 installation or environment variables are needed.

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
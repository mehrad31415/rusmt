use crate::ir::fun::FunRegistry;
use crate::ir::name::SmtSortName;
use crate::ir::sort::{Sort, TypeRegistry};
use crate::parser::ctxt::ContextWithFunc;
use crate::parser::generics::Generics;
use crate::parser::infer::TypeRef;
use crate::parser::name::TypeParamName;
use std::collections::{BTreeMap, BTreeSet};

/// A context for intermediate representation.
#[derive(Debug)]
pub struct IRContext {
    /// A type parameter is converted to a smt sort name in the ir (intermediate representation).
    /// This set contains all the type parameters of the functions only (not the types) because z3 smtlib support polymophic types but not polymophic functions.
    pub undef_sorts: BTreeSet<SmtSortName>,
    /// type registry (idx_named, idx_tuple, defs). The named types are user-defined types. The tuple types are stored as types that are not named. The defs are the definitions of the types.
    pub ty_registry: TypeRegistry,
    /// function registry (lookup, signature, definition). The lookup is a map from user-defined functions to a map from list of parameter types to function id.
    /// The signature is a map from function id to function signature. The definition is a map from function id to function body.
    pub fn_registry: FunRegistry,
    /// Total number of unique Error IDs generated via `Error::fresh()` across all functions.
    /// Each call to `Error::fresh()` is assigned the next available integer (starting from 0).
    pub error_count: usize,
}

impl IRContext {
    /// Create an empty context
    /// The only place this is called is in the first line of the `IRBuilder::build` function
    fn new() -> Self {
        Self {
            undef_sorts: BTreeSet::new(),
            ty_registry: TypeRegistry::new(),
            fn_registry: FunRegistry::new(),
            error_count: 0,
        }
    }
}

/// IRBuilder is responsible for constructing the IR by integrating information from the AST from parser.
pub struct IRBuilder<'a, 'ctx: 'a> {
    /// the context with function
    pub ctxt: &'ctx ContextWithFunc,
    /// type instantiation in the current context (current mapping from type parameter names to IR-level sorts.)
    pub ty_inst: BTreeMap<TypeParamName, Sort>,
    /// the ir to be accumulated (shared mutable reference across all builders)
    pub ir: &'a mut IRContext,
}

impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Create a new IRBuilder with a specific type instantiation context.
    fn new(
        ctxt: &'ctx ContextWithFunc,
        ty_inst: BTreeMap<TypeParamName, Sort>,
        ir: &'a mut IRContext,
    ) -> Self {
        Self { ctxt, ty_inst, ir }
    }

    /// Derive a new IRBuilder with specialized type parameter bindings.
    pub fn derive(&mut self, generics: &Generics, ty_args: Vec<Sort>) -> IRBuilder {
        // Verify: number of generic parameters must match number of type arguments
        let ty_params = &generics.params;
        if ty_params.len() != ty_args.len() {
            panic!("generics mismatch");
        }

        // Build new type instantiation map: {T -> Integer, U -> Boolean, ...}
        let mut ty_inst = BTreeMap::new();
        for (param, arg) in ty_params.iter().zip(ty_args.iter()) {
            match ty_inst.insert(param.clone(), arg.clone()) {
                None => (),
                Some(_) => panic!("duplicated type parameter {param}"),
            }
        }

        // Create a NEW builder with the specialized type bindings
        IRBuilder::new(self.ctxt, ty_inst, self.ir)
    }

    /// Main entry point: Convert the entire parser context (types + functions) to IR.
    ///
    /// Two-phase process:
    /// 1. **Register all types first** (so they're available when processing functions)
    /// 2. **Register all functions** (which may reference the types)
    pub fn build(ctxt: &'ctx ContextWithFunc) -> IRContext {
        // Create empty IR context (will be populated)
        let mut ir = IRContext::new();

        // For each user-defined type (struct/enum), register it
        //
        // Z3 SMT-LIB requires all datatypes (including tuples) to be explicitly declared using `(declare-datatypes ...)`.
        // So anonymous tuples like `(Int, Bool)` must be registered and given a name like `Tuple_Int_Bool` in the SMT-LIB output.
        for (type_name, type_def) in &ctxt.types {
            // Unlike functions, type parameters in types don't go in undef_sorts because
            // SMT-LIB supports polymorphic types via (declare-datatypes ((MyType 1)) ...)
            let mut ty_inst = BTreeMap::new();

            for ty_param in &type_def.head.params {
                // Create a unique sort name for this type's type parameter: "MyType_T"
                let smt_name = SmtSortName::new_type_param(type_name, ty_param);
                let smt_sort = Sort::Uninterpreted(smt_name);

                // Bind the type parameter name to the uninterpreted sort
                // This allows resolve_type() to find T when processing the type body
                ty_inst.insert(ty_param.clone(), smt_sort);
            }

            // Create a builder with type parameter bindings for THIS type
            // Now when register_type() calls resolve_type() on TypeRef::Parameter(T),
            // it will find T in ty_inst and resolve it to the uninterpreted sort
            let mut type_builder = IRBuilder::new(ctxt, ty_inst, &mut ir);

            // Register the type:
            // - Resolve TypeRef::Parameter(T) using ty_inst -> Sort::Uninterpreted("MyType_T")
            // - Process the type body, resolving references to T using ty_inst
            // - Store the type definition with the generic parameters as uninterpreted sorts
            type_builder.register_type(
                Some(type_name),
                &type_def
                    .head
                    .params
                    .iter()
                    .map(|param| TypeRef::Parameter(param.clone()))
                    .collect::<Vec<_>>(),
            );
        }

        // Register all functions
        for (func_name, func_def) in &ctxt.funcs {
            let generics = &func_def.head.generics.params;
            let mut ty_args_refs = vec![]; // TypeRef version (for register_func)
            let mut ty_inst = BTreeMap::new(); // Sort version (for the builder)

            for ty_param in generics {
                // Create unique SMT sort name: "foo_T" (function_name + type_param)
                let smt_name = SmtSortName::new_func_param(func_name, ty_param);
                ir.undef_sorts.insert(smt_name.clone());

                // Map: T -> Sort::Uninterpreted("foo_T")
                let smt_sort = Sort::Uninterpreted(smt_name);
                if ty_inst.insert(ty_param.clone(), smt_sort).is_some() {
                    panic!("duplicated type parameter {ty_param} in function {func_name}");
                }

                // Also create TypeRef version for register_func call
                ty_args_refs.push(TypeRef::Parameter(ty_param.clone()));
            }

            // Create a NEW builder with THIS function's type parameter bindings
            let mut func_builder = IRBuilder::new(ctxt, ty_inst, &mut ir);

            // Register the function
            func_builder.register_func(func_name, &ty_args_refs);
        }

        ir
    }
}

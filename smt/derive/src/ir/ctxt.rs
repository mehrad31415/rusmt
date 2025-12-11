use crate::ir::fun::FunRegistry;
use crate::ir::name::SmtSortName;
use crate::ir::sort::{Sort, TypeRegistry};
use crate::parser::ctxt::ContextWithFunc;
use crate::parser::generics::Generics;
use crate::parser::infer::TypeRef;
use crate::parser::name::{TypeParamName, UsrFuncName};
use crate::parser::ty::TypeTag;
use log::trace;
use std::collections::{BTreeMap, BTreeSet};

/// A context for intermediate representation.
#[derive(Debug)]
pub struct IRContext {
    /// uninterpreted sorts
    /// A type parameter is converted to a smt sort name in the ir (intermediate representation).
    pub undef_sorts: BTreeSet<SmtSortName>,
    /// type registry (idx_named, idx_tuple, defs). The named types are user-defined types. The tuple types are stored as types that are not named. The defs are the definitions of the types.
    pub ty_registry: TypeRegistry,
    /// function registry (lookup, signature, definition). The lookup is a map from user-defined functions to a map from list of parameter types to function id.
    /// The signature is a map from function id to function signature. The definition is a map from function id to function body.
    pub fn_registry: FunRegistry,
}

impl IRContext {
    /// Create an empty context
    /// The only place this is called is in the first line of the `IRBuilder::build` function
    fn new() -> Self {
        Self {
            undef_sorts: BTreeSet::new(),
            ty_registry: TypeRegistry::new(),
            fn_registry: FunRegistry::new(),
        }
    }

    /// Reverse resolve a `Sort` (IR-level type) back to a `TypeTag` (parser-level type).
    /// This is used when you need to map IR types back to their parser-level representations.
    fn reverse_sort(&self, sort: &Sort) -> TypeTag {
        match sort {
            Sort::Boolean => TypeTag::Boolean,
            Sort::Integer => TypeTag::Integer,
            Sort::Real => TypeTag::Real,
            Sort::F32 => TypeTag::F32,
            Sort::F64 => TypeTag::F64,
            Sort::I32 => TypeTag::I32,
            Sort::I64 => TypeTag::I64,
            Sort::U32 => TypeTag::U32,
            Sort::U64 => TypeTag::U64,
            Sort::String => TypeTag::String,
            Sort::Seq(sub) => TypeTag::Seq(self.reverse_sort(sub).into()),
            Sort::Set(sub) => TypeTag::Set(self.reverse_sort(sub).into()),
            Sort::Array(key, val) => {
                TypeTag::Array(self.reverse_sort(key).into(), self.reverse_sort(val).into())
            }
            Sort::Error => TypeTag::Error,
            Sort::User(sid) => {
                let (sort_name, sort_inst) = self.ty_registry.reverse_lookup(*sid);
                let inst = sort_inst.iter().map(|s| self.reverse_sort(s)).collect();
                match sort_name {
                    None => TypeTag::Pack(inst),
                    Some(name) => TypeTag::User(name.into(), inst),
                }
            }
            Sort::Uninterpreted(name) => TypeTag::Parameter(name.into()),
        }
    }

    /// Reverse map all registered function instances into a vector of (function name, type instantiation)
    /// pairs, where the instantiation is expressed as a vector of TypeTags.
    fn reverse_function_instances(&self) -> Vec<(UsrFuncName, Vec<TypeTag>)> {
        let mut instances = vec![];
        for (name, insts) in &self.fn_registry.lookup {
            for inst in insts.keys() {
                let tags = inst.iter().map(|e| self.reverse_sort(e)).collect();
                instances.push((name.into(), tags));
            }
        }
        instances
    }
}

/// IRBuilder is responsible for constructing the IR by integrating information from the AST from parser,
pub struct IRBuilder<'a, 'ctx: 'a> {
    pub ctxt: &'ctx ContextWithFunc,
    /// type instantiation in the current context (current mapping from type parameter names to IR-level sorts.)
    /// The Sort are the IR representation of the Type arguments in the call and the TypeParamName match the type parameters in the definition to the call.
    pub ty_inst: BTreeMap<TypeParamName, Sort>,
    /// the ir to be accumulated
    pub ir: &'a mut IRContext,
}

impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Change the analysis context
    fn new(
        ctxt: &'ctx ContextWithFunc,
        ty_inst: BTreeMap<TypeParamName, Sort>,
        ir: &'a mut IRContext,
    ) -> Self {
        Self { ctxt, ty_inst, ir }
    }

    /// Derive a new IRBuilder context specialized with given generics instantiation.
    /// This verifies that the number of type arguments in the call matches the generic parameters in the definition and
    /// then builds a new mapping to be used in the derived context.
    pub fn derive(&mut self, generics: &Generics, ty_args: Vec<Sort>) -> IRBuilder {
        let ty_params = &generics.params;
        if ty_params.len() != ty_args.len() {
            panic!("generics mismatch");
        }

        let mut ty_inst = BTreeMap::new();
        for (param, arg) in ty_params.iter().zip(ty_args.iter()) {
            match ty_inst.insert(param.clone(), arg.clone()) {
                None => (),
                Some(_) => panic!("duplicated type parameter {param}"),
            }
        }
        IRBuilder::new(self.ctxt, ty_inst, self.ir)
    }

    /// ContextWithFunc is the context of the rusmart file being processed
    /// Convert the entire context (types + functions) to IR.
    pub fn build(ctxt: &'ctx ContextWithFunc) -> IRContext {
        let mut ir = IRContext::new();

        let mut type_builder = IRBuilder::new(ctxt, BTreeMap::new(), &mut ir);

        for (type_name, _type_def) in &ctxt.types {
            type_builder.register_type(
                Some(type_name),
                &_type_def
                    .head
                    .params
                    .iter()
                    .map(|f| TypeRef::Parameter(f.clone()))
                    .collect::<Vec<_>>(),
            );
        }

        for (func_name, func_def) in &ctxt.funcs {
            // 1. Setup Generics for this function
            let generics = &func_def.head.generics.params;
            let mut ty_args_refs = vec![];
            let mut ty_inst = BTreeMap::new();

            for ty_param in generics {
                // Unique name: function_name_T
                let smt_name = SmtSortName::new_func_param(func_name, ty_param);
                ir.undef_sorts.insert(smt_name.clone());

                let smt_sort = Sort::Uninterpreted(smt_name);
                if ty_inst.insert(ty_param.clone(), smt_sort).is_some() {
                    panic!("duplicated type parameter {ty_param} in function {func_name}");
                }
                ty_args_refs.push(TypeRef::Parameter(ty_param.clone()));
            }

            trace!(
                "Translating function {}: generics <{}>",
                func_name,
                ty_args_refs
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            // 2. Create Builder with function scope
            let mut func_builder = IRBuilder::new(ctxt, ty_inst, &mut ir);

            // 3. Register the function
            // This parses the signature and body, resolving types against the registry we built in Pass 1.
            func_builder.register_func(func_name, &ty_args_refs);
        }

        ir
    }
}

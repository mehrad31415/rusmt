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
/// An IRContext for each of the refinement relations in the rusmart file is created.
/// desc: "<impl> ~> <spec>" (description of the IR context)
#[derive(Debug)]
pub struct IRContext {
    /// uninterpreted sorts
    /// A type parameter is converted to a smt sort name in the ir (intermediate representation).
    /// These are the type parameters of the impl function in the definition
    /// (the impl and spec function must have the same generic names or at least be unifiable).
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
            Sort::Rational => TypeTag::Rational,
            Sort::Text => TypeTag::Text,
            Sort::Seq(sub) => TypeTag::Seq(self.reverse_sort(sub).into()),
            Sort::Set(sub) => TypeTag::Set(self.reverse_sort(sub).into()),
            Sort::Map(key, val) => {
                TypeTag::Map(self.reverse_sort(key).into(), self.reverse_sort(val).into())
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

/// A context builder originated from a refinement relation
/// IRBuilder is responsible for constructing the IR by integrating information from the AST from parser,
/// current type instantiations, and the mutable IRContext. It is created per refinement relation.
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

    /// ASTContext is the context of the rusmart file being processed
    /// The `refinement` is the relation between the spec and the impl and contains the spec and impl function names
    /// this function is called for each refinement (spec-impl pair) and constructs the IR for each refinement
    pub fn build(ctxt: &'ctx ContextWithFunc) -> IRContext {
        // ir is an empty IRContext with the description of the refinement
        // this line just basically creates a new IRContext with the description of the refinement like the following:
        // IRContext {
        //     desc: "<impl> ~> <spec>",
        //     undef_sorts: BTreeSet::new(),
        //     ty_registry: TypeRegistry::new(),
        //     fn_registry: FunRegistry::new(),
        //     axiom_registry: AxiomRegistry::new(),
        // }
        let mut ir = IRContext::new();

        // get the pair
        let fn_impl = ctxt.get_func(&rel.fn_impl); // get the function signature and body of the implementation
        // assign uninterpreted sorts as type arguments for function type parameters
        let generics_impl = &fn_impl.head.generics.params;

        // type instantiation for impl
        let mut ty_args_impl = vec![];
        let mut ty_inst_impl = BTreeMap::new();

        // for each type parameter in the function signature of the implementation
        for ty_param in generics_impl {
            // create a new uninterpreted sort name for the type parameter
            // for function impl and parameter T, the uninterpreted sort name would be SmtSortName { ident: "impl_T" }
            let smt_name = SmtSortName::new_func_param(&rel.fn_impl, ty_param);
            // insert the uninterpreted sort name into the set of uninterpreted sorts in the ir
            ir.undef_sorts.insert(smt_name.clone()); // so undef_sorts just basically contains all the generics of the function impl + spec

            // create a new uninterpreted sort for the type parameter
            let smt_sort = Sort::Uninterpreted(smt_name);
            // ty_inst_impl is a map from type parameter name to sort
            match ty_inst_impl.insert(ty_param.clone(), smt_sort) {
                None => (),
                Some(_) => panic!("duplicated type parameter {ty_param}"), // can't have duplicate type parameters for a function
            }
            // ty_args_impl is a list of types for the function
            ty_args_impl.push(TypeRef::Parameter(ty_param.clone()));
        }
        trace!(
            "top-level type parameters impl: <{}>",
            ty_args_impl
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // ty_inst_impl is a map from type parameter name to sort for the respective impl
        // for example for T, it would be TypeParamName {ident: "T"} -> Sort::Uninterpreted(SmtSortName { ident: "impl_T" })
        let mut builder_impl = IRBuilder::new(ctxt, ty_inst_impl.clone(), &mut ir);
        // process the impl and updates the fn_registry
        // The register type is done in the parsing of the body of the function.
        builder_impl.register_func(&ty_args_impl); // the user function name and the type parameters of the impl

        ir
    }
}

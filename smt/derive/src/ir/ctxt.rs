use crate::ir::axiom::AxiomRegistry;
use crate::ir::fun::FunRegistry;
use crate::ir::mono::add_instantiation;
use crate::ir::name::SmtSortName;
use crate::ir::sort::{Sort, TypeRegistry};
use crate::parser::ctxt::{ASTContext, Refinement};
use crate::parser::generics::{Generics, PartialInst};
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
    /// description
    pub desc: Refinement,
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
    /// axiom registry. Each axiom on the parser level is a function. The return type of the axiom is always Boolean.
    /// AxiomRegistry contains a lookup map from an axiom’s name (with parameter types) to a unique axiom ID and a mapping from each unique axiom ID to its actual `Predicate` definition.
    /// The Predicate contains a list of parameters (each has a Symbol (variable name on parser level) and a Sort which is the type on the parser level), expression registry (body of the predicate), and expression ID of the predicate body.
    /// Each expression has a unique ID.
    pub axiom_registry: AxiomRegistry,
}

impl IRContext {
    /// Create an empty context
    /// The only place this is called is in the first line of the `IRBuilder::build` function
    fn new(desc: &Refinement) -> Self {
        Self {
            desc: desc.clone(),
            undef_sorts: BTreeSet::new(),
            ty_registry: TypeRegistry::new(),
            fn_registry: FunRegistry::new(),
            axiom_registry: AxiomRegistry::new(),
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
    /// context provider (AST context (e.g. function definitions, axioms, etc.))
    pub ctxt: &'ctx ASTContext,
    /// type instantiation in the current context (current mapping from type parameter names to IR-level sorts.)
    /// The Sort are the IR representation of the Type arguments in the call and the TypeParamName match the type parameters in the definition to the call.
    pub ty_inst: BTreeMap<TypeParamName, Sort>,
    /// the ir to be accumulated
    pub ir: &'a mut IRContext,
}

impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Change the analysis context
    fn new(
        ctxt: &'ctx ASTContext,
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
                Some(_) => panic!("duplicated type parameter {}", param),
            }
        }
        IRBuilder::new(self.ctxt, ty_inst, self.ir)
    }

    /// ASTContext is the context of the rusmart file being processed
    /// The `refinement` is the relation between the spec and the impl and contains the spec and impl function names
    /// this function is called for each refinement (spec-impl pair) and constructs the IR for each refinement
    pub fn build(ctxt: &'ctx ASTContext, rel: &'ctx Refinement) -> IRContext {
        // ir is an empty IRContext with the description of the refinement
        // this line just basically creates a new IRContext with the description of the refinement like the following:
        // IRContext {
        //     desc: "<impl> ~> <spec>",
        //     undef_sorts: BTreeSet::new(),
        //     ty_registry: TypeRegistry::new(),
        //     fn_registry: FunRegistry::new(),
        //     axiom_registry: AxiomRegistry::new(),
        // }
        let mut ir = IRContext::new(rel); // rel.to_string() is the description of the refinement which gives <impl> ~> <spec>

        // get the pair
        let fn_impl = ctxt.get_func(&rel.fn_impl); // get the function signature and body of the implementation
        let fn_spec = ctxt.get_func(&rel.fn_spec); // get the function signature and body of the specification

        if !fn_impl.head.is_compatible(&fn_spec.head) {
            panic!("function signature mismatch in ir");
        }
        // assign uninterpreted sorts as type arguments for function type parameters
        let generics_impl = &fn_impl.head.generics.params;
        let generics_spec = &fn_spec.head.generics.params;

        // type instantiation for both spec and impl
        // (stores the type parameters of the impl and spec)
        let mut ty_args_impl = vec![];
        let mut ty_args_spec = vec![];

        // type arguments for IR builder context
        // (stores the type parameters mapped to their respective Smt sorts)
        let mut ty_inst_impl = BTreeMap::new();
        let mut ty_inst_spec = BTreeMap::new();

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
                Some(_) => panic!("duplicated type parameter {}", ty_param), // can't have duplicate type parameters for a function
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

        // for each type parameter in the function signature of the specification
        for ty_param in generics_spec {
            // create a new uninterpreted sort name for the type parameter
            // for function spec and parameter T, the uninterpreted sort name would be SmtSortName { ident: "spec_T" }
            let smt_name = SmtSortName::new_func_param(&rel.fn_spec, ty_param);
            // insert the uninterpreted sort name into the set of uninterpreted sorts in the ir
            ir.undef_sorts.insert(smt_name.clone());

            // create a new uninterpreted sort for the type parameter
            let smt_sort = Sort::Uninterpreted(smt_name);
            // ty_inst_spec is a map from type parameter name to sort
            match ty_inst_spec.insert(ty_param.clone(), smt_sort) {
                None => (),
                Some(_) => panic!("duplicated type parameter {}", ty_param),
            }
            // ty_args_spec is a list of types for the function
            ty_args_spec.push(TypeRef::Parameter(ty_param.clone()));
        }
        trace!(
            "top-level type parameters spec: <{}>",
            ty_args_spec
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // initialize the builder
        // the ir at this point is:
        // IRContext {
        //     desc: "<impl> ~> <spec>",
        //     undef_sorts: {
        //         SmtSortName { ident: "impl_T" },
        //         SmtSortName { ident: "impl_U" },
        //         SmtSortName { ident: "spec_T" },
        //         SmtSortName { ident: "spec_U" },
        //         ... for any number of type parameters for the respective impl and spec
        //     },
        //     ty_registry: TypeRegistry::new(),
        //     fn_registry: FunRegistry::new(),
        //     axiom_registry: AxiomRegistry::new(),
        // }

        // ty_inst_impl is a map from type parameter name to sort for the respective impl
        // for example for T, it would be TypeParamName {ident: "T"} -> Sort::Uninterpreted(SmtSortName { ident: "impl_T" })
        let mut builder_impl = IRBuilder::new(ctxt, ty_inst_impl.clone(), &mut ir);
        // process the impl and updates the fn_registry
        // The register type is done in the parsing of the body of the function.
        builder_impl.register_func(&rel.fn_impl, &ty_args_impl); // the user function name and the type parameters of the impl

        // ty_inst_spec is a map from type parameter name to sort for the respective spec
        // for example for T, it would be TypeParamName {ident: "T"} -> Sort::Uninterpreted(SmtSortName { ident: "spec_T" })
        let mut builder_spec = IRBuilder::new(ctxt, ty_inst_spec.clone(), &mut ir);
        // process the spec and updates the fn_registry
        builder_spec.register_func(&rel.fn_spec, &ty_args_spec); // the user function name and the type parameters of the spec

        // pull in all relevant axioms
        let mut relevant_axioms = BTreeMap::new();
        let mut uninterpreted_axiom_params = BTreeMap::new();
        let mut processed_axioms = BTreeSet::new();

        let mut fixedpoint;
        loop {
            // always assume that we don't have more to analyze at the beginning
            fixedpoint = true;

            // consolidate all related axioms and their instantiations
            let mut batch = BTreeMap::new();
            // For every function instance registered in the IR, get related axioms.
            // basically ir.reverse_function_instances() for any implementation and specification or function calls inside their bodies, it returns a vector of (function name, type tag for generics) pairs
            for (func_name, func_inst) in ir.reverse_function_instances() {
                // probe_related_axioms returns a mapping from an axiom name to a set of instantiations.
                for (axiom_name, mut axiom_insts) in
                    ctxt.probe_related_axioms(&func_name, &func_inst)
                // we get the related axioms for each function instance
                {
                    batch
                        .entry(axiom_name)
                        .or_insert_with(BTreeSet::new)
                        .append(&mut axiom_insts);
                }
            }

            // self-interference (for more mono instances) and register each axiom mono instance
            for (name, insts) in batch {
                let axiom = ctxt.get_axiom(&name);
                // if the relevant axiom is not already in the relevant axioms, insert it
                // existing_insts is a mutable set of instantiations for the axiom
                let existing_insts = relevant_axioms
                    .entry(name.clone())
                    .or_insert_with(BTreeSet::new);

                let mut all_new_insts = vec![];
                for inst in insts {
                    trace!("axiom {}{} is relevant", name, inst);
                    let additions = add_instantiation(&axiom.head.generics, existing_insts, inst);
                    all_new_insts.extend(additions.into_iter());
                }
                trace!(
                    "monomorphization yields {} new instantiation(s) for axiom {}",
                    all_new_insts.len(),
                    name
                );

                // register axiom under each new instantiation
                for inst in all_new_insts {
                    trace!("processing axiom {}{}", name, inst);

                    // first collect unspecified type parameters
                    for ty_arg_inst in &inst.args {
                        match ty_arg_inst {
                            PartialInst::Assigned(_) => (),
                            PartialInst::Unassigned(n) => {
                                let axiom_params_map = uninterpreted_axiom_params
                                    .entry(name.clone())
                                    .or_insert_with(BTreeMap::new);
                                if !axiom_params_map.contains_key(n) {
                                    let smt_name = SmtSortName::new_axiom_param(&name, n);
                                    ir.undef_sorts.insert(smt_name.clone());

                                    let smt_sort = Sort::Uninterpreted(smt_name);
                                    axiom_params_map.insert(n.clone(), smt_sort);
                                }
                            }
                        }
                    }

                    // specialized builder just for axiom type arguments
                    let mut axiom_ty_builder_impl =
                        IRBuilder::new(ctxt, ty_inst_impl.clone(), &mut ir);

                    // type instantiation for axiom
                    let mut axiom_ty_args_impl = vec![];
                    // type arguments for IR builder context
                    let mut axiom_ty_inst_impl = BTreeMap::new();

                    for (ty_param, ty_arg_inst) in
                        axiom.head.generics.params.iter().zip(inst.args.iter())
                    {
                        let (ty_arg_ref, ty_arg_sort) = match ty_arg_inst {
                            PartialInst::Assigned(t) => {
                                let tref = t.into();
                                let sort = axiom_ty_builder_impl.resolve_type(&tref);
                                (tref, sort)
                            }
                            PartialInst::Unassigned(n) => {
                                let tref = TypeRef::Parameter(n.clone());
                                let sort = uninterpreted_axiom_params
                                    .get(&name)
                                    .and_then(|v| v.get(n))
                                    .expect("axiom type parameter variable created")
                                    .clone();
                                (tref, sort)
                            }
                        };
                        match axiom_ty_inst_impl.insert(ty_param.clone(), ty_arg_sort) {
                            None => (),
                            Some(_) => panic!("duplicated type parameter {}", ty_param),
                        }
                        axiom_ty_args_impl.push(ty_arg_ref);
                    }

                    let mut axiom_ty_builder_spec =
                        IRBuilder::new(ctxt, ty_inst_spec.clone(), &mut ir);

                    // type instantiation for axiom
                    let mut axiom_ty_args_spec = vec![];
                    // type arguments for IR builder context
                    let mut axiom_ty_inst_spec = BTreeMap::new();

                    for (ty_param, ty_arg_inst) in
                        axiom.head.generics.params.iter().zip(inst.args.iter())
                    {
                        let (ty_arg_ref, ty_arg_sort) = match ty_arg_inst {
                            PartialInst::Assigned(t) => {
                                let tref = t.into();
                                let sort = axiom_ty_builder_spec.resolve_type(&tref);
                                (tref, sort)
                            }
                            PartialInst::Unassigned(n) => {
                                let tref = TypeRef::Parameter(n.clone());
                                let sort = uninterpreted_axiom_params
                                    .get(&name)
                                    .and_then(|v| v.get(n))
                                    .expect("axiom type parameter variable created")
                                    .clone();
                                (tref, sort)
                            }
                        };
                        match axiom_ty_inst_spec.insert(ty_param.clone(), ty_arg_sort) {
                            None => (),
                            Some(_) => panic!("duplicated type parameter {}", ty_param),
                        }
                        axiom_ty_args_spec.push(ty_arg_ref);
                    }

                    // specialized builder for axiom body
                    let mut axiom_ty_builder_impl: IRBuilder<'_, '_> =
                        IRBuilder::new(ctxt, axiom_ty_inst_impl, &mut ir);
                    axiom_ty_builder_impl.register_axiom(
                        &name,
                        &axiom_ty_args_impl,
                        &mut processed_axioms,
                    );
                    let mut axiom_ty_builder_spec: IRBuilder<'_, '_> =
                        IRBuilder::new(ctxt, axiom_ty_inst_spec, &mut ir);
                    axiom_ty_builder_spec.register_axiom(
                        &name,
                        &axiom_ty_args_spec,
                        &mut processed_axioms,
                    );

                    // not reaching fixedpoint yet as long as we find a new monomorphization instance
                    fixedpoint = false;
                }
            }

            // exit the loop if we have reached a fixedpoint
            if fixedpoint {
                break;
            }
        }
        ir
    }
}

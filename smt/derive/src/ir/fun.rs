//! Intermediate representation (IR) functions.

use crate::ir::ctxt::IRBuilder;
use crate::ir::exp::{ExpBuilder, ExpRegistry};
use crate::ir::index::{ExpId, UsrFunId};
use crate::ir::name::{Symbol, UsrFunName};
use crate::ir::sort::Sort;
use crate::parser::func::{FuncDef, FuncSig};
use crate::parser::infer::TypeRef;
use crate::parser::name::UsrFuncName;
use std::collections::BTreeMap;

/// A function signature in the IR.
///
/// It holds the parameters and the return type.
/// Each parameter is a tuple containing: a symbol (IR of a variable name) and a sort (IR of a type tag).
#[derive(Clone, Debug)]
pub struct FunSig {
    /// List of parameters, where each parameter is a (name, sort) pair.
    pub params: Vec<(Symbol, Sort)>,
    /// The sort (or type) of the function's return value.
    pub ret_ty: Sort,
}

#[derive(Debug, Clone)]
/// A function definition in the IR.
pub struct FunDef {
    /// The body of the function
    pub body: ExpRegistry,
    /// The ID of the root expression of the function body
    pub root_exp_id: ExpId,
}

/// A registry that tracks all functions within the IR.
///
/// 1. A lookup table from a user function name (IR). A function name can be defined with multiple instantiations. That is why the value is a map itself, mapping from a list of sorts (generics) to the function ID which is unique.
/// 2. A map from each unique function ID to its corresponding function signature.
/// 3. A map from each unique function ID to its corresponding function definition.
#[derive(Default, Debug)]
pub struct FunRegistry {
    /// a map from user-defined functions and instantiations to function id
    pub lookup: BTreeMap<UsrFunName, BTreeMap<Vec<Sort>, UsrFunId>>,
    /// a map for function signatures
    sigs: BTreeMap<UsrFunId, FunSig>,
    /// a map for function definitions
    defs: BTreeMap<UsrFunId, FunDef>,
}

impl FunRegistry {
    /// Initialize an empty registry
    pub fn new() -> Self {
        Self {
            lookup: BTreeMap::new(),
            sigs: BTreeMap::new(),
            defs: BTreeMap::new(),
        }
    }

    /// Returns the unique function ID associated with a given function name and list of parameter types.
    fn get_index(&self, name: &UsrFunName, inst: &[Sort]) -> Option<UsrFunId> {
        self.lookup.get(name)?.get(inst).copied() // return a copy of the value UsrFunId
    }

    /// Creates a new function instance entry in the registry.
    fn create(&mut self, name: UsrFunName, inst: Vec<Sort>) -> UsrFunId {
        // This gives the number of registered functions so far.
        let idx = UsrFunId {
            index: self.lookup.values().map(|v| v.len()).sum::<usize>(),
        };
        // or_default() gives an empty BTreeMap if the value does not exist.
        let existing = self.lookup.entry(name).or_default().insert(inst, idx);
        if existing.is_some() {
            panic!("function instance already registered"); // will never happen
        }
        idx
    }

    /// Registers a function signature for a given function ID.
    fn register_sig(&mut self, idx: UsrFunId, sig: FunSig) {
        let existing = self.sigs.insert(idx, sig);
        if existing.is_some() {
            panic!("function signature already registered for register_sig");
        }
    }

    /// Retrieves the function signature associated with a given function ID.
    pub fn retrieve_sig(&self, idx: UsrFunId) -> &FunSig {
        self.sigs
            .get(&idx)
            .expect("no such function id in retrieve_sig")
    }

    /// Registers a function definition for a given function ID.
    fn register_def(&mut self, idx: UsrFunId, def: FunDef) {
        let existing = self.defs.insert(idx, def);
        if existing.is_some() {
            panic!("function definition already registered for register_def");
        }
    }

    /// Retrieves the function definition associated with a given function ID.
    pub fn retrieve_def(&self, idx: UsrFunId) -> &FunDef {
        self.defs
            .get(&idx)
            .expect("no such function id in retrieve_def")
    }
}

impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Registers a function with the IR and returns its unique function ID.
    /// This is used for the function definition, and any function calls in the body of the function definition.
    pub fn register_func(&mut self, fn_name: &UsrFuncName, ty_args: &[TypeRef]) -> UsrFunId {
        // Convert the parser-level function name into its IR-level UsrFunName (UsrFunName is the IR of UsrFuncName)
        let name = fn_name.into();

        // ty_args is a bunch of TypeRef::Parameter(TypeParamName) and will become a bunch of Sort::Uninterpreted(SmtSortName)
        let ty_args = self.resolve_type_ref_vec(ty_args);

        // Check if a function with the given name and instantiation is already registered.
        match self.ir.fn_registry.get_index(&name, &ty_args) {
            None => (),
            Some(idx) => return idx, // if the function is already processed, return the index
        }

        // If not registered, create a new entry in the registry.
        let idx = self.ir.fn_registry.create(name, ty_args.clone());

        // Retrieve the function definition from the parser context.
        let FuncDef {
            head:
                FuncSig {
                    generics,
                    params,
                    ret_ty,
                },
            body,
        } = self.ctxt.get_func(fn_name);

        // prepare the builder for definition processing
        // this creates a new builder with the same parser context (ASTContext) and IR context (IRContext) but the ty_inst which is BTreeMap<TypeParamName, Sort> will be from the generics and the type args which are basically just taken from the generics of the function definition (TypeRef::Parameter(TypeParamName { name: "T" })) and the converted to Sort. So they are the same thing.
        let mut builder = self.derive(&generics, ty_args);

        // resolve type in function signatures
        let mut resolved_params = vec![];
        for (param_name, param_ty) in params {
            let param_sort = builder.resolve_type(&(param_ty.into())); // param_ty.into() converts TypeTag to TypeRef
            resolved_params.push((param_name.into(), param_sort)); // param_name.into() converts the parser-level variable name into its IR-level representation Symbol
        }
        let resolved_ret_ty = builder.resolve_type(&(ret_ty.into()));

        // register signature
        let sig = FunSig {
            params: resolved_params,
            ret_ty: resolved_ret_ty,
        };
        builder.ir.fn_registry.register_sig(idx, sig.clone());

        // materialize the entire function
        // the builder is passed on for three things: 
            // 1) type resolution (self.parent.resolve_type(...) to convert TypeRef → Sort using ty_inst)
            // 2) type registration (self.parent.register_type(...) when encountering new type instantiations like Option<i32> in an expression)
            // 3) function registration (self.parent.register_func(...) when a function call like foo::<i32>(x) is encountered, which may trigger registering a new monomorphized instance)
        let (exp_reg, exp_id) = ExpBuilder::materialize(builder, &sig, body);
        let def = FunDef {
            body: exp_reg,
            root_exp_id: exp_id,
        };
        // register the function definition
        self.ir.fn_registry.register_def(idx, def);

        // done
        idx
    }
}

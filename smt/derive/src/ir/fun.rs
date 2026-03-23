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

impl FunSig {
    /// Returns the symbols of the parameters in the function signature
    pub fn get_params(&self) -> Vec<Symbol> {
        let mut symbols = vec![];
        for (name, _) in &self.params {
            symbols.push(name.clone());
        }
        symbols
    }
}

#[derive(Debug, Clone)]
/// A function body in the IR.
pub enum FunDef {
    /// A function that is defined by an explicit body.
    /// It contains an expression registry (tracking all sub-expressions) and the ID of the root expression
    Defined(ExpRegistry, ExpId),
}

/// A registry that tracks all functions within the IR.
///
/// 1. A lookup table from a user function name (IR). A function name can be defined with multiple instantiations. That is why the value is a map itself, mapping from a list of sorts (generics) to the function ID which is unique.
/// 2. A map from each unique function ID to its corresponding function signature.
/// 3. A map from each unique function ID to its corresponding function definition.
// For fn y<T>(one:T, two:i32) -> i32 { 3 } the UsrFunName will be y, the sorts will be T, the id for example will be 1, the FunSig will encapsulate (one:T, two:i32) -> i32 and the return FunDef will have 3.
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
    /// Panics if the function instance (i.e. same name and parameters) is already registered.
    fn create(&mut self, name: UsrFunName, inst: Vec<Sort>) -> UsrFunId {
        // the new index is the sum of all the lengths of the values in the lookup table.
        // So if we had 15 functions already defined it will be 16.
        let idx = UsrFunId {
            index: self.lookup.values().map(|v| v.len()).sum::<usize>(),
        };
        // or_default() gives an empty BTreeMap if the value does not exist.
        let existing = self.lookup.entry(name).or_default().insert(inst, idx);
        if existing.is_some() {
            panic!("function instance already registered"); // will never happen because of the check in get_index in line 206
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

    /// Returns a reference to the lookup table for a given function.
    pub fn get_lookup(&self, name: &UsrFuncName) -> &BTreeMap<Vec<Sort>, UsrFunId> {
        self.lookup
            .get(&UsrFunName::from(name))
            .expect("Function not found")
    }
}

impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Registers a function with the IR and returns its unique function ID.
    /// This is called for the implementation, and any function calls in the body of the implementation.
    pub fn register_func(&mut self, fn_name: &UsrFuncName, ty_args: &[TypeRef]) -> UsrFunId {
        // Convert the parser-level function name into its IR-level UsrFunName (UsrFunName is the IR of UsrFuncName)
        let name = fn_name.into();

        // Resolve type arguments (from TypeRef to IR Sort). Sort is the IR of TypeTag (or TypeRef since TypeRef:::TypeVar cannot exist in the IR)
        // ty_args is a bunch of Sort::Uninterpreted(SmtSortName)
        let ty_args = self.resolve_type_ref_vec(ty_args);

        // Check if a function with the given name and instantiation is already registered.
        match self.ir.fn_registry.get_index(&name, &ty_args) {
            None => (),
            Some(idx) => return idx, // if the function is already processed, return the index
        }

        // If not registered, create a new entry in the registry.
        let idx = self.ir.fn_registry.create(name, ty_args.clone());

        // Retrieve the function definition from the parser context.
        // This includes both the signature (with generics, parameters, and return type) and an optional body.
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
        // this creates a new builder with the same parser context (ASTContext) and IR context (IRContext) but the ty_inst which is BTreeMap<TypeParamName, Sort> will be from the generics and the type args which are basically just taken from the generics of the function definition (TypeRef::Parameter(TypeParamName { name: "T" })) and the converted to Sort. So they are the same thing roughly...
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
        let (exp_reg, exp_id) = ExpBuilder::materialize(builder, &sig, body);
        let def = FunDef::Defined(exp_reg, exp_id);

        // register the function definition
        self.ir.fn_registry.register_def(idx, def);

        // done
        idx
    }
}

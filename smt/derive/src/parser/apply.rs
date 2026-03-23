//! This module defines the `ApplyDatabase` struct, which is used to store function signatures and their types.

use crate::parser::expr::{Expr, Op};
use crate::parser::func::FuncSig;
use crate::parser::generics::{Generics, GenericsInstFull, GenericsInstPartial};
use crate::parser::infer::{TIError, TypeRef, TypeUnifier};
use crate::parser::intrinsics::Intrinsic;
use crate::parser::name::{TypeParamName, UsrFuncName};
use crate::parser::ty::{SysTypeName, TypeTag};
use anyhow::{Result, bail};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// Represents a function type.
pub struct TypeFn {
    /// Generic parameters of the function.
    pub generics: Generics,
    /// Types of the function's parameters.
    pub params: Vec<TypeTag>, // unlike FuncSig, this is a vector of TypeTag instead of a vector of (UsrVarName, TypeTag). So it only contains the types of the parameters, not the names.
    /// Return type of the function.
    pub ret_ty: TypeTag,
}

impl TypeFn {
    /// Creates a new function type `TypeFn` from its function signature.
    pub fn new_from_sig(sig: &FuncSig) -> Self {
        Self {
            generics: sig.generics.clone(), // Clone the generics from the signature.
            params: sig.params.iter().map(|(_, ty)| ty.clone()).collect(), // Collect parameter types.
            ret_ty: sig.ret_ty.clone(),                                    // Clone the return type.
        }
    }

    /// Instantiates a function type by substituting generics with concrete types refs
    /// GenericsInstFull is a struct that holds a full mapping from generic parameters to concrete types.
    /// pub struct GenericsInstFull {
    ///     args: BTreeMap<TypeParamName, (usize, TypeRef)>,
    /// }
    pub fn instantiate(&self, subst: &GenericsInstFull) -> Option<(Vec<TypeRef>, TypeRef)> {
        // Instantiate parameter types.
        let params: Vec<TypeRef> = self
            .params
            .iter()
            .map(|t| subst.instantiate(t))
            .collect::<Option<Vec<TypeRef>>>()?;
        let ret_ty = subst.instantiate(&self.ret_ty)?;
        Some((params, ret_ty)) // Return instantiated types.
    }
}

#[derive(Debug)]
/// A database of function types, used for looking up functions.
pub struct ApplyDatabase {
    /// User-defined functions without a qualifier (standalone functions).
    /// It is a map from function names to their respective function signatures.
    unqualified: BTreeMap<UsrFuncName, TypeFn>,
    /// It is a map from function names to a map of system type names to their respective function signatures.
    /// UsrFuncName is the name of the built-in function for example "add", "sub", etc. In "add" for example, it is associated with the system type "Integer" & "Real". That is why the value is a map itself. The "add" associated with "Integer" will have a different function signature and kind to the "add" associated with "Rational". So basically, we will have somthing like this:
    /// on_sys_type = { "add" => { "Integer" => TypeFn_For_Integer, "Rational" => TypeFn_For_Rational } ... }
    /// SysTypeName is the system type name that the method is associated with.
    /// TypeFn is the function type encapsulating the function signature.
    on_sys_type: BTreeMap<UsrFuncName, BTreeMap<SysTypeName, TypeFn>>,
}

impl ApplyDatabase {
    /// Creates a new, empty `ApplyDatabase`.
    fn new() -> Self {
        Self {
            unqualified: BTreeMap::new(),
            on_sys_type: BTreeMap::new(),
        }
    }

    /// Registers a built-in function (intrinsic) in the database.
    ///
    /// # Arguments
    ///
    /// * `fn_name` - The name of the function.
    /// * `ty_name` - The system type the function is associated with.
    /// * `sig` - The function signature.
    ///
    /// These functions are added to the `on_sys_type` field of the `ApplyDatabase` struct.
    fn builtin(&mut self, fn_name: &str, ty_name: SysTypeName, sig: TypeFn) {
        match self
            .on_sys_type
            .entry(UsrFuncName::intrinsic(fn_name)) // The fn_name is converted to UsrFuncName if it is recognized as an intrinsic function. Otherwise, it panics. See the UsrFuncName::intrinsic function for a list of recognized intrinsic functions.
            .or_default() // the entry gets the given key's corresponding entry in the map for in-place manipulation. The or_default() method returns a mutable reference to the value corresponding to the key if the key exists, and inserts the key with a default value if it doesn't. The default value for a BTreeMap is an empty map. So in case it doesn't exist, it will create mutable pointer to an empty map.
            .insert(ty_name, sig) // Insert the function signature.
        {
            None => (), // Successfully inserted.
            Some(_) => {
                // Duplicate entry found.
                // This happens if for a particular fn_name, the ty_name already exists in the map, but we are trying to insert it again. So the insert only concerns the map value associated with the key, not the key itself. Basically this happens when for a specific type, there exists multiple implementations of the same function.
                panic!("duplicated built-in: {ty_name}::{fn_name}");
            }
        }
    }

    /// Initializes the database with built-in functions (intrinsics).
    ///
    /// This method registers common functions for system types like `Integer`, `Boolean`, `Real`, `String`, `F32`, `F64`, `I32`, `I64`, `U32`, `U64`, `Cloak`, `Seq`, `Set`, `Array`, `Error`.
    ///
    /// # Returns
    ///
    /// An `ApplyDatabase` populated with intrinsic functions.
    /// This function is only called once - ApplyDatabase::with_intrinsics() - in the parse_func_sigs function of ctx.rs. It is used to populate the ApplyDatabase with intrinsic functions for system types. After that, the ApplyDatabase is populated with user-defined functions.
    pub fn with_intrinsics() -> Self {
        use SysTypeName as Q;
        use TypeTag::*;

        let mut db = Self::new();
        let empty = || Generics::intrinsic(vec![]);

        let fn0 = |rty: TypeTag| TypeFn {
            generics: empty(),
            params: vec![],
            ret_ty: rty,
        };

        let fn1 = |a0: TypeTag, rty: TypeTag| TypeFn {
            generics: empty(),
            params: vec![a0],
            ret_ty: rty,
        };

        let fn2 = |a0: TypeTag, a1: TypeTag, rty: TypeTag| TypeFn {
            generics: empty(),
            params: vec![a0, a1],
            ret_ty: rty,
        };

        let fn3 = |a0: TypeTag, a1: TypeTag, a2: TypeTag, rty: TypeTag| TypeFn {
            generics: empty(),
            params: vec![a0, a1, a2],
            ret_ty: rty,
        };

        let fn1_arith = |t: TypeTag| fn1(t.clone(), t);
        let fn2_arith = |t: TypeTag| fn2(t.clone(), t.clone(), t);
        let fn2_cmp = |t: TypeTag| fn2(t.clone(), t, Boolean);

        let t = || Parameter(TypeParamName::intrinsic("T"));
        let box_t = || Cloak(t().into());
        let seq_t = || Seq(t().into());
        let set_t = || Set(t().into());

        let k = || Parameter(TypeParamName::intrinsic("K"));
        let v = || Parameter(TypeParamName::intrinsic("V"));
        let map_kv = || Array(k().into(), v().into());

        db.builtin("not", Q::Boolean, fn1_arith(Boolean));
        db.builtin("and", Q::Boolean, fn2_arith(Boolean));
        db.builtin("or", Q::Boolean, fn2_arith(Boolean));
        db.builtin("xor", Q::Boolean, fn2_arith(Boolean));
        db.builtin("nand", Q::Boolean, fn2_arith(Boolean));
        db.builtin("nor", Q::Boolean, fn2_arith(Boolean));
        db.builtin("xnor", Q::Boolean, fn2_arith(Boolean));
        db.builtin("implies", Q::Boolean, fn2_arith(Boolean));
        db.builtin("iff", Q::Boolean, fn2_arith(Boolean));
        // `Boolean::ite` is function-generic (T), not type-generic.
        // T must be declared in generics so instantiate() can resolve it.
        db.builtin(
            "ite",
            Q::Boolean,
            TypeFn {
                generics: Generics::intrinsic(vec![TypeParamName::intrinsic("T")]),
                params: vec![Boolean, t(), t()],
                ret_ty: t(),
            },
        );

        db.builtin("neg", Q::Integer, fn1_arith(Integer));
        db.builtin("add", Q::Integer, fn2_arith(Integer));
        db.builtin("sub", Q::Integer, fn2_arith(Integer));
        db.builtin("mul", Q::Integer, fn2_arith(Integer));
        db.builtin("div", Q::Integer, fn2_arith(Integer));
        db.builtin("div_trunc", Q::Integer, fn2_arith(Integer));
        db.builtin("modulo", Q::Integer, fn2_arith(Integer));
        db.builtin("rem", Q::Integer, fn2_arith(Integer));
        db.builtin("pow", Q::Integer, fn2_arith(Integer));
        db.builtin("abs", Q::Integer, fn1_arith(Integer));
        db.builtin("divides", Q::Integer, fn2_cmp(Integer));
        db.builtin("lt", Q::Integer, fn2_cmp(Integer));
        db.builtin("le", Q::Integer, fn2_cmp(Integer));
        db.builtin("gt", Q::Integer, fn2_cmp(Integer));
        db.builtin("ge", Q::Integer, fn2_cmp(Integer));
        db.builtin("to_real", Q::Integer, fn1(Integer, Real));
        db.builtin("to_i32", Q::Integer, fn1(Integer, I32));
        db.builtin("to_i64", Q::Integer, fn1(Integer, I64));
        db.builtin("to_u32", Q::Integer, fn1(Integer, U32));
        db.builtin("to_u64", Q::Integer, fn1(Integer, U64));
        db.builtin("to_f32", Q::Integer, fn1(Integer, F32));
        db.builtin("to_f64", Q::Integer, fn1(Integer, F64));
        db.builtin("from_hex_str", Q::Integer, fn1(String, Integer));
        db.builtin("from_oct_str", Q::Integer, fn1(String, Integer));
        db.builtin("from_bin_str", Q::Integer, fn1(String, Integer));
        db.builtin("is_gt_i64_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_i64_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_u64_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_u64_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_i32_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_i32_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_u32_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_u32_max", Q::Integer, fn1(Integer, Boolean));

        db.builtin("neg", Q::Real, fn1_arith(Real));
        db.builtin("add", Q::Real, fn2_arith(Real));
        db.builtin("sub", Q::Real, fn2_arith(Real));
        db.builtin("mul", Q::Real, fn2_arith(Real));
        db.builtin("div", Q::Real, fn2_arith(Real));
        db.builtin("pow", Q::Real, fn2_arith(Real));
        db.builtin("abs", Q::Real, fn1_arith(Real));
        db.builtin("round", Q::Real, fn1(Real, Integer));
        db.builtin("floor", Q::Real, fn1(Real, Integer));
        db.builtin("ceil", Q::Real, fn1(Real, Integer));
        db.builtin("is_integer", Q::Real, fn1(Real, Boolean));
        db.builtin("lt", Q::Real, fn2_cmp(Real));
        db.builtin("le", Q::Real, fn2_cmp(Real));
        db.builtin("gt", Q::Real, fn2_cmp(Real));
        db.builtin("ge", Q::Real, fn2_cmp(Real));
        db.builtin("to_int", Q::Real, fn1(Real, Integer));
        db.builtin("to_f32", Q::Real, fn1(Real, F32));
        db.builtin("to_f64", Q::Real, fn1(Real, F64));

        db.builtin("new", Q::String, fn0(String));
        db.builtin("length", Q::String, fn1(String, Integer));
        db.builtin("concat", Q::String, fn2_arith(String));
        db.builtin("at", Q::String, fn2(String, Integer, String));
        db.builtin("index_of", Q::String, fn3(String, String, Integer, Integer));
        db.builtin("index_of_default", Q::String, fn2(String, String, Integer));
        db.builtin("substr", Q::String, fn3(String, Integer, Integer, String));
        db.builtin("is_empty", Q::String, fn1(String, Boolean));
        db.builtin("contains", Q::String, fn2_cmp(String));
        db.builtin("starts_with", Q::String, fn2_cmp(String));
        db.builtin("ends_with", Q::String, fn2_cmp(String));
        db.builtin("is_digit", Q::String, fn1(String, Boolean));
        db.builtin("lt", Q::String, fn2_cmp(String));
        db.builtin("le", Q::String, fn2_cmp(String));
        db.builtin("gt", Q::String, fn2_cmp(String));
        db.builtin("ge", Q::String, fn2_cmp(String));
        db.builtin("replace", Q::String, fn3(String, String, String, String));
        db.builtin(
            "replace_all",
            Q::String,
            fn3(String, String, String, String),
        );
        db.builtin("to_int", Q::String, fn1(String, Integer));
        db.builtin("from_int", Q::String, fn1(Integer, String));
        db.builtin("from_code", Q::String, fn1(U32, String));
        db.builtin("to_code", Q::String, fn1(String, U32));

        db.builtin("shield", Q::Cloak, fn1(t(), box_t()));
        db.builtin("reveal", Q::Cloak, fn1(box_t(), t()));

        db.builtin("new", Q::Seq, fn0(seq_t()));
        db.builtin("unit", Q::Seq, fn1(t(), seq_t()));
        db.builtin("length", Q::Seq, fn1(seq_t(), Integer));
        db.builtin("append", Q::Seq, fn2(seq_t(), t(), seq_t()));
        db.builtin("concat", Q::Seq, fn2(seq_t(), seq_t(), seq_t()));
        db.builtin("at", Q::Seq, fn2(seq_t(), Integer, t()));
        db.builtin("at_seq", Q::Seq, fn2(seq_t(), Integer, seq_t()));
        db.builtin("extract", Q::Seq, fn3(seq_t(), Integer, Integer, seq_t()));
        db.builtin("index_of", Q::Seq, fn3(seq_t(), seq_t(), Integer, Integer));
        db.builtin("index_of_default", Q::Seq, fn2(seq_t(), seq_t(), Integer));
        db.builtin("contains", Q::Seq, fn2(seq_t(), t(), Boolean));
        db.builtin("prefix_of", Q::Seq, fn2(seq_t(), seq_t(), Boolean));
        db.builtin("suffix_of", Q::Seq, fn2(seq_t(), seq_t(), Boolean));
        db.builtin("replace", Q::Seq, fn3(seq_t(), t(), t(), seq_t()));
        db.builtin("is_empty", Q::Seq, fn1(seq_t(), Boolean));

        db.builtin("new", Q::Set, fn0(set_t()));
        db.builtin("length", Q::Set, fn1(set_t(), Integer));
        db.builtin("insert", Q::Set, fn2(set_t(), t(), set_t()));
        db.builtin("remove", Q::Set, fn2(set_t(), t(), set_t()));
        db.builtin("contains", Q::Set, fn2(set_t(), t(), Boolean));
        db.builtin("is_empty", Q::Set, fn1(set_t(), Boolean));
        db.builtin("intersection", Q::Set, fn2(set_t(), set_t(), set_t()));
        db.builtin("union", Q::Set, fn2(set_t(), set_t(), set_t()));
        db.builtin("difference", Q::Set, fn2(set_t(), set_t(), set_t()));
        db.builtin(
            "symmetric_difference",
            Q::Set,
            fn2(set_t(), set_t(), set_t()),
        );
        db.builtin("is_subset", Q::Set, fn2(set_t(), set_t(), Boolean));
        db.builtin("is_proper_subset", Q::Set, fn2(set_t(), set_t(), Boolean));
        db.builtin("is_disjoint", Q::Set, fn2(set_t(), set_t(), Boolean));
        db.builtin("has_size", Q::Set, fn2(set_t(), Integer, Boolean));

        db.builtin("new", Q::Array, fn0(map_kv()));
        db.builtin("length", Q::Array, fn1(map_kv(), Integer));
        db.builtin("store", Q::Array, fn3(map_kv(), k(), v(), map_kv()));
        db.builtin("select", Q::Array, fn2(map_kv(), k(), v()));
        db.builtin("del", Q::Array, fn2(map_kv(), k(), map_kv()));
        db.builtin("contains_key", Q::Array, fn2(map_kv(), k(), Boolean));
        db.builtin("is_empty", Q::Array, fn1(map_kv(), Boolean));

        for &q in &[Q::I32, Q::I64, Q::U32, Q::U64] {
            let ty = match q {
                Q::I32 => I32,
                Q::I64 => I64,
                Q::U32 => U32,
                Q::U64 => U64,
                _ => unreachable!(),
            };
            db.builtin("bv_not", q, fn1_arith(ty.clone()));
            db.builtin("bv_and", q, fn2_arith(ty.clone()));
            db.builtin("bv_or", q, fn2_arith(ty.clone()));
            db.builtin("bv_xor", q, fn2_arith(ty.clone()));
            db.builtin("bv_nand", q, fn2_arith(ty.clone()));
            db.builtin("bv_nor", q, fn2_arith(ty.clone()));
            db.builtin("bv_xnor", q, fn2_arith(ty.clone()));
            db.builtin("bv_redand", q, fn1(ty.clone(), Boolean));
            db.builtin("bv_redor", q, fn1(ty.clone(), Boolean));
            db.builtin("bv_neg", q, fn1_arith(ty.clone()));
            db.builtin("bv_add", q, fn2_arith(ty.clone()));
            db.builtin("bv_sub", q, fn2_arith(ty.clone()));
            db.builtin("bv_mul", q, fn2_arith(ty.clone()));
            db.builtin("bv_div", q, fn2_arith(ty.clone()));
            db.builtin("bv_rem", q, fn2_arith(ty.clone()));
            db.builtin("bv_mod", q, fn2_arith(ty.clone()));
            db.builtin("bv_shl", q, fn2_arith(ty.clone()));
            db.builtin("bv_lshr", q, fn2_arith(ty.clone()));
            db.builtin("bv_ashr", q, fn2_arith(ty.clone()));
            db.builtin("bv_rotate_left", q, fn2_arith(ty.clone()));
            db.builtin("bv_rotate_right", q, fn2_arith(ty.clone()));
            db.builtin("bv_lt", q, fn2_cmp(ty.clone()));
            db.builtin("bv_le", q, fn2_cmp(ty.clone()));
            db.builtin("bv_gt", q, fn2_cmp(ty.clone()));
            db.builtin("bv_ge", q, fn2_cmp(ty.clone()));
            db.builtin("to_int", q, fn1(ty, Integer));
        }

        for &q in &[Q::F32, Q::F64] {
            let ty = if q == Q::F32 { F32 } else { F64 };
            db.builtin("add", q, fn2_arith(ty.clone()));
            db.builtin("sub", q, fn2_arith(ty.clone()));
            db.builtin("mul", q, fn2_arith(ty.clone()));
            db.builtin("div", q, fn2_arith(ty.clone()));
            db.builtin("neg", q, fn1_arith(ty.clone()));
            db.builtin("abs", q, fn1_arith(ty.clone()));
            db.builtin("rem", q, fn2_arith(ty.clone()));
            db.builtin("sqrt", q, fn1_arith(ty.clone()));
            db.builtin("min", q, fn2_arith(ty.clone()));
            db.builtin("max", q, fn2_arith(ty.clone()));
            db.builtin("is_nan", q, fn1(ty.clone(), Boolean));
            db.builtin("is_infinite", q, fn1(ty.clone(), Boolean));
            db.builtin("is_zero", q, fn1(ty.clone(), Boolean));
            db.builtin("is_normal", q, fn1(ty.clone(), Boolean));
            db.builtin("is_subnormal", q, fn1(ty.clone(), Boolean));
            db.builtin("is_negative", q, fn1(ty.clone(), Boolean));
            db.builtin("is_positive", q, fn1(ty.clone(), Boolean));
            db.builtin("lt", q, fn2_cmp(ty.clone()));
            db.builtin("le", q, fn2_cmp(ty.clone()));
            db.builtin("gt", q, fn2_cmp(ty.clone()));
            db.builtin("ge", q, fn2_cmp(ty.clone()));
            db.builtin("nan", q, fn0(ty.clone()));
            db.builtin("infinity", q, fn0(ty.clone()));
            db.builtin("neg_infinity", q, fn0(ty.clone()));
            db.builtin("pos_zero", q, fn0(ty.clone()));
            db.builtin("neg_zero", q, fn0(ty.clone()));
            db.builtin("to_integer", q, fn1(ty.clone(), Integer));
            db.builtin("to_real", q, fn1(ty.clone(), Real));
            db.builtin("to_u32", q, fn1(ty.clone(), U32));
            db.builtin("to_i32", q, fn1(ty.clone(), I32));
            db.builtin("to_u64", q, fn1(ty.clone(), U64));
            db.builtin("to_i64", q, fn1(ty.clone(), I64));
            db.builtin("ceil", q, fn1(ty.clone(), ty.clone()));
            db.builtin("floor", q, fn1(ty.clone(), ty.clone()));
            db.builtin("trunc", q, fn1(ty.clone(), ty.clone()));
            db.builtin("nearest", q, fn1(ty.clone(), ty.clone()));
            db.builtin("fp_eq", q, fn2_cmp(ty.clone()));
            db.builtin("from_hex_str", q, fn1(String, ty));
        }

        db.builtin("fresh", Q::Error, fn0(Error));
        db.builtin("merge", Q::Error, fn2_arith(Error));

        db
    }

    /// Registers a user-defined function.
    pub fn register_user_func(&mut self, name: &UsrFuncName, sig: &FuncSig) -> Result<()> {
        // Register the function as unqualified (standalone function).
        let func = TypeFn::new_from_sig(sig); // builds a TypeFn from the function signature.

        // sanity check that the function is not in the intrinsics database
        if self.lookup_unqualified(name).is_some() {
            panic!("duplicated registration of user-defined function: {name}");
        }
        // if not, insert the function into the unqualified database
        match self.unqualified.insert(name.clone(), func) {
            None => (), // Successfully inserted.
            Some(_) => panic!("duplicated registration of user-defined function: {name}"),
        }

        Ok(()) // Return success.
    }

    /// Looks up an unqualified user function by name.
    pub fn lookup_unqualified(&self, fn_name: &UsrFuncName) -> Option<&TypeFn> {
        self.unqualified.get(fn_name)
    }

    /// Looks up a user function on a system type by name.
    pub fn lookup_usr_func_on_sys_type(
        &self,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.on_sys_type.get(fn_name).and_then(|s| s.get(ty_name))
    }

    /// Queries for a function with type inference, given a function name and arguments.
    pub fn query_with_inference(
        &self,
        unifier: &mut TypeUnifier,
        name: &UsrFuncName,
        inst: Option<&[TypeTag]>,
        args: Vec<Expr>,
        rval: &TypeRef,
    ) -> Result<Op> {
        // Collect candidate intrinsic functions (defined on system types) matching the name.
        let candidates: Vec<(&SysTypeName, &TypeFn)> = self
            .on_sys_type
            .get(name)
            .map(|options| options.iter().collect())
            .unwrap_or_default();

        // Variable to hold a suitable candidate if found.
        let mut suitable = None;
        // sys_name is the system type the method is defined on
        // fty is the function signature
        for (sys_name, fty) in candidates {
            // Early filter by number of parameters.
            // the length of the parameters of the function signature should be the same as the length of the arguments. If they are not the same, we move to the next candidate.
            if fty.params.len() != args.len() {
                continue;
            }

            // Instantiate type parameters for the system type the method is defined on.
            let ty_inst = GenericsInstPartial::new_without_args(&sys_name.generics());
            // Instantiate function generics, possibly using provided type arguments.
            // fn_inst is the generics of the function signature.
            let fn_inst = match inst {
                // inst is the turbufish type arguments provided to the function call.
                None => GenericsInstPartial::new_without_args(&fty.generics), // the turbofish is for the function arguments.
                Some(tags) => match GenericsInstPartial::try_with_args(&fty.generics, tags) {
                    None => continue, // Type arguments don't match.
                    Some(inst) => inst,
                },
            };

            // Use a probing unifier to check type compatibility without affecting the main unifier.
            let mut probing = unifier.clone();

            // Merge type and function instantiations.
            let inst_full = match ty_inst
                .complete(&mut probing)
                .merge(&fn_inst.complete(&mut probing))
            {
                None => bail!("[invariant] conflicting type parameter name"),
                Some(inst) => inst,
            };

            // Instantiate the function parameters and return type.
            // we convert the parameters and return type of the function signature to the actual types. If there are any type parameters in the function signature, we replace them with the actual types taken from the type arguments provided to the function call.
            let (params, ret_ty) = match fty.instantiate(&inst_full) {
                None => bail!("no such type parameter"),
                Some(instantiated) => instantiated,
            };

            // Attempt to unify each argument type with the corresponding parameter type.
            let mut unified = true;
            for (param_ty, arg) in params.iter().zip(args.iter()) {
                // updates the probing unifier with the unification of the parameter type and the argument type. Note that the arguments were all tentatively converted to ADT Exprs with type variables.
                match probing.unify(param_ty, arg.ty()) {
                    Ok(Some(_)) => {
                        // Successfully unified.
                    }
                    Ok(None) => {
                        // Types do not unify.
                        unified = false;
                        break;
                    }
                    Err(TIError::CyclicUnification) => bail!("cyclic type unification"),
                };
            }
            if !unified {
                continue; // Move to the next candidate.
            }

            // Attempt to unify the return type.
            match probing.unify(&ret_ty, rval) {
                Ok(Some(_)) => {
                    // Successfully unified.
                }
                Ok(None) => {
                    // Return types do not unify.
                    continue;
                }
                Err(TIError::CyclicUnification) => bail!("cyclic type unification"),
            }

            // Check for multiple suitable candidates (ambiguity).
            if suitable.is_some() {
                bail!("more than one candidate match the method call");
            }
            suitable = Some((*sys_name, probing, inst_full)); // Store the suitable candidate.
        }

        // Ensure a suitable function was found.
        let (sys_name, probing, inst_full) = match suitable {
            None => bail!("no candidates matches the method call"),
            Some(matched) => matched,
        };

        // Update the main unifier with the successful unification.
        *unifier = probing;

        // Extract type arguments from the type's generics (not function's generics).
        // For intrinsic functions on generic types, type parameters come from the type.
        let ty_generics = sys_name.generics();
        let ty_args = match inst_full.vec_for_generics(&ty_generics) {
            None => bail!("missing type parameter in instantiation"),
            Some(args) => args,
        };
        let intrinsic = Intrinsic::new(&sys_name, name, ty_args, args)?;
        Ok(Op::Intrinsic(Box::new(intrinsic)))
    }
}

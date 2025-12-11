//! This module defines the `ApplyDatabase` struct, which is used to store function signatures and their types.

use crate::parser::expr::{CtxtForExpr, Expr, Op};
use crate::parser::func::FuncSig;
use crate::parser::generics::{Generics, GenericsInstFull, GenericsInstPartial};
use crate::parser::infer::{TIError, TypeRef, TypeUnifier};
use crate::parser::intrinsics::Intrinsic;
use crate::parser::name::{TypeParamName, UsrFuncName, UsrTypeName};
use crate::parser::ty::{SysTypeName, TypeName, TypeTag};
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

    /// Instantiates a function type by substituting generics with concrete types refs.
    ///
    /// # Arguments
    ///
    /// * `subst` - A full mapping from generic parameters to concrete types.
    ///
    /// GenericsInstFull is a struct that holds a full mapping from generic parameters to concrete types.
    /// pub struct GenericsInstFull {
    ///     args: BTreeMap<TypeParamName, (usize, TypeRef)>,
    /// }
    ///
    /// # Returns
    ///
    /// An `Option` containing a tuple of instantiated parameter types and return type. By instantiating, we mean converting the TypeTag to TypeRef.
    ///
    /// This function returns None if any of the type tags contain a type parameter that is not assigned; meaning that the type parameter is not present in the GenericsInstFull. If it is present, it is replaced with the assigned type reference. Through this process, all TypeTags in the parameter list and return type are converted to TypeRefs.
    pub fn instantiate(&self, subst: &GenericsInstFull) -> Option<(Vec<TypeRef>, TypeRef)> {
        // Instantiate parameter types.
        let params: Vec<TypeRef> = self
            .params
            .iter()
            .map(|t| subst.instantiate(t))
            .collect::<Option<Vec<TypeRef>>>()?;
        // collect() is a method provided by Rust's Iterator trait that transforms an iterator into a collection, such as a Vec, HashMap, an Option, etc.
        // The ::<Option<_>> part is a type hint that tells the compiler to collect the iterator into an <Option<Vec<TypeRef>>> type. The ? operator then unwraps the Option, returning the value inside if it is Some, or returning None if it is None.
        // The Option<_> type means that collect() will return Some(collection) if all elements of the iterator successfully produce Some(value). None if any element produces None.
        // So each element is moved out of the Option (subst.instantiate(t) gives Option<TypeRef>) but the whole collection is moved into the outer Option.
        // Instantiate return type.
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
    /// User-defined functions with a user-defined type qualifier (methods on custom types).
    /// The `UsrFuncName` is the name of the method. A method can be implemented for multiple types. So the value is a map from the method name to a map. The value map is from the user-defined type name to their respective arguments (Vec<TypeTag>) and function signature (TypeFn).
    pub on_usr_type: BTreeMap<UsrFuncName, BTreeMap<UsrTypeName, (Vec<TypeTag>, TypeFn)>>,
}

impl ApplyDatabase {
    /// Creates a new, empty `ApplyDatabase`.
    fn new() -> Self {
        Self {
            unqualified: BTreeMap::new(),
            on_sys_type: BTreeMap::new(),
            on_usr_type: BTreeMap::new(),
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
    /// This method registers common functions for system types like `Integer`, `Boolean`, etc.
    ///
    /// # Returns
    ///
    /// An `ApplyDatabase` populated with intrinsic functions.
    /// This function is only called once - ApplyDatabase::with_intrinsics() - in the parse_func_sigs function of ctx.rs. It is used to populate the ApplyDatabase with intrinsic functions for system types. After that, the ApplyDatabase is populated with user-defined functions & methods.
    pub fn with_intrinsics() -> Self {
        use SysTypeName as Q; // Alias for system type names.
        use TypeTag::*; // Import type tags.
        // Note the similarities and differences between SysTypeName and TypeTag. SysTypeName is for system types like Integer, Boolean, Real, String, F32, F64, I32, I64, U32, U64, Cloak, Seq, Set, Array, Error. TypeTag is for all types including system types, user-defined types, type parameters, and tuples. Also in Seq for example, in SysTypeName, it is just Seq. But in TypeTag, it is Seq(Box<TypeTag>). The reason is that in TypeTag, we need to specify the type of the elements in the sequence. But in SysTypeName, we just want to say that Seq is a system reserved type.

        // Initialize the database.
        let mut db = Self::new();

        // Utility closure for initiating an empty generics.
        let empty = || Generics::intrinsic(vec![]);

        // Utility closures for creating function types. fn0 is a nullary function, fn1 is a unary function, fn2 is a binary function, fn3 is a ternary function. They are all implementation functions.
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

        let fn1_arith = |t: TypeTag| fn1(t.clone(), t); // Unary function used for negation of Booleans. It takes a type and returns the same type.
        let fn2_arith = |t: TypeTag| fn2(t.clone(), t.clone(), t); // Binary function. It takes two types and returns the same type.
        let fn2_cmp = |t: TypeTag| fn2(t.clone(), t, Boolean); // Comparison function. It takes two types and returns a Boolean. This is used for `lt`, `le`, `ge`, `gt` operations.

        // Type parameters for generics.
        let t = || Parameter(TypeParamName::intrinsic("T")); // gives a Parameter(TypeParamName { ident: String::from("T") }) when called.
        let box_t = || Cloak(t().into()); // gives a Cloak(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.
        let seq_t = || Seq(t().into()); // gives a Seq(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.
        let set_t = || Set(t().into()); // gives a Set(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.

        let k = || Parameter(TypeParamName::intrinsic("K"));
        let v = || Parameter(TypeParamName::intrinsic("V"));
        let map_kv = || Array(k().into(), v().into()); // gives a Array(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") }))) when called.

        // Boolean operations.
        db.builtin("not", Q::Boolean, fn1_arith(Boolean)); // `not` is for Boolean type. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Boolean], ret_ty: Boolean, } so the parameter is a Boolean and the return type is a Boolean.
        db.builtin("and", Q::Boolean, fn2_arith(Boolean)); // `and` is for Boolean type. It is a binary arithmetic function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Boolean, Boolean], ret_ty: Boolean, } so the parameters are two Booleans and the return type is a Boolean.
        db.builtin("or", Q::Boolean, fn2_arith(Boolean)); // `or` is the same as `and` but for the `or` operation.
        db.builtin("xor", Q::Boolean, fn2_arith(Boolean)); // `xor` is the same as `and` but for the `xor` operation.
        db.builtin("implies", Q::Boolean, fn2_arith(Boolean)); // `implies` is the same as `and` but for the `implies` operation.
        db.builtin("iff", Q::Boolean, fn2_arith(Boolean)); // `iff` is the same as `and` but for the `iff` operation.
        db.builtin("nand", Q::Boolean, fn2_arith(Boolean)); // `nand` is the same as `and` but for the `nand` operation.
        db.builtin("nor", Q::Boolean, fn2_arith(Boolean)); // `nor` is the same as `and` but for the `nor` operation.
        db.builtin("xnor", Q::Boolean, fn2_arith(Boolean)); // `xnor` is the same as `and` but for the `xnor` operation.

        // Integer arithmetic and comparison.
        // add, sub, mul, div, rem, pow are arithmetic operations. They are binary arithmetic functions, with the signature TypeFn { generics: Generics { params: [], }, params: [Integer, Integer], ret_ty: Integer, } so the parameters are two Integers and the return type is an Integer.
        // lt, le, ge, gt are comparison operations. They are binary comparison functions, with the signature TypeFn { generics: Generics { params: [], }, params: [Integer, Integer], ret_ty: Boolean, } so the parameters are two Integers and the return type is a Boolean.
        // Integer arithmetic operations.
        db.builtin("add", Q::Integer, fn2_arith(Integer)); // Binary: Integer + Integer -> Integer
        db.builtin("sub", Q::Integer, fn2_arith(Integer)); // Binary: Integer - Integer -> Integer
        db.builtin("mul", Q::Integer, fn2_arith(Integer)); // Binary: Integer * Integer -> Integer
        db.builtin("div", Q::Integer, fn2_arith(Integer)); // Binary: Integer / Integer -> Integer
        db.builtin("mod", Q::Integer, fn2_arith(Integer)); // Binary: Integer % Integer -> Integer (modulo)
        db.builtin("rem", Q::Integer, fn2_arith(Integer)); // Binary: Integer % Integer -> Integer (remainder)
        db.builtin("pow", Q::Integer, fn2_arith(Integer)); // Binary: Integer ^ Integer -> Integer
        db.builtin("neg", Q::Integer, fn1_arith(Integer)); // Unary: -Integer -> Integer
        db.builtin("abs", Q::Integer, fn1_arith(Integer)); // Unary: abs(Integer) -> Integer

        // Integer comparison operations.
        db.builtin("lt", Q::Integer, fn2_cmp(Integer)); // Binary: Integer < Integer -> Boolean
        db.builtin("le", Q::Integer, fn2_cmp(Integer)); // Binary: Integer <= Integer -> Boolean
        db.builtin("gt", Q::Integer, fn2_cmp(Integer)); // Binary: Integer > Integer -> Boolean
        db.builtin("ge", Q::Integer, fn2_cmp(Integer)); // Binary: Integer >= Integer -> Boolean
        db.builtin("divides", Q::Integer, fn2_cmp(Integer)); // Binary: checks if arg1 divides arg2 -> Boolean

        // Type conversions (Integer -> Other).
        db.builtin("to_real", Q::Integer, fn1(Integer, Real)); // Unary: Integer -> Real
        db.builtin("to_i32", Q::Integer, fn1(Integer, I32)); // Unary: Integer -> I32
        db.builtin("to_i64", Q::Integer, fn1(Integer, I64)); // Unary: Integer -> I64
        db.builtin("to_u32", Q::Integer, fn1(Integer, U32)); // Unary: Integer -> U32
        db.builtin("to_u64", Q::Integer, fn1(Integer, U64)); // Unary: Integer -> U64
        db.builtin("to_f32", Q::Integer, fn1(Integer, F32)); // Unary: Integer -> F32
        db.builtin("to_f64", Q::Integer, fn1(Integer, F64)); // Unary: Integer -> F64

        // String parsing constructors (String -> Integer).
        // These take a String and return an Integer.
        db.builtin("from_hex_str", Q::Integer, fn1(String, Integer));
        db.builtin("from_oct_str", Q::Integer, fn1(String, Integer));
        db.builtin("from_bin_str", Q::Integer, fn1(String, Integer));

        // Range checks (Integer -> Boolean).
        db.builtin("is_gt_i64_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_i64_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_u64_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_u64_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_i32_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_i32_min", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_gt_u32_max", Q::Integer, fn1(Integer, Boolean));
        db.builtin("is_lt_u32_min", Q::Integer, fn1(Integer, Boolean));

        // Real arithmetic operations.
        db.builtin("add", Q::Real, fn2_arith(Real)); // Binary: Real + Real -> Real
        db.builtin("sub", Q::Real, fn2_arith(Real)); // Binary: Real - Real -> Real
        db.builtin("mul", Q::Real, fn2_arith(Real)); // Binary: Real * Real -> Real
        db.builtin("div", Q::Real, fn2_arith(Real)); // Binary: Real / Real -> Real
        db.builtin("pow", Q::Real, fn2_arith(Real)); // Binary: Real ^ Real -> Real
        db.builtin("neg", Q::Real, fn1_arith(Real)); // Unary: -Real -> Real
        db.builtin("abs", Q::Real, fn1_arith(Real)); // Unary: abs(Real) -> Real

        // Real comparison operations.
        db.builtin("lt", Q::Real, fn2_cmp(Real)); // Binary: Real < Real -> Boolean
        db.builtin("le", Q::Real, fn2_cmp(Real)); // Binary: Real <= Real -> Boolean
        db.builtin("gt", Q::Real, fn2_cmp(Real)); // Binary: Real > Real -> Boolean
        db.builtin("ge", Q::Real, fn2_cmp(Real)); // Binary: Real >= Real -> Boolean

        // Rounding and Conversion to Integer (Real -> Integer).
        db.builtin("round", Q::Real, fn1(Real, Integer));
        db.builtin("floor", Q::Real, fn1(Real, Integer));
        db.builtin("ceil", Q::Real, fn1(Real, Integer));
        db.builtin("to_int", Q::Real, fn1(Real, Integer));

        // Fraction Inspection (Real -> Integer).
        db.builtin("numerator", Q::Real, fn1(Real, Integer));
        db.builtin("denominator", Q::Real, fn1(Real, Integer));

        // Checks (Real -> Boolean).
        db.builtin("is_integer", Q::Real, fn1(Real, Boolean));

        // Conversion to Floating Point (Real -> Float).
        db.builtin("to_f32", Q::Real, fn1(Real, F32));
        db.builtin("to_f64", Q::Real, fn1(Real, F64));

        // Text
        // String Construction and Basic Operations
        db.builtin("new", Q::String, fn0(String)); // new() -> String (empty string)
        db.builtin("concat", Q::String, fn2_arith(String)); // Binary: String ++ String -> String
        db.builtin("length", Q::String, fn1(String, Integer)); // Unary: length(String) -> Integer
        db.builtin("is_empty", Q::String, fn1(String, Boolean)); // Unary: is_empty(String) -> Boolean

        // String Comparisons (Lexicographical)
        db.builtin("lt", Q::String, fn2_cmp(String)); // Binary: String < String -> Boolean
        db.builtin("le", Q::String, fn2_cmp(String)); // Binary: String <= String -> Boolean
        db.builtin("gt", Q::String, fn2_cmp(String)); // Binary: String > String -> Boolean
        db.builtin("ge", Q::String, fn2_cmp(String)); // Binary: String >= String -> Boolean

        // Substring Checks
        db.builtin("contains", Q::String, fn2_cmp(String)); // Binary: contains(String, String) -> Boolean
        db.builtin("starts_with", Q::String, fn2_cmp(String)); // Binary: starts_with(String, String) -> Boolean
        db.builtin("ends_with", Q::String, fn2_cmp(String)); // Binary: ends_with(String, String) -> Boolean

        // at(self, index) -> char_as_string
        db.builtin("at", Q::String, fn2(String, Integer, String));
        // index_of(self, substr, offset) -> index
        db.builtin("index_of", Q::String, fn3(String, String, Integer, Integer));
        // replace(self, src, dst) -> string
        db.builtin("replace", Q::String, fn3(String, String, String, String));

        // replace_all(self, src, dst) -> string
        db.builtin(
            "replace_all",
            Q::String,
            fn3(String, String, String, String),
        );

        // Type Conversions (String <-> Integer/U32)
        db.builtin("to_int", Q::String, fn1(String, Integer)); // Unary: String -> Integer
        db.builtin("from_int", Q::String, fn1(Integer, String)); // Static: Integer -> String
        db.builtin("to_code", Q::String, fn1(String, U32)); // Unary: String -> U32 (codepoint)
        db.builtin("from_code", Q::String, fn1(U32, String)); // Static: U32 -> String
        db.builtin("is_digit", Q::String, fn1(String, Boolean)); // Unary: String -> Boolean

        // Cloak type operations (e.g., for encapsulation).
        // The `shield` function is a unary function that takes a type and returns a Cloak type. The TypeFn { generics: Generics { params: [], }, params: [T], ret_ty: Cloak(T), } so the parameter is a type Parameter(TypeParamName {ident: String::from("T")}) and the return type is a Cloak(Box(Parameter(TypeParamName {ident: String::from("T")})))
        // `reveal` is a unary function with the oppsite signature of `shield`.
        db.builtin("shield", Q::Cloak, fn1(t(), box_t()));
        db.builtin("reveal", Q::Cloak, fn1(box_t(), t()));

        // Sequence operations.
        let gen_t = || Generics::intrinsic(vec![TypeParamName::intrinsic("T")]);
        // new<T>() -> Seq<T>
        db.builtin(
            "new",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![],
                ret_ty: seq_t(),
            },
        );
        // unit<T>(e: T) -> Seq<T>
        db.builtin(
            "unit",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![t()],
                ret_ty: seq_t(),
            },
        );
        // length<T>(s: Seq<T>) -> Integer
        db.builtin(
            "length",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t()],
                ret_ty: Integer,
            },
        );
        // is_empty<T>(s: Seq<T>) -> Boolean
        db.builtin(
            "is_empty",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t()],
                ret_ty: Boolean,
            },
        );
        // append<T>(s: Seq<T>, e: T) -> Seq<T>
        db.builtin(
            "append",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), t()],
                ret_ty: seq_t(),
            },
        );
        // concat<T>(s1: Seq<T>, s2: Seq<T>) -> Seq<T>
        db.builtin(
            "concat",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), seq_t()],
                ret_ty: seq_t(),
            },
        );
        // replace<T>(s: Seq<T>, src: T, dst: T) -> Seq<T>
        db.builtin(
            "replace",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), t(), t()],
                ret_ty: seq_t(),
            },
        );
        // at<T>(s: Seq<T>, i: Integer) -> T
        db.builtin(
            "at",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), Integer],
                ret_ty: t(),
            },
        );
        // at_seq<T>(s: Seq<T>, i: Integer) -> Seq<T> (Returns singleton sequence)
        db.builtin(
            "at_seq",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), Integer],
                ret_ty: seq_t(),
            },
        );
        // extract<T>(s: Seq<T>, offset: Integer, length: Integer) -> Seq<T>
        db.builtin(
            "extract",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), Integer, Integer],
                ret_ty: seq_t(),
            },
        );
        // contains<T>(s: Seq<T>, e: T) -> Boolean
        db.builtin(
            "contains",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), t()],
                ret_ty: Boolean,
            },
        );
        // prefix_of<T>(self: Seq<T>, other: Seq<T>) -> Boolean
        db.builtin(
            "prefix_of",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), seq_t()],
                ret_ty: Boolean,
            },
        );
        // suffix_of<T>(self: Seq<T>, other: Seq<T>) -> Boolean
        db.builtin(
            "suffix_of",
            Q::Seq,
            TypeFn {
                generics: gen_t(),
                params: vec![seq_t(), seq_t()],
                ret_ty: Boolean,
            },
        );

        // Set operations.
        // new<T>() -> Set<T>
        db.builtin(
            "new",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![],
                ret_ty: set_t(),
            },
        );
        // insert<T>(s: Set<T>, e: T) -> Set<T>
        db.builtin(
            "insert",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), t()],
                ret_ty: set_t(),
            },
        );
        // remove<T>(s: Set<T>, e: T) -> Set<T>
        db.builtin(
            "remove",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), t()],
                ret_ty: set_t(),
            },
        );
        // union<T>(s1: Set<T>, s2: Set<T>) -> Set<T>
        db.builtin(
            "union",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: set_t(),
            },
        );
        // intersection<T>(s1: Set<T>, s2: Set<T>) -> Set<T>
        db.builtin(
            "intersection",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: set_t(),
            },
        );
        // difference<T>(s1: Set<T>, s2: Set<T>) -> Set<T>
        db.builtin(
            "difference",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: set_t(),
            },
        );
        // symmetric_difference<T>(s1: Set<T>, s2: Set<T>) -> Set<T>
        db.builtin(
            "symmetric_difference",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: set_t(),
            },
        );
        // length<T>(s: Set<T>) -> Integer
        db.builtin(
            "length",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t()],
                ret_ty: Integer,
            },
        );
        // is_empty<T>(s: Set<T>) -> Boolean
        db.builtin(
            "is_empty",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t()],
                ret_ty: Boolean,
            },
        );
        // contains<T>(s: Set<T>, e: T) -> Boolean
        db.builtin(
            "contains",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), t()],
                ret_ty: Boolean,
            },
        );
        // is_subset<T>(self: Set<T>, other: Set<T>) -> Boolean
        db.builtin(
            "is_subset",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: Boolean,
            },
        );
        // is_proper_subset<T>(self: Set<T>, other: Set<T>) -> Boolean
        db.builtin(
            "is_proper_subset",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: Boolean,
            },
        );
        // is_superset<T>(self: Set<T>, other: Set<T>) -> Boolean
        db.builtin(
            "is_superset",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: Boolean,
            },
        );
        // is_disjoint<T>(self: Set<T>, other: Set<T>) -> Boolean
        db.builtin(
            "is_disjoint",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), set_t()],
                ret_ty: Boolean,
            },
        );
        // has_size<T>(self: Set<T>, k: Integer) -> Boolean
        db.builtin(
            "has_size",
            Q::Set,
            TypeFn {
                generics: gen_t(),
                params: vec![set_t(), Integer],
                ret_ty: Boolean,
            },
        );

        // Array operations.
        // Helper to define the generic parameter list <K, V>
        let gen_kv = || {
            Generics::intrinsic(vec![
                TypeParamName::intrinsic("K"),
                TypeParamName::intrinsic("V"),
            ])
        };
        // new<K, V>() -> Array<K, V>
        db.builtin(
            "new",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![],
                ret_ty: map_kv(),
            },
        );
        // store<K, V>(arr: Array<K, V>, k: K, v: V) -> Array<K, V>
        db.builtin(
            "store",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv(), k(), v()],
                ret_ty: map_kv(),
            },
        );
        // del<K, V>(arr: Array<K, V>, k: K) -> Array<K, V>
        db.builtin(
            "del",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv(), k()],
                ret_ty: map_kv(),
            },
        );
        // select<K, V>(arr: Array<K, V>, k: K) -> V
        db.builtin(
            "select",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv(), k()],
                ret_ty: v(),
            },
        );
        // contains_key<K, V>(arr: Array<K, V>, k: K) -> Boolean
        db.builtin(
            "contains_key",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv(), k()],
                ret_ty: Boolean,
            },
        );
        // length<K, V>(arr: Array<K, V>) -> Integer
        db.builtin(
            "length",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv()],
                ret_ty: Integer,
            },
        );
        // is_empty<K, V>(arr: Array<K, V>) -> Boolean
        db.builtin(
            "is_empty",
            Q::Array,
            TypeFn {
                generics: gen_kv(),
                params: vec![map_kv()],
                ret_ty: Boolean,
            },
        );

        // Bitwise Logic (I32)
        db.builtin("bv_not", Q::I32, fn1_arith(I32)); // Unary: ~I32 -> I32
        db.builtin("bv_and", Q::I32, fn2_arith(I32)); // Binary: I32 & I32 -> I32
        db.builtin("bv_or", Q::I32, fn2_arith(I32)); // Binary: I32 | I32 -> I32
        db.builtin("bv_xor", Q::I32, fn2_arith(I32)); // Binary: I32 ^ I32 -> I32
        db.builtin("bv_nand", Q::I32, fn2_arith(I32));
        db.builtin("bv_nor", Q::I32, fn2_arith(I32));
        db.builtin("bv_xnor", Q::I32, fn2_arith(I32));

        // Reduction (I32 -> Boolean)
        db.builtin("bv_redand", Q::I32, fn1(I32, Boolean));
        db.builtin("bv_redor", Q::I32, fn1(I32, Boolean));

        // Arithmetic (I32)
        db.builtin("bv_neg", Q::I32, fn1_arith(I32)); // Unary: -I32 -> I32
        db.builtin("bv_add", Q::I32, fn2_arith(I32));
        db.builtin("bv_sub", Q::I32, fn2_arith(I32));
        db.builtin("bv_mul", Q::I32, fn2_arith(I32));
        db.builtin("bv_div", Q::I32, fn2_arith(I32));
        db.builtin("bv_rem", Q::I32, fn2_arith(I32));
        db.builtin("bv_mod", Q::I32, fn2_arith(I32));

        // Shifts and Rotation (I32)
        db.builtin("bv_shl", Q::I32, fn2_arith(I32));
        db.builtin("bv_lshr", Q::I32, fn2_arith(I32));
        db.builtin("bv_ashr", Q::I32, fn2_arith(I32));
        db.builtin("bv_rotate_left", Q::I32, fn2_arith(I32));
        db.builtin("bv_rotate_right", Q::I32, fn2_arith(I32));

        // Comparisons (I32 -> Boolean)
        db.builtin("bv_lt", Q::I32, fn2_cmp(I32));
        db.builtin("bv_le", Q::I32, fn2_cmp(I32));
        db.builtin("bv_gt", Q::I32, fn2_cmp(I32));
        db.builtin("bv_ge", Q::I32, fn2_cmp(I32));

        // Overflow Checks (I32 -> Boolean)
        db.builtin("checked_bvadd_no_overflow", Q::I32, fn2_cmp(I32));
        db.builtin("checked_bvsub_no_overflow", Q::I32, fn2_cmp(I32));
        db.builtin("checked_bvmul_no_overflow", Q::I32, fn2_cmp(I32));
        db.builtin("checked_bvsdiv_no_overflow", Q::I32, fn2_cmp(I32));
        db.builtin("checked_bvneg_no_overflow", Q::I32, fn1(I32, Boolean));

        // Conversion (I32 -> Integer)
        db.builtin("to_int", Q::I32, fn1(I32, Integer));

        // Bitwise Logic (I64)
        db.builtin("bv_not", Q::I64, fn1_arith(I64));
        db.builtin("bv_and", Q::I64, fn2_arith(I64));
        db.builtin("bv_or", Q::I64, fn2_arith(I64));
        db.builtin("bv_xor", Q::I64, fn2_arith(I64));
        db.builtin("bv_nand", Q::I64, fn2_arith(I64));
        db.builtin("bv_nor", Q::I64, fn2_arith(I64));
        db.builtin("bv_xnor", Q::I64, fn2_arith(I64));

        // Reduction (I64 -> Boolean)
        db.builtin("bv_redand", Q::I64, fn1(I64, Boolean));
        db.builtin("bv_redor", Q::I64, fn1(I64, Boolean));

        // Arithmetic (I64)
        db.builtin("bv_neg", Q::I64, fn1_arith(I64));
        db.builtin("bv_add", Q::I64, fn2_arith(I64));
        db.builtin("bv_sub", Q::I64, fn2_arith(I64));
        db.builtin("bv_mul", Q::I64, fn2_arith(I64));
        db.builtin("bv_div", Q::I64, fn2_arith(I64));
        db.builtin("bv_rem", Q::I64, fn2_arith(I64));
        db.builtin("bv_mod", Q::I64, fn2_arith(I64));

        // Shifts and Rotation (I64)
        db.builtin("bv_shl", Q::I64, fn2_arith(I64));
        db.builtin("bv_lshr", Q::I64, fn2_arith(I64));
        db.builtin("bv_ashr", Q::I64, fn2_arith(I64));
        db.builtin("bv_rotate_left", Q::I64, fn2_arith(I64));
        db.builtin("bv_rotate_right", Q::I64, fn2_arith(I64));

        // Comparisons (I64 -> Boolean)
        db.builtin("bv_lt", Q::I64, fn2_cmp(I64));
        db.builtin("bv_le", Q::I64, fn2_cmp(I64));
        db.builtin("bv_gt", Q::I64, fn2_cmp(I64));
        db.builtin("bv_ge", Q::I64, fn2_cmp(I64));

        // Overflow Checks (I64 -> Boolean)
        db.builtin("checked_bvadd_no_overflow", Q::I64, fn2_cmp(I64));
        db.builtin("checked_bvsub_no_overflow", Q::I64, fn2_cmp(I64));
        db.builtin("checked_bvmul_no_overflow", Q::I64, fn2_cmp(I64));
        db.builtin("checked_bvsdiv_no_overflow", Q::I64, fn2_cmp(I64));
        db.builtin("checked_bvneg_no_overflow", Q::I64, fn1(I64, Boolean));

        // Conversion (I64 -> Integer)
        db.builtin("to_int", Q::I64, fn1(I64, Integer));

        // Bitwise Logic (U32)
        db.builtin("bv_not", Q::U32, fn1_arith(U32));
        db.builtin("bv_and", Q::U32, fn2_arith(U32));
        db.builtin("bv_or", Q::U32, fn2_arith(U32));
        db.builtin("bv_xor", Q::U32, fn2_arith(U32));
        db.builtin("bv_nand", Q::U32, fn2_arith(U32));
        db.builtin("bv_nor", Q::U32, fn2_arith(U32));
        db.builtin("bv_xnor", Q::U32, fn2_arith(U32));

        // Reduction (U32 -> Boolean)
        db.builtin("bv_redand", Q::U32, fn1(U32, Boolean));
        db.builtin("bv_redor", Q::U32, fn1(U32, Boolean));

        // Arithmetic (U32)
        db.builtin("bv_neg", Q::U32, fn1_arith(U32));
        db.builtin("bv_add", Q::U32, fn2_arith(U32));
        db.builtin("bv_sub", Q::U32, fn2_arith(U32));
        db.builtin("bv_mul", Q::U32, fn2_arith(U32));
        db.builtin("bv_div", Q::U32, fn2_arith(U32));
        db.builtin("bv_rem", Q::U32, fn2_arith(U32));
        db.builtin("bv_mod", Q::U32, fn2_arith(U32));

        // Shifts and Rotation (U32)
        db.builtin("bv_shl", Q::U32, fn2_arith(U32));
        db.builtin("bv_lshr", Q::U32, fn2_arith(U32));
        db.builtin("bv_ashr", Q::U32, fn2_arith(U32));
        db.builtin("bv_rotate_left", Q::U32, fn2_arith(U32));
        db.builtin("bv_rotate_right", Q::U32, fn2_arith(U32));

        // Comparisons (U32 -> Boolean)
        db.builtin("bv_lt", Q::U32, fn2_cmp(U32));
        db.builtin("bv_le", Q::U32, fn2_cmp(U32));
        db.builtin("bv_gt", Q::U32, fn2_cmp(U32));
        db.builtin("bv_ge", Q::U32, fn2_cmp(U32));

        // Overflow Checks (U32 -> Boolean)
        db.builtin("checked_bvadd_no_overflow", Q::U32, fn2_cmp(U32));
        db.builtin("checked_bvsub_no_overflow", Q::U32, fn2_cmp(U32));
        db.builtin("checked_bvmul_no_overflow", Q::U32, fn2_cmp(U32));
        db.builtin("checked_bvsdiv_no_overflow", Q::U32, fn2_cmp(U32));
        db.builtin("checked_bvneg_no_overflow", Q::U32, fn1(U32, Boolean));

        // Conversion (U32 -> Integer)
        db.builtin("to_int", Q::U32, fn1(U32, Integer));

        // Bitwise Logic (U64)
        db.builtin("bv_not", Q::U64, fn1_arith(U64));
        db.builtin("bv_and", Q::U64, fn2_arith(U64));
        db.builtin("bv_or", Q::U64, fn2_arith(U64));
        db.builtin("bv_xor", Q::U64, fn2_arith(U64));
        db.builtin("bv_nand", Q::U64, fn2_arith(U64));
        db.builtin("bv_nor", Q::U64, fn2_arith(U64));
        db.builtin("bv_xnor", Q::U64, fn2_arith(U64));

        // Reduction (U64 -> Boolean)
        db.builtin("bv_redand", Q::U64, fn1(U64, Boolean));
        db.builtin("bv_redor", Q::U64, fn1(U64, Boolean));

        // Arithmetic (U64)
        db.builtin("bv_neg", Q::U64, fn1_arith(U64));
        db.builtin("bv_add", Q::U64, fn2_arith(U64));
        db.builtin("bv_sub", Q::U64, fn2_arith(U64));
        db.builtin("bv_mul", Q::U64, fn2_arith(U64));
        db.builtin("bv_div", Q::U64, fn2_arith(U64));
        db.builtin("bv_rem", Q::U64, fn2_arith(U64));
        db.builtin("bv_mod", Q::U64, fn2_arith(U64));

        // Shifts and Rotation (U64)
        db.builtin("bv_shl", Q::U64, fn2_arith(U64));
        db.builtin("bv_lshr", Q::U64, fn2_arith(U64));
        db.builtin("bv_ashr", Q::U64, fn2_arith(U64));
        db.builtin("bv_rotate_left", Q::U64, fn2_arith(U64));
        db.builtin("bv_rotate_right", Q::U64, fn2_arith(U64));

        // Comparisons (U64 -> Boolean)
        db.builtin("bv_lt", Q::U64, fn2_cmp(U64));
        db.builtin("bv_le", Q::U64, fn2_cmp(U64));
        db.builtin("bv_gt", Q::U64, fn2_cmp(U64));
        db.builtin("bv_ge", Q::U64, fn2_cmp(U64));

        // Overflow Checks (U64 -> Boolean)
        db.builtin("checked_bvadd_no_overflow", Q::U64, fn2_cmp(U64));
        db.builtin("checked_bvsub_no_overflow", Q::U64, fn2_cmp(U64));
        db.builtin("checked_bvmul_no_overflow", Q::U64, fn2_cmp(U64));
        db.builtin("checked_bvsdiv_no_overflow", Q::U64, fn2_cmp(U64));
        db.builtin("checked_bvneg_no_overflow", Q::U64, fn1(U64, Boolean));

        // Conversion (U64 -> Integer)
        db.builtin("to_int", Q::U64, fn1(U64, Integer));

        // Arithmetic Operations (Binary: F32, F32 -> F32)
        db.builtin("add", Q::F32, fn2_arith(F32));
        db.builtin("sub", Q::F32, fn2_arith(F32));
        db.builtin("mul", Q::F32, fn2_arith(F32));
        db.builtin("div", Q::F32, fn2_arith(F32));
        db.builtin("rem", Q::F32, fn2_arith(F32)); // Remainder
        db.builtin("min", Q::F32, fn2_arith(F32));
        db.builtin("max", Q::F32, fn2_arith(F32));

        // Arithmetic Operations (Unary: F32 -> F32)
        db.builtin("neg", Q::F32, fn1_arith(F32));
        db.builtin("abs", Q::F32, fn1_arith(F32));
        db.builtin("sqrt", Q::F32, fn1_arith(F32));

        // Comparisons (Binary: F32, F32 -> Boolean)
        db.builtin("lt", Q::F32, fn2_cmp(F32));
        db.builtin("le", Q::F32, fn2_cmp(F32));
        db.builtin("gt", Q::F32, fn2_cmp(F32));
        db.builtin("ge", Q::F32, fn2_cmp(F32));

        // Predicates / Checks (Unary: F32 -> Boolean)
        db.builtin("is_nan", Q::F32, fn1(F32, Boolean));
        db.builtin("is_infinite", Q::F32, fn1(F32, Boolean));
        db.builtin("is_zero", Q::F32, fn1(F32, Boolean));
        db.builtin("is_normal", Q::F32, fn1(F32, Boolean));
        db.builtin("is_subnormal", Q::F32, fn1(F32, Boolean));
        db.builtin("is_negative", Q::F32, fn1(F32, Boolean));
        db.builtin("is_positive", Q::F32, fn1(F32, Boolean));

        // Constructors / Constants (Nullary: -> F32)
        db.builtin("nan", Q::F32, fn0(F32));
        db.builtin("infinity", Q::F32, fn0(F32));
        db.builtin("neg_infinity", Q::F32, fn0(F32));
        db.builtin("pos_zero", Q::F32, fn0(F32));
        db.builtin("neg_zero", Q::F32, fn0(F32));

        // Conversions (Unary: F32 -> Other)
        db.builtin("to_integer", Q::F32, fn1(F32, Integer));
        db.builtin("to_real", Q::F32, fn1(F32, Real));

        // Conversions (Unary: Other -> F32)
        // Based on `impl From<String> for F32`
        db.builtin("from_string", Q::F32, fn1(String, F32));

        // Arithmetic Operations (Binary: F64, F64 -> F64)
        db.builtin("add", Q::F64, fn2_arith(F64));
        db.builtin("sub", Q::F64, fn2_arith(F64));
        db.builtin("mul", Q::F64, fn2_arith(F64));
        db.builtin("div", Q::F64, fn2_arith(F64));
        db.builtin("rem", Q::F64, fn2_arith(F64));
        db.builtin("min", Q::F64, fn2_arith(F64));
        db.builtin("max", Q::F64, fn2_arith(F64));

        // Arithmetic Operations (Unary: F64 -> F64)
        db.builtin("neg", Q::F64, fn1_arith(F64));
        db.builtin("abs", Q::F64, fn1_arith(F64));
        db.builtin("sqrt", Q::F64, fn1_arith(F64));

        // Comparisons (Binary: F64, F64 -> Boolean)
        db.builtin("lt", Q::F64, fn2_cmp(F64));
        db.builtin("le", Q::F64, fn2_cmp(F64));
        db.builtin("gt", Q::F64, fn2_cmp(F64));
        db.builtin("ge", Q::F64, fn2_cmp(F64));

        // Predicates / Checks (Unary: F64 -> Boolean)
        db.builtin("is_nan", Q::F64, fn1(F64, Boolean));
        db.builtin("is_infinite", Q::F64, fn1(F64, Boolean));
        db.builtin("is_zero", Q::F64, fn1(F64, Boolean));
        db.builtin("is_normal", Q::F64, fn1(F64, Boolean));
        db.builtin("is_subnormal", Q::F64, fn1(F64, Boolean));
        db.builtin("is_negative", Q::F64, fn1(F64, Boolean));
        db.builtin("is_positive", Q::F64, fn1(F64, Boolean));

        // Constructors / Constants (Nullary: -> F64)
        db.builtin("nan", Q::F64, fn0(F64));
        db.builtin("infinity", Q::F64, fn0(F64));
        db.builtin("neg_infinity", Q::F64, fn0(F64));
        db.builtin("pos_zero", Q::F64, fn0(F64));
        db.builtin("neg_zero", Q::F64, fn0(F64));

        // Conversions (Unary: F64 -> Other)
        db.builtin("to_integer", Q::F64, fn1(F64, Integer));
        db.builtin("to_real", Q::F64, fn1(F64, Real));

        // Conversions (Unary: Other -> F64)
        // Based on `impl From<String> for F64`
        db.builtin("from_string", Q::F64, fn1(String, F64));

        // Error handling functions.
        db.builtin("fresh", Q::Error, fn0(Error)); // `fresh` is a function that returns a new error. It is a nullary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [], ret_ty: Error, } so it returns an error.
        db.builtin("merge", Q::Error, fn2_arith(Error)); // `merge` is a function that merges two errors. It is a binary arithmetic function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Error, Error], ret_ty: Error, } so the parameters are two errors and the return type is an error.

        db // Return the populated database.
    }

    /// Registers a user-defined function (either a method or a standalone function).
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function.
    /// * `sig` - The function signature.
    /// * `method` - An optional method name if exists.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub fn register_user_func(
        &mut self,
        name: &UsrFuncName,
        sig: &FuncSig,
        method: Option<&UsrFuncName>,
    ) -> Result<()> {
        if let Some(method_name) = method {
            // Extract the receiver type (the first parameter). This is the type that the method implements.
            let (self_ty, self_ty_name, self_ty_args) = match sig.params.first() {
                None => bail!("no receiver argument"), // Error if there's no receiver. A method cannot be in the annotation for a function with no parameters.
                Some((_, t)) => match t {
                    TypeTag::User(ty_name, ty_args) => (t, ty_name.clone(), ty_args.clone()), // the type of the first parameter, the name of the type, the arguments of the type are extracted.
                    _ => bail!("the receiver argument is not a user-defined type"), // because the method is implemented on the type, the receiver argument must be a user-defined type. Rust does not allow methods on primitive types. so if we want to write impl TYPE { fn my_method() {} }, we need to define TYPE first in the SAME crate.
                },
            };
            // Extract all the type parameters used in the receiver type.
            let self_ty_generics = self_ty.type_params_used();

            let ty_params = &sig.generics.params;
            // the functions signature has a list of type parameters, which surely includes the type parameters used in the receiver type. The type parameters used in the receiver type are a subset of the type parameters in the function signature. So the function signature must have at least the type parameters used in the receiver type.
            if self_ty_generics.len() > ty_params.len() {
                bail!("[invariant] the receiver argument takes too many type arguments");
            }

            let method = TypeFn {
                generics: sig.generics.filter(&self_ty_generics), // remove the type parameters used in the receiver type from the function signature.
                params: sig
                    .params
                    .iter()
                    .map(|(_, ty)| ty.clone())
                    .collect::<Vec<_>>(),
                ret_ty: sig.ret_ty.clone(),
            };

            // Register the method under the user-defined type.
            match self
                .on_usr_type
                .entry(method_name.clone())
                .or_default()
                .insert(self_ty_name, (self_ty_args, method))
            {
                None => (), // Successfully inserted.
                Some(_) => bail!(
                    "duplicated registration of user-defined function: {}::{}",
                    self_ty,
                    method_name,
                ), // error will be caused if we try to implement the same method on the same type more than once. So for example this is wrong: #[smt_impl(method = my_method)] fn my_impl_1(Type_1) {} #[smt_impl(method = my_method)] fn my_impl_2(Type_1, Type_2) {} because my_method is implemented on Type_1 twice. (also different signatures in this case)
            }
        }

        // Register the function as unqualified (standalone function).
        let func = TypeFn::new_from_sig(sig); // builds a TypeFn from the function signature.
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

    /// Looks up a user function on a user-defined type by name.
    pub fn lookup_usr_func_on_usr_type(
        &self,
        ty_name: &UsrTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.on_usr_type
            .get(fn_name)
            .and_then(|s| s.get(ty_name))
            .map(|(_, v)| v)
    }

    /// Queries for a function with type inference, given a function name and arguments.
    pub fn query_with_inference<T: CtxtForExpr>(
        &self,
        unifier: &mut TypeUnifier,
        ctxt: &T,
        name: &UsrFuncName,
        inst: Option<&[TypeTag]>,
        args: Vec<Expr>,
        rval: &TypeRef,
    ) -> Result<Op> {
        // Collect candidate functions matching the name and kind.
        let mut candidates = vec![];
        // first look at methods defined on the system types.
        match self.on_sys_type.get(name) {
            None => (),
            Some(options) => candidates.extend(options.iter().map(|(n, t)| (TypeName::Sys(*n), t))),
        }
        // then look at methods defined on the user-defined types.
        match self.on_usr_type.get(name) {
            None => (),
            Some(options) => candidates.extend(
                options
                    .iter()
                    .map(|(n, (_, t))| (TypeName::Usr(n.clone()), t)),
            ),
        }

        // Variable to hold a suitable candidate if found.
        let mut suitable = None;
        // ty_name is the type the method is defined on
        // fty is the function signature
        for (ty_name, fty) in candidates {
            // Early filter by number of parameters.
            // the length of the parameters of the function signature should be the same as the length of the arguments. If they are not the same, we move to the next candidate.
            if fty.params.len() != args.len() {
                continue;
            }

            // Instantiate type parameters for the type name.
            // ty_inst is the generics of the type the method is defined on. This is only created so to check that the generics of the function signature and the generics of the type the method is defined on do not have any conflicting type parameter names.
            let ty_inst = match &ty_name {
                TypeName::Sys(sys_name) => {
                    GenericsInstPartial::new_without_args(&sys_name.generics())
                }
                TypeName::Usr(usr_name) => GenericsInstPartial::new_without_args(
                    ctxt.get_type_generics(usr_name).expect("user-defined type"),
                ),
                TypeName::Param(_) => panic!("unexpected type parameter in type name"),
            };
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
            // here we merge the type and function instantiations. If there are any conflicting type parameter names, we bail.
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
            suitable = Some((ty_name, probing, inst_full)); // Store the suitable candidate.
        }

        // Ensure a suitable function was found.
        let (ty_name, probing, inst_full) = match suitable {
            None => bail!("no candidates matches the method call"),
            Some(matched) => matched,
        };

        // Update the main unifier with the successful unification.
        *unifier = probing;

        // Construct the operation to represent the function call.
        let op = match ty_name {
            TypeName::Sys(sys_name) => {
                let intrinsic = Intrinsic::new(&sys_name, name, inst_full.vec(), args)?;
                Op::Intrinsic(Box::new(intrinsic))
            }
            TypeName::Usr(_) => Op::Procedure {
                name: name.clone(),
                inst: inst_full.vec(),
                args,
            },
            TypeName::Param(_) => panic!("unexpected type parameter in type name"),
        };
        Ok(op) // Return the operation.
    }
}

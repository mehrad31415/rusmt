// Import BTreeMap for storing data in a sorted order in the `ApplyDatabase` struct (field `unqualified`, `on_sys_type`, `on_usr_type`).
use std::collections::BTreeMap;
// The Result type is a type that represents either success (Ok) or failure (Err).
// It is a type alias of the std::result::Result type: pub type Result<T, E = Error> = std::result::Result<T, E>;
// `anyhow` macro is used to create an error from a string or an error type that can be converted into an anyhow::Error.
// bail macro is used to return an error from a function without explicitly mentioning return. bail!(....) = return Err(anyhow!(...)).
// The Context trait allows you to add extra context when an operation returns an error. This is done by calling the context or with_context methods on a Result.
// Import error handling utilities from the anyhow crate.
use anyhow::{bail, Result};

use crate::parser::expr::{CtxtForExpr, Expr, Op}; // Import expression-related types and traits.
use crate::parser::func::FuncSig; // Import function signature ADT.
use crate::parser::generics::{Generics, GenericsInstFull, GenericsInstPartial}; // Import generics handling utilities.
use crate::parser::infer::{TIError, TypeRef, TypeUnifier}; // Import type inference utilities.
use crate::parser::intrinsics::Intrinsic; // Import intrinsic function handling.
use crate::parser::name::{TypeParamName, UsrFuncName, UsrTypeName}; // Import type parameters, user-defined function names, and user-defined type names.
use crate::parser::ty::{SysTypeName, TypeName, TypeTag};

/// Marks whether this function is for implementation (`Impl`) or specification (`Spec`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    /// Actual implementation of the function.
    Impl,
    /// Formal specification of the function.
    Spec,
}

#[derive(Debug)]
/// Represents a function type, with inference allowed.
///
/// This struct holds the kind of the function, its generics, parameter types, and return type. The latter three, encapsulate the type signature of the function.
pub struct TypeFn {
    /// Indicates whether this function is an implementation or a specification.
    pub kind: Kind,
    /// Generic parameters of the function.
    pub generics: Generics,
    /// Types of the function's parameters.
    pub params: Vec<TypeTag>, // unlike FuncSig, this is a vector of TypeTag instead of a vector of (UsrVarName, TypeTag). So it only contains the types of the parameters, not the names.
    /// Return type of the function.
    pub ret_ty: TypeTag,
}

impl TypeFn {
    /// Creates a new function type `TypeFn` from its function signature.
    ///
    /// # Arguments
    ///
    /// * `sig` - The function signature ADT to create from.
    /// * `kind` - The kind of the function (`Impl` or `Spec`).
    ///
    /// # Returns
    ///
    /// * A new `TypeFn` instance.
    pub fn new_from_sig(sig: &FuncSig, kind: Kind) -> Self {
        Self {
            kind,
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
    /// It is a map from function names to their respective function signatures and kinds (Impl or Spec).
    unqualified: BTreeMap<UsrFuncName, TypeFn>,
    /// Pre-defined functions with a system type qualifier (built-in methods on built-in types).
    /// It is a map from function names to a map of system type names to their respective function signatures and kinds (Impl or Spec).
    /// UsrFuncName is the name of the built-in function for example "add", "sub", etc. In "add" for example, it is associated with the system type "Integer" & "Rational". That is why the value is a map itself. The "add" associated with "Integer" will have a different function signature and kind to the "add" associated with "Rational". So basically, we will have somthing like this:
    /// on_sys_type = { "add" => { "Integer" => TypeFn_For_Integer, "Rational" => TypeFn_For_Rational } ... }
    /// SysTypeName is the system type name that the method is associated with.
    /// TypeFn is the function type encapsulating the function signature and kind (Impl or Spec).
    on_sys_type: BTreeMap<UsrFuncName, BTreeMap<SysTypeName, TypeFn>>,
    /// User-defined functions with a user-defined type qualifier (methods on custom types).
    /// The `UsrFuncName` is the name of the method. A method can be implemented for multiple types. So the value is a map from the method name to a map. The value map is from the user-defined type name to their respective arguments (Vec<TypeTag>) and function signature (TypeFn).
    on_usr_type: BTreeMap<UsrFuncName, BTreeMap<UsrTypeName, (Vec<TypeTag>, TypeFn)>>,
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
    /// * `ty_name` - The system type the function is associated with (e.g., `Integer`, `Boolean`, `Rational`, `Text`, `Cloak`, `Seq`, `Set`, `Map`, `Error`).
    /// * `sig` - The function signature and kind (`Impl` or `Spec`).
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
                // we cannot add a specification for a built in type because it only differs in TypeFn::kind. So if we try to add a specification for a built-in type, it will panic.
                panic!("duplicated built-in: {}::{}", ty_name, fn_name);
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
                        // Note the similarities and differences between SysTypeName and TypeTag. SysTypeName is for system types like Integer, Boolean, Rational, Text, Cloak, Seq, Set, Map, Error. TypeTag is for all types including system types, user-defined types, type parameters, and tuples. Also in Seq for example, in SysTypeName, it is just Seq. But in TypeTag, it is Seq(Box<TypeTag>). The reason is that in TypeTag, we need to specify the type of the elements in the sequence. But in SysTypeName, we just want to say that Seq is a system reserved type.

        // Initialize the database.
        let mut db = Self::new();

        // Utility closure for initiating an empty generics.
        let empty = || Generics::intrinsic(vec![]);

        // Utility closures for creating function types. fn0 is a nullary function, fn1 is a unary function, fn2 is a binary function, fn3 is a ternary function. They are all implementation functions.
        let fn0 = |rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![],
            ret_ty: rty,
        };

        let fn1 = |a0: TypeTag, rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![a0],
            ret_ty: rty,
        };

        let fn2 = |a0: TypeTag, a1: TypeTag, rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![a0, a1],
            ret_ty: rty,
        };

        let fn3 = |a0: TypeTag, a1: TypeTag, a2: TypeTag, rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![a0, a1, a2],
            ret_ty: rty,
        };

        let fn1_arith = |t: TypeTag| fn1(t.clone(), t); // Unary function used for negation of Booleans. It takes a type and returns the same type.

        let fn2_arith = |t: TypeTag| fn2(t.clone(), t.clone(), t); // Binary function. It takes two types and returns the same type. The is used in `and`, `or`, `xor`, `implies` operations for Boolean type. Also used for `add`, `sub`, `mul`, `div`, `rem` operations for Integer type. Also used for `add`, `sub`, `mul`, `div` operations for Rational type. Lastly, used for `merge` operation for Error type.
        let fn2_cmp = |t: TypeTag| fn2(t.clone(), t, Boolean); // Comparison function. It takes two types and returns a Boolean. This is used for `lt`, `le`, `ge`, `gt` operations for Integer, Rational and Text types.

        // Type parameters for generics.
        let t = || Parameter(TypeParamName::intrinsic("T")); // gives a Parameter(TypeParamName { ident: String::from("T") }) when called.
        let box_t = || Cloak(t().into()); // gives a Cloak(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.
        let seq_t = || Seq(t().into()); // gives a Seq(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.
        let set_t = || Set(t().into()); // gives a Set(Box(Parameter(TypeParamName { ident: String::from("T") }))) when called.

        let k = || Parameter(TypeParamName::intrinsic("K"));
        let v = || Parameter(TypeParamName::intrinsic("V"));
        let map_kv = || Map(k().into(), v().into()); // gives a Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") }))) when called.
        let seq_k = || Seq(k().into()); // gives a Seq(Box(Parameter(TypeParamName { ident: String::from("K") })) when called.

        // Register intrinsic functions for different system types.

        // Boolean operations.
        db.builtin("not", Q::Boolean, fn1_arith(Boolean)); // `not` is for Boolean type. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Boolean], ret_ty: Boolean, } so the parameter is a Boolean and the return type is a Boolean.
        db.builtin("and", Q::Boolean, fn2_arith(Boolean)); // `and` is for Boolean type. It is a binary arithmetic function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Boolean, Boolean], ret_ty: Boolean, } so the parameters are two Booleans and the return type is a Boolean.
        db.builtin("or", Q::Boolean, fn2_arith(Boolean)); // `or` is the same as `and` but for the `or` operation.
        db.builtin("xor", Q::Boolean, fn2_arith(Boolean)); // `xor` is the same as `and` but for the `xor` operation.
        db.builtin("implies", Q::Boolean, fn2_arith(Boolean)); // `implies` is the same as `and` but for the `implies` operation.

        // Integer arithmetic and comparison.
        // add, sub, mul, div, rem, are arithmetic operations. They are binary arithmetic functions, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Integer, Integer], ret_ty: Integer, } so the parameters are two Integers and the return type is an Integer.
        // lt, le, ge, gt are comparison operations. They are binary comparison functions, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Integer, Integer], ret_ty: Boolean, } so the parameters are two Integers and the return type is a Boolean.
        db.builtin("add", Q::Integer, fn2_arith(Integer));
        db.builtin("sub", Q::Integer, fn2_arith(Integer));
        db.builtin("mul", Q::Integer, fn2_arith(Integer));
        db.builtin("div", Q::Integer, fn2_arith(Integer));
        db.builtin("rem", Q::Integer, fn2_arith(Integer));
        db.builtin("lt", Q::Integer, fn2_cmp(Integer));
        db.builtin("le", Q::Integer, fn2_cmp(Integer));
        db.builtin("ge", Q::Integer, fn2_cmp(Integer));
        db.builtin("gt", Q::Integer, fn2_cmp(Integer));

        // Rational number operations.
        // Rational is the same as Integer but for Rational numbers.
        // It does not have the rem operation.
        db.builtin("add", Q::Rational, fn2_arith(Rational));
        db.builtin("sub", Q::Rational, fn2_arith(Rational));
        db.builtin("mul", Q::Rational, fn2_arith(Rational));
        db.builtin("div", Q::Rational, fn2_arith(Rational));
        db.builtin("lt", Q::Rational, fn2_cmp(Rational));
        db.builtin("le", Q::Rational, fn2_cmp(Rational));
        db.builtin("ge", Q::Rational, fn2_cmp(Rational));
        db.builtin("gt", Q::Rational, fn2_cmp(Rational));

        // Text comparison.
        // It has the comparison operations but not the arithmetic operations.
        // It is a binary comparison function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Text, Text], ret_ty: Boolean, } so the parameters are two Texts and the return type is a Boolean.
        db.builtin("lt", Q::Text, fn2_cmp(Text));
        db.builtin("le", Q::Text, fn2_cmp(Text));
        db.builtin("ge", Q::Text, fn2_cmp(Text));
        db.builtin("gt", Q::Text, fn2_cmp(Text));

        // Cloak type operations (e.g., for encapsulation).
        // The `shield` function is a unary function that takes a type and returns a Cloak type. The TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [T], ret_ty: Cloak(T), } so the parameter is a type Parameter(TypeParamName {ident: String::from("T")}) and the return type is a Cloak(Box(Parameter(TypeParamName {ident: String::from("T")})))
        // `reveal` is a unary function with the oppsite signature of `shield`.
        db.builtin("shield", Q::Cloak, fn1(t(), box_t()));
        db.builtin("reveal", Q::Cloak, fn1(box_t(), t()));

        // Sequence operations.
        db.builtin("new", Q::Seq, fn0(seq_t())); // `new` is a function that returns a new sequence. It is a nullary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [], ret_ty: Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), } so it returns a sequence of type T. T can be any type.
        db.builtin("length", Q::Seq, fn1(seq_t(), Integer)); // `length` is a function that returns the length of a sequence. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") }))], ret_ty: Integer, } so the parameter is a sequence of type T and the return type is an Integer.
        db.builtin("append", Q::Seq, fn2(seq_t(), t(), seq_t())); // `append` is a function that appends an element to a sequence. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), Parameter(TypeParamName { ident: String::from("T") })], ret_ty: Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), } so the parameters are a sequence of type T and an element of type T and the return type is a sequence of type T.
        db.builtin("at_unchecked", Q::Seq, fn2(seq_t(), Integer, t())); // `at_unchecked` is a function that returns an element at a given index in a sequence. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), Integer], ret_ty: Parameter(TypeParamName { ident: String::from("T") }), } so the parameters are a sequence of type T and an Integer and the return type is a type T.
        db.builtin("includes", Q::Seq, fn2(seq_t(), t(), Boolean)); // `includes` is a function that checks if a sequence includes an element. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), Parameter(TypeParamName { ident: String::from("T") })], ret_ty: Boolean, } so the parameters are a sequence of type T and an element of type T and the return type is a Boolean.
        db.builtin("is_empty", Q::Seq, fn1(seq_t(), Boolean)); // `is_empty` is a function that checks if a sequence is empty. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") }))], ret_ty: Boolean, } so the parameter is a sequence of type T and the return type is a Boolean.
        db.builtin("iterator", Q::Seq, fn1(seq_t(), Seq(Box::new(Integer)))); // `iterator` is a function that returns an iterator for a sequence. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Seq(Box(Parameter(TypeParamName { ident: String::from("T") }))], ret_ty: Seq(Box(Integer)), } so the parameter is a sequence of type T and the return type is a sequence of Integers starting from 0 to the length of the sequence.

        // Set operations.
        db.builtin("new", Q::Set, fn0(set_t())); // `new` is a function that returns a new set. It is a nullary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [], ret_ty: Set(Box(Parameter(TypeParamName { ident: String::from("T") })), } so it returns a set of type T. T can be any type.
        db.builtin("length", Q::Set, fn1(set_t(), Integer)); // same as for Seq.
        db.builtin("insert", Q::Set, fn2(set_t(), t(), set_t())); // insert is the same as append but for sets.
        db.builtin("remove", Q::Set, fn2(set_t(), t(), set_t())); // `remove` is a function that removes an element from a set. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Set(Box(Parameter(TypeParamName { ident: String::from("T") })), Parameter(TypeParamName { ident: String::from("T") })], ret_ty: Set(Box(Parameter(TypeParamName { ident: String::from("T") })), } so the parameters are a set of type T and an element of type T and the return type is a set of type T. The function is not in-place, it returns a new set without the element.
        db.builtin("contains", Q::Set, fn2(set_t(), t(), Boolean)); // contains is the same as includes but for sets.
        db.builtin("is_empty", Q::Set, fn1(set_t(), Boolean)); // is_empty is the same as for Seq.
        db.builtin("iterator", Q::Set, fn1(set_t(), seq_t())); // The iterator is quite different from Seq. It returns a sequence of elements in the set. The sequence is not sorted. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Set(Box(Parameter(TypeParamName { ident: String::from("T") }))], ret_ty: Seq(Box(Parameter(TypeParamName { ident: String::from("T") })), } so the parameter is a set of type T and the return type is a sequence of type T.

        // Map operations.
        db.builtin("new", Q::Map, fn0(map_kv())); // `new` is a function that returns a new map. It is a nullary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [], ret_ty: Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") }))), } so it returns a map of type K to type V. K and V can be any types.
        db.builtin("length", Q::Map, fn1(map_kv(), Integer)); // `length` is a function that returns the length of a map. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") }))], ret_ty: Integer, } so the parameter is a map of type K to type V and the return type is an Integer.
        db.builtin("put_unchecked", Q::Map, fn3(map_kv(), k(), v(), map_kv())); // `put_unchecked` is a function that puts a key-value pair in a map. It is a ternary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") }), Parameter(TypeParamName { ident: String::from("V") })], ret_ty: Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), } so the parameters are a map of type K to type V, a key of type K, a value of type V and the return type is a map of type K to type V.
        db.builtin("get_unchecked", Q::Map, fn2(map_kv(), k(), v())); // `get_unchecked` is a function that gets a value from a map. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") })], ret_ty: Parameter(TypeParamName { ident: String::from("V") }), } so the parameters are a map of type K to type V, a key of type K and the return type is a type V.
        db.builtin("del_unchecked", Q::Map, fn2(map_kv(), k(), map_kv())); // `del_unchecked` is a function that deletes a key-value pair from a map. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") })], ret_ty: Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), } so the parameters are a map of type K to type V, a key of type K and the return type is a map of type K to type V.
        db.builtin("contains_key", Q::Map, fn2(map_kv(), k(), Boolean)); // `contains_key` is a function that checks if a map contains a key. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") })], ret_ty: Boolean, } so the parameters are a map of type K to type V, a key of type K and the return type is a Boolean.
        db.builtin("is_empty", Q::Map, fn1(map_kv(), Boolean)); // `is_empty` is the same as for Seq.
        db.builtin("iterator", Q::Map, fn1(map_kv(), seq_k())); // The iterator is quite different from Seq. It returns a sequence of keys in the map. The sequence is not sorted. It is a unary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") }))], ret_ty: Seq(Box(Parameter(TypeParamName { ident: String::from("K") })), } so the parameter is a map of type K to type V and the return type is a sequence of type K.

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
    /// * `kind` - The kind of the function (`Impl` or `Spec`).
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// Note that the method does not indicate whether the function is a method or a standalone function.
    // #[smt_impl(method = my_method, specs = [spec1, spec2])]
    // fn my_impl() {
    //     some code here
    // }
    // In the above example, the function my_impl is an smt annotated function. The name of the function is my_impl. The method is my_method. The specs are spec1 and spec2. The function has a method name. This is what the method parameter is for. If the function has a method in the annotation, the method parameter will be Some(&method). If it does not have a method, the method parameter will be None. A method parameter can ONLY optionally exist for functions marked with smt_impl and smt_spec annotations.
    pub fn register_user_func(
        &mut self,
        name: &UsrFuncName,
        sig: &FuncSig,
        method: Option<&UsrFuncName>,
        kind: Kind,
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
            // aren't self_ty_generics and self_ty_args the same list (basically typetag:typeparameter version of it)?
            let self_ty_generics = self_ty.type_params_used();

            let ty_params = &sig.generics.params;
            // the functions signature has a list of type parameters, which surely includes the type parameters used in the receiver type. The type parameters used in the receiver type are a subset of the type parameters in the function signature. So the function signature must have at least the type parameters used in the receiver type.
            if self_ty_generics.len() > ty_params.len() {
                bail!("[invariant] the receiver argument takes too many type arguments");
            }

            let method = TypeFn {
                kind,
                generics: sig.generics.filter(&self_ty_generics), // remove the type parameters used in the receiver type from the function signature.
                params: sig
                    .params
                    .iter()
                    .skip(1) // Skip the first element because it is the receiver type and is no longer the parameter of the method.
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
        let func = TypeFn::new_from_sig(sig, kind); // builds a TypeFn from the function signature.
        match self.unqualified.insert(name.clone(), func) {
            None => (), // Successfully inserted.
            Some(_) => panic!("duplicated registration of user-defined function: {}", name),
        }

        Ok(()) // Return success.
    }

    /// Filters a function type by kind, returning it if it matches.
    ///
    /// # Arguments
    ///
    /// * `ty` - The function type to filter.
    /// * `kind` - The desired kind we are looking for (`Impl` or `Spec`).
    ///
    /// # Returns
    ///
    /// An `Option` containing the function type if it matches the kind.
    /// If we are looking for a function of kind `Impl`, we can get the type of the function (whether it is a `Spec` or `Impl`).
    /// If we are looking for a function of kind `Spec`, we can only get the type of the function if it is a `Spec`. If it is an `Impl`, we cannot get the type.
    fn filter_by_kind(ty: &TypeFn, kind: Kind) -> Option<&TypeFn> {
        match (kind, ty.kind) {
            (Kind::Impl, Kind::Impl) => Some(ty),
            (Kind::Impl, Kind::Spec) => None,
            (Kind::Spec, Kind::Impl | Kind::Spec) => Some(ty),
        }
    }

    /// Looks up an unqualified user function by name and kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The desired kind (`Impl` or `Spec`).
    /// * `fn_name` - The name of the function.
    ///
    /// # Returns
    ///
    /// An `Option` containing the function type if found.
    ///
    /// This will return None:
    /// - if the function is not found.
    /// - if the function is found and we are looking for a Spec, but the function is an Impl.
    pub fn lookup_unqualified(&self, kind: Kind, fn_name: &UsrFuncName) -> Option<&TypeFn> {
        self.unqualified
            .get(fn_name)
            .and_then(|ty| Self::filter_by_kind(ty, kind))
    }

    /// Looks up a user function on a system type by name and kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The desired kind (`Impl` or `Spec`).
    /// * `ty_name` - The system type name.
    /// * `fn_name` - The function name.
    ///
    /// # Returns
    ///
    /// An `Option` containing the function type if found.
    ///
    /// This will return None:
    /// - if the function is not found.
    /// - if the function is found but the system type is not found.
    /// - if the function is found and the system type is found we are looking for a Spec, but the function is an Impl. Basically this should not happen as all functions in the system type are impl so looking for a spec is illogical.
    pub fn lookup_usr_func_on_sys_type(
        &self,
        kind: Kind,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.on_sys_type
            .get(fn_name)
            .and_then(|s| s.get(ty_name))
            .and_then(|ty| Self::filter_by_kind(ty, kind))
    }

    /// Looks up a user function on a user-defined type by name and kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The desired kind (`Impl` or `Spec`).
    /// * `ty_name` - The user-defined type name.
    /// * `fn_name` - The function name.
    ///
    /// # Returns
    ///
    /// An `Option` containing the function type if found.
    ///
    /// This will return None:
    /// - if the function is not found.
    /// - if the function is found but it is not implemented on the user-defined type.
    /// - if the function is found and it is implemented on the user-defined type, but we are looking for a Spec, but the function is an Impl. A method will only be a spec, if the function which it is annotated on is a spec.
    pub fn lookup_usr_func_on_usr_type(
        &self,
        kind: Kind,
        ty_name: &UsrTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.on_usr_type
            .get(fn_name)
            .and_then(|s| s.get(ty_name))
            .and_then(|(_, ty)| Self::filter_by_kind(ty, kind))
    }

    /// Queries for a function with type inference, given a function name and arguments.
    ///
    /// This method attempts to find a function that matches the provided name and arguments,
    /// performing type inference to resolve generics and ensure type compatibility.
    ///
    /// This function is used for x.<some user defined function>(args) calls. The name is the name of the function, inst is the type arguments, args is the receiver (x) and the arguments of the function converted to ADT Exprs, and rval is the expected return type. rval is the return type of the function call.
    /// # Arguments
    ///
    /// * `unifier` - The type unifier used for type inference.
    /// * `ctxt` - The context for expressions, providing additional information.
    /// * `name` - The function name to query.
    /// * `inst` - Optional explicit type arguments for generic functions.
    /// * `args` - The arguments provided to the function.
    /// * `rval` - The expected return type.
    ///
    /// # Returns
    ///
    /// A `Result` containing the operation (`Op`) representing the function call.
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
        // we first collect all the functions that have the same name as the function we are looking for. We collect all the functions that have the same name as the function we are looking for, because the function we are looking for can be implemented on multiple types. So we need to check all the types that the function is implemented on. It is also possible that the function we are looking for is either a system function or a user-defined function. It cannot be a standalone function because we are looking for method calls. The kind of the method call needs to be the same as the kind of the function the method call is happening inside of. So if the function is a spec, the method call needs to be a spec. If the function is an impl, the method call needs to be an impl. The function we are looking for can be implemented on a system type or a user-defined type. So we need to check both.
        let kind = ctxt.kind(); // if the kind is an impl, and the function we find is a spec, we cannot use it and we can only use it if the function we find is an impl. If the kind is a spec, we can use the function we find if it is a spec or an impl.
        let mut candidates = vec![];
        // first look at methods defined on the system types.
        match self.on_sys_type.get(name) {
            None => (),
            Some(options) => candidates.extend(options.iter().filter_map(|(n, t)| {
                Self::filter_by_kind(t, kind).map(|t: &TypeFn| (TypeName::Sys(*n), t)) // TypeName::Sys(*n) is the type the function is defined on. t is the function signature along with kind.
            })),
        }
        // then look at methods defined on the user-defined types.
        match self.on_usr_type.get(name) {
            None => (),
            Some(options) => candidates.extend(options.iter().filter_map(|(n, (_, t))| {
                Self::filter_by_kind(t, kind).map(|t| (TypeName::Usr(n.clone()), t))
            })),
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
            let fn_inst = match inst { // inst is the turbufish type arguments provided to the function call.
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
                Op::Intrinsic(intrinsic)
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

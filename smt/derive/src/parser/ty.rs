//! This module parses the rust defined types and converts them to the type tags of Rusmart.

use crate::parser::ctxt::{ContextWithGenerics, MarkedType};
use crate::parser::generics::Generics;
use crate::parser::name::{ReservedIdent, TypeParamName, UsrTypeName};
use crate::{bail_if_empty, bail_if_exists, bail_if_missing, bail_on};
use itertools::Itertools; // imported to use the format method on iterators (std::slice::Iter types). This method does not exist on iterators by default, but itertools provides it. The format method takes an iterator and returns a string with the elements of the iterator separated by a separator string.
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use syn::{
    AngleBracketedGenericArguments, Field, FieldMutability, Fields, FieldsNamed, FieldsUnnamed,
    GenericArgument, Ident, ItemEnum, ItemStruct, Path, PathArguments, PathSegment, Result, Token,
    Type, TypePath, TypeTuple as TypePack, Variant, punctuated::Punctuated,
};

/// A context suitable for type analysis
/// Three types implement this trait: TypeParseCtxt (in ty.rs), FuncSigParseCtxt (in func.rs), and ExprParserRoot (in expr.rs)
pub trait CtxtForType {
    /// Retrieve the generics of the current type
    fn generics(&self) -> &Generics;

    /// Retrieve the generics declared (if any) for a user-defined type. If the user-defined type is not found, return None.
    fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics>;
}

/// A context provider for type parsing
/// It encapsulates the whole context and the generics of the current type
struct TypeParseCtxt<'a> {
    ctxt: &'a ContextWithGenerics,
    generics: &'a Generics,
}

impl CtxtForType for TypeParseCtxt<'_> {
    fn generics(&self) -> &Generics {
        self.generics
    }

    fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.ctxt.get_type_generics(name)
    }
}

/// Reserved type name
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum SysTypeName {
    Boolean,
    Integer,
    Real,
    F32,
    F64,
    I32,
    I64,
    U32,
    U64,
    String,
    Cloak,
    Seq,
    Set,
    Array,
    Error,
}

impl Display for SysTypeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Real => "Real",
            Self::F32 => "F32",
            Self::F64 => "F64",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::U32 => "U32",
            Self::U64 => "U64",
            Self::String => "String",
            Self::Cloak => "Cloak",
            Self::Seq => "Seq",
            Self::Set => "Set",
            Self::Array => "Array",
            Self::Error => "Error",
        };
        f.write_str(name)
    }
}

// System type names are reserved and cannot be used as user-defined type names
impl ReservedIdent for SysTypeName {
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident.to_string().as_str() {
            "Boolean" => Self::Boolean,
            "Integer" => Self::Integer,
            "Real" => Self::Real,
            "F32" => Self::F32,
            "F64" => Self::F64,
            "I32" => Self::I32,
            "I64" => Self::I64,
            "U32" => Self::U32,
            "U64" => Self::U64,
            "String" => Self::String,
            "Cloak" => Self::Cloak,
            "Seq" => Self::Seq,
            "Set" => Self::Set,
            "Array" => Self::Array,
            "Error" => Self::Error,
            _ => return None,
        };
        Some(matched)
    }
}

impl SysTypeName {
    /// Create the associated generics for an intrinsic type
    /// This is used because initially all the generics of the function delcaration in the db are empty.
    pub fn generics(&self) -> Generics {
        match self {
            Self::Boolean
            | Self::Integer
            | Self::Real
            | Self::String
            | Self::I32
            | Self::I64
            | Self::U32
            | Self::U64
            | Self::F32
            | Self::F64
            | Self::Error => Generics::intrinsic(vec![]),
            Self::Cloak | Self::Seq | Self::Set => {
                Generics::intrinsic(vec![TypeParamName::intrinsic("T")])
            }
            Self::Array => Generics::intrinsic(vec![
                TypeParamName::intrinsic("K"),
                TypeParamName::intrinsic("V"),
            ]),
        }
    }

    /// Convert an intrinsic type to a type tag
    pub fn as_type_tag(&self) -> TypeTag {
        let t = || Box::new(TypeTag::Parameter(TypeParamName::intrinsic("T")));
        let k = || Box::new(TypeTag::Parameter(TypeParamName::intrinsic("K")));
        let v = || Box::new(TypeTag::Parameter(TypeParamName::intrinsic("V")));

        match self {
            Self::Boolean => TypeTag::Boolean,
            Self::Integer => TypeTag::Integer,
            Self::Real => TypeTag::Real,
            Self::F32 => TypeTag::F32,
            Self::F64 => TypeTag::F64,
            Self::I32 => TypeTag::I32,
            Self::I64 => TypeTag::I64,
            Self::U32 => TypeTag::U32,
            Self::U64 => TypeTag::U64,
            Self::String => TypeTag::String,
            Self::Cloak => TypeTag::Cloak(t()),
            Self::Seq => TypeTag::Seq(t()),
            Self::Set => TypeTag::Set(t()),
            Self::Array => TypeTag::Array(k(), v()),
            Self::Error => TypeTag::Error,
        }
    }
}

/// A type name
/// The three variants replicate a system defined type, a user defined type, and a generic type parameter
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TypeName {
    Sys(SysTypeName),
    Usr(UsrTypeName),
    Param(TypeParamName),
}

impl TypeName {
    /// Try to convert an ident into a type name
    pub fn try_from(generics: &Generics, ident: &Ident) -> Result<Self> {
        let name = ident.to_string();
        // see if it is a reserved type name (Boolean, Integer, Real, F32, F64, I32, I64, U32, U64, String, Cloak, Seq, Set, Array, Error)
        let parsed = match SysTypeName::from_str(&name) {
            // if it is not a reserved type name, we try to convert it to a type parameter
            None => {
                // type parameters take priority over user-defined type names (this is a design choice made by the rust authors as well)
                // ident.try_into()? validates the ident so that the ident is not a reserved name (it cannot be a SysTypeName at this point) but it can be other reserved names like SysTrait (SMT), SysFuncName (eq, ne) ...
                // If it is a reserved name, an error is thrown.
                let param_name = ident.try_into()?;
                if generics.params.contains(&param_name) {
                    Self::Param(param_name)
                } else {
                    // if it is not a type parameter, we convert it to a user-defined type name
                    Self::Usr(ident.try_into()?)
                }
            }
            // if it is a reserved type name, we convert it to a SysTypeName
            Some(n) => Self::Sys(n),
        };
        Ok(parsed)
    }
}

impl Display for TypeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sys(name) => name.fmt(f),
            Self::Usr(name) => name.fmt(f),
            Self::Param(name) => name.fmt(f),
        }
    }
}

/// A unique and complete reference to an SMT-related type
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum TypeTag {
    /// boolean
    Boolean,
    /// integer (unlimited precision)
    Integer,
    /// rational numbers (unlimited precision)
    Real,
    /// floating point 32-bit
    F32,
    /// floating point 64-bit
    F64,
    /// signed 32-bit integer
    I32,
    /// signed 64-bit integer
    I64,
    /// unsigned 32-bit integer
    U32,
    /// unsigned 64-bit integer
    U64,
    /// string
    String,
    /// inductively defined type
    Cloak(Box<TypeTag>),
    /// SMT-sequence
    Seq(Box<TypeTag>),
    /// SMT-set
    Set(Box<TypeTag>),
    /// SMT-array
    Array(Box<TypeTag>, Box<TypeTag>),
    /// dynamic error type
    Error,
    /// user-defined type with type parameters as a list
    User(UsrTypeName, Vec<TypeTag>),
    /// a tuple of types
    Pack(Vec<TypeTag>),
    /// a type parameter
    Parameter(TypeParamName),
}

impl TypeTag {
    /// Convert from a type argument pack (in type path)
    /// Returns a vector of TypeTags of the arguments
    fn from_args<CTX: CtxtForType>(
        ctxt: &CTX,
        args: &Punctuated<GenericArgument, Token![,]>,
    ) -> Result<Vec<Self>> {
        let mut list = vec![];
        for arg in args {
            match arg {
                GenericArgument::Type(ty) => {
                    // first convert the type to TypeTag and then push it to the list
                    list.push(Self::from_type(ctxt, ty)?);
                }
                _ => bail_on!(arg, "invalid type argument"),
            }
        }
        Ok(list)
    }

    /// Convert from a type argument pack (in type path)
    pub fn from_args_in_type_path<CTX: CtxtForType>(
        ctxt: &CTX,
        pack: &AngleBracketedGenericArguments,
    ) -> Result<Vec<Self>> {
        let AngleBracketedGenericArguments {
            colon2_token: _,  // Allow turbofish syntax (::) - just ignore it
            lt_token: _,
            args,
            gt_token: _,
        } = pack;
        
        Self::from_args(ctxt, args)
    }

    /// Convert from a type argument pack, expecting 1 argument
    fn from_args_expect_1<CTX: CtxtForType>(
        ctxt: &CTX,
        pack: &AngleBracketedGenericArguments,
    ) -> Result<Self> {
        let mut iter = Self::from_args_in_type_path(ctxt, pack)?.into_iter();
        let a1 = match iter.next() {
            None => bail_on!(pack, "expect 1 argument, found 0"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail_on!(pack, "expect 1 argument, found 2+");
        }
        Ok(a1)
    }

    /// Convert from a type argument pack, expecting 2 arguments
    fn from_args_expect_2<CTX: CtxtForType>(
        ctxt: &CTX,
        pack: &AngleBracketedGenericArguments,
    ) -> Result<(Self, Self)> {
        let mut iter = Self::from_args_in_type_path(ctxt, pack)?.into_iter();
        let a1 = match iter.next() {
            None => bail_on!(pack, "expect 2 argument, found 0"),
            Some(t) => t,
        };
        let a2 = match iter.next() {
            None => bail_on!(pack, "expect 2 argument, found 1"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail_on!(pack, "expect 2 argument, found 3+");
        }
        Ok((a1, a2))
    }

    /// Convert from a path to TypeTag
    fn from_type_path<CTX: CtxtForType>(ctxt: &CTX, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        // no leading colon allowed
        bail_if_exists!(leading_colon);
        // there must be exactly one segment
        let mut iter = segments.iter();
        let segment = bail_if_missing!(iter.next(), path, "ident");
        bail_if_exists!(iter.next());

        // extract the ident and arguments
        let PathSegment { ident, arguments } = segment;

        // try to convert the identifier of the path into a TypeName & ctxt.generics() is used to get the generics of the current type
        // This checks if the type name is a reserved type name or a user-defined name or a type parameter
        let tag = match TypeName::try_from(ctxt.generics(), ident)? {
            TypeName::Sys(intrinsic) => match (intrinsic, arguments) {
                // a boolean, integer, rational, text, or error type cannot have any arguments
                (SysTypeName::Boolean, PathArguments::None) => Self::Boolean,
                (SysTypeName::Integer, PathArguments::None) => Self::Integer,
                (SysTypeName::Real, PathArguments::None) => Self::Real,
                (SysTypeName::F32, PathArguments::None) => Self::F32,
                (SysTypeName::F64, PathArguments::None) => Self::F64,
                (SysTypeName::I32, PathArguments::None) => Self::I32,
                (SysTypeName::I64, PathArguments::None) => Self::I64,
                (SysTypeName::U32, PathArguments::None) => Self::U32,
                (SysTypeName::U64, PathArguments::None) => Self::U64,
                (SysTypeName::String, PathArguments::None) => Self::String,
                // a cloak, seq, set type must have exactly one argument
                (SysTypeName::Cloak, PathArguments::AngleBracketed(pack)) => {
                    let sub = Self::from_args_expect_1(ctxt, pack)?;
                    Self::Cloak(sub.into())
                }
                (SysTypeName::Seq, PathArguments::AngleBracketed(pack)) => {
                    let sub = Self::from_args_expect_1(ctxt, pack)?;
                    Self::Seq(sub.into())
                }
                (SysTypeName::Set, PathArguments::AngleBracketed(pack)) => {
                    let sub = Self::from_args_expect_1(ctxt, pack)?;
                    Self::Set(sub.into())
                }
                // a map type must have exactly two arguments
                (SysTypeName::Array, PathArguments::AngleBracketed(pack)) => {
                    let (key, val) = Self::from_args_expect_2(ctxt, pack)?;
                    Self::Array(key.into(), val.into())
                }
                (SysTypeName::Error, PathArguments::None) => Self::Error,
                _ => bail_on!(segment, "invalid type tag for intrinsic type"),
            },
            // a user-defined type name
            // ctxt.get_type_generics(&name) checks if the user-defined type name exists at all
            // the way this happens is that the generics are checked for the type name in the WHOLE context
            // because a user defined type is either an enum or a struct so they are stored in the context
            // Note that any type needs to be annotated with #[smt_type] to be stored in the context
            // (that is why the whole context needs to be passed on from TypeBody::from_marked - for the user-type definition check)
            // and if it exists (even empty), the type name is valid
            TypeName::Usr(name) => match ctxt.get_type_generics(&name) {
                None => bail_on!(ident, "no such type"), // if the type name does not exist, an error is thrown
                Some(generics) => match generics.params.len() {
                    // a defined type with no generics is a simple type
                    0 => {
                        // obviously, if the defined type has no generics, it cannot have any arguments (like Vec<i32>)
                        if !matches!(arguments, PathArguments::None) {
                            bail_on!(arguments, "unexpected");
                        }
                        Self::User(name, vec![])
                    }
                    n => match arguments {
                        // a defined type with generics must have the same number of arguments
                        // for example, this is not allowed: MyType<i32, i32> where MyType is defined as MyType<T>
                        PathArguments::None => bail_on!(ident, "expect type arguments"),
                        PathArguments::AngleBracketed(pack) => {
                            // convert the type arguments to TypeTag
                            let args = Self::from_args_in_type_path(ctxt, pack)?;
                            if args.len() != n {
                                bail_on!(arguments, "type argument number mismatch");
                            }
                            Self::User(name, args)
                        }
                        // Parenthesized path arguments are not allowed
                        // for example, this is not allowed: MyType(i32) as this doesn't show generic arguments!
                        _ => bail_on!(arguments, "invalid parenthesized type arguments"),
                    },
                },
            },
            // a type parameter in TypeName is converted to a TypeTag::Parameter
            TypeName::Param(name) => Self::Parameter(name),
        };

        Ok(tag)
    }

    /// Convert from a type to TypeTag
    ///
    /// Use cases:
    ///
    /// Used for fields of each of the variants of an enum for example
    /// enum MyEnum {Variant(i32,String)} the i32, String part is processed by this function.
    /// enum MyEnum {Variant {a: i32, b: i32}} the i32, i32 part is processed by this function.
    ///
    /// Also used for fields of a struct for example
    /// struct MyStruct(i32, i32) the i32, i32 part is processed by this function.
    /// struct MyStruct {a: i32, b: i32} the i32, i32 part is processed by this function.
    ///
    /// Process:
    ///
    /// A tuple is converted into TypeTag::Pack and a path is converted calling Self::from_type_path(ctxt, path)
    ///
    /// Errors:
    ///
    /// If the type is not a path or a tuple, an error is thrown. So only tuples and path types are allowed for fields of variants.
    /// If the path has a qself, an error is thrown. For example, this is not allowed: <Vec<T> as SomeTrait>::Associated
    pub fn from_type<CTX: CtxtForType>(ctxt: &CTX, ty: &Type) -> Result<Self> {
        match ty {
            // simple types
            Type::Path(TypePath { qself, path }) => {
                // no qself allowed, for example this is not allowed: <Vec<T> as SomeTrait>::Associated
                bail_if_exists!(qself.as_ref().map(|q| q.ty.as_ref()));
                Self::from_type_path(ctxt, path)
            }
            // tuple types (TypeTuple has been renamed to TypePack because an already defined TypeTuple struct exists in the current scope)
            Type::Tuple(TypePack {
                paren_token: _,
                elems,
            }) => {
                let mut pack = vec![];
                for elem in elems {
                    // recursively convert each element of the tuple to TypeTag
                    pack.push(Self::from_type(ctxt, elem)?); // allows embedded tuples.
                }
                // a tuple is represented as a pack of types
                Ok(Self::Pack(pack))
            }
            // no other types are allowed except for paths and tuples
            _ => bail_on!(ty, "expect type path or tuple"),
        }
    }

    /// Convert a series of generic arguments
    /// This function is used when calling a function with generic arguments. like: MyType::<i32, i32>. The function is called on the ::<i32,i32> in this case.
    pub fn from_generics<CTX: CtxtForType>(
        ctxt: &CTX,
        generics: &AngleBracketedGenericArguments,
    ) -> Result<Vec<Self>> {
        let AngleBracketedGenericArguments {
            colon2_token,
            lt_token: _,
            args,
            gt_token: _,
        } = generics;
        bail_if_missing!(colon2_token, generics, "::");
        bail_if_empty!(args, generics, "type argument");

        let mut arguments = vec![];
        for item in args {
            match item {
                GenericArgument::Type(ty_arg) => {
                    let t = Self::from_type(ctxt, ty_arg)?;
                    arguments.push(t);
                }
                _ => bail_on!(item, "not a type argument"),
            }
        }
        Ok(arguments)
    }
}

impl Display for TypeTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean => write!(f, "Boolean"),
            Self::Integer => write!(f, "Integer"),
            Self::Real => write!(f, "Real"),
            Self::F32 => write!(f, "F32"),
            Self::F64 => write!(f, "F64"),
            Self::I32 => write!(f, "I32"),
            Self::I64 => write!(f, "I64"),
            Self::U32 => write!(f, "U32"),
            Self::U64 => write!(f, "U64"),
            Self::String => write!(f, "String"),
            Self::Cloak(sub) => write!(f, "Cloak<{sub}>"),
            Self::Seq(sub) => write!(f, "Seq<{sub}>"),
            Self::Set(sub) => write!(f, "Set<{sub}>"),
            Self::Array(key, val) => write!(f, "Array<{key},{val}>"),
            Self::Error => write!(f, "Error"),
            Self::User(name, args) => {
                if args.is_empty() {
                    name.fmt(f)
                } else {
                    write!(f, "{}<{}>", name, args.iter().format(","))
                }
            }
            Self::Pack(elems) => {
                write!(f, "({})", elems.iter().format(","))
            }
            Self::Parameter(name) => name.fmt(f),
        }
    }
}

#[derive(Debug)]
/// Represents a tuple definition
/// TypeTuple encapsulates a list of types
/// ADT for a struct with unnamed fields or an enum variant with unnamed fields
/// for example: struct MyStruct(i32,String); or X in enum MyEnum {X(String,i32),}
pub struct TypeTuple {
    pub slots: Vec<TypeTag>,
}

impl TypeTuple {
    /// Convert from a fields token
    /// This is used to parse the fields of a tuple struct or each of the variants of an enum
    /// for example in enum MyEnum {Variant(i32,String)}, the i32, String part is processed by this function.
    /// or in struct MyStruct(i32, i32), the i32, i32 part is processed by this function.
    fn from_fields<'a, I: Iterator<Item = &'a Field>, CTX: CtxtForType>(
        ctxt: &CTX,
        items: I,
    ) -> Result<Self> {
        let mut slots = vec![];

        // for each of the fields, TypeTag::from_type(ctxt, ty)? generates the tag from the type and inserts it into the slot.
        for field in items {
            let Field {
                attrs: _,
                vis: _,
                mutability,
                ident,
                colon_token,
                ty,
            } = field;

            // this will never happen *for now* as FieldMutability is always None
            if !matches!(mutability, FieldMutability::None) {
                bail_on!(field, "unexpected slot mutability");
            }

            // if the slot has a name, an error is returned
            bail_if_exists!(ident);
            bail_if_exists!(colon_token);

            let tag = TypeTag::from_type(ctxt, ty)?; // convert the type to TypeTag
            slots.push(tag);
        }

        Ok(Self { slots })
    }
}

impl Display for TypeTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.slots.iter().format(","))
    }
}

#[derive(Debug)]
/// Represents a record definition
/// TypeRecord encapsulates a map between the field names and their respective types
/// ADT for a struct with named fields or an enum variant with named fields
pub struct TypeRecord {
    pub fields: BTreeMap<String, TypeTag>,
}

impl TypeRecord {
    /// Convert from a fields token
    /// This is used to parse the fields of a struct or each of the variants of an enum
    /// for example in enum MyEnum {Variant {a: i32, b: i32}}, the a: i32, b: i32 part is processed by this function.
    /// or in struct MyStruct {a: i32, b: i32}, the a: i32, b: i32 part is processed by this function.
    fn from_fields<'b, I: Iterator<Item = &'b Field>, CTX: CtxtForType>(
        ctxt: &CTX,
        items: I,
    ) -> Result<Self> {
        let mut fields = BTreeMap::new();

        // for each of the fields, TypeTag::from_type(ctxt, ty)? generates the tag from the type. It is paired with the name and inserted into the map.
        for field in items {
            let Field {
                attrs: _,
                vis: _,
                mutability,
                ident,
                colon_token,
                ty,
            } = field;

            // this will never happen *for now* as FieldMutability is always None
            if !matches!(mutability, FieldMutability::None) {
                bail_on!(field, "unexpected field mutability");
            }

            // if the field has no name, an error is returned
            let name = bail_if_missing!(ident, field, "name");
            // if the field has no colon, an error is returned
            bail_if_missing!(colon_token, field, "colon");

            let tag = TypeTag::from_type(ctxt, ty)?;
            // if the field name already exists, an error is returned (caught by the rust compiler)
            match fields.insert(name.to_string(), tag) {
                None => (),
                Some(_) => bail_on!(ident, "duplicated field name"),
            }
        }

        Ok(Self { fields })
    }
}

impl Display for TypeRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{{}}}",
            self.fields
                .iter()
                .format_with(",", |(n, t), p| p(&format_args!("{n}:{t}"))),
        )
    }
}

#[derive(Debug)]
/// A helper enum to represent a variant definition in an ADT type
/// ADT for an enum variant
/// - Unit: no fields like Variant1
/// - Tuple: unnamed fields like Variant2(i32)
/// - Record: named fields like Variant3 {sub_variant:i32}
///   These three variations are exactly similar for an empty struct, tuple struct, and record struct respectively.
pub enum EnumVariant {
    Unit,
    Tuple(TypeTuple),
    Record(TypeRecord),
}

impl EnumVariant {
    /// Convert a struct or each of the variants of an enum to EnumVariant.
    /// Depending on whether the field is a unit, unnamed, or named, the EnumVariant for that variant is built for unit, tuple, or record, respectively.
    fn from_fields<CTX: CtxtForType>(ctxt: &CTX, fields: &Fields) -> Result<Self> {
        let variant = match fields {
            Fields::Unit => Self::Unit, // the field is Variant1 for example
            Fields::Named(FieldsNamed {
                // the field is Variant2 {sub_variant:i32} for example
                brace_token: _,
                named,
            }) => Self::Record(TypeRecord::from_fields(ctxt, named.iter())?), // construct the TypeRecord
            Fields::Unnamed(FieldsUnnamed {
                // the field is Variant3(i32) for example
                paren_token: _,
                unnamed,
            }) => Self::Tuple(TypeTuple::from_fields(ctxt, unnamed.iter())?), // construct the TypeTuple
        };
        Ok(variant)
    }
}

impl Display for EnumVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => f.write_str(""),
            Self::Tuple(details) => details.fmt(f),
            Self::Record(details) => details.fmt(f),
        }
    }
}

#[derive(Debug)]
/// Represents an ADT definition
/// TypeEnum encapsulates a map between the variant names and their respective EnumVariant (structure) for a single enum type
pub struct TypeEnum {
    pub variants: BTreeMap<String, EnumVariant>,
}

impl TypeEnum {
    /// Convert from a list of variants
    fn from_variants<'a, I: Iterator<Item = &'a Variant>, CTX: CtxtForType>(
        ctxt: &CTX,
        items: I,
    ) -> Result<Self> {
        let mut variants = BTreeMap::new();

        // for each of the variants, EnumVariant::from_fields(ctxt, fields)? generates the EnumVariant from the fields.
        for variant in items {
            let Variant {
                attrs: _,
                ident,
                fields,
                discriminant,
            } = variant;

            // if there is a discriminant, an error is returned
            bail_if_exists!(discriminant.as_ref().map(|(_, e)| e));

            // for each variant, the EnumVariant is constructed.
            let branch = EnumVariant::from_fields(ctxt, fields)?;

            // check variant consistency
            match &branch {
                EnumVariant::Unit => (), // inside an enum, a unit variant is allowed but a unit struct is not allowed
                EnumVariant::Tuple(tuple) => bail_if_empty!(tuple.slots, variant, "slots"), // this will throw an error for: pub enum MyEnum { A() } becuase A is a tuple variant with no slots
                EnumVariant::Record(record) => bail_if_empty!(record.fields, variant, "fields"), // this will throw an error for: pub enum MyEnum { A {} } becuase A is a record variant with no fields
            }

            // insert the variant
            // if the variant name already exists, an error is returned
            match variants.insert(ident.to_string(), branch) {
                None => (),
                Some(_) => bail_on!(ident, "duplicated variant name"),
            }
        }

        Ok(Self { variants })
    }
}

impl Display for TypeEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.variants
                .iter()
                .format_with("\n", |(n, t), p| p(&format_args!("{n}:{t}"))),
        )
    }
}

#[derive(Debug)]
/// The body part of a type definition
/// A type definition is either an enum or a struct
/// enums are represented by TypeBody::Enum(TypeEnum)
/// structs are represented by TypeBody::Tuple(TypeTuple) or TypeBody::Record(TypeRecord)
pub enum TypeBody {
    Tuple(TypeTuple),
    Record(TypeRecord),
    Enum(TypeEnum),
}

impl TypeBody {
    /// Convert from a MarkedType (marked type) to a TypeBody (type body)
    pub fn from_marked(
        ctxt: &ContextWithGenerics,
        generics: &Generics,
        item: &MarkedType,
    ) -> Result<Self> {
        // we encapsulate the `context with generics` and the `generics` themselves
        // A context provider for type parsing
        let ctxt = TypeParseCtxt {
            ctxt,     // the whole context containing all the types
            generics, // generics of the current type as the from_marked function is used inside a loop in ctxt.rs for each type
        };

        // depending on the item type (enum or struct) we have different parsing strategies
        let parsed = match item {
            MarkedType::Enum(item) => {
                let ItemEnum {
                    attrs: _,
                    vis: _,
                    enum_token: _,
                    ident: _, // handled earlier (the fact that the ident must be unique is checked in the context - in sanity check, etc.)
                    generics: _, // handled earlier (when the conversion from Context to ContextWithGenerics happened in parse_generics)
                    brace_token: _, // enums do not have semicolons at the end
                    variants,
                } = item;
                bail_if_empty!(variants, item, "variants"); // expect at least one variant so enum MyEnum {} is not allowed

                // converts MarkedType::Enum(ItemEnum) to TypeBody::Enum(TypeEnum)
                // TypeBody::Enum is the same as MarkedType::Enum but the latter is a wrapper for the syn enum type and the former is a wrapper for the TypeEnum.
                // So the syn enum type is converted into TypeEnum. The TypeEnum inside is built by the TypeEnum::from_variants function.
                // A TypeEnum is a wrapper of a map between the variant names and EnumVariant.
                // so it encapsulates the list of all variants of the enum.
                Self::Enum(TypeEnum::from_variants(&ctxt, variants.iter())?)
            }
            // only handles the fields of the struct
            MarkedType::Struct(item) => {
                let ItemStruct {
                    attrs: _,
                    vis: _,
                    struct_token: _,
                    ident: _, // handled earlier (the fact that the ident must be unique is checked in the context - in sanity check, etc.)
                    generics: _, // handled earlier (when the conversion from Context to ContextWithGenerics happened in parse_generics)
                    fields,
                    semi_token, // record structs do not have semicolons at the end but tuple structs do. This is enforced by the rust compiler.
                } = item;

                // exploit the similarity with ADT variant
                // basically each variant in an enum is either a unit struct, unnamed struct (tuple struct), or named struct (record struct)
                match EnumVariant::from_fields(&ctxt, fields)? {
                    EnumVariant::Unit => bail_on!(item, "expect fields or slots"), // a unit struct is not allowed so struct MyStruct; is not allowed
                    EnumVariant::Tuple(tuple) => {
                        // a tuple struct must have at least one slot so struct MyStruct() is not allowed
                        bail_if_empty!(tuple.slots, item, "slots");
                        // a tuple struct must have a semicolon at the end (like struct MyStruct(i32);) - this will never happen because it is caught by the rust compiler
                        bail_if_missing!(semi_token, item, "expect ; at the end");
                        Self::Tuple(tuple)
                    }
                    EnumVariant::Record(record) => {
                        // a record struct must have at least one field so struct MyStruct{} is not allowed
                        bail_if_empty!(record.fields, item, "fields");
                        // a record struct must not have a semicolon at the end (like struct MyStruct {a:i32}) - this will never happen because it is caught by the rust compiler
                        bail_if_exists!(semi_token);
                        Self::Record(record)
                    }
                }
            }
        };
        Ok(parsed)
    }
}

impl Display for TypeBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tuple(details) => details.fmt(f),
            Self::Record(details) => details.fmt(f),
            Self::Enum(details) => details.fmt(f),
        }
    }
}

#[derive(Debug)]
/// A complete definition of a type (generics + body)
pub struct TypeDef {
    pub head: Generics,
    pub body: TypeBody,
}

impl Display for TypeDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{{ {} }}", self.head, self.body)
    }
}

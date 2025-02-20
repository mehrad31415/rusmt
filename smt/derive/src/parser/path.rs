use std::fmt::{Display, Formatter};
use syn::{Expr as Exp, ExprPath, Path, PathArguments, PathSegment, Result};

use crate::parser::expr::CtxtForExpr;
use crate::parser::func::{CastFuncName, FuncName, SysFuncName};
use crate::parser::generics::{Generics, GenericsInstFull, GenericsInstPartial};
use crate::parser::infer::TypeUnifier;
use crate::parser::name::{TypeParamName, UsrFuncName, UsrTypeName, VarName};
use crate::parser::ty::{SysTypeName, TypeBody, TypeName};
use crate::{bail_if_exists, bail_if_missing, bail_on};

/// An identifier for a ADT variant
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct ADTBranch {
    pub ty_name: UsrTypeName, // the enum type name
    pub variant: String,      // the variant name
}

impl Display for ADTBranch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.ty_name, self.variant)
    }
}

/// An identifier to a tuple with optional type arguments
pub struct TuplePath {
    ty_name: UsrTypeName,
    ty_args: GenericsInstPartial,
}

impl TuplePath {
    /// Extract a reference to a tuple from a path
    pub fn from_path<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        bail_if_exists!(leading_colon);
        let mut iter = segments.iter();

        // type
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "type name");
        let ty_name = ident.try_into()?;
        let generics = match ctxt.get_type_def(&ty_name) {
            None => bail_on!(ident, "no such type"),
            Some(def) => {
                // it needs to be a tuple type
                if !matches!(def.body, TypeBody::Tuple(_)) {
                    bail_on!(ident, "not a tuple type");
                }
                &def.head
            }
        };
        // match the type arguments with the generics
        let ty_args = GenericsInstPartial::from_args(ctxt, generics, arguments)?;

        // ensure that there are no more tokens (there is only one segment allowed)
        bail_if_exists!(iter.next());
        Ok(Self { ty_name, ty_args })
    }

    /// Turn the path into a name and full instantiation
    pub fn complete(self, unifier: &mut TypeUnifier) -> (UsrTypeName, GenericsInstFull) {
        let Self { ty_name, ty_args } = self;
        (ty_name, ty_args.complete(unifier))
    }
}

/// An identifier to a record with optional type arguments
pub struct RecordPath {
    ty_name: UsrTypeName,
    ty_args: GenericsInstPartial,
}

impl RecordPath {
    /// Extract a reference to a record from a path (converts the Path into a RecordPath)
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn from_path<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        // leading_colon is not allowed, for example this is not allowed: ::SomeType
        bail_if_exists!(leading_colon);
        let mut iter = segments.iter();

        // type
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "type name"); // first segment is the type name

        // ensure that there are no more tokens (only 1 segment is allowed)
        bail_if_exists!(iter.next());

        // get the type name from the identifier (if the ident is not a reserved keyword and not an underscore, it is converted to a UsrTypeName)
        let ty_name: UsrTypeName = ident.try_into()?;

        let generics = match ctxt.get_type_def(&ty_name) {
            None => bail_on!(ident, "no such type"), // if the type name is not found in the context, return an error (so the type should be marked as #[smt_type]). This means that no built in type from rust can be used (or no enums/structs which are not marked with #[smt_type]). Instead a wrapper enum marked as #[smt_type] should be created to use the built in type.
            Some(def) => {
                // Note that tuple structs are Expr::Call not Expr::Struct so they are not handled here (so I assume that this error will never be triggered)
                if !matches!(def.body, TypeBody::Record(_)) {
                    bail_on!(ident, "not a record type"); // if the type is defined but not a record type, return an error
                }
                &def.head
            }
        };

        // match the type arguments with the generics
        let ty_args = GenericsInstPartial::from_args(ctxt, generics, arguments)?;

        Ok(Self { ty_name, ty_args }) // ty_name is the type name and ty_args is the type arguments associated with the type parameters
    }

    /// Turn the path into a name and full instantiation
    pub fn complete(self, unifier: &mut TypeUnifier) -> (UsrTypeName, GenericsInstFull) {
        let Self { ty_name, ty_args } = self;
        (ty_name, ty_args.complete(unifier))
    }
}

/// An identifier to an ADT variant with optional type arguments
pub struct ADTPath {
    ty_name: UsrTypeName,         // the enum type name
    ty_args: GenericsInstPartial, // the type arguments
    variant: String,              // the respective variant
}

impl ADTPath {
    /// Extract a reference to an ADT variant from a path (converts the path into an ADTPath)
    /// This basically just parses a enum variant which is a unit struct for example: MyEnum::MyVariant and this can have type arguments like MyEnum::<T>::MyVariant
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn from_path<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        // leading_colon is not allowed, for example this is not allowed: ::SomeType
        bail_if_exists!(leading_colon);
        let mut iter = segments.iter();

        // type (first segment is the enum type name)
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "type name");
        let ty_name = ident.try_into()?; // if the ident is not a reserved keyword and not an underscore, it is converted to a UsrTypeName
        let (generics, variants) = match ctxt.get_type_def(&ty_name) {
            // get the type definition from the context for the user type name
            None => bail_on!(ident, "no such type"), // if the type name is not found in the context, return an error (so the type should be marked as #[smt_type]). This means that no built in type from rust can be used (or no enums/structs which are not marked with #[smt_type]). Instead a wrapper enum marked as #[smt_type] should be created to use the built in type. This is so that the rusmart is closed and no external types can be used.
            Some(def) => match &def.body {
                TypeBody::Enum(details) => (&def.head, &details.variants), // if the type is an enum type, return the generics of the definition and variants (BTreeMap<String, EnumVariant>). details is a wrapper around the enum variants.
                _ => bail_on!(ident, "not an enum type"), // if the type is defined but not an enum type (it is a struct), return an error. A struct is an Expr::Struct and not an Expr::Path. However, a unit struct is Expr::Path. But in rusmart a unit struct is not allowed to be used as a type.
            },
        };

        // match the type arguments with the generics
        // the arguments are the type arguments for the first path segment
        // generics is the generics of the type definition
        let ty_args = GenericsInstPartial::from_args(ctxt, generics, arguments)?;

        // variant (second segment is the branch)
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "branch");

        // this is allowed in rust:
        // enum MyEnum<T, U> {
        //     Struct {
        //         data: MyStruct<T, U>,
        //     },
        //     AnotherVariant,
        // }
        // this is not allowed in rust:
        // enum MyEnum {
        //     Struct<T, U> {
        //         data: MyStruct<T, U>,
        //     },
        //     AnotherVariant,
        // }
        // so in inline direct nesting of a struct inside an enum, the type parameters cannot be specified for the struct
        if !matches!(arguments, PathArguments::None) {
            bail_on!(arguments, "unexpected arguments"); // if there are arguments for the variant, return an error
        }

        let variant = ident.to_string();
        // `variants` is the list of variants in the enum definition
        // if the variant is not found in the variants of the enum definition, return an error
        if !variants.contains_key(&variant) {
            bail_on!(ident, "no such variant");
        }

        // ensure that there are no more tokens (there are only 2 segments allowed)
        bail_if_exists!(iter.next());
        Ok(Self {
            ty_name, // the enum type name
            ty_args, // the type arguments
            variant, // the respective variant
        })
    }

    /// Turn the path into a branch and full instantiation
    /// The ADTBranch is a self-defined ADT that contains the enum type name and the variant
    pub fn complete(self, unifier: &mut TypeUnifier) -> (ADTBranch, GenericsInstFull) {
        let Self {
            ty_name,
            ty_args,
            variant,
        } = self;
        (ADTBranch { ty_name, variant }, ty_args.complete(unifier))
    }
}

/// An identifier to a function without a qualified type
pub struct FuncPath {
    fn_name: UsrFuncName,
    ty_args: GenericsInstPartial,
}

impl FuncPath {
    /// Extract a reference to a function from a path
    pub fn from_path<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        bail_if_exists!(leading_colon);
        let mut iter = segments.iter();

        // func
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "function name"); // first segment is the function name

        // ensure that there are no more tokens
        bail_if_exists!(iter.next());

        let fn_name = ident.try_into()?;
        let generics = match ctxt.lookup_unqualified(&fn_name) {
            None => bail_on!(ident, "no such function"), // if the function name is not found in the context, return an error
            Some(fty) => &fty.generics,
        };
        let ty_args = GenericsInstPartial::from_args(ctxt, generics, arguments)?;

        Ok(Self { fn_name, ty_args })
    }

    /// Turn the path into a name and full instantiation
    pub fn complete(self, unifier: &mut TypeUnifier) -> (UsrFuncName, GenericsInstFull) {
        let Self { fn_name, ty_args } = self;
        (fn_name, ty_args.complete(unifier))
    }
}

/// An identifier for a function with type variables
pub enum QualifiedPath {
    /// `Boolean::from(<literal>)`
    CastFromBool,
    /// `Integer::from(<literal>)`
    CastFromInt,
    /// `Rational::from(<literal>)`
    CastFromFloat,
    /// `Text::from(<literal>)`
    CastFromStr,
    /// `<sys-type>::[type-inst]::<sys-func>(<args>)`
    SysFuncOnSysType(SysTypeName, GenericsInstPartial, SysFuncName),
    /// `<usr-type>::[type-inst]::<sys-func>(<args>)`
    SysFuncOnUsrType(UsrTypeName, GenericsInstPartial, SysFuncName),
    /// `<type-param>::<sys-func>(<args>)`
    SysFuncOnParamType(TypeParamName, SysFuncName),
    /// `<sys-type>::[type-inst]::<usr-func>::[type-inst](<args>)`
    UsrFuncOnSysType(
        SysTypeName,
        GenericsInstPartial,
        UsrFuncName,
        GenericsInstPartial,
    ),
    /// `<usr-type>::[type-inst]::<usr-func>::[type-inst](<args>)`
    UsrFuncOnUsrType(
        UsrTypeName,
        GenericsInstPartial,
        UsrFuncName,
        GenericsInstPartial,
    ),
}

impl QualifiedPath {
    /// Extract a reference to a qualified call from a path
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn from_path<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        let Path {
            leading_colon,
            segments,
        } = path;
        bail_if_exists!(leading_colon);
        let mut iter = segments.iter();

        // type
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "type name"); // first segment is the type name

        // ctxt.generics() retrieves the generics of the current function in the signature
        let ty_name = TypeName::try_from(ctxt.generics(), ident)?;
        let ty_generics = match &ty_name {
            TypeName::Sys(name) => name.generics(),
            TypeName::Usr(name) => match ctxt.get_type_def(name) {
                None => bail_on!(ident, "no such type"),
                Some(def) => def.head.clone(),
            },
            TypeName::Param(_) => Generics::intrinsic(vec![]),
        };
        let inst_for_ty = GenericsInstPartial::from_args(ctxt, &ty_generics, arguments)?;

        // func
        let PathSegment { ident, arguments } = bail_if_missing!(iter.next(), path, "type name");
        let fn_name = FuncName::try_from(ident)?;
        let converted = match &fn_name {
            // into is not allowed
            // from is only allowed for the following types: Boolean, Integer, Rational, Text (literal types)
            FuncName::Cast(name) => match name {
                CastFuncName::Into => bail_on!(ident, "not allowed"),
                CastFuncName::From => {
                    if !matches!(arguments, PathArguments::None) {
                        bail_on!(arguments, "unexpected");
                    }
                    match ty_name {
                        TypeName::Sys(SysTypeName::Boolean) => Self::CastFromBool,
                        TypeName::Sys(SysTypeName::Integer) => Self::CastFromInt,
                        TypeName::Sys(SysTypeName::Rational) => Self::CastFromFloat,
                        TypeName::Sys(SysTypeName::Text) => Self::CastFromStr,
                        _ => bail_on!(ident, "not a literal type {:?}", fn_name),
                    }
                }
            },
            // eq and ne
            FuncName::Sys(name) => {
                // none of the smt-native functions take type arguments
                if !matches!(arguments, PathArguments::None) {
                    bail_on!(arguments, "unexpected");
                }
                match ty_name {
                    TypeName::Sys(sys_name) => Self::SysFuncOnSysType(sys_name, inst_for_ty, *name),
                    TypeName::Usr(usr_name) => Self::SysFuncOnUsrType(usr_name, inst_for_ty, *name),
                    TypeName::Param(param) => Self::SysFuncOnParamType(param, *name),
                }
            }
            FuncName::Usr(name) => match ty_name {
                TypeName::Param(_) => {
                    bail_on!(ident, "user-defined function on type parameter")
                }
                TypeName::Sys(ty_name) => match ctxt.lookup_usr_func_on_sys_type(&ty_name, name) {
                    None => bail_on!(ident, "no such intrinsic function"),
                    Some(fty) => {
                        let inst_for_fn =
                            GenericsInstPartial::from_args(ctxt, &fty.generics, arguments)?;
                        // the first parameter is like Integer, the second parameter is the mappping of the type arguments, the third is like add, the fourth is the mapping of the function arguments for example in Integer::<U>::add::<V>(x, y) the first parameter is Integer, the second is U, the third is add, the fourth is V
                        Self::UsrFuncOnSysType(ty_name, inst_for_ty, name.clone(), inst_for_fn)
                    }
                },
                TypeName::Usr(ty_name) => match ctxt.lookup_usr_func_on_usr_type(&ty_name, name) {
                    None => bail_on!(ident, "no such function"),
                    Some(fty) => {
                        let inst_for_fn =
                            GenericsInstPartial::from_args(ctxt, &fty.generics, arguments)?;
                        Self::UsrFuncOnUsrType(ty_name, inst_for_ty, name.clone(), inst_for_fn)
                    }
                },
            },
            // cannot be clone or default
            FuncName::Reserved(_) => bail_on!(ident, "reserved function"),
        };

        // ensure that there are no more tokens
        bail_if_exists!(iter.next());
        Ok(converted)
    }
}

/// Marks what a path serving as a standalone expression can be
/// An ADT for the defined `ExprPath` in the library
pub enum ExprPathAsTarget {
    /// a local variable
    Var(VarName),
    /// a unit variant in an enum
    EnumUnit(ADTPath),
}

impl ExprPathAsTarget {
    /// Parse from an expression
    /// This function is used to parse the inner part of Exp::Path(ExprPath); i.e. the ExprPath
    /// Converts the ExprPath into an ExprPathAsTarget (which is a self-defined ADT for the ExprPath)
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn parse<T: CtxtForExpr>(ctxt: &T, expr_path: &ExprPath) -> Result<Self> {
        let ExprPath {
            attrs: _,
            qself,
            path,
        } = expr_path;
        // no qself (short for Qualified Self) allowed, for example this is not allowed:
        // <Type as Trait>::x::y ... qself specifies the part before the as (e.g., Type).
        // Trait, x, and y are the path segments.
        // <Type as Trait>::x::y becomes handy when a Type implements different traits with the same function name for example. Writing Type::function_name would be ambiguous in this case.
        bail_if_exists!(qself.as_ref().map(|q| q.ty.as_ref()));

        // case by number of path segments
        let parsed = match path.segments.len() {
            // if there is only 1 segment, then it is a ExprPathAsTarget::Var(VarName)
            1 => Self::Var(path.try_into()?), // this gives an error if the path has a leading colon or has more than 1 segment or if the path has a type argument (like Vec<T>), otherwise it extracts the identifier. Should it not be a reserved keyword or underscore, then it is converted to a VarName
            // if there are 2 segments, then it is a ExprPathAsTarget::EnumUnit(ADTPath)
            2 => Self::EnumUnit(ADTPath::from_path(ctxt, path)?), // ADTPath::from_path is a function that extracts the ADTPath from the Path
            _ => bail_on!(path, "unrecognized path"), // if there are more than 2 segments, it is not recognized for example: Vec<T>::push::<T> has 3 segments. So everything should be defined in the context to be recognized. So for example, bringing use syn; and then using syn::Expr::Path will not be recognized.
        };
        Ok(parsed)
    }
}

/// Marks what a path serving as a callee expr can be
pub enum ExprPathAsCallee {
    /// `<type-name>::[ty-args]::<func-name>::[ty-args](...)`
    FuncWithType(QualifiedPath),
    /// `<usr-func-name>::[ty-args](...)`
    FuncNoPrefix(FuncPath),
    /// `<adt>::[ty-args]::<variant>(...)`
    CtorEnum(ADTPath),
    /// `<tuple>::[ty-args](...)`
    CtorTuple(TuplePath),
}

impl ExprPathAsCallee {
    /// Parse from an expression (converts the expression into an ExprPathAsCallee)
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn parse<T: CtxtForExpr>(ctxt: &T, expr: &Exp) -> Result<Self> {
        let path = match expr {
            Exp::Path(p) => {
                let ExprPath {
                    attrs: _,
                    qself,
                    path,
                } = p;
                bail_if_exists!(qself.as_ref().map(|q| q.ty.as_ref()));
                path
            }
            // we can only have invoke(a) where `invoke` is a path
            _ => bail_on!(expr, "invalid callee"),
        };

        // case by number of path segments
        let parsed = match path.segments.len() {
            1 => {
                match (
                    TuplePath::from_path(ctxt, path), // like let a = MyTuple(1) where struct MyTuple(i32);
                    FuncPath::from_path(ctxt, path), // like let a = my_func() where fn my_func() -> i32 { 1 }
                ) {
                    (Ok(_), Ok(_)) => bail_on!(path, "ambiguous callee"),
                    (Ok(tuple), Err(_)) => Self::CtorTuple(tuple),
                    (Err(_), Ok(func)) => Self::FuncNoPrefix(func),
                    (Err(_), Err(e2)) => return Err(e2),
                }
            }
            2 => {
                match (
                    ADTPath::from_path(ctxt, path),
                    QualifiedPath::from_path(ctxt, path),
                ) {
                    (Ok(_), Ok(_)) => bail_on!(path, "ambiguous callee"),
                    (Ok(adt), Err(_)) => Self::CtorEnum(adt), // like let a = MyEnum::MyVariant(13) where enum MyEnum { MyVariant(i32) }
                    (Err(_), Ok(qualified)) => Self::FuncWithType(qualified), // like let a = MyType::my_func() where fn my_func() -> i32 { 1 } in this case my)func is either a function on a system type or a method in the smt of a user type (becaue impl blocks are not read into the context)
                    (Err(_), Err(e2)) => return Err(e2),
                }
            }
            // if there are more than 2 segments, it is not recognized
            _ => bail_on!(path, "unrecognized callee"),
        };
        Ok(parsed)
    }
}

/// Marks what a path serving as a record expr can be
/// An ADT for the Path inside ExprStruct in the library
pub enum ExprPathAsRecord {
    /// `<adt>::[ty-args]::<variant> { ... }`
    CtorEnum(ADTPath),
    /// `<record>::[ty-args] { ... }`
    CtorRecord(RecordPath),
}

impl ExprPathAsRecord {
    /// Parse from an expression
    /// A Path inside a struct is parsed into an ExprPathAsRecord ADT
    /// There are two cases: CtorEnum and CtorRecord
    /// - CtorRecord is when the path has only 1 segment (these are standalone structs)
    /// - CtorEnum is when the path has 2 segments (these are variants of enums which are structs themselves)
    pub fn parse<T: CtxtForExpr>(ctxt: &T, path: &Path) -> Result<Self> {
        // case by number of path segments
        let parsed = match path.segments.len() {
            1 => Self::CtorRecord(RecordPath::from_path(ctxt, path)?), // if it has only 1 segment, then it is a struct. RecordPath::from_path is a function that extracts the RecordPath from the Path
            2 => Self::CtorEnum(ADTPath::from_path(ctxt, path)?), // if it has 2 segments, then it is an enum variant (which is a struct). ADTPath::from_path is a function that extracts the ADTPath from the Path (so enum variants can be a struct like enum MyEnum { MyStruct { ... }, NotMyStruct })
            // enum MyEnum { MyStruct { x:i32 }, NotMyStruct }
            // usage: MyEnum::MyStruct { x: 1 } is a struct
            _ => bail_on!(path, "unrecognized record"), // if it has more than 2 segments, then it is not recognized (one may wonder what about x::y::z, in this case x cannot be a struct because y is a segment. Thus, x is an enum, y is a variant, which in turn should be an enum, and z is a variant of y. But nested enums are not supported in rust)
                                                        // to be precise, nested enums can be supported in rust:
                                                        // enum Inner { A, B }
                                                        // enum Outer { C(Inner) }
                                                        // usage: Outer::C(Inner::A) However in this case the path is Outer::C which is treated as a tuple struct.
                                                        // we cannot have enum Outer { Inner {A, B} } (so Outer::Inner::A is not allowed)
        };
        Ok(parsed)
    }
}

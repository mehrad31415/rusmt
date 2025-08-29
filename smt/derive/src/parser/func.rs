//! This module defines the function signature and function definition structures

use crate::parser::ctxt::ContextWithType;
use crate::parser::expr::Expr;
use crate::parser::generics::Generics;
use crate::parser::name::{ReservedIdent, TypeParamName, UsrFuncName, UsrTypeName, VarName};
use crate::parser::ty::{CtxtForType, TypeTag};
use crate::{bail_if_exists, bail_if_non_empty, bail_on};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use syn::{
    Expr as Exp, ExprMacro, FnArg, Ident, Macro, MacroDelimiter, PatType, Result, ReturnType,
    Signature, Stmt,
};

/// Represents casting-related function names
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum CastFuncName {
    /// The `from` function, used for converting from one type to another. For example, Integer::from("42") would convert the string "42" to the integer 42.
    From,
    /// The `into` function, used for converting from one type to another. For example, "42".into() would convert the string "42" to the integer 42.
    Into,
}

/// CastFuncName is a reserved identifier
impl ReservedIdent for CastFuncName {
    /// Attempts to parse a string slice into a `CastFuncName`.
    ///
    /// Returns `Some(CastFuncName)` if the string matches "from" or "into",
    /// otherwise returns `None`.
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident {
            "from" => Self::From,
            "into" => Self::Into,
            _ => return None,
        };
        Some(matched)
    }
}

impl Display for CastFuncName {
    /// Formats the `CastFuncName` as a string.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::From => "from",
            Self::Into => "into",
        };
        f.write_str(name)
    }
}

/// This enum represents names of system functions, such as `eq` and `ne`.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum SysFuncName {
    /// The `eq` function, representing equality.
    Eq,
    /// The `ne` function, representing inequality.
    Ne,
}

/// SysFuncName is a reserved identifier
impl ReservedIdent for SysFuncName {
    /// Attempts to parse a string slice into a `SysFuncName`.
    ///
    /// Returns `Some(SysFuncName)` if the string matches "eq" or "ne",
    /// otherwise returns `None`.
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident {
            "eq" => Self::Eq,
            "ne" => Self::Ne,
            _ => return None,
        };
        Some(matched)
    }
}

impl Display for SysFuncName {
    /// Formats the `SysFuncName` as a string.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
        };
        f.write_str(name)
    }
}

/// This enum represents names of reserved functions, such as `clone` and `default`.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum ReservedFuncName {
    /// The `clone` function.
    Clone,
    /// The `default` function.
    Default,
}

impl ReservedIdent for ReservedFuncName {
    /// Attempts to parse a string slice into a `ReservedFuncName`.
    ///
    /// Returns `Some(ReservedFuncName)` if the string matches "clone" or "default",
    /// otherwise returns `None`.
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident {
            "clone" => Self::Clone,
            "default" => Self::Default,
            _ => return None,
        };
        Some(matched)
    }
}

impl Display for ReservedFuncName {
    /// Formats the `ReservedFuncName` as a string.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Clone => "clone",
            Self::Default => "default",
        };
        f.write_str(name)
    }
}

/// A function name.
///
/// This enum represents a function name, which can be one of several types:
/// - A casting function (`Cast`), such as `from` or `into`.
/// - A system function (`Sys`), such as `eq` or `ne`.
/// - A user-defined function (`Usr`).
/// - A reserved function (`Reserved`), such as `clone` or `default`.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum FuncName {
    /// A casting function name.
    Cast(CastFuncName),
    /// A system function name.
    Sys(SysFuncName),
    /// A user-defined function name.
    Usr(UsrFuncName),
    /// A reserved function name.
    Reserved(ReservedFuncName),
}

impl FuncName {
    /// Attempts to convert an identifier into a `FuncName`.
    pub fn try_from(ident: &Ident) -> Result<Self> {
        let name = ident.to_string();
        let parsed = match ReservedFuncName::from_str(&name) {
            // clone and default are reserved keywords
            Some(n) => Self::Reserved(n),
            None => match CastFuncName::from_str(&name) {
                // from and into are reserved keywords
                Some(n) => Self::Cast(n),
                None => match SysFuncName::from_str(&name) {
                    // eq and ne are reserved keywords
                    Some(n) => Self::Sys(n),
                    None => Self::Usr(ident.try_into()?), // returns an error if ident is a reserved keyword or an underscore. Invokes method `fn try_from(value: &Ident) -> Result<Self> { validate_user_ident(value).map(|ident| Self { ident }) } from the name module`. Returns UsrFuncName if the ident is a valid user-defined function name.
                },
            },
        };
        Ok(parsed)
    }
}

impl Display for FuncName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cast(name) => name.fmt(f),
            Self::Sys(name) => name.fmt(f),
            Self::Usr(name) => name.fmt(f),
            Self::Reserved(name) => name.fmt(f),
        }
    }
}

/// A context provider for function signature parsing.
struct FuncSigParseCtxt<'a> {
    ctxt: &'a ContextWithType, // this field is used to check if a user defined type is present in the ctxt when calling get_type_generics which will give None if the type is not present
    generics: &'a Generics,
}

impl CtxtForType for FuncSigParseCtxt<'_> {
    /// Returns the generics associated with the type.
    fn generics(&self) -> &Generics {
        self.generics
    }

    fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.ctxt.get_type_generics(name)
    }
}

#[derive(Debug, Clone)]
/// Represents a function signature.
pub struct FuncSig {
    /// The generics associated with the function.
    pub generics: Generics,
    /// The parameters of the function, each with a name and type.
    pub params: Vec<(VarName, TypeTag)>,
    /// The return type of the function.
    pub ret_ty: TypeTag,
}

impl FuncSig {
    /// Constructs a `FuncSig` from a `syn::Signature`.
    /// Used to create ADT from the signature of a function.
    pub fn from_sig(driver: &ContextWithType, sig: &Signature) -> Result<Self> {
        let Signature {
            constness,
            asyncness,
            unsafety,
            abi,
            fn_token: _,
            ident: _, // handled earlier (the fact that the function name is unique was handled in the ctxt.rs file in the sanity check)
            generics,
            paren_token: _,
            inputs,
            variadic,
            output,
        } = sig;

        // These features are not supported and should not appear.
        bail_if_exists!(constness);
        bail_if_exists!(asyncness);
        bail_if_exists!(unsafety);
        bail_if_exists!(abi);
        bail_if_exists!(variadic);

        // Parse generics (this also ensures that the type parameters are unique and ONLY implement the SMT trait)
        let generics = Generics::from_generics(generics)?;
        let ctxt = FuncSigParseCtxt {
            ctxt: driver, // the ctxt is needed because when converting a type to TypeTag, it checks for any user defined types and that can only be checked through the typs fields in the ctxt.
            generics: &generics,
        };

        // Parse parameters
        let mut param_decls = vec![];
        let mut param_names = BTreeSet::new();
        for param in inputs {
            match param {
                FnArg::Receiver(recv) => bail_on!(recv, "unexpected self param"),
                FnArg::Typed(typed) => {
                    let PatType {
                        attrs: _,
                        pat,
                        colon_token: _,
                        ty,
                    } = typed;
                    // Parse parameter name
                    // if the pat is mutable, reference, has a sub-pattern, is a reserved keyword or underscore, return an error
                    // otherwise, convert the pattern into a variable name
                    let name: VarName = pat.as_ref().try_into()?;
                    // Check for duplicated parameter names
                    if !param_names.insert(name.clone()) {
                        bail_on!(pat, "duplicated parameter name");
                    }

                    let ty = TypeTag::from_type(&ctxt, ty)?;
                    param_decls.push((name, ty));
                }
            }
        }

        // Parse return type
        let ret_ty = match output {
            ReturnType::Default => bail_on!(sig, "expect return type"), // all functions in rusmart must have a return type (like functionally typed languages)
            ReturnType::Type(_, rty) => TypeTag::from_type(&ctxt, rty)?, // construct the TypeTag from the return type
        };

        // Construct and return the `FuncSig`
        Ok(Self {
            generics,
            params: param_decls, // holds the parameters input and their respective types
            ret_ty,
        })
    }

    /// Collects variables declared in the parameter list into a map.
    ///
    /// Returns a `BTreeMap` mapping each parameter name to its type.
    /// Basically converts a Vec<(VarName, TypeTag)> into a BTreeMap<VarName, TypeTag>
    pub fn param_map(&self) -> BTreeMap<VarName, TypeTag> {
        let param: BTreeMap<VarName, TypeTag> = self
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .collect();

        // can never happen in the pipeline because in constructing the FuncSig, we checked for duplicated parameter names
        if param.len() != self.params.len() {
            panic!("duplicated parameter name");
        }

        param
    }

    /// Tests whether two function signatures (impl-spec) are type-compatible.
    pub fn is_compatible(&self, sig: &FuncSig) -> bool {
        let FuncSig {
            generics: impl_gens,
            params: impl_params,
            ret_ty: impl_ret_ty,
        } = self;
        let FuncSig {
            generics: spec_gens,
            params: spec_params,
            ret_ty: spec_ret_ty,
        } = sig;

        // if the length of the generics lists are different, they are not compatible
        if impl_gens.params.len() != spec_gens.params.len() {
            return false;
        }

        // check if the length of the parameter lists are different, they are not compatible
        if impl_params.len() != spec_params.len() {
            return false;
        }

        // parameters and return types must be compatible
        let impl_types = impl_params
            .iter()
            .map(|(_, t)| t.clone())
            .chain(std::iter::once(impl_ret_ty.clone()))
            .collect::<Vec<TypeTag>>();
        let spec_types = spec_params
            .iter()
            .map(|(_, t)| t.clone())
            .chain(std::iter::once(spec_ret_ty.clone()))
            .collect::<Vec<TypeTag>>();

        let mut zip_gens = BTreeSet::new();
        for (impl_ty, spec_ty) in impl_types.iter().zip(spec_types.iter()) {
            if !unify(impl_ty, spec_ty, &mut zip_gens) {
                return false;
            }
        }

        // in zip_gens there needs to be a one-to-one mapping between the type parameters of the impl and the spec (check this!)
        let mut set_impl = BTreeSet::new();
        let mut set_spec = BTreeSet::new();
        for (impl_ty, spec_ty) in zip_gens {
            if !set_impl.insert(impl_ty) {
                return false;
            }
            if !set_spec.insert(spec_ty) {
                return false;
            }
        }
        true
    }
}

/// Given two `TypeTag` values, this function attempts to unify them.
pub fn unify(
    impl_params: &TypeTag,
    spec_params: &TypeTag,
    zip_gens: &mut BTreeSet<(TypeParamName, TypeParamName)>,
) -> bool {
    match impl_params {
        TypeTag::Boolean => {
            if !matches!(spec_params, TypeTag::Boolean) {
                // panic!("unification failed as the types are not compatible on Boolean");
                return false;
            }
            true
        }
        TypeTag::Integer => {
            if !matches!(spec_params, TypeTag::Integer) {
                // panic!("unification failed as the types are not compatible on Integer");
                return false;
            }
            true
        }
        TypeTag::Rational => {
            if !matches!(spec_params, TypeTag::Rational) {
                // panic!("unification failed as the types are not compatible on Rational");
                return false;
            }
            true
        }
        TypeTag::Text => {
            if !matches!(spec_params, TypeTag::Text) {
                // panic!("unification failed as the types are not compatible on Text");
                return false;
            }
            true
        }
        TypeTag::Cloak(inside) => {
            if let TypeTag::Cloak(inside_spec) = spec_params {
                unify(inside, inside_spec, zip_gens)
            } else {
                // panic!("unification failed as the types are not compatible on Cloak");
                false
            }
        }
        TypeTag::Seq(inside) => {
            if let TypeTag::Seq(inside_spec) = spec_params {
                unify(inside, inside_spec, zip_gens)
            } else {
                // panic!("unification failed as the types are not compatible on Seq");
                false
            }
        }
        TypeTag::Set(inside) => {
            if let TypeTag::Set(inside_spec) = spec_params {
                unify(inside, inside_spec, zip_gens)
            } else {
                // panic!("unification failed as the types are not compatible on Set");
                false
            }
        }
        TypeTag::Map(key, value) => {
            if let TypeTag::Map(key_spec, value_spec) = spec_params {
                unify(key, key_spec, zip_gens) && unify(value, value_spec, zip_gens)
            } else {
                // panic!("unification failed as the types are not compatible on Map");
                false
            }
        }
        TypeTag::Error => {
            if !matches!(spec_params, TypeTag::Error) {
                // panic!("unification failed as the types are not compatible on Error");
                return false;
            }
            true
        }
        TypeTag::User(name, args) => {
            if let TypeTag::User(name_spec, args_spec) = spec_params {
                if name != name_spec {
                    // panic!("unification failed as the types are not compatible on User");
                    return false;
                }
                if args.len() != args_spec.len() {
                    // panic!("unification failed as the number of arguments are not compatible on User");
                    return false;
                }
                for (arg, arg_spec) in args.iter().zip(args_spec.iter()) {
                    if !unify(arg, arg_spec, zip_gens) {
                        // panic!("unification failed as the types are not compatible on User");
                        return false;
                    }
                }
                true
            } else {
                // panic!("unification failed as the types are not compatible on User");
                false
            }
        }
        TypeTag::Pack(elems) => {
            if let TypeTag::Pack(elems_spec) = spec_params {
                if elems.len() != elems_spec.len() {
                    // panic!("unification failed as the number of elements are not compatible on Pack");
                    return false;
                }
                for (elem, elem_spec) in elems.iter().zip(elems_spec.iter()) {
                    if !unify(elem, elem_spec, zip_gens) {
                        // panic!("unification failed as the types are not compatible on Pack");
                        return false;
                    }
                }
                true
            } else {
                // panic!("unification failed as the types are not compatible on Pack");
                false
            }
        }
        TypeTag::Parameter(x) => {
            if let TypeTag::Parameter(y) = spec_params {
                zip_gens.insert((x.clone(), y.clone()));
                true
            } else {
                // panic!("unification failed as the types are not compatible on Parameter");
                false
            }
        }
    }
}

/// Function definition for implementation.
pub struct ImplFuncDef {
    /// The function signature.
    pub head: FuncSig,
    /// The function body as an expression.
    pub body: Expr,
}

/// Function definition for specification.
pub struct SpecFuncDef {
    /// The function signature.
    pub head: FuncSig,
    /// An optional function body as an expression. `None` is for uninterpreted functions.
    pub body: Option<Expr>,
}

/// Function definition for both implementation and specification.
pub struct FuncDef {
    /// The function signature.
    pub head: FuncSig,
    /// An optional function body as an expression.
    pub body: Option<Expr>,
}

// conversion from ImplFuncDef to FuncDef
impl From<ImplFuncDef> for FuncDef {
    /// Converts an `ImplFuncDef` into a `FuncDef`.
    fn from(def: ImplFuncDef) -> Self {
        let ImplFuncDef { head, body } = def;
        Self {
            head,
            body: Some(body),
        }
    }
}

// conversion from SpecFuncDef to FuncDef
impl From<SpecFuncDef> for FuncDef {
    /// Converts a `SpecFuncDef` into a `FuncDef`.
    fn from(def: SpecFuncDef) -> Self {
        let SpecFuncDef { head, body } = def;
        Self { head, body }
    }
}

/// Function definition for an axiom.
pub struct Axiom {
    /// The function signature.
    pub head: FuncSig,
    /// The body of the axiom as an expression.
    pub body: Expr,
}

impl Axiom {
    /// Checks whether the entire function body is `unimplemented!()`.
    pub fn is_unimplemented(stmts: &[Stmt]) -> Result<bool> {
        // If the function body contains more (or less) than one statement, it is not `unimplemented!()`
        if stmts.len() != 1 {
            return Ok(false);
        }

        // Check if the statement is an expression macro (e.g., `unimplemented!()`)
        let mac = match stmts.first().unwrap() {
            Stmt::Expr(Exp::Macro(ExprMacro { attrs: _, mac }), _) => mac, // extract the macro from the expression
            _ => return Ok(false), // if the statement is not an expression macro, it is for sure not `unimplemented!()`
        };

        let Macro {
            path,
            bang_token: _,
            delimiter,
            tokens,
        } = mac;

        // Checks if the path is an ident (only has one segment, no arguments, no leading colon) and whether it is `unimplemented`
        if !path.is_ident("unimplemented") {
            return Ok(false);
        }

        // sanity check: the delimiter must be parentheses & no tokens are allowed
        if !matches!(delimiter, MacroDelimiter::Paren(_)) {
            bail_on!(mac, "invalid delimiter");
        }
        bail_if_non_empty!(tokens);

        // If all checks pass, the function body is `unimplemented!()`
        Ok(true)
    }
}

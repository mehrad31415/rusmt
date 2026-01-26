//! Generics parsing and instantiation

use crate::parser::ctxt::MarkedType;
use crate::parser::expr::CtxtForExpr;
use crate::parser::infer::{TypeRef, TypeUnifier};
use crate::parser::name::{ReservedIdent, TypeParamName, UsrTypeName};
use crate::parser::ty::TypeTag;
use crate::{bail_if_exists, bail_if_missing, bail_on};
use itertools::Itertools;
use quote::quote_spanned;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use syn::{
    AngleBracketedGenericArguments, GenericArgument, GenericParam, Generics as GenericsDecl,
    ItemEnum, ItemStruct, PathArguments, Result, TraitBound, TraitBoundModifier, Type, TypeParam,
    TypeParamBound,
};

/// Reserved trait
#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SysTrait {
    SMT,
}

impl ReservedIdent for SysTrait {
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident {
            "SMT" => Self::SMT,
            _ => return None,
        };
        Some(matched)
    }
}

impl SysTrait {
    /// Ensure that a generics declaration satisfies the SMT trait
    ///
    /// # Errors
    ///
    /// 1. If there is an equality token or a default value
    /// 2. If there is no colon token
    /// 3. If there is a parenthesized trait bound, lifetimes, or a modifier
    /// 4. If the trait bound is not SMT
    /// 5. If there are more than one trait bounds
    fn validate_type_param_decl(param: &TypeParam) -> Result<TypeParamName> {
        let TypeParam {
            attrs: _,
            ident,
            colon_token,
            bounds,
            eq_token,
            default,
        } = param;

        // equality token and default value are not allowed
        bail_if_exists!(eq_token);
        bail_if_exists!(default);

        // colon token is required
        bail_if_missing!(colon_token, param, "trait bound");

        // create an iterator for the bounds
        let mut iter = bounds.iter();
        // must have at least one bound
        let bound = bail_if_missing!(iter.next(), bounds, "trait bound");
        // must have only one bound
        bail_if_exists!(iter.next());

        // ensure that the trait bound includes and only includes SMT
        match bound {
            TypeParamBound::Trait(trait_bound) => {
                let TraitBound {
                    paren_token,
                    modifier,
                    lifetimes,
                    path,
                } = trait_bound;

                // cannot have a parenthesized trait bound like Fn()
                // cannot have lifetimes like 'a
                // cannot have a modifier like ?Sized
                bail_if_exists!(paren_token.as_ref().map(|e| quote_spanned!(e.span=>)));
                bail_if_exists!(lifetimes);
                if !matches!(modifier, TraitBoundModifier::None) {
                    bail_on!(modifier, "unexpected");
                }

                // returns an error if the path is not a reserved identifier
                // in the case of SysTrait, the only reserved identifier is SMT
                let trait_name = SysTrait::parse_path(path)?;
                // the below check is not necessary since the only reserved identifier is SMT
                if !matches!(trait_name, SysTrait::SMT) {
                    bail_on!(path, "unexpected trait")
                }
            }
            // cannot be a lifetime or const bound
            _ => bail_on!(bound, "invalid bound"),
        }

        // all checks passed (if `ident` is a reserved keyword, then Err will be returned)
        ident.try_into()
    }
}

/// Declaration of generics
/// For a simple type, the vector of type parameters is empty
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Generics {
    pub params: Vec<TypeParamName>,
}

impl Generics {
    /// Create a new generics for intrinsics
    pub fn intrinsic(params: Vec<TypeParamName>) -> Self {
        // convert params to a set to check for duplicates
        let params_set: BTreeSet<_> = params.clone().into_iter().collect();
        // check for duplicates
        if params.len() != params_set.len() {
            panic!("duplicated type parameter name");
        }
        Self { params }
    }

    /// Convert from generics
    ///
    /// # Errors
    ///
    /// 1. If there is a where clause
    /// 2. If there are duplicate type parameters
    /// 3. If there are non-type parameters like lifetimes, consts, etc.
    /// 4. If the type parameters do not implement the SMT trait
    pub fn from_generics(generics: &GenericsDecl) -> Result<Self> {
        let GenericsDecl {
            lt_token,
            params,
            where_clause,
            gt_token,
        } = generics;

        // where clause is not allowed
        bail_if_exists!(where_clause);
        // list of type parameters
        let mut declared = vec![];

        if params.is_empty() {
            // if no generics are declared, then no <...> is needed
            bail_if_exists!(lt_token);
            bail_if_exists!(gt_token);
        } else {
            // if generics are declared, then <...> is needed
            bail_if_missing!(lt_token, generics, "<");
            bail_if_missing!(gt_token, generics, ">");

            for item in params {
                match item {
                    GenericParam::Type(ty_param) => {
                        // checks if the type parameters ONLY implement the SMT trait
                        let name = SysTrait::validate_type_param_decl(ty_param)?;
                        // duplicate type parameters are not allowed
                        if declared.contains(&name) {
                            bail_on!(ty_param, "duplicate type parameter {}", name);
                        }
                        declared.push(name);
                    }
                    _ => bail_on!(item, "type parameters only"), // lifetimes, consts, etc. not allowed
                }
            }
        };

        Ok(Self { params: declared })
    }

    /// Convert from a marked type
    /// This is used to extract the generics in the conversion between Context to ContextWithGenerics in the module ctxt
    pub fn from_marked_type(item: &MarkedType) -> Result<Self> {
        let generics = match item {
            MarkedType::Enum(ItemEnum {
                attrs: _,
                vis: _,
                enum_token: _,
                ident: _,
                generics,
                brace_token: _,
                variants: _,
            }) => generics,
            MarkedType::Struct(ItemStruct {
                attrs: _,
                vis: _,
                struct_token: _,
                ident: _,
                generics,
                fields: _,
                semi_token: _,
            }) => generics,
        };
        Self::from_generics(generics)
    }
}

impl Display for Generics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.params.is_empty() {
            f.write_str("")
        } else {
            write!(f, "<{}>", self.params.iter().format(","))
        }
    }
}

/// Partial instantiation of generics
/// In BTreeMap<TypeParamName, (usize, Option<TypeTag>)>, the key is the type parameter name, which is the generics used in the type definition. The value is a tuple of the index of the type parameter in the generics and an optional type tag. The type tag is None if the type parameter is not assigned in the argument call, otherwise it is Some(TypeTag). For example:
/// ```
/// enum Option<T> {
///   Some(T),
///   None,
/// }
/// let x = Option::<i32>::Some(3);
/// ```
/// In the above example, the type parameter T is assigned to i32, so the partial instantiation is {T: (0, Some(Integer))}.
/// If the type parameter is not assigned, then the partial instantiation is {T: (0, None)}: let x = Option::Some(3);
#[derive(Debug)]
pub struct GenericsInstPartial {
    args: BTreeMap<TypeParamName, (usize, Option<TypeTag>)>,
}

impl GenericsInstPartial {
    /// Create an instantiation by setting every type parameter to None
    pub fn new_without_args(generics: &Generics) -> Self {
        let ty_args = generics
            .params
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), (i, None)))
            .collect();
        Self { args: ty_args }
    }

    /// Create an instantiation with generics and type argument (optionally parsed)
    pub fn try_with_args(generics: &Generics, args: &[TypeTag]) -> Option<Self> {
        if generics.params.len() != args.len() {
            return None;
        }
        let ty_args = generics
            .params
            .iter()
            .zip(args)
            .enumerate()
            .map(|(i, (n, t))| (n.clone(), (i, Some(t.clone()))))
            .collect();
        Some(Self { args: ty_args })
    }

    /// A utility function to parse type arguments and match them with type parameters (generics) to create a partial instantiation
    /// As ExprParserRoot is the only type that implements the CtxtForExpr trait, ctxt is a reference to an ExprParserRoot
    pub fn from_args<T: CtxtForExpr>(
        ctxt: &T,
        generics: &Generics,
        arguments: &PathArguments,
    ) -> Result<Self> {
        let ty_params = &generics.params;
        let ty_args: BTreeMap<TypeParamName, (usize, Option<TypeTag>)> = match arguments {
            // if there are no arguments, then all type parameters are set to None
            PathArguments::None => ty_params
                .iter()
                .enumerate()
                .map(|(i, n)| (n.clone(), (i, None)))
                .collect(),
            // if there are angle-bracketed arguments, then the type parameters are set to the parsed type arguments
            PathArguments::AngleBracketed(pack) => {
                // probe for arguments
                let AngleBracketedGenericArguments {
                    colon2_token,
                    lt_token: _,
                    args,
                    gt_token: _,
                } = pack;
                // The :: is required for example let success = Result::<i32, &str>::Ok(3); Note that <i32, &str> is not a segment, but a type argument list.
                bail_if_missing!(colon2_token, pack, "::"); // turbofish syntax is required

                // check if the number of type arguments matches the number of type parameters, otherwise return an error
                if args.len() != ty_params.len() {
                    bail_on!(pack, "type argument number mismatch");
                }

                let mut ty_args = vec![];
                for arg in args {
                    match arg {
                        // only type arguments are allowed
                        GenericArgument::Type(ty) => {
                            let elem = match ty {
                                // when using _ explicitly used in a type position in Rust, it corresponds to a type inference placeholder, represented as Type::Infer in the Rust Abstract Syntax Tree (AST).
                                // in this case, the type argument is set to None
                                Type::Infer(_) => None,
                                _ => Some(TypeTag::from_type(ctxt, ty)?), // otherwise, the type argument is parsed (which only allows Type::Path and Type::Tuple)
                            };
                            ty_args.push(elem); // add the parsed type argument to the list
                        }
                        _ => bail_on!(arg, "invalid type argument"), // only type arguments are allowed for example lifetimes, consts, etc. are not allowed
                    }
                }

                // construct partial instantiation
                // first element is the type parameter name in the definition
                // second element is a tuple of the index of the type parameter in the generics and the parsed type argument
                ty_params
                    .iter()
                    .zip(ty_args)
                    .enumerate()
                    .map(|(i, (n, t))| (n.clone(), (i, t)))
                    .collect()
            }
            // parenthesized arguments are not allowed
            PathArguments::Parenthesized(_) => bail_on!(arguments, "invalid arguments"),
        };
        Ok(Self { args: ty_args })
    }

    /// Complete the instantiation with the help of a type unifier
    pub fn complete(self, unifier: &mut TypeUnifier) -> GenericsInstFull {
        let args = self
            .args
            .into_iter()
            .map(|(k, (i, inst))| {
                let completed = match inst {
                    None => TypeRef::Var(unifier.mk_var()), // if the type argument is None, then a fresh type variable is created
                    Some(tag) => (&tag).into(), // otherwise, the type argument is converted to a TypeRef
                };
                (k, (i, completed))
            })
            .collect();
        GenericsInstFull { args }
    }
}

/// Complete instantiation of generics
pub struct GenericsInstFull {
    args: BTreeMap<TypeParamName, (usize, TypeRef)>,
}

impl GenericsInstFull {
    /// Shape the arguments in its declaration order, filling missed ones with fresh type variables
    pub fn vec(&self) -> Vec<TypeRef> {
        let rev: BTreeMap<_, _> = self
            .args
            .iter()
            .map(|(_, (i, t))| (*i, t.clone()))
            .collect();
        rev.into_values().collect()
    }

    /// Make a type ref combined with type name
    pub fn make_ty(&self, name: UsrTypeName) -> TypeRef {
        TypeRef::User(name, self.vec())
    }

    /// Extract type arguments in the order specified by a list of parameter names
    /// Returns None if any parameter name is missing from the instantiation
    pub fn vec_for_generics(&self, generics: &Generics) -> Option<Vec<TypeRef>> {
        generics
            .params
            .iter()
            .map(|param_name| self.args.get(param_name).map(|(_, ty)| ty.clone()))
            .collect()
    }

    /// Merge two generics into one
    pub fn merge(&self, other: &Self) -> Option<Self> {
        let args: BTreeMap<_, _> = self
            .args
            .iter()
            .chain(&other.args)
            .map(|(name, (idx, ty))| (name.clone(), (*idx + self.args.len(), ty.clone())))
            .collect();

        // should not have conflicting type parameter names
        if args.len() == self.args.len() + other.args.len() {
            Some(Self { args })
        } else {
            None
        }
    }

    /// Instantiate a type tag by applying type parameter substitution
    /// This is used in the conversion from TypeTag to TypeRef
    /// In the case of a type parameter, the corresponding TypeRef is found in the args field of the GenericsInstFull struct and returned
    /// In other cases, the TypeTag is simply converted to a TypeRef
    /// It only returns None if the type parameter is not found in the args field
    pub fn instantiate(&self, tag: &TypeTag) -> Option<TypeRef> {
        let updated = match tag {
            TypeTag::Boolean => TypeRef::Boolean,
            TypeTag::Integer => TypeRef::Integer,
            TypeTag::Real => TypeRef::Real,
            TypeTag::String => TypeRef::String,
            TypeTag::F32 => TypeRef::F32,
            TypeTag::F64 => TypeRef::F64,
            TypeTag::U32 => TypeRef::U32,
            TypeTag::U64 => TypeRef::U64,
            TypeTag::I32 => TypeRef::I32,
            TypeTag::I64 => TypeRef::I64,
            TypeTag::Error => TypeRef::Error,
            // the into method is for converting TypeRef to Box<TypeRef>
            TypeTag::Cloak(sub) => TypeRef::Cloak(self.instantiate(sub)?.into()),
            TypeTag::Seq(sub) => TypeRef::Seq(self.instantiate(sub)?.into()),
            TypeTag::Set(sub) => TypeRef::Set(self.instantiate(sub)?.into()),
            TypeTag::Array(key, val) => {
                TypeRef::Array(self.instantiate(key)?.into(), self.instantiate(val)?.into())
            }
            TypeTag::User(name, args) => TypeRef::User(
                name.clone(),
                args.iter()
                    .map(|t| self.instantiate(t))
                    .collect::<Option<Vec<TypeRef>>>()?,
            ),
            TypeTag::Pack(elems) => TypeRef::Pack(
                elems
                    .iter()
                    .map(|t| self.instantiate(t))
                    .collect::<Option<Vec<TypeRef>>>()?,
            ),
            // the type parameter is substituted with the corresponding type ref
            TypeTag::Parameter(name) => self.args.get(name).map(|(_, t)| t)?.clone(),
        };
        Some(updated)
    }
}

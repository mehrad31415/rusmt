use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter}; // used to implement the Display trait

use syn::{
    Expr as Exp,    // Renaming `Expr` to `Exp` to avoid conflict with our own `Expr`
    ExprMacro,      // Represents a macro invocation `expression`
    FnArg, // Represents a function argument in a function signature (either `Receiver` or `Typed`)
    Ident, // Represents an identifier (e.g., variable or function name)
    Macro, // Represents a macro invocation
    MacroDelimiter, // Represents the delimiter used in a macro (e.g., parentheses)
    PatType, // Represents a pattern with a type annotation
    Result, // Alias for `Result<T, syn::Error>`
    ReturnType, // Represents the return type, which is either `Default` or `Type`(Token![->], Box<Type>). Functions default to `()` and closures default to type inference.
    Signature,  // Represents a function signature
    Stmt,       // Represents a statement in Rust code
};

use crate::parser::ctxt::ContextWithType; // Context manager after type analysis is done
use crate::parser::expr::Expr; // Our own expression type
use crate::parser::generics::Generics; // Declaration of generics
use crate::parser::name::{ReservedIdent, TypeParamName, UsrFuncName, UsrTypeName, VarName}; // Name handling
use crate::parser::ty::{CtxtForType, TypeTag};
use crate::{bail_if_exists, bail_if_non_empty, bail_on}; // Error handling macros // CtxtForType: A context suitable for type analysis and TypeTag: A unique and complete reference to an SMT-related type

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
    ///
    /// This method checks if the identifier matches any of the reserved, system, or casting function names.
    /// If it does, it returns the corresponding `FuncName` variant.
    /// Otherwise, it tries to convert it into a user-defined function name (`UsrFuncName`).
    ///
    /// # Arguments
    ///
    /// * `ident` - The identifier to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `FuncName` if successful, or a `syn::Error` if the identifier is a reserved keyword.
    pub fn try_from(ident: &Ident) -> Result<Self> {
        let name = ident.to_string();
        let parsed = match ReservedFuncName::from_str(&name) { // clone and default are reserved keywords
            Some(n) => Self::Reserved(n),
            None => match CastFuncName::from_str(&name) { // from and into are reserved keywords
                Some(n) => Self::Cast(n),
                None => match SysFuncName::from_str(&name) { // eq and ne are reserved keywords
                    Some(n) => Self::Sys(n),
                    None => Self::Usr(ident.try_into()?), // returns an error if ident is a reserved keyword or an underscore. Invokes method `fn try_from(value: &Ident) -> Result<Self> { validate_user_ident(value).map(|ident| Self { ident }) } from the name module`. Returns UsrFuncName if the ident is a valid user-defined function name.
                },
            },
        };
        Ok(parsed)
    }
}

impl Display for FuncName {
    /// Formats the `FuncName` as a string.
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
///
/// This struct provides context information needed when parsing function signatures,
/// such as generics and type information.
struct FuncSigParseCtxt<'a> {
    ctxt: &'a ContextWithType, // this field is used to check if a user defined type is present in the ctxt when calling get_type_generics which will give None if the type is not present
    generics: &'a Generics, // this field is to check if a type is a type parameter (and if it is not, possibly a user defined type)
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
///
/// This struct contains information about a function's signature, including its
/// generics, parameters, and return type.
pub struct FuncSig {
    /// The generics associated with the function.
    pub generics: Generics,
    /// The parameters of the function, each with a name and type.
    pub params: Vec<(VarName, TypeTag)>, // the reason Vec was used not BtreeMap is because the original order of the parameters is important
    /// The return type of the function.
    pub ret_ty: TypeTag,
}

impl FuncSig {
    /// Constructs a `FuncSig` from a `syn::Signature`.
    /// Used to create ADT from the signature of a function.
    ///
    /// This method parses a function signature from the syntax tree and extracts
    /// the generics, parameters, and return type.
    ///
    /// # Arguments
    ///
    /// * `driver` - The context with type information.
    /// * `sig` - The function signature to parse.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `FuncSig` if successful, or a `syn::Error` if parsing fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The function is `const`, `async`, or `unsafe`, or has an ABI (these are not supported).
    /// - The function has a variadic parameter list (e.g., `fn foo(...){}`).
    /// - The function has a `self` parameter.
    /// - The function has duplicated parameter names.
    /// - The function lacks a return type.
    /// - The return type can only be a tuple or a path otherwise an error is thrown.
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

    /// Tests whether two function signatures are type-compatible.
    ///
    /// This method checks if the generics, parameter types, and return types are the same.
    ///
    /// # Arguments
    ///
    /// * `sig` - The other `FuncSig` to compare with.
    ///
    /// # Returns
    ///
    /// `true` if the signatures are compatible, `false` otherwise.
    ///
    /// This function is used in ctxt.rs to check that the implementation and the corresponding specification have compatible function signatures.
    pub fn is_compatible(&self, sig: &FuncSig) -> bool {
        // if the length of the type parameter lists are different, they are not compatible
        if self.generics.params.len() != sig.generics.params.len() {
            return false;
        }

        let mut self_generics = self.generics.params.clone();
        let mut sig_generics = sig.generics.params.clone();
        self_generics.sort(); //? is the sort a correct decision?
        sig_generics.sort(); //? is the sort a correct decision?

        let gens: BTreeMap<TypeParamName, TypeParamName> =
            self_generics.into_iter().zip(sig_generics).collect();

        // Compare parameter types
        let lhs_params: Vec<TypeTag> = self.params.clone().into_iter().map(|(_, t)| t).collect();
        let lhs_params = unify(lhs_params, &gens).expect("unification failed in params");
        let rhs_params: Vec<TypeTag> =
            sig.params.clone().into_iter().map(|(_, t)| t).collect();
        if lhs_params != rhs_params {
            return false;
        }

        // Compare return types
        let ret_ty = vec![self.ret_ty.clone()];
        let ret_ty = unify(ret_ty, &gens)
            .expect("unification failed in return type")
            .first()
            .expect("function must have a return type")
            .clone();
        ret_ty == sig.ret_ty
    }
}

/// This function traverses the given `params` (a vector of `TypeTag` values) and tries to unify type parameters based on the provided `gens` map, which is a `BTreeMap` mapping from `TypeParamName` to `TypeParamName`. If a type parameter in the `params` is found in `gens`, it replaces it with the corresponding type from `gens`.
///
/// # Arguments
///
/// * `params` - A vector of `TypeTag` values that need to be unified.
/// * `gens` - A reference to a `BTreeMap` that maps `TypeParamName` to `TypeParamName`.
///
/// # Returns
///
/// Returns a `Result` containing a `BTreeSet<TypeTag>` of unified types, or an error if the unification process encounters an unbound type parameter.
///
/// # Errors
///
/// Returns an error using the `bail_on!` macro if a `TypeTag::Parameter` is found in `params` but is not defined in the `gens` map (an unbound type parameter).
///
/// # Recursive Unification
///
/// The function handles nested types (like `Cloak`, `Seq`, `Set`, `Map`, `User`, and `Pack`) by calling `unify` recursively for the inner type(s).
/// It ensures that all nested type parameters are also unified.
pub fn unify(
    params: Vec<TypeTag>,
    gens: &BTreeMap<TypeParamName, TypeParamName>,
) -> Result<Vec<TypeTag>> {
    let mut res: Vec<TypeTag> = vec![];
    // Iterate over each type in the `params` vector
    for i in params {
        match i {
            // Handle type parameters
            TypeTag::Parameter(p) => {
                if let Some(t) = gens.get(&p.clone()) {
                    // If the generic parameter exists in `gens`, insert the unified type
                    res.push(TypeTag::Parameter(t.clone()));
                } else {
                    // If the generic parameter is not found in `gens`, return an error
                    bail_on!(p.ident, "unbound type parameter");
                }
            }
            // Handle the `Cloak` type tag (recursively unify the inner type)
            TypeTag::Cloak(t) => {
                res.push(TypeTag::Cloak(Box::new(
                    unify(vec![*t.clone()], gens)
                        .expect("unification failed in cloak")
                        .into_iter()
                        .next()
                        .unwrap(),
                )));
            }
            // Handle the `Seq` type tag (recursively unify the inner type)
            TypeTag::Seq(t) => {
                res.push(TypeTag::Seq(Box::new(
                    unify(vec![*t.clone()], gens)
                        .expect("unification failed in seq")
                        .into_iter()
                        .next()
                        .unwrap(),
                )));
            }
            // Handle the `Set` type tag (recursively unify the inner type)
            TypeTag::Set(t) => {
                res.push(TypeTag::Set(Box::new(
                    unify(vec![*t.clone()], gens)
                        .expect("unification failed in set")
                        .into_iter()
                        .next()
                        .unwrap(),
                )));
            }
            // Handle the `Map` type tag (recursively unify both key and value types)
            TypeTag::Map(k, v) => {
                res.push(TypeTag::Map(
                    Box::new(
                        unify(vec![*k.clone()], gens)
                            .expect("unification failed in map")
                            .into_iter()
                            .next()
                            .unwrap(),
                    ),
                    Box::new(
                        unify(vec![*v.clone()], gens)
                            .expect("unification failed in map")
                            .into_iter()
                            .next()
                            .unwrap(),
                    ),
                ));
            }
            // Handle the `User` type tag (recursively unify each argument)
            TypeTag::User(name, args) => {
                res.push(TypeTag::User(
                    name,
                    unify(args, gens)?.into_iter().collect(),
                ));
            }
            // Handle the `Pack` type tag (recursively unify each element in the pack)
            TypeTag::Pack(t) => {
                // println!("{:?}", t);
                // println!("{:?}", unify(t.clone(), gens));
                res.push(TypeTag::Pack(unify(t, gens)?.into_iter().collect()));
                // println!("{:?}", res);
            }
            // Handle other type tags (Boolean, Integer, Rational, Text, Error)
            _ => {
                res.push(i);
            }
        }
    }

    // done
    Ok(res)
}

/// Function definition for implementation.
///
/// This struct represents a function definition in an implementation block,
/// containing the function signature and the function body as an expression.
pub struct ImplFuncDef {
    /// The function signature.
    pub head: FuncSig,
    /// The function body as an expression.
    pub body: Expr,
}

/// Function definition for specification.
///
/// This struct represents a function definition in a specification block,
/// containing the function signature and an optional function body.
pub struct SpecFuncDef {
    /// The function signature.
    pub head: FuncSig,
    /// An optional function body as an expression. `None` is for uninterpreted functions.
    pub body: Option<Expr>,
}

/// Function definition for both implementation and specification.
///
/// This struct represents a function definition that can be used in both implementation
/// and specification contexts.
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
///
/// This struct represents an axiom, which is a function with a body.
/// The body of an axiom is an expression that represents the axiom's assertion.
pub struct Axiom {
    /// The function signature.
    pub head: FuncSig,
    /// The body of the axiom as an expression.
    pub body: Expr,
}

impl Axiom {
    /// Checks whether the entire function body is `unimplemented!()`.
    ///
    /// This method is used to determine if the function body consists solely of a call
    /// to `unimplemented!()`, indicating that the function has no implementation. This is only used to check for #[smt_spec] marked functions.
    ///
    /// # Arguments
    ///
    /// * `stmts` - A slice of statements representing the function body.
    ///
    /// # Returns
    ///
    /// A `Result` containing `true` if the function body is `unimplemented!()`, or `false` otherwise.
    ///
    /// # Errors
    ///
    /// This method returns an error if the macro invocation unimplemented!() is not used correctly (with arguments or different delimiters).
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

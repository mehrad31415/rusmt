//! Module for parsing quantifiers in the macro system.
//! `exists`, `forall`, and `choose`.

use crate::bail_on;
use crate::parser::expr::CtxtForExpr;
use crate::parser::name::{ReservedIdent, VarName};
use syn::{
    Expr, ExprMacro, Ident, Macro, MacroDelimiter, Result, Token,
    parse::{Parse, ParseStream},
    parse2,
    punctuated::Punctuated,
};

/// Represents reserved macro names for expressions in stdlib.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SysMacroName {
    /// The `exists` quantifier.
    Exists,
    /// The `forall` quantifier.
    Forall,
    /// The `choose` quantifier.
    Choose,
}

// the macro names are reserved identifiers
impl ReservedIdent for SysMacroName {
    /// Attempts to parse a string into a `SysMacroName`.
    ///
    /// Returns `Some(SysMacroName)` if the string matches one of the reserved macro names,
    /// otherwise returns `None`.
    /// This function is used in the `validate_user_ident` method of `name` module.
    /// The `validate_user_ident` method is used to check if the user-defined identifier is a reserved keyword. If it is, the method returns an error.
    fn from_str(ident: &str) -> Option<Self> {
        let matched = match ident {
            "exists" => Self::Exists,
            "forall" => Self::Forall,
            "choose" => Self::Choose,
            _ => return None,
        };
        Some(matched)
    }
}

/// AST node representing a variable declaration in an iterative quantifier.
///
/// For example, in `x in xs`, `ident` would be `x`, and `collection` would be `xs`.
/// This is only used to define the variable declarations in the IterQuant struct.
struct IterVar {
    /// The identifier of the variable.
    ident: Ident,
    /// The `in` token.
    _in_token: Token![in],
    /// The collection expression the variable iterates over.
    collection: Expr,
}

impl Parse for IterVar {
    /// Parses an `IterVar` from the given parse stream.
    ///
    /// Expects the syntax `ident in collection`.
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _in_token: input.parse()?,
            collection: input.parse()?,
        })
    }
}

/// AST node representing an iterative quantifier.
///
/// For example, `x in xs, y in ys => body`.
/// This is only used to define the AST of the iterated quantifier in the Quantifier enum.
struct IterQuant {
    /// The list of variable declarations delimited by commas.
    vars: Punctuated<IterVar, Token![,]>,
    /// The `=>` token separating the variables and the body.
    _imply_token: Token![=>],
    /// The body expression of the quantifier.
    body: Expr,
}

impl Parse for IterQuant {
    /// Parses an `IterQuant` from the given parse stream.
    ///
    /// Expects the syntax `vars => body`, where `vars` is a non-empty
    /// list of `IterVar` separated by commas.
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            vars: Punctuated::parse_separated_nonempty(input)?, // input.parse()? is used to parse a single token from the input stream not multiple tokens.
            _imply_token: input.parse()?,
            body: input.parse()?,
        })
    }
}

/// Represents an iterated quantifier: `var in collection => predicate`.
///
/// Only the iteration form is supported in function bodies.
pub struct Quantifier {
    /// The name of the quantifier (`exists`, `forall`, `choose`).
    pub name: SysMacroName,
    /// Variable–collection pairs, e.g. `(x, xs)` from `x in xs`.
    pub vars: Vec<(VarName, Expr)>,
    /// The predicate body.
    pub body: Expr,
}

impl Quantifier {
    /// Parses a `Quantifier` from an `ExprMacro`.
    /// This function is used in parsing an expression macro in the `convert_expr` function of the `expr` module.
    pub fn parse<T: CtxtForExpr>(_ctxt: &T, expr: &ExprMacro) -> Result<Self> {
        // Destructure the macro expression to extract the macro path, delimiter, and tokens
        let ExprMacro {
            attrs: _,
            mac:
                Macro {
                    path,
                    bang_token: _,
                    delimiter,
                    tokens,
                },
        } = expr;

        // Ensure that the macro is invoked with parentheses
        if !matches!(delimiter, MacroDelimiter::Paren(..)) {
            bail_on!(expr, "expect macro invocation with parenthesis");
        }

        // Parse the macro name from the path
        // This will return an error if the path has leading colons, or the path does not have one and only one segment, or the path has arguments. It will also return an error if the path is not `exists`, `forall`, or `choose`.
        let name = SysMacroName::parse_path(path)?;

        match tokens.clone().into_iter().next() {
            None => bail_on!(expr, "must have at least one token in the quantifier"),
            Some(_) => {
                let syntax = parse2::<IterQuant>(tokens.clone())?;
                let IterQuant {
                    vars,
                    _imply_token: _,
                    body,
                } = syntax;

                let mut var_decls = vec![];
                for var in vars {
                    let IterVar {
                        ident,
                        _in_token: _,
                        collection,
                    } = var;

                    let name: VarName = (&ident).try_into()?;

                    if var_decls.iter().any(|(n, _)| n == &name) {
                        bail_on!(&ident, "conflicting quantifier variable name");
                    }
                    var_decls.push((name, collection));
                }

                Ok(Self {
                    name,
                    vars: var_decls,
                    body,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::parse_quote;

    // simple test to check if the from_str method works
    #[test]
    fn test_sys_macro_name_from_str_exists() {
        let par = SysMacroName::from_str("exists");
        assert!(par.is_some_and(|v| v == SysMacroName::Exists));
    }

    #[test]
    fn test_sys_macro_name_from_str_forall() {
        let par = SysMacroName::from_str("forall");
        assert!(par.is_some_and(|v| v == SysMacroName::Forall));
    }

    #[test]
    fn test_sys_macro_name_from_str_choose() {
        let par = SysMacroName::from_str("choose");
        assert!(par.is_some_and(|v| v == SysMacroName::Choose));
    }

    #[test]
    fn test_sys_macro_name_from_str_invalid() {
        let par = SysMacroName::from_str("invalid");
        assert!(par.is_none());
    }

    // simple test to check if the parse method works for IterQuant and IterVar
    #[test]
    fn test_parse_iter() {
        let par: IterQuant = parse_quote!(x in x_collection, y in y_collection => x > y);

        let IterQuant {
            vars,
            _imply_token: _,
            body,
        } = par;

        // now destructure vars to get IterVar
        let IterVar {
            ident: x_ident,
            _in_token: _,
            collection: x_collection,
        } = &vars[0];

        let IterVar {
            ident: y_ident,
            _in_token: _,
            collection: y_collection,
        } = &vars[1];

        assert_eq!(x_ident.to_string(), "x");
        assert_eq!(x_collection.to_token_stream().to_string(), "x_collection");

        assert_eq!(y_ident.to_string(), "y");
        assert_eq!(y_collection.to_token_stream().to_string(), "y_collection");

        // check the body
        assert_eq!(body.to_token_stream().to_string(), "x > y");
    }
}

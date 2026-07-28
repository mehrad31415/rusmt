//! The `TypeParamGroup` struct is used to collect and manage type parameters in generic definitions.

use crate::{bail_on, ensure_none, ensure_some};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::BTreeSet;
use syn::{
    GenericParam, Generics, Path, PathArguments, PathSegment, Result, TraitBound, TypeParam,
    TypeParamBound,
};

/// Represents a group of type parameters in generic definitions.
#[derive(Debug)]
pub struct TypeParamGroup {
    /// A vector of identifiers for the type parameters.
    params: Vec<Ident>,
}

impl TypeParamGroup {
    /// Parses the generics from a `syn::Generics` and returns a `TypeParamGroup`.
    pub fn parse_generics(generics: &Generics) -> Result<Self> {
        // Destructure the Generics for easier access to its components
        let Generics {
            lt_token,
            params,
            gt_token,
            where_clause,
        } = generics;

        // Sanity check: Ensure that the angle brackets are used correctly
        if params.is_empty() {
            // If there are no parameters, there should be no angle brackets
            ensure_none!(lt_token, "expecting no angle brackets"); // unreachable
            ensure_none!(gt_token, "expecting no angle brackets"); // unreachable
        } else {
            // If there are parameters, angle brackets must exist
            ensure_some!(lt_token, generics, "<"); // unreachable
            ensure_some!(gt_token, generics, ">"); // unreachable code as the rust compiler catches it beforehand. you cannot have fn temp T:SMT (x:T) {....}
        }

        // The where clause is not expected in this context
        ensure_none!(where_clause, "unexpected");

        // Collect type parameters
        let mut ty_params_set = BTreeSet::new(); // To ensure uniqueness
        let mut ty_params_vec = vec![]; // To maintain order

        for param in params {
            match param {
                // We expect Generic type parameters only
                GenericParam::Type(TypeParam {
                    attrs: _,
                    ident,
                    colon_token,
                    bounds,
                    default,
                }) => {
                    // Colon token must be present (e.g., `T: Trait`) so basically the bound is expected
                    ensure_some!(colon_token, param, ":");

                    // Equal token and default should not be present
                    if let Some((_eq, def)) = default {
                        bail_on!(def, "no default value expected")
                    }

                    // The rest check that the `SMT` trait is enforced as a bound
                    let mut iter = bounds.iter();
                    let bound = ensure_some!(iter.next(), param, "trait");

                    //  only one bound is expected and it should be the SMT trait
                    ensure_none!(iter.next(), "no extra bounds expected");
                    match bound {
                        TypeParamBound::Trait(TraitBound {
                            paren_token,
                            // `TraitBoundModifiers` is `#[non_exhaustive]` and currently empty.
                            modifiers,
                            lifetimes,
                            maybe,
                            path:
                                Path {
                                    leading_colon,
                                    segments,
                                },
                        }) => {
                            if paren_token.is_some() {
                                // Parentheses are not expected in the trait bound for example T: (SMT)
                                bail_on!(bound, "invalid bound");
                            }
                            modifiers.require_empty()?; // No modifiers are expected
                            ensure_none!(lifetimes, "no lifetimes expected"); // Lifetimes are not expected for example for<'a> Foo<&'a T> Higher ranked trait bounds are not expected
                            ensure_none!(maybe, "unexpected maybe"); // Maybe is not expected for example T: ?SMT

                            // HRTB are the same as lifetimes but they mean that the trait is generic over all lifetimes 'a but if we wrote Foo<&'a T> it would mean that the trait is generic over a specific lifetime 'a
                            ensure_none!(leading_colon, "no leading colon expected"); // Leading colon is not expected for example T: ::std

                            // Check the path segments
                            let mut iter = segments.iter();
                            let segment = ensure_some!(iter.next(), bound, "trait name"); // never panics!
                            ensure_none!(iter.next(), "no extra segments expected"); // Only one segment is expected and the trait name is expected to be SMT

                            let PathSegment { ident, arguments } = segment;
                            if !matches!(arguments, PathArguments::None) {
                                // Type arguments are not expected
                                bail_on!(arguments, "unexpected");
                            }
                            if ident.to_string().as_str() != "SMT" {
                                // The trait should be `SMT`
                                bail_on!(ident, "expect SMT trait");
                            }
                        }
                        // if the bound is not a trait bound (is lifetime or verbatim) return an error
                        _ => bail_on!(bound, "expect trait bound"),
                    }

                    // Save the type parameter name after duplication check
                    if !ty_params_set.insert(ident.clone()) {
                        bail_on!(ident, "duplicated declaration");
                    }
                    ty_params_vec.push(ident.clone());
                }
                // cannot be a constant or a lifetime
                _ => bail_on!(param, "expect generic type parameter"),
            }
        }

        // Return the type parameter group
        Ok(Self {
            params: ty_params_vec,
        })
    }

    /// Converts the type parameters into a syntax suitable for definition (e.g., `<T: SMT>`).
    /// The result of this is used after `impl`, `struct` in defnition, `enum` in definition, etc. keywords to define type parameters and trait bounds.
    pub fn to_syntax_def(&self) -> TokenStream {
        if self.params.is_empty() {
            TokenStream::new()
        } else {
            let content = self.params.iter().map(|n| quote!(#n: SMT));
            quote!(<#(#content),*>) //interpolates the content into the token stream: <T: SMT, U: SMT>
        }
    }

    /// Converts the type parameters into a syntax suitable for use (e.g., `<T>`).
    /// The result of this is used when referring to the type parameters in the code for example after the name of the struct or enum in impl block.
    pub fn to_syntax_use(&self) -> TokenStream {
        if self.params.is_empty() {
            TokenStream::new()
        } else {
            let content = self.params.iter().map(|n| quote!(#n));
            quote!(<#(#content),*>) //interpolates the content into the token stream: <T, U>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{Generics, parse_quote};

    #[test]
    fn test_parse_generics_params_empty_bail() {
        // this invokes ensure_none!(lt_token); inside is param empty.
        let generics: Generics = parse_quote! {<>};
        let type_param_group: Result<TypeParamGroup> = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "expecting no angle brackets"
        );
    }

    #[test]
    fn test_parse_generics_params_empty_ok() {
        // this one creates TypeParamGroup {params:vec![]}
        let generics: Generics = parse_quote! {};
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_ok_and(|x| { x.params.is_empty() }));
    }

    #[test]
    fn test_parse_generics_bail_where() {
        // this invokes ensure_none!(where_clause);
        let item_fn: syn::ItemFn = parse_quote! {
            fn foo<T: Default>(x: T) -> T
            where T: std::fmt::Debug
            {
                x
            }
        };
        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
    }

    #[test]
    // ensure_some!(colon_token, param, ":"); is invoked
    fn test_parse_generics_colon_missing() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T>(x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "expect :");
    }

    #[test]
    // ensure_none!(eq_token); is invoked
    fn test_parse_generics_eq_token() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T : SMT = String >(x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "no default value expected"
        );
    }

    #[test]
    // let bound = ensure_some!(iter.next(), param, "trait"); will be invoked
    fn test_parse_generics_no_trait() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T : >(x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "expect trait");
    }

    #[test]
    // ensure_none!(iter.next()); // No extra bounds expected is invoked
    //* so only one bound is expected and it should be the SMT trait
    fn test_parse_generics_one_trait() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: SMT + a> (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "no extra bounds expected"
        );
    }

    #[test]
    // bail_on!(bound, "invalid bound");
    fn test_parse_generics_invalid_bound() {
        let item_fn: syn::ItemTrait = parse_quote! {
            trait Example<T : (?Sized)> {
                type T;
            }
        };

        let generics: Generics = item_fn.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "invalid bound");
    }

    #[test]
    // bail_on!(modifier, "invalid modifier"); is invoked
    fn test_parse_generics_invalid_modifier() {
        let item_fn: syn::ItemTrait = parse_quote! {
            trait Example<T : ?Sized> {
                type T;
            }
        };

        let generics: Generics = item_fn.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "unexpected maybe"
        );
    }

    // ensure_none!(lifetimes); // Lifetimes are not expected
    // for<'a> Foo<&'a T>
    #[test]
    fn test_parse_generics_lifetimes() {
        let generics: Generics = parse_quote! {
            <T: for<'a> Foo<&'a T>>
        };

        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "no lifetimes expected"
        );
    }

    #[test]
    // ensure_none!(leading_colon); // Leading colon is not expected is invoked
    fn test_parse_generics_leading_colon() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: ::std>(x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "no leading colon expected"
        );
    }

    #[test]
    //ensure_none!(iter.next()); // Only one segment is expected and the trait name is expected to be SMT
    fn test_parse_generics_one_segment() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: std::SMT > (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "no extra segments expected"
        );
    }

    // if !matches!(arguments, PathArguments::None) {
    //     bail_on!(arguments, "unexpected");
    // }
    // the above is invoked
    #[test]
    fn test_parse_generics_path_args_not_none() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: SMT<String> > (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
    }

    // if ident.to_string().as_str() != "SMT" {
    //     bail_on!(ident, "expect SMT trait");
    // }
    // the above is invoked
    #[test]
    fn test_parse_generics_smt() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: SMTT > (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "expect SMT trait"
        );
    }

    //  _ => bail_on!(bound, "expect trait bound"), is invoked
    #[test]
    fn test_parse_generics_expect_trait_bound() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: 'a> (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "expect trait bound"
        );
    }

    // if !ty_params_set.insert(ident.clone()) {
    //     bail_on!(ident, "duplicated declaration");
    // }
    // the above is invoked
    #[test]
    fn parse_generics_duplicated_declaration() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: SMT, T: SMT> (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "duplicated declaration"
        );
    }

    //_ => bail_on!(param, "expect generic type parameter"), is invoked
    #[test]
    fn test_parse_generics_expect_generic_type_parameter() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<'a> (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(
            type_param_group.err().unwrap().to_string(),
            "expect generic type parameter"
        );
    }

    // no bail
    #[test]
    fn test_parse_generics() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn temp<T: SMT> (x:T) -> String {
                x.to_string()
            }
        };

        let generics: Generics = item_fn.sig.generics;
        let type_param_group = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_ok());
        assert_eq!(type_param_group.unwrap().params.len(), 1);
    }

    #[test]
    fn test_to_syntax_def_one() {
        let group = TypeParamGroup { params: vec![] };

        let token_stream = group.to_syntax_def();
        assert!(token_stream.is_empty());
    }

    #[test]
    fn test_to_syntax_def_two() {
        // Test converting to syntax definition
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let tokens = group.to_syntax_def();
        let expected: TokenStream = quote!(<T: SMT, U: SMT>);
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn test_to_syntax_use_one() {
        // Test converting to syntax usage
        let group = TypeParamGroup { params: vec![] };
        let tokens = group.to_syntax_use();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_to_syntax_use_two() {
        // Test converting to syntax usage
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let tokens = group.to_syntax_use();
        let expected: TokenStream = quote!(<T, U>);
        assert_eq!(tokens.to_string(), expected.to_string());
    }
}

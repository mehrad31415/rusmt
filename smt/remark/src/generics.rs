//! The `TypeParamGroup` struct is used to collect and manage type parameters in generic definitions.

use crate::{bail_on, ensure_none, ensure_some};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::BTreeSet;
use syn::{
    AngleBracketedGenericArguments, GenericArgument, GenericParam, Generics, Path, PathArguments,
    PathSegment, Result, TraitBound, TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath,
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
                    eq_token,
                    default,
                }) => {
                    // Colon token must be present (e.g., `T: Trait`) so basically the bound is expected
                    ensure_some!(colon_token, param, ":");

                    // Equal token and default should not be present
                    ensure_none!(eq_token, "no equal sign expected");
                    ensure_none!(default, "no default value expected"); // unreachable invocation because it cannot exist if the = doesnt exist according to rust compiler

                    // The rest check that the `SMT` trait is enforced as a bound
                    let mut iter = bounds.iter();
                    let bound = ensure_some!(iter.next(), param, "trait");

                    //  only one bound is expected and it should be the SMT trait
                    ensure_none!(iter.next(), "no extra bounds expected");
                    match bound {
                        TypeParamBound::Trait(TraitBound {
                            paren_token,
                            modifier,
                            lifetimes,
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
                            if !matches!(modifier, TraitBoundModifier::None) {
                                // Modifier should be none (e.g., no `?` or `for<>`)
                                bail_on!(modifier, "invalid modifier");
                            }
                            ensure_none!(lifetimes, "no lifetimes expected"); // Lifetimes are not expected for example for<'a> Foo<&'a T> Higher ranked trait bounds are not expected

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

    /// Checks if the group contains a specific type parameter.
    pub fn contains(&self, name: &Ident) -> bool {
        self.params.contains(name)
    }

    /// Collects type arguments from a given type based on the current type parameters.
    ///
    /// This function recursively inspects the given type and collects any type parameters
    /// that are used as type arguments. It helps in tracking which type parameters are
    /// actually used in a type definition.
    pub fn collect_type_arguments(&self, ty: &Type) -> Result<Self> {
        let mut ty_args_set = BTreeSet::new();
        let mut ty_args_vec = vec![];
        collect_type_arguments_recursive(ty, self, &mut ty_args_set, &mut ty_args_vec)?;
        Ok(Self {
            params: ty_args_vec,
        })
    }

    /// Calculates the difference of type parameters between this group and another.
    ///
    /// This function is used when auto generating the impl block for a type,
    pub fn diff(&self, other: &Self) -> Self {
        let filtered: Vec<Ident> = self
            .params
            .iter()
            .filter(|n| !other.contains(n))
            .cloned()
            .collect();
        Self { params: filtered }
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

    /// Converts the type parameters into a syntax suitable for function invocation (e.g., `::<T>`).
    /// The result of this is used when invoking a function with type parameters for example function_name::<T>
    pub fn to_syntax_invoke(&self) -> TokenStream {
        if self.params.is_empty() {
            TokenStream::new()
        } else {
            let content = self.params.iter().map(|n| quote!(#n));
            quote!(::<#(#content),*>) //interpolates the content into the token stream: ::<T, U>
        }
    }
}

/// Recursively collects type arguments from a given type.
///
/// This helper function is used to traverse a type and collect any type parameters
/// that are used as type arguments. It updates the provided sets and vectors with
/// the collected identifiers.
fn collect_type_arguments_recursive(
    ty: &Type,
    ty_params: &TypeParamGroup,
    ty_args: &mut BTreeSet<Ident>,
    ty_args_ordered: &mut Vec<Ident>,
) -> Result<()> {
    // Extract the path segment from the type
    let segment = match ty {
        Type::Path(TypePath {
            qself,
            path: Path {
                leading_colon,
                segments,
            },
        }) => {
            // Qualified Self types and leading colons are not expected
            ensure_none!(
                qself.as_ref().map(|q| q.ty.as_ref()),
                "type with qualified self is not expected"
            );
            ensure_none!(leading_colon, "no leading colon expected");

            // We expect exactly one segment in the path
            let mut iter = segments.iter();
            let segment = ensure_some!(iter.next(), ty, "type name");
            ensure_none!(iter.next(), "no extra segments expected"); // should only have one
            segment
        }
        _ => bail_on!(ty, "expect type path"), // can be invoked with tuple (A,B)
    };
    let PathSegment { ident, arguments } = segment;

    // Analyze the segments
    match arguments {
        PathArguments::None => {
            if !ty_params.contains(ident) {
                // here the group is checked if it contains the ident
                // Just a type, not a type argument; do nothing
                return Ok(());
            }
            if ty_args.insert(ident.clone()) {
                // insert into ordered vector if it's a new argument
                ty_args_ordered.push(ident.clone());
            }
        }
        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
            colon2_token,
            lt_token: _,
            args,
            gt_token: _,
        }) => {
            ensure_none!(colon2_token, "no double colon expected"); // Double colon is not expected
            if ty_params.contains(ident) {
                // Type parameters should not have arguments
                bail_on!(arguments, "type parameter should not have arguments");
            }

            for arg in args {
                match arg {
                    // Extract type arguments recursively
                    GenericArgument::Type(sub_ty) => {
                        collect_type_arguments_recursive(
                            sub_ty,
                            ty_params,
                            ty_args,
                            ty_args_ordered,
                        )?;
                    }
                    _ => bail_on!(arg, "expect type argument"),
                }
            }
        }
        PathArguments::Parenthesized(args) => bail_on!(args, "invalid type arguments"),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::punctuated::Punctuated;
    use syn::{Generics, Type, parse_quote};

    #[test]
    fn test_parse_generics_params_empty_bail() {
        // this invokes ensure_none!(lt_token); inside is param empty.
        let generics: Generics = parse_quote! {<>};
        let type_param_group: Result<TypeParamGroup> = TypeParamGroup::parse_generics(&generics);

        assert!(type_param_group.is_err());
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
            "invalid modifier"
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
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
        assert_eq!(type_param_group.err().unwrap().to_string(), "unexpected");
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
    fn test_contains() {
        let type_param_group = TypeParamGroup {
            params: vec![parse_quote! {U}, parse_quote! {T}],
        };

        let ident: Ident = parse_quote! {U};
        let res = type_param_group.contains(&ident);
        assert!(res);
    }

    #[test]
    fn test_diff() {
        let type_param_group_one = TypeParamGroup {
            params: vec![parse_quote!(U), parse_quote!(T)],
        };
        let type_param_group_two = TypeParamGroup {
            params: vec![parse_quote!(U)],
        };

        let res = type_param_group_one.diff(&type_param_group_two);

        assert_eq!(res.params.len(), 1);
        assert!(res.contains(&parse_quote!(T)));
        assert!(!res.contains(&parse_quote!(U)));
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

    #[test]
    fn test_to_syntax_invoke_one() {
        // Test converting to syntax invocation
        let group = TypeParamGroup { params: vec![] };
        let tokens = group.to_syntax_invoke();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_to_syntax_invoke_two() {
        // Test converting to syntax invocation
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let tokens = group.to_syntax_invoke();
        let expected: TokenStream = quote!(::<T, U>);
        assert_eq!(tokens.to_string(), expected.to_string());
    }

    #[test]
    fn test_collect_type_arguments() {
        // Test collecting type arguments from a type
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(Option<T>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_ok());
        let collected = result.unwrap();
        assert_eq!(collected.params.len(), 1);

        // get params
        let params = collected.params;
        assert_eq!(params[0].to_string(), "T");

        let ty: Type = parse_quote!(Result<T, U>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_ok());
        let collected = result.unwrap();
        assert_eq!(collected.params.len(), 2);

        // get params
        let params = collected.params;
        assert_eq!(params[0].to_string(), "T");
        assert_eq!(params[1].to_string(), "U");
    }

    // ensure_none!(qself.as_ref().map(|q| q.ty.as_ref()));
    #[test]
    fn test_collect_type_arguments_qself() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(<T as Trait>::AssociatedType);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    // ensure_none!(leading_colon);
    #[test]
    fn test_collect_type_arguments_leading_colon() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(::std::String);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    // _ => bail_on!(ty, "expect type path"), is invoked
    #[test]
    fn test_collect_type_arguments_expect_type_path() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!((A, B));
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect type path");
    }

    // PathArguments::None => {
    //     if !ty_params.contains(ident) {
    //         return Ok(());
    //     }
    #[test]
    fn test_collect_type_arguments_normal_type() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(String); // params does not contain String
        let result = group.collect_type_arguments(&ty);

        assert!(result.is_ok());
        let collected = result.unwrap();
        assert_eq!(collected.params.len(), 0);
    }

    // PathArguments::None
    #[test]
    fn test_collect_type_arguments_normal_type_two() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(T); // params does contain T
        let result = group.collect_type_arguments(&ty);

        assert!(result.is_ok());
        let collected = result.unwrap();
        assert_eq!(collected.params.len(), 1);
        assert_eq!(collected.params[0].to_string(), "T");
    }

    // ensure_none!(colon2_token); // Double colon is not expected
    #[test]
    fn test_collect_type_arguments_colon2_token() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(Option::<T>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    // if ty_params.contains(ident) {
    //     // Type parameters should not have arguments
    //     bail_on!(arguments, "type parameter should not have arguments");
    // }
    #[test]
    fn test_collect_type_arguments_type_parameter() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(Option), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(Option<T>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "type parameter should not have arguments"
        );
    }

    //  _ => bail_on!(arg, "expect type argument"),
    #[test]
    fn test_collect_type_arguments_expect_type_argument() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(Option<32>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect type argument");
    }

    // ensure_none!(iter.next()); // should only have one
    // segment
    #[test]
    fn test_collect_type_arguments_more_segments() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        let ty: Type = parse_quote!(std::Option<T, U>);
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    // PathArguments::Parenthesized(args) => bail_on!(args, "invalid type arguments"),
    #[test]
    fn test_collect_type_arguments_parenthesized() {
        let group = TypeParamGroup {
            params: vec![parse_quote!(T), parse_quote!(U)],
        };
        // let ty: Type = parse_quote!(myString(T)); //? this does not get parsed as a type
        let ty: Type = Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                // punctuated path segments myString(T)
                segments: Punctuated::from_iter(vec![PathSegment {
                    ident: parse_quote!(myString),
                    arguments: PathArguments::Parenthesized(parse_quote!((T))),
                }]),
            },
        });
        let result = group.collect_type_arguments(&ty);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "invalid type arguments");
    }
}

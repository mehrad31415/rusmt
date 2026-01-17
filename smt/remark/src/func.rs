//! Provides function restriction checks and code derivation for SMT conversion.

use crate::attr::{MetaValue, parse_dict};
use crate::generics::TypeParamGroup;
use crate::{bail_on, ensure_none, ensure_some};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Path, PathSegment, Result, Signature, Type, TypePath};

/// Checks the function signature and derives additional code if a (method) attribute is present.
///
/// This function inspects the target function's signature to ensure it meets certain criteria,
/// such as not being `const`, `async`, `unsafe`, or having an ABI or variadic parameters.
/// Additionally, all of the generics used in the function signature must ONLY implement the `SMT` trait.
/// If a method attribute is provided (it is some), the function `argument` must have at least one input parameter.
/// The first parameter must not be a receiver (e.g., `self`) and must be a typed pattern.
/// The type of the first parameter cannot be T, U, etc., as these are type arguments.
/// The type of the first parameter can contain nested type arguments.
///
/// It then generates an implementation of a method for the type specified by the first
/// parameter, if a `method` attribute is provided.
///
/// */* input
///         fn ADD<T:SMT>(a: i32, b: T) -> i32 {
///             a
///         }
/// method : Some(add)
/// */* output structure
///         impl #tokenized_impl_generics(empty) #self_ty_name(i32) #self_ty_args(empty) {
///             #vis(empty) fn #method(add) #tokenized_method_generics(<T:SMT>) /(#tokenized_method_params)(self, b: T) #output(-> i32) {
///                 #func_name(ADD) #tokenized_func_ty_args(#tokenized_func_args)
///             }
///         }
///         ** So basically it implements the name of the method provided as the second argument of check_and_derive to the type of the first argument of the function in the first argument of check_and_derive. **
///         impl i32 {
///             fn add<T:SMT>(self, b: T) -> i32 {
///                 ADD::<T>(self, b)
///             }
///         }
/// */*
fn check_and_derive(target: &ItemFn, method: Option<&Ident>) -> Result<Option<TokenStream>> {
    // Unpack the function components.
    // #[inline]
    // pub fn add(a: i32, b: i32) -> i32 {
    //     a + b
    // }
    // attrs: Vec<Attribute>, vis: Visibility, sig: Signature, block: Box<Block>
    // in the above example, attrs is #[inline], vis is pub, sig is fn add(a: i32, b: i32) -> i32
    // and block is {a + b}
    let ItemFn {
        attrs: _,
        vis,
        sig,
        block: _,
    } = target;

    // extern "C" const unsafe async fn example<T: Copy>(a: i32, b: i32, ...) -> Result<T, &'static str> {
    //      function body
    // }
    // in the above example, constness is const, asyncness is async, unsafety is unsafe, abi is "C"
    // fn_token is fn, ident is example, generics is <T: Copy>, paren_token is (), inputs is a: i32, b: i32
    // variadic is ... (this can only be used in external code - from C for example - as Rust does not support variadics - that is why macros exist), output is Result<T, &'static str>
    let Signature {
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token: _,
        ident: func_name,
        generics,
        paren_token: _,
        inputs,
        variadic,
        output,
    } = sig;

    // Ensure the function is not const, async, unsafe, has an ABI, or is variadic.
    ensure_none!(constness, "unexpected constness in function declaration");
    ensure_none!(asyncness, "unexpected asyncness in function declaration");
    ensure_none!(unsafety, "unexpected unsafety in function declaration");
    ensure_none!(abi, "unexpected ABI in function declaration");
    ensure_none!(
        variadic,
        "unexpected variadic parameters in function declaration"
    );

    // Extract the function generics.
    // this only gives an error if the generics are not of the form <T:SMT, U:SMT ...>
    let ty_params = TypeParamGroup::parse_generics(generics)?;

    // Probe on the first parameter.
    let param0 = match (inputs.first(), method) {
        // If there are no inputs and no method attribute, return None (nothing to derive).
        (None, None) => return Ok(None),
        // If there is a method attribute but no inputs, return an error (need at least one parameter).
        (None, Some(_)) => bail_on!(sig, "at least one parameter"),
        // Otherwise, get the first parameter.
        (Some(p), _) => p,
    };

    // Extract the self type from the first parameter.
    let self_ty = match param0 {
        // If the first parameter is a receiver (e.g., `self`), return an error.
        FnArg::Receiver(_) => bail_on!(param0, "expect type declaration"),
        // If the first parameter is a typed pattern, extract the type.
        FnArg::Typed(PatType {
            attrs: _,
            pat: _,
            colon_token: _,
            ty,
        }) => ty.as_ref(),
    };

    // Collect and validate type arguments involved.
    // self_ty_consumed_params will contain all the generics used in the type of the first argument of the function.
    let self_ty_consumed_params = ty_params.collect_type_arguments(self_ty)?;

    // Decide whether to early terminate.
    let method = match method {
        // If no method attribute is provided, return None (nothing to derive).
        None => return Ok(None),
        // Otherwise, proceed with the provided method identifier.
        Some(m) => m,
    };

    // Validate the self type.
    let (self_ty_name, self_ty_args) = match self_ty {
        // Extract the type that should be marked as `self`.
        Type::Path(TypePath {
            qself: _,
            path: Path {
                leading_colon: _,
                segments,
            },
        }) => {
            let mut iter = segments.iter();
            // Get the first segment (the type name).
            let segment = ensure_some!(iter.next(), param0, "type name"); // unreachable invocation

            // Ensure there are no additional segments (no nested types).
            ensure_none!(iter.next(), "unexpected nested type segments");

            let PathSegment { ident, arguments } = segment;
            // Ensure the type is not a type argument (cannot derive a method for a type argument).
            // so basically the first argument cannot be like x:T because in that case, T should implement the SMT trait, thus it is in the self_ty_consumed_params and ident is T.
            if self_ty_consumed_params.contains(ident) {
                bail_on!(ident, "cannot derive a method for a type argument");
            }

            // Done with the parsing, return the type name and its arguments.
            (ident, arguments)
        }
        // For other types, this should not occur.
        _ => unreachable!(),
    };

    // Derive the generics tokens for the impl block.
    // <U:SMT, T:SMT ...> will be created for all the generics used inside the first arg of func.
    // this will be used for the impl signature
    let tokenized_impl_generics = self_ty_consumed_params.to_syntax_def();

    // Get the method type parameters by removing the self type parameters.
    // this basically gets all the generic types that are not used in the first arg of the func for the method signature in impl block
    let method_ty_params = ty_params.diff(&self_ty_consumed_params);
    // <U:SMT, T:SMT ...> will be created for all the generics NOT used inside the first arg of func.
    let tokenized_method_generics = method_ty_params.to_syntax_def();

    // Generate tokens for function type arguments.
    // ::<T,U> for all the generics used in the args of the func
    let tokenized_func_ty_args = ty_params.to_syntax_invoke();

    // Derive the parameter tokens.
    let mut iter = inputs.iter();
    // Skip the first parameter (already handled).
    iter.next().unwrap();
    let mut method_params: Vec<TokenStream> = vec![quote!(self)];
    let mut func_args: Vec<TokenStream> = vec![quote!(self)];
    for arg in iter.by_ref() {
        match arg {
            // Receivers are unexpected for any of the arguments of the function.
            // Receiver Types (self, &self, &mut self) must be the first parameter in methods. so this part is unreachable
            FnArg::Receiver(_) => unreachable!(),
            FnArg::Typed(param) => {
                // Get the pattern (parameter name). in x:i32, x is the pattern.
                let pat = param.pat.as_ref();
                // Add the parameter to the method parameters.
                method_params.push(quote!(#param)); // foo: f64
                // Add the pattern to the function arguments.
                func_args.push(quote!(#pat)); // foo
            }
        }
    }
    // tokenized_method_params for fn add<T:SMT, U:SMT> (x : String, foo: f64) -> f64
    // is: self, foo: f64 (the first arg is skipped)
    let tokenized_method_params = quote!(#(#method_params),*);
    // tokenized_func_args for fn add<T:SMT, U:SMT> (x : String, foo: f64) -> f64
    // is: self, foo (the first arg is skipped)
    let tokenized_func_args = quote!(#(#func_args),*); // this is used for function invocation

    // Construct the expanded token stream (the generated code).
    let extended = quote! {
        impl #tokenized_impl_generics #self_ty_name #self_ty_args {
            #vis fn #method #tokenized_method_generics (#tokenized_method_params) #output {
                #func_name #tokenized_func_ty_args(#tokenized_func_args)
            }
        }
    };
    Ok(Some(extended))
}

/// Derives code for functions based on attributes.
pub fn derive_for_func(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // Parse the attributes into a dictionary.
    let mut dict = parse_dict(&attr)?;

    // Ensure that the underlying item is a function.
    let target = syn::parse2::<ItemFn>(item.clone())?;

    let derived;
    // Derive the method if requested.
    derived = match dict.remove("method") {
        // If no method attribute, check and derive without method.
        // this just checks the function signature
        None => check_and_derive(&target, None)?,
        // If a method attribute is provided with a single identifier, derive with the method name. [method = ident]
        Some(MetaValue::One(ident)) => check_and_derive(&target, Some(&ident))?,
        // If method attribute is a set (multiple identifiers), return an error. [method = { ... }]
        Some(MetaValue::Set(ident)) => bail_on!(
            attr,
            "invalid method attribute: {{ {} }}",
            ident
                .into_iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    };

    // If there are any remaining attributes, return an error so only method is allowed
    if !dict.is_empty() {
        bail_on!(attr, "unknown attributes");
    }

    // Consolidate the extended token stream.
    let output = match derived {
        // If no derivation occurred, return the original item.
        None => item,
        // If derivation occurred, append the generated code to the original item.
        Some(extended) => {
            let mut output = item;
            output.extend(extended);
            output
        }
    };

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    // ensure_none!(constness); is invoked in the check_and_derive function
    fn test_check_and_derive_bail_constness() {
        let item_fn: syn::ItemFn = parse_quote! {
            const fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    #[test]
    // ensure_none!(asyncness); is invoked in the check_and_derive function
    fn test_check_and_derive_bail_asyncness() {
        let item_fn: syn::ItemFn = parse_quote! {
            async fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    #[test]
    // ensure_none!(unsafety); is invoked in the check_and_derive function
    fn test_check_and_derive_bail_unsafety() {
        let item_fn: syn::ItemFn = parse_quote! {
            unsafe fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    #[test]
    // ensure_none!(abi); is invoked in the check_and_derive function
    // extern "C" is typically used for FFI (Foreign Function Interface).
    fn test_check_and_derive_bail_abi() {
        let item_fn: syn::ItemFn = parse_quote! {
            extern "C" fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    #[test]
    // ensure_none!(variadic); is invoked in the check_and_derive function
    fn test_check_and_derive_bail_variadic() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(a: i32, b: i32, ...) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected");
    }

    // Extract the function generics.
    // let ty_params = TypeParamGroup::parse_generics(generics)?; is invoked in the check_and_derive function
    // this part forces that all generic types should ONLY implement the SMT trait
    #[test]
    fn test_check_and_derive_ty_params() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add<T: Copy>(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect SMT trait");
    }

    // If there are no inputs and no method attribute, return None (nothing to derive).
    // (inputs.first(), method) => (None, None) => return Ok(None), this pattern is cover in the test case
    #[test]
    fn test_check_and_derive_no_inputs_no_method() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add() -> i32 {
                1
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_ok());
    }

    // If there is a method attribute but no inputs, return an error (need at least one parameter).
    // (inputs.first(), method) => (None, Some(_)) => bail_on!(sig, "at least one parameter"), this pattern is cover in the test case
    #[test]
    fn test_check_and_derive_no_inputs_method() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add() -> i32 {
                1
            }
        };
        let method = &parse_quote! { add };
        let method = Some(method);
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "at least one parameter");
    }

    // If the first parameter is a receiver (e.g., `self`), return an error.
    // param0 => FnArg::Receiver(_) => bail_on!(param0, "expect type declaration"), this pattern is cover in the test case
    #[test]
    fn test_check_and_derive_receiver() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(&self, a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect type declaration");
    }

    // Collect and validate type arguments involved.
    // let self_ty_consumed_params = ty_params.collect_type_arguments(self_ty)?; is invoked in the check_and_derive function
    #[test]
    fn test_check_and_derive_collect_type_arguments() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add<T : SMT, Option : SMT>(a: Option<T>, b: i32) -> i32 {
                b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "type parameter should not have arguments"
        );
    }

    // If no method attribute is provided, return None (nothing to derive).
    // match method => None => return Ok(None), this pattern is cover in the test case
    #[test]
    fn test_check_and_derive_no_method() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let method = None;
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_ok());
    }

    // Ensure the type is not a type argument (cannot derive a method for a type argument).
    // if self_ty_consumed_params.contains(ident) {
    //     bail_on!(ident, "cannot derive a method for a type argument");
    // }  this part is cover in the test case
    #[test]
    fn test_check_and_derive_type_argument() {
        // so basically the first input should never have a generic type if a method exists. Because the method should be implemented for the generic type and that is not possible.
        // unless the generics are embedded like fn add<T: SMT, U : SMT>(a: Struct<T>, b: U) this is totally fine.
        let item_fn: syn::ItemFn = parse_quote! {
            fn add<T: SMT, U : SMT>(a: T, b: U) -> i32 {
                b
            }
        };
        let method = &parse_quote! { add };
        let method = Some(method);
        let result = check_and_derive(&item_fn, method);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "cannot derive a method for a type argument"
        );
    }

    // this tests the happy flow of the check_and_derive function
    #[test]
    fn test_check_and_derive_happy_flow() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn ADD<T:SMT>(a: i32, b: T) -> i32 {
                a
            }
        };
        let method: Ident = parse_quote! { add };
        let method = Some(&method);

        let result = check_and_derive(&item_fn, method);

        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.is_some());
    }

    // test derive_for_func
    // let mut dict = parse_dict(&attr)?; is invoked in the derive_for_func function
    #[test]
    fn test_derive_for_func_parse_dict() {
        let attr = parse_quote! { 1 = value };
        let item = quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "key not an identifier");
    }

    // test derive_for_func
    // let target = syn::parse2::<ItemFn>(item.clone())?;
    #[test]
    fn test_derive_for_func_parse_item_fn() {
        let attr = parse_quote! { key = value };
        let item = parse_quote! {
            struct MyStruct{
                ident: i32
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
    }

    // test derive_for_func
    // None => check_and_derive(&target, None)?, this pattern is covered in the test case
    #[test]
    fn test_derive_for_func_no_method() {
        let attr = parse_quote! { k = v };
        let item = parse_quote! {
            const fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        }; // function should not be const
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
    }

    // test derive_for_func
    // Some(MetaValue::Set(ident)) => bail_on!(attr, "invalid method attribute: {{ {} }}", ident.into_iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")), this pattern is covered in the test case
    #[test]
    fn test_derive_for_func_set_method() {
        let attr = parse_quote! { method = [ add, sub ] };
        let item = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "invalid method attribute: { add,sub }"
        );
    }

    // test derive_for_func
    // bail_on!(attr, "unknown attributes"); is invoked in the derive_for_func function
    #[test]
    fn test_derive_for_func_unknown_attributes() {
        let attr = parse_quote! { k = v };
        let item = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unknown attributes");
    }

    // test derive_for_func
    // None => item, this pattern is covered in the test case
    // either the method key is not provided or the function signature does not have any inputs
    #[test]
    fn test_derive_for_func_no_derivation() {
        let attr = parse_quote! {};
        let item = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_ok());
    }
}

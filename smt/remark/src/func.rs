//! Provides function restriction checks for SMT conversion.

use crate::generics::TypeParamGroup;
use crate::{bail_on, ensure_none};
use proc_macro2::TokenStream;
use syn::{FnArg, ItemFn, Result, Signature};

/// Checks the function signature.
///
/// This function inspects the target function's signature to ensure it meets certain criteria,
/// such as not being `const`, `async`, `unsafe`, or having an ABI or variadic parameters.
/// Additionally, all of the generics used in the function signature must ONLY implement the `SMT` trait.
fn check(target: &ItemFn) -> Result<()> {
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
        vis: _,
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
        ident: _,
        generics,
        paren_token: _,
        inputs,
        variadic,
        output: _,
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
    let _ty_params = TypeParamGroup::parse_generics(generics)?;

    // Disallow receiver arguments (methods). `#[smt_fn]` is intended for free functions.
    for arg in inputs.iter() {
        if matches!(arg, FnArg::Receiver(_)) {
            bail_on!(arg, "expect type declaration");
        }
    }

    Ok(())
}

/// Derives code for functions based on attributes.
pub fn derive_for_func(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // `#[smt_fn]` is validation-only. No arguments are accepted.
    if !attr.is_empty() {
        bail_on!(attr, "unexpected attributes");
    }

    // Ensure that the underlying item is a function.
    let target = syn::parse2::<ItemFn>(item.clone())?;
    check(&target)?;

    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    // ensure_none!(constness); is invoked in the check function
    fn test_check_bail_constness() {
        let item_fn: syn::ItemFn = parse_quote! {
            const fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected constness in function declaration");
    }

    #[test]
    // ensure_none!(asyncness); is invoked in the check function
    fn test_check_bail_asyncness() {
        let item_fn: syn::ItemFn = parse_quote! {
            async fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected asyncness in function declaration");
    }

    #[test]
    // ensure_none!(unsafety); is invoked in the check function
    fn test_check_bail_unsafety() {
        let item_fn: syn::ItemFn = parse_quote! {
            unsafe fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected unsafety in function declaration");
    }

    #[test]
    // ensure_none!(abi); is invoked in the check function
    // extern "C" is typically used for FFI (Foreign Function Interface).
    fn test_check_bail_abi() {
        let item_fn: syn::ItemFn = parse_quote! {
            extern "C" fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected ABI in function declaration");
    }

    #[test]
    // ensure_none!(variadic); is invoked in the check function
    fn test_check_bail_variadic() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(a: i32, b: i32, ...) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected variadic parameters in function declaration");
    }

    // Extract the function generics.
    // let ty_params = TypeParamGroup::parse_generics(generics)?; is invoked in the check function
    // this part forces that all generic types should ONLY implement the SMT trait
    #[test]
    fn test_check_ty_params() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add<T: Copy>(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect SMT trait");
    }

    #[test]
    fn test_check_no_inputs_ok() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add() -> i32 {
                1
            }
        };
        let result = check(&item_fn);

        assert!(result.is_ok());
    }

    #[test]
    fn test_check_receiver() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(&self, a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = check(&item_fn);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "expect type declaration");
    }

    #[test]
    fn test_derive_for_func_parse_item_fn() {
        let attr = parse_quote! {};
        let item = parse_quote! {
            struct MyStruct{
                ident: i32
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
    }

    #[test]
    fn test_derive_for_func_checks_signature() {
        let attr = parse_quote! {};
        let item = parse_quote! {
            const fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        }; // function should not be const
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
    }

    #[test]
    fn test_derive_for_func_rejects_attributes() {
        let attr = parse_quote! { method = add };
        let item = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let result = derive_for_func(attr, item);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().to_string(), "unexpected attributes");
    }

    #[test]
    fn test_derive_for_func_ok() {
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

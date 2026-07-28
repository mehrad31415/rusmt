//! Handling Annotation for Types

use crate::generics::TypeParamGroup;
use crate::{bail_on, ensure_none, ensure_some};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, Fields, Index, Item, ItemEnum, ItemStruct, Result, Variant, parse_quote};

/// Derives the `SMT` trait implementation for a struct.
fn derive_for_struct(item: &mut ItemStruct) -> Result<TokenStream> {
    // #[derive(Debug)]
    // struct MyStruct<T> {
    //     a: i32,
    //     b: T,
    // };
    // attrs = #[derive(Debug)] // Attributes applied to the struct
    // vis = Visibility::Inherited // Visibility of the struct
    // struct_token = struct // The struct keyword
    // ident = MyStruct // The name of the struct
    // generics = <T> // Generics of the struct
    // fields = {a: i32, b: T} // Fields of the struct
    // semi_token = ; // The semicolon at the end of the struct definition (Option)
    let ItemStruct {
        attrs,
        vis: _,
        struct_token: _,
        ident: struct_name,
        generics,
        fields,
        semi_token: _,
    } = item;

    // Add derive attributes to the struct such as `Debug`, `Clone`, `Copy`, `Default`, and `Hash`.
    // every SMT type is copyable (as there are no references in the SMT types). If a type implements Copy, it must also implement Clone.
    // Implementing Debug is for testing purposes. Ord is not implemented as the SMT trait has a cmp method.
    // Default is implemented for the default method in the expression macros. Note that `Default` can be derived automatically for structs.
    // The Hash trait is implemented for storing in BTreeMaps in the SMT.
    // all types in SMT must derive the following traits: Debug, Clone, Copy, Default, and Hash. (also Send and Sync)
    attrs.push(parse_quote!(
        #[derive(Debug, Clone, Copy, Default, Hash)]
    ));

    // Generate the `SMT::_cmp` function implementation based on the struct fields.
    let cmp_fn = match fields {
        // Unit structs (struct MyUnitStruct;) are not supported.
        Fields::Unit => bail_on!(item, "unexpected unit fields in struct"),

        // Tuple structs (unnamed fields) - struct MyTupleStruct(i32, String);
        Fields::Unnamed(defs) => {
            let mut cmp_exprs = vec![];
            for (i, field) in defs.unnamed.iter().enumerate() {
                // struct MyTupleStruct(String); // field example
                // attrs: [],
                // vis: Visibility::Inherited,
                // mutability: FieldMutability::None,
                // ident: None,
                // colon_token: None,
                // ty: Type::Path {
                //     qself: None,
                //     path: Path {
                //         leading_colon: None,
                //         segments: [
                //             PathSegment {
                //                 ident: Ident(
                //                     String,
                //                 ),
                //                 arguments: PathArguments::None,
                //             },
                //         ],
                //     },
                // },
                let Field {
                    attrs: _, // attributes applied to the field
                    vis: _,   // visibility of the field
                    ident,    // name of the field
                    colon_token,
                    ty: _,
                    default,
                    modifiers,
                } = field;

                // Sanity checks.
                modifiers.require_empty()?; // No modifiers are expected
                if let Some((_eq, def)) = default {
                    bail_on!(def, "no default value expected")
                }

                // ident and colon_token are None for tuple structs.
                ensure_none!(ident, "the field of a tuple struct cannot have a name"); //unreachable invocation as the rust compiler doesn't allow it.
                ensure_none!(
                    colon_token,
                    "the field of a tuple struct cannot have a colon token"
                ); //unreachable invocation as the rust compiler doesn't allow it.

                // Generate comparison expressions for each field.
                let index = Index::from(i);
                let cmp_expr = quote! {
                    match self.#index._cmp(rhs.#index) {
                        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
                        std::cmp::Ordering::Equal => (),
                        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                    };
                };

                // Collect the expressions.
                cmp_exprs.push(cmp_expr);
            }

            // Build the `_cmp` function.
            quote! {
                fn _cmp(self, rhs: Self) -> std::cmp::Ordering {
                    #(#cmp_exprs)*
                    std::cmp::Ordering::Equal
                }
            }
        }

        // Structs with named fields.
        Fields::Named(defs) => {
            let mut cmp_exprs = vec![];
            for field in defs.named.iter() {
                let Field {
                    attrs: _,
                    vis: _,
                    ident,
                    colon_token,
                    ty: _,
                    default,
                    modifiers,
                } = field;

                // Sanity checks.
                modifiers.require_empty()?; // No modifiers are expected
                if let Some((_eq, def)) = default {
                    bail_on!(def, "no default value expected")
                }

                let name = ensure_some!(ident, field, "name");
                ensure_some!(colon_token, field, "colon");

                // Generate comparison expressions for each field.
                let cmp_expr = quote! {
                    match self.#name._cmp(rhs.#name) {
                        std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
                        std::cmp::Ordering::Equal => (),
                        std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                    };
                };

                // Collect the expressions.
                cmp_exprs.push(cmp_expr);
            }

            // Build the `_cmp` function.
            quote! {
                fn _cmp(self, rhs: Self) -> std::cmp::Ordering {
                    #(#cmp_exprs)*
                    std::cmp::Ordering::Equal
                }
            }
        }
    };

    // Parse and validate generics.
    let ty_params = TypeParamGroup::parse_generics(generics)?;

    // Generate the implementation block.
    let tokenized_impl_generics = ty_params.to_syntax_def();
    let tokenized_type_ty_insts = ty_params.to_syntax_use();

    let extended = quote! {
        impl #tokenized_impl_generics SMT for #struct_name #tokenized_type_ty_insts {
            #cmp_fn
        }
    };
    Ok(extended)
}

/// Derives the `SMT` trait implementation for an enum.
fn derive_for_enum(item: &mut ItemEnum) -> Result<TokenStream> {
    // #[derive(Debug)]
    // enum MyEnum<T> {
    //     Variant1,
    //     Variant2(i32, T),
    //     Variant3 { a: String, b: f64 },
    // };
    // attrs = #[derive(Debug)] // Attributes applied to the enum
    // vis = Visibility::Inherited // Visibility of the enum. This is defined at the enum level and applies to all variants.
    // enum_token = enum // The enum keyword
    // ident = MyEnum // The name of the enum
    // generics = <T> // Generics of the enum
    // brace_token = {} // The brace tokens for the enum definition
    // variants = [Variant1, Variant2(i32, T), Variant3 { a: String, b: f64 }] // Variants of the enum
    let ItemEnum {
        attrs,
        vis: _,
        enum_token: _,
        ident: enum_name,
        generics,
        brace_token: _,
        variants,
    } = item;

    // Add derive attributes to the enum such as `Debug`, `Clone`, `Copy`, and `Hash`.
    // Note: `Default` is NOT derived for enums automatically.
    attrs.push(parse_quote!(
        #[derive(Debug, Clone, Copy, Hash)]
    ));

    // Generate the `Default::default` function.
    let variant = ensure_some!(variants.first(), item, "variants");
    // enum MyEnum { #[my_attr] Variant1, Variant2(i32, T), Variant3 { a: String, b: f64 }, i32 = 42 }
    // attrs = #[my_attr] // Attributes applied to the variant
    // ident = Variant1 or Variant2 or Variant3 or i32 // The name of the variant
    // fields = Fields::Unit or (i32, T) or { a: String, b: f64 }
    // discriminant = Some((=, 42)) // Discriminant of the variant
    let Variant {
        attrs: _,
        ident: variant_ident,
        fields,
        discriminant: _,
    } = variant;

    let default_fn = match fields {
        // Unit variant.
        Fields::Unit => quote! {
            fn default() -> Self { Self::#variant_ident }
        },
        // Tuple variant.
        Fields::Unnamed(fields_unnamed) => {
            let init: Vec<_> = fields_unnamed
                .unnamed
                .iter()
                .map(|f| {
                    let field_type = &f.ty;
                    quote!(#field_type::default())
                })
                .collect();
            quote! {
                fn default() -> Self { Self::#variant_ident(#(#init),*) }
            }
        }
        // Variant with named fields.
        Fields::Named(fields_named) => {
            let init: Vec<_> = fields_named
                .named
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().expect("named field");
                    let field_type = &f.ty;
                    quote!(#field_name: #field_type::default())
                })
                .collect();
            quote! {
                fn default() -> Self { Self::#variant_ident { #(#init),* } }
            }
        }
    };

    // Holds the match arms for the `SMT::_cmp` function.
    let mut cmp_arms = vec![];
    for (i, variant) in variants.iter().enumerate() {
        let Variant {
            attrs: _,
            ident: variant_name,
            fields,
            discriminant,
        } = variant;

        // Sanity checks: variants should not have discriminants.
        ensure_none!(
            discriminant.as_ref().map(|(_, e)| e),
            "unexpected discriminant in enum variant"
        );

        // Generate patterns to match the variants.
        let lhs_arm_skip = match fields {
            Fields::Unit => quote! {#enum_name::#variant_name},
            Fields::Unnamed(..) => quote! {#enum_name::#variant_name(..)},
            Fields::Named(..) => quote! {#enum_name::#variant_name{ .. }},
        };

        // Compare the current variant with other variants.
        for (j, rhs_variant) in variants.iter().enumerate() {
            // Skip comparing the variant with itself for now.
            if i == j {
                continue;
            }

            let rhs_variant_name = &rhs_variant.ident;
            let rhs_arm_skip = match rhs_variant.fields {
                Fields::Unit => quote! {#enum_name::#rhs_variant_name},
                Fields::Unnamed(..) => quote! {#enum_name::#rhs_variant_name(..)},
                Fields::Named(..) => quote! {#enum_name::#rhs_variant_name{ .. }},
            };

            // Generate match arms for different variants.
            let match_arm = if i < j {
                quote! {
                    (#lhs_arm_skip, #rhs_arm_skip) => return std::cmp::Ordering::Less,
                }
            } else {
                quote! {
                    (#lhs_arm_skip, #rhs_arm_skip) => return std::cmp::Ordering::Greater,
                }
            };
            cmp_arms.push(match_arm);
        }

        // Generate match arms for the same variant.
        let match_arm = match fields {
            // Unit variants.
            Fields::Unit => {
                quote! {
                    (#enum_name::#variant_name, #enum_name::#variant_name) => (),
                }
            }
            // Tuple variants (unnamed fields).
            Fields::Unnamed(defs) => {
                let mut cmp_lhs_vars = vec![];
                let mut cmp_rhs_vars = vec![];
                let mut cmp_exprs = vec![];

                for (f, field) in defs.unnamed.iter().enumerate() {
                    let Field {
                        attrs: _,
                        vis: _,
                        ident,
                        colon_token,
                        ty: _,
                        default,
                        modifiers,
                    } = field;

                    // Sanity checks.
                    modifiers.require_empty()?; // No modifiers are expected
                    if let Some((_eq, def)) = default {
                        bail_on!(def, "no default value expected")
                    }

                    ensure_none!(ident, "the field of a tuple struct cannot have a name");
                    ensure_none!(
                        colon_token,
                        "the field of a tuple struct cannot have a colon token"
                    );

                    // Create variable names for pattern matching.
                    let cmp_lhs_var = format_ident!("l{}", f);
                    let cmp_rhs_var = format_ident!("r{}", f);
                    let cmp_expr = quote! {
                        match #cmp_lhs_var._cmp(#cmp_rhs_var) {
                            std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
                            std::cmp::Ordering::Equal => (),
                            std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                        };
                    };

                    cmp_lhs_vars.push(cmp_lhs_var);
                    cmp_rhs_vars.push(cmp_rhs_var);
                    cmp_exprs.push(cmp_expr);
                }

                // Build the match arm.
                quote! {
                    (
                        #enum_name::#variant_name(#(#cmp_lhs_vars),*),
                        #enum_name::#variant_name(#(#cmp_rhs_vars),*)
                    ) => {
                        #(#cmp_exprs)*
                    }
                }
            }
            // Variants with named fields.
            Fields::Named(defs) => {
                let mut cmp_lhs_decls = vec![];
                let mut cmp_rhs_decls = vec![];
                let mut cmp_exprs = vec![];

                for field in defs.named.iter() {
                    let Field {
                        attrs: _,
                        vis: _,
                        ident,
                        colon_token,
                        ty: _,
                        default,
                        modifiers,
                    } = field;

                    // Sanity checks.
                    modifiers.require_empty()?; // No modifiers are expected
                    if let Some((_eq, def)) = default {
                        bail_on!(def, "no default value expected")
                    }

                    let field_name = ensure_some!(ident, field, "name");
                    ensure_some!(colon_token, field, "colon");

                    // Create variable names for pattern matching.
                    let cmp_lhs_var = format_ident!("l_{}", field_name);
                    let cmp_rhs_var = format_ident!("r_{}", field_name);
                    let cmp_expr = quote! {
                        match #cmp_lhs_var._cmp(#cmp_rhs_var) {
                            std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
                            std::cmp::Ordering::Equal => (),
                            std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                        };
                    };

                    cmp_lhs_decls.push(quote! {#field_name: #cmp_lhs_var});
                    cmp_rhs_decls.push(quote! {#field_name: #cmp_rhs_var});
                    cmp_exprs.push(cmp_expr);
                }

                // Build the match arm.
                quote! {
                    (
                        #enum_name::#variant_name{ #(#cmp_lhs_decls),* },
                        #enum_name::#variant_name{ #(#cmp_rhs_decls),* }
                    ) => {
                        #(#cmp_exprs)*
                    }
                }
            }
        };
        cmp_arms.push(match_arm);
    }

    // Generate the `_cmp` function.
    let cmp_fn = quote! {
        fn _cmp(self, rhs: Self) -> std::cmp::Ordering {
            match (self, rhs) {
                #(#cmp_arms)*
            }
            std::cmp::Ordering::Equal
        }
    };

    // Parse and validate generics.
    let ty_params = TypeParamGroup::parse_generics(generics)?;

    // Generate the implementation blocks.
    let tokenized_impl_generics = ty_params.to_syntax_def();
    let tokenized_type_ty_insts = ty_params.to_syntax_use();

    let extended = quote! {
        impl #tokenized_impl_generics Default for #enum_name #tokenized_type_ty_insts {
            #default_fn
        }

        impl #tokenized_impl_generics SMT for #enum_name #tokenized_type_ty_insts {
            #cmp_fn
        }
    };
    Ok(extended)
}

/// Derives the `SMT` trait implementation for a type (struct or enum).
pub fn derive_for_type(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // Check that no attributes are provided.
    if !attr.is_empty() {
        bail_on!(attr, "unexpected attributes provided");
    }

    // Parse the input item.
    let mut original = syn::parse2::<Item>(item)?;
    let extended = match &mut original {
        Item::Struct(item_struct) => derive_for_struct(item_struct)?,
        Item::Enum(item_enum) => derive_for_enum(item_enum)?,
        t => bail_on!(
            t,
            "expect a type definition (i.e., struct or enum) - smt_type only supports struct and enum"
        ), // smt_type only supports struct and enum
    };

    // Combine the original item with the generated code.
    let combined = quote! {
        #original // the original item needs to be returned in the output as the [pro_macro_attribute] consumes the original input. So if the original input is not returned, the original item will be lost. Unlike the [pro_macro_derive] which does not consume the original input and only the generated code is returned. #[proc_macro] consumes the passed input as well.
        #extended
    };
    Ok(combined)
}

// Unit tests for the derive functions.
#[cfg(test)]
mod tests {
    use super::*;
    use syn::{ItemEnum, ItemStruct, parse_quote};

    #[test]
    // Fields::Unit => bail_on!(item, "unexpected unit fields in struct"),
    // the argument cannot be a unit struct.
    fn test_derive_for_struct_field_unit() {
        let mut s: ItemStruct = parse_quote! {
            struct MyStruct;
        };

        let res = derive_for_struct(&mut s);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "unexpected unit fields in struct"
        );
    }

    #[test]
    // Fields::Unnamed(defs)
    // the argument of derive_for_struct is a struct tuple.
    fn test_derive_for_struct_field_unnamed() {
        let mut s: ItemStruct = parse_quote! {
            struct MyStruct(i32, u64);
        };

        let res = derive_for_struct(&mut s);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        assert_eq!(
            generated_code,
            "impl SMT for MyStruct { fn _cmp (self , rhs : Self) -> std :: cmp :: Ordering { match self . 0 . _cmp (rhs . 0) { std :: cmp :: Ordering :: Less => return std :: cmp :: Ordering :: Less , std :: cmp :: Ordering :: Equal => () , std :: cmp :: Ordering :: Greater => return std :: cmp :: Ordering :: Greater , } ; match self . 1 . _cmp (rhs . 1) { std :: cmp :: Ordering :: Less => return std :: cmp :: Ordering :: Less , std :: cmp :: Ordering :: Equal => () , std :: cmp :: Ordering :: Greater => return std :: cmp :: Ordering :: Greater , } ; std :: cmp :: Ordering :: Equal } }"
        );
    }

    // invoke let ty_params = TypeParamGroup::parse_generics(generics)?;
    // the argument of derive_for_struct is a struct tuple.
    #[test]
    fn test_derive_for_struct_field_unnamed_ty_params() {
        let mut s: ItemStruct = parse_quote! {
            struct MyStruct<T:Clone>(T, u64);
        };

        let res = derive_for_struct(&mut s);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "expect SMT trait");
    }

    // Fields::Named(defs)
    // the argument of derive_for_struct is a struct with named fields.
    #[test]
    fn test_derive_for_struct_field_named() {
        let mut s: ItemStruct = parse_quote! {
            struct MyStruct {
                a: i32,
                b: u64,
            }
        };

        let res = derive_for_struct(&mut s);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        assert_eq!(
            generated_code,
            "impl SMT for MyStruct { fn _cmp (self , rhs : Self) -> std :: cmp :: Ordering { match self . a . _cmp (rhs . a) { std :: cmp :: Ordering :: Less => return std :: cmp :: Ordering :: Less , std :: cmp :: Ordering :: Equal => () , std :: cmp :: Ordering :: Greater => return std :: cmp :: Ordering :: Greater , } ; match self . b . _cmp (rhs . b) { std :: cmp :: Ordering :: Less => return std :: cmp :: Ordering :: Less , std :: cmp :: Ordering :: Equal => () , std :: cmp :: Ordering :: Greater => return std :: cmp :: Ordering :: Greater , } ; std :: cmp :: Ordering :: Equal } }"
        );
    }

    #[test]
    //invoke let variant = ensure_some!(variants.first(), item, "variants");
    fn test_derive_for_enum_missing_variant() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "expect variants");
    }

    #[test]
    // test the default function where the first variant is a unit variant.
    fn test_derive_for_enum_unit_variant_default() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
                Variant1,
                Variant2(i32, u64),
                Variant3 { a: String, b: f64 },
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        dbg!(&generated_code);
        assert!(generated_code.contains("fn default () -> Self { Self :: Variant1 }"));
    }

    #[test]
    // test the default function where the first variant is a tuple struct.
    fn test_derive_for_enum_tuple_variant_default() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
                Variant1(i32, u64),
                Variant2 { a: String, b: f64 },
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        dbg!(&generated_code);
        assert!(generated_code.contains(
            "fn default () -> Self { Self :: Variant1 (i32 :: default () , u64 :: default ()) }"
        ));
    }

    #[test]
    // test the default function where the first variant is a normal struct.
    fn test_derive_for_enum_struct_variant_default() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
                Variant1 { a: String, b: f64 },
                Variant2(i32, u64),
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        dbg!(&generated_code);
        assert!(generated_code.contains("fn default () -> Self { Self :: Variant1 { a : String :: default () , b : f64 :: default () } }"));
    }

    #[test]
    // invoke ensure_none!(discriminant.as_ref().map(|(_, e)| e));
    fn test_derive_for_enum_discriminant() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
                Variant1 = 42,
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "unexpected discriminant in enum variant"
        );
    }

    #[test]
    // invoke let ty_params = TypeParamGroup::parse_generics(generics)?;
    fn test_derive_for_enum_ty_params() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum<T:Clone> {
                Variant1,
                Variant1(i32, T),
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "expect SMT trait");
    }

    #[test]
    // test happy flow
    fn test_derive_for_enum() {
        let mut item_enum: ItemEnum = parse_quote! {
            enum TestEnum {
                Variant1,
                Variant2(i32, u64),
                Variant3 { a: String, b: f64 },
            }
        };

        let res = derive_for_enum(&mut item_enum);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        // dbg!(&generated_code);
        // generated code should contain the following:
        // impl Default for TestEnum {
        //     fn default() -> Self {
        //         Self::Variant1
        //     }
        // }

        // impl SMT for TestEnum {
        //     fn _cmp(self, rhs: Self) -> std::cmp::Ordering {
        //         match (self, rhs) {
        //             (TestEnum::Variant1, TestEnum::Variant2(..)) => {
        //                 return std::cmp::Ordering::Less;
        //             }
        //             (TestEnum::Variant1, TestEnum::Variant3 { .. }) => {
        //                 return std::cmp::Ordering::Less;
        //             }
        //             (TestEnum::Variant1, TestEnum::Variant1) => (),
        //             (TestEnum::Variant2(..), TestEnum::Variant1) => {
        //                 return std::cmp::Ordering::Greater;
        //             }
        //             (TestEnum::Variant2(..), TestEnum::Variant3 { .. }) => {
        //                 return std::cmp::Ordering::Less;
        //             }
        //             (TestEnum::Variant2(l0, l1), TestEnum::Variant2(r0, r1)) => {
        //                 match l0._cmp(r0) {
        //                     std::cmp::Ordering::Less => {
        //                         return std::cmp::Ordering::Less;
        //                     }
        //                     std::cmp::Ordering::Equal => (),
        //                     std::cmp::Ordering::Greater => {
        //                         return std::cmp::Ordering::Greater;
        //                     }
        //                 }

        //                 match l1._cmp(r1) {
        //                     std::cmp::Ordering::Less => {
        //                         return std::cmp::Ordering::Less;
        //                     }
        //                     std::cmp::Ordering::Equal => (),
        //                     std::cmp::Ordering::Greater => {
        //                         return std::cmp::Ordering::Greater;
        //                     }
        //                 }
        //             }
        //             (TestEnum::Variant3 { .. }, TestEnum::Variant1) => {
        //                 return std::cmp::Ordering::Greater;
        //             }
        //             (TestEnum::Variant3 { .. }, TestEnum::Variant2(..)) => {
        //                 return std::cmp::Ordering::Greater;
        //             }
        //             (TestEnum::Variant3 { a: l_a, b: l_b }, TestEnum::Variant3 { a: r_a, b: r_b }) => {
        //                 match l_a._cmp(r_a) {
        //                     std::cmp::Ordering::Less => {
        //                         return std::cmp::Ordering::Less;
        //                     }
        //                     std::cmp::Ordering::Equal => (),
        //                     std::cmp::Ordering::Greater => {
        //                         return std::cmp::Ordering::Greater;
        //                     }
        //                 }

        //                 match l_b._cmp(r_b) {
        //                     std::cmp::Ordering::Less => {
        //                         return std::cmp::Ordering::Less;
        //                     }
        //                     std::cmp::Ordering::Equal => (),
        //                     std::cmp::Ordering::Greater => {
        //                         return std::cmp::Ordering::Greater;
        //                     }
        //                 }
        //             }
        //         }
        //         std::cmp::Ordering::Equal
        //     }
        // }
        assert!(generated_code.contains("impl"));
        assert!(generated_code.contains("SMT for TestEnum"));
    }

    #[test]
    // if !attr.is_empty() {
    //     bail_on!(attr, "unexpected attributes provided");
    // }
    fn test_derive_for_type_attr() {
        let attr: TokenStream = parse_quote! { #[derive(Debug)] };
        let item: TokenStream = parse_quote! { struct MyStruct; };
        // converting from TokenStream to Syntax can be done by using the `into` method
        let res = derive_for_type(attr, item);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "unexpected attributes provided"
        );
    }

    #[test]
    // let mut original = syn::parse2::<Item>(item)?;
    // procedural macro API is used outside of a procedural macro
    fn test_derive_for_type_parse() {
        let attr: TokenStream = parse_quote! {};
        // item is not of type Item
        let item: TokenStream = parse_quote! { i32 };
        let res = derive_for_type(attr, item);
        assert!(res.is_err());
    }

    #[test]
    // t => bail_on!(t, "expect a type definition (i.e., struct or enum)") // smt_type only supports struct and enum
    fn test_derive_for_type_struct_or_enum() {
        let attr: TokenStream = parse_quote! {};
        let item: TokenStream = parse_quote! {
            fn test() {
                println!("Hello, world!");
            }
        };
        let res = derive_for_type(attr, item);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "expect a type definition (i.e., struct or enum) - smt_type only supports struct and enum"
        );
    }

    #[test]
    // Item::Enum(item_enum) => derive_for_enum(item_enum)?,
    fn test_derive_for_type_enum() {
        let attr: TokenStream = parse_quote! {};
        let item: TokenStream = parse_quote! {
            enum MyEnum {
                Variant1,
                Variant2(i32, u64),
                Variant3 { a: String, b: f64 },
            }
        };
        let res = derive_for_type(attr, item);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        assert!(generated_code.contains("impl SMT for MyEnum"));
    }

    #[test]
    // Item::Struct(item_struct) => derive_for_struct(item_struct)?,
    fn test_derive_for_type_struct() {
        let attr: TokenStream = parse_quote! {};
        let item: TokenStream = parse_quote! {
            struct MyStruct<T:SMT> {
                a: T,
                b: u64,
            }
        };
        let res = derive_for_type(attr, item);
        assert!(res.is_ok());
        let tokens = res.unwrap();
        let generated_code = tokens.to_string();
        assert!(generated_code.contains("impl < T : SMT > SMT for MyStruct < T >"));
    }
}

//! This module defines the `ReservedIdent` trait. It provides functionality to parse and validate reserved identifiers, including type names, function names, and macro names.

use crate::parser::dsl::SysMacroName;
use crate::parser::func::CastFuncName;
use crate::parser::func::ReservedFuncName;
use crate::parser::func::SysFuncName;
use crate::parser::generics::SysTrait;
use crate::parser::ty::SysTypeName;
use crate::{bail_if_exists, bail_on};
use std::fmt::{Display, Formatter};
use syn::{Ident, Pat, PatIdent, Path, Result};

/// Trait to mark that a type is a reserved identifier.
/// This trait provides methods to parse reserved identifiers from strings or paths.
/// SysMacroName (dsl), CastFuncName, SysFuncName, ReservedFuncName (func), SysTrait (generics), and SysTypeName (ty) implement this trait.
/// SysMacroName: //* Exists, Forall, Choose
/// CastFuncName: //* From, Into
/// SysFuncName: //* Eq, Ne
/// ReservedFuncName: //* Clone, Default
/// SysTrait: //* SMT
/// SysTypeName: //* Boolean, Integer, Real, F32, F64, I32, I64, U32, U64, String, Cloak, Seq, Set, Array, Error
/// Sized is a supertrait of ReservedIdent, meaning that all types implementing ReservedIdent must also implement Sized.
/// - ? This trait is not object safe. It cannot be used as a trait object because it is not DST (Dynamically Sized Type). For example, Box<dyn ReservedIdent> is not allowed. https://doc.rust-lang.org/std/marker/trait.Sized.html
pub trait ReservedIdent: Sized {
    /// Tries to parse a reserved identifier from a string.
    ///
    /// # Arguments
    ///
    /// * `ident` - A string slice representing the identifier.
    ///
    /// # Returns
    ///
    /// * `Option<Self>` - Some if the string matches a reserved identifier, None otherwise.
    fn from_str(ident: &str) -> Option<Self>;

    /// Expects a reserved identifier from a path.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `syn::Path` from which to parse the identifier.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Ok if the path contains a reserved identifier, Err otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not contain a reserved identifier.
    /// The path must not have a leading colon, the number of path segments must be 1, and the first path segment must not have angle bracketed or parenthesized; otherwise an error is returned.
    fn parse_path(path: &Path) -> Result<Self> {
        // Parse the identifier from the path.
        let ident = path
            .get_ident()
            .expect(stringify!("The path <{}> is not an identifier", path));
        // Try to convert the identifier to the reserved type.
        match Self::from_str(ident.to_string().as_str()) {
            None => bail_on!(ident, "not a reserved identifier"),
            Some(v) => Ok(v),
        }
    }
}

/// Tests whether an identifier is a reserved keyword.
/// Returns the name if it is a valid user identifier, or an error if it's reserved.
///
/// # Arguments
///
/// * `ident` - A reference to a `syn::Ident` to validate.
///
/// # Returns
///
/// * `Result<String>` - Ok containing the identifier name if valid, Err if reserved.
///
/// # Errors
///
/// Returns an error if the identifier is a reserved keyword or not allowed.
fn validate_user_ident(ident: &Ident) -> Result<String> {
    let name: String = ident.to_string();

    // Check for reserved trait names.
    if SysTrait::from_str(&name).is_some() {
        bail_on!(ident, "reserved trait name (SMT)");
    }
    // Check for reserved type names.
    if SysTypeName::from_str(&name).is_some() {
        bail_on!(
            ident,
            "reserved type name (Boolean, Integer, Real, F32, F64, I32, I64, U32, U64, String, Cloak, Seq, Set, Array, Error)"
        );
    }
    // Check for reserved function names.
    if SysFuncName::from_str(&name).is_some() {
        bail_on!(ident, "reserved method name (eq, ne)");
    }
    if ReservedFuncName::from_str(&name).is_some() {
        bail_on!(ident, "reserved function name (clone, default)");
    }
    if CastFuncName::from_str(&name).is_some() {
        bail_on!(ident, "reserved function name (from, into)");
    }
    // Check for reserved macro names.
    if SysMacroName::from_str(&name).is_some() {
        bail_on!(ident, "reserved macro name (exists, forall, choose)");
    }
    // identifier cannot be an underscore
    if name.as_str() == "_" {
        bail_on!(ident, "underscore not allowed as an identifier");
    }

    // Return the valid user identifier name.
    Ok(name)
}

/// Parses a plain identifier from a pattern.
///
/// # Arguments
///
/// * `pat` - A reference to a `syn::Pat` from which to parse the identifier.
///
/// # Returns
///
/// * `Result<&Ident>` - Ok containing a reference to the identifier if successful.
///
/// # Errors
///
/// Returns an error if the pattern is not an identifier, or if it includes
/// references, mutability, or sub-patterns.
/// let pat: Pat = parse_quote! { ref mut my_var @ Some(sub_var)};
/// by_ref and mutability are Some in the above pattern
/// ref in the left side is the same as & in the right side
/// ident is my_var and subpat is @ Some(sub_var) in the above pattern
/// the subpat field of PatIdent is a tuple with two elements, the first element is the @ symbol and the second element is the sub-pattern. when we write - if let a @ Some(b) = Some(1) {/* */}, a is the identifier with value Some(1) and b is the sub-pattern with value 1.
fn parse_ident_from_pat(pat: &Pat) -> Result<&Ident> {
    match pat {
        Pat::Ident(decl) => {
            let PatIdent {
                attrs: _,
                by_ref,
                mutability,
                ident,
                subpat,
            } = decl;

            // Only allow plain names without references, mutability, or sub-patterns.
            bail_if_exists!(by_ref);
            bail_if_exists!(mutability);
            bail_if_exists!(subpat.as_ref().map(|(_, sub)| sub));

            // Return the identifier.
            Ok(ident)
        }
        // If it's not an identifier pattern, return an error.
        _ => bail_on!(pat, "not an identifier pattern"),
    }
}

/// Utility macro to define a struct with common implementations.
/// It generates a struct with a single `ident` field and implements various traits:
/// `Ord`, `PartialOrd`, `Eq`, `PartialEq`, `Clone`, and `Debug`.
/// The struct also implements `AsRef<str>` to allow borrowing the identifier as a string slice.
/// It implements `Display` to format the identifier for display purposes.
/// It provides `TryFrom` implementations for `syn::Ident`, `syn::Path`, and `syn::Pat`.
/// The `TryFrom` implementations validate the identifier, path, or pattern and return an error if invalid.
/// The macro supports one pattern: with meta attributes, name, and an optional child type.
macro_rules! name {
    // Match the pattern with meta attributes, name, and an optional child type.
    ($(#[$meta:meta])* $name:ident $(> $child:ty)?) => {
        $(#[$meta])*
        #[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Debug)]
        pub struct $name {
            pub ident: String,
        }

        /// AsRef is to be used when wishing to convert to a reference of another type
        impl AsRef<str> for $name {
            /// Allows borrowing the identifier as a string slice.
            fn as_ref(&self) -> &str {
                &self.ident
            }
        }

        impl Display for $name {
            /// Formats the identifier for display purposes.
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.ident)
            }
        }

        impl TryFrom<&Ident> for $name {
            type Error = syn::Error;

            /// Attempts to create an instance from a `syn::Ident`.
            /// Validates that the identifier is not reserved.
            ///
            /// # Arguments
            ///
            /// * `value` - A reference to a `syn::Ident`.
            ///
            /// # Returns
            ///
            /// * `Result<Self>` - Ok containing the new instance if successful.
            ///
            /// # Errors
            ///
            /// Returns an error if the identifier is reserved or an underscore.
            fn try_from(value: &Ident) -> Result<Self> {
                validate_user_ident(value).map(|ident| Self { ident })
            }
        }

        impl TryFrom<&Path> for $name {
            type Error = syn::Error;

            /// Attempts to create an instance from a `syn::Path`.
            /// Validates the path, then extracts the identifier, and validates it.
            ///
            /// # Arguments
            ///
            /// * `value` - A reference to a `syn::Path`.
            ///
            /// # Returns
            ///
            /// * `Result<Self>` - Ok containing the new instance if successful.
            ///
            /// # Errors
            ///
            /// Returns an error if the path has leading colons, multiple segments, or unexpected arguments.
            /// Otherwise, an identifier is extracted and validated.
            /// If the identifier is reserved or an underscore, an error is returned.
            fn try_from(value: &Path) -> Result<Self> {
                value.get_ident().expect(stringify!("The path <{}> is not an identifier", value)).try_into()
            }
        }

        impl TryFrom<&Pat> for $name {
            type Error = syn::Error;

            /// Attempts to create an instance from a `syn::Pat`.
            /// Validates the Pat, then extracts the identifier and validates it.
            ///
            /// # Arguments
            ///
            /// * `value` - A reference to a `syn::Pat`.
            ///
            /// # Returns
            ///
            /// * `Result<Self>` - Ok containing the new instance if successful.
            ///
            /// # Errors
            ///
            /// Returns an error if the pattern is not an identifier or includes references, mutability, or sub-patterns.
            /// Otherwise, an identifier is extracted and validated.
            /// If the identifier is reserved or an underscore, an error is returned.
            fn try_from(value: &Pat) -> Result<Self> {
                parse_ident_from_pat(value)?.try_into()
            }
        }

        $(impl From<$child> for $name {
            /// Converts from the child type into the name struct.
            ///
            /// # Arguments
            ///
            /// * `name` - An instance of the child type.
            ///
            /// # Returns
            ///
            /// * `Self` - A new instance of the name struct.
            fn from(name: $child) -> Self {
                Self::from(&name)
            }
        }

        impl From<&$child> for $name {
            /// Converts from a reference to the child type into the name struct.
            ///
            /// # Arguments
            ///
            /// * `name` - A reference to an instance of the child type.
            ///
            /// # Returns
            ///
            /// * `Self` - A new instance of the name struct.
            fn from(name: &$child) -> Self {
                Self {
                    ident: name.to_string(), // the to_string method is automatically implemented for any type that implements the Display trait.
                }
            }
        })?
    };
}

// Define UsrTypeName using the name! macro (this is used to store user-defined types marked with smt_type attribute).
name! {
    /// Identifier for a user-defined type (i.e., structs and enums).
    /// A user-defined type name is converted to a user sort name in the ir (intermediate representation).
    UsrTypeName
        > crate::ir::name::UsrSortName
}

// Define UsrFuncName using the name! macro (this is used to store user-defined functions marked with smt_fn attribute).
name! {
    /// Identifier for a user-defined function (i.e., non-reserved).
    /// A user-defined function name is converted to a user function name in the ir (intermediate representation).
    UsrFuncName
        > crate::ir::name::UsrFunName
}

// Define TypeParamName using the name! macro.
// we can use `from` to convert from SmtSortName to TypeParamName
// for example, let a = SmtSortName { ident: "SortName".to_string() };
// let b: TypeParamName = TypeParamName::from(a);
// or let b = TypeParamName::from(&a);
name! {
    /// Identifier for a type parameter.
    /// An type parameter is converted to a smt sort name in the ir (intermediate representation).
    TypeParamName
        > crate::ir::name::SmtSortName
}

// Define VarName using the name! macro.
name! {
    /// Identifier for a variable.
    /// A variable name is converted to a symbol in the ir (intermediate representation).
    VarName
        > crate::ir::name::Symbol
}

impl TypeParamName {
    /// Creates a type parameter name for intrinsic usage.
    ///
    /// Only allows specific names ("T", "K", "V"); panics otherwise.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice representing the intrinsic type parameter name.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `TypeParamName`.
    ///
    /// # Panics
    ///
    /// Panics if the provided name is not one of the allowed intrinsic type parameters.
    pub fn intrinsic(name: &str) -> Self {
        match name {
            "T" | "K" | "V" => Self {
                ident: name.to_string(),
            },
            // All other names are invalid.
            _ => panic!("not an intrinsic type parameter: {name}"),
        }
    }
}

impl UsrFuncName {
    /// Creates a function name for intrinsic functions.
    ///
    /// Only allows specific function names; panics otherwise.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice representing the intrinsic function name.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `UsrFuncName`.
    ///
    /// # Panics
    ///
    /// Panics if the provided name is not a recognized intrinsic function.
    /// This function is used in the `builtin` function of the `apply` module to create intrinsic functions.
    // do not warn unreached patterns because we want to explicitly list all intrinsic function names.
    #[allow(unreachable_patterns)]
    pub fn intrinsic(name: &str) -> Self {
        match name {
            // Logical operators.
            "not" | "and" | "or" | "xor" | "nand" | "nor" | "xnor" | "implies" | "iff" | "ite"
            // bit vectors operations.
            | "bv_not" | "bv_redand" | "bv_redor" | "bv_and" | "bv_or" | "bv_xor" | "bv_nand" | "bv_nor" | "bv_xnor" 
            | "bv_neg" | "bv_add" | "bv_sub" | "bv_mul" | "bv_div" | "bv_rem" | "bv_mod" 
            | "bv_shl" | "bv_lshr" | "bv_ashr" | "bv_rotate_left" | "bv_rotate_right" 
            | "bv_lt" | "bv_le" | "bv_gt" | "bv_ge" | "to_int"
            // float 
            | "add" | "sub" | "mul" | "div" | "neg" | "abs" | "rem" | "sqrt" | "min" | "max" | "is_nan" | "is_infinite" | "is_zero" | "is_normal" | "is_subnormal" | "is_negative" | "is_positive" | "lt" | "le" | "gt" | "ge" | "nan" | "infinity" | "neg_infinity" | "pos_zero" | "neg_zero" | "to_integer" | "to_real" | "to_u32" | "to_i32" | "to_i64" | "to_u64" | "ceil" | "floor" | "trunc" | "nearest" | "fp_eq" | "from_hex_str"
            // Integer operations.
            | "add" | "mul" | "sub" | "neg" | "div" | "div_trunc" | "modulo" | "rem" | "pow" | "abs" | "divides" | "lt" | "le" | "gt" | "ge" | "to_real" | "to_i32" | "to_i64" | "to_u32" | "to_u64" | "to_f32" | "to_f64" | "from_hex_str" | "from_oct_str" | "from_bin_str" | "is_gt_i64_max" | "is_lt_i64_min" | "is_gt_u64_max" | "is_lt_u64_min" | "is_lt_i32_min" | "is_gt_i32_max" | "is_lt_u32_min" | "is_gt_u32_max"
            // Real operations.
            | "add" | "mul" | "sub" | "neg" | "div" | "pow" | "abs" | "round" | "floor" | "ceil" | "is_integer" | "lt" | "le" | "gt" | "ge" | "to_int" | "to_f32" | "to_f64"
            // String operations.
            | "new" | "length" | "concat" | "at" | "index_of" | "index_of_default" | "substr" | "contains" | "starts_with" | "ends_with" | "to_int" | "from_int" | "le" | "lt" | "ge" | "gt" | "is_digit" | "from_code" | "to_code" | "is_empty" | "replace" | "replace_all" 
            // Error handling.
            | "fresh" | "merge"
            // Cloak operations.
            | "shield" | "reveal"
            // Sequence operations.
            | "new" | "length" | "unit" | "append" | "concat" | "at" | "at_seq" | "extract" | "index_of" | "index_of_default" | "contains" | "prefix_of" | "suffix_of"  | "is_empty" | "replace"
            // Set operations.
            | "new" | "length" | "insert" | "remove" | "contains" | "is_empty" | "intersection" | "union" | "difference" | "is_subset" | "has_size" | "is_disjoint" | "symmetric_difference" | "is_proper_subset"
            // Array operations.
            | "new" | "store" | "contains_key" | "select" | "del" | "length"  | "is_empty"
            // All the above are valid intrinsic function names.
            => Self { ident: name.to_string() },
            // All other names are invalid.
            _ => panic!("not an intrinsic function: {name}"),
        }
    }
}

impl UsrTypeName {
    /// Creates a sort name for intrinsic types.
    ///
    /// Only allows specific type names; panics otherwise.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice representing the intrinsic type name.
    ///
    /// # Returns
    ///
    /// * `Self` - A new instance of `UsrTypeName`.
    ///
    /// # Panics
    ///
    /// Panics if the provided name is not a recognized intrinsic type.
    /// This function is used in the `builtin` function of the `apply` module to create intrinsic types.
    pub fn intrinsic(name: &str) -> Self {
        match name {
            // Logical types.
            "Boolean"
            // Numeric types.
            | "Integer" | "Real" | "F32" | "F64" | "I32" | "I64" | "U32" | "U64"
            // String type.
            | "String"
            // Cloak type.
            | "Cloak"
            // Collection types.
            | "Seq" | "Set" | "Array"
            // Error type.
            | "Error"
            // All the above are valid intrinsic type names.
            => Self { ident: name.to_string() },
            // All other names are invalid.
            _ => panic!("not an intrinsic type: {name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use syn::Token;
    use syn::parse_quote;

    #[test]
    /// Tests the parse_path function of the ReservedIdent trait.
    fn test_parse_path_ok_sys_type_name() {
        let ident = parse_quote! { Boolean };
        let res = SysTypeName::parse_path(&ident);

        assert!(res.is_ok_and(|v| v == SysTypeName::Boolean));
    }

    #[test]
    /// Tests the parse_path function of the ReservedIdent trait.
    fn test_parse_path_err() {
        let ident = parse_quote! { SMT };
        let res = SysTypeName::parse_path(&ident);

        assert!(res.is_err());
    }

    #[test]
    /// Tests the parse_path function of the ReservedIdent trait.
    fn test_parse_path_ok_sys_trait() {
        let ident = parse_quote! { SMT };
        let res = SysTrait::parse_path(&ident);

        assert!(res.is_ok_and(|v| v == SysTrait::SMT));
    }

    #[test]
    /// Tests the validate_user_ident function for being a SysTrait reserved name.
    fn test_validate_user_ident_err_sys_trait() {
        let ident = parse_quote! { SMT };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved trait name (SMT)")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being a SysTypeName reserved name.
    fn test_validate_user_ident_err_sys_type_name() {
        let ident = parse_quote! { Boolean };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved type name")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being a SysFuncName reserved name.
    fn test_validate_user_ident_err_sys_func_name() {
        let ident = parse_quote! { eq };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved method name (eq, ne)")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being a ReservedFuncName reserved name.
    fn test_validate_user_ident_err_reserved_func_name() {
        let ident = parse_quote! { clone };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved function name (clone, default)")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being a CastFuncName reserved name.
    fn test_validate_user_ident_err_cast_func_name() {
        let ident = parse_quote! { from };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved function name (from, into)")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being a SysMacroName reserved name.
    fn test_validate_user_ident_err_sys_macro_name() {
        let ident = parse_quote! { exists };
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("reserved macro name (exists, forall, choose)")
        );
    }

    #[test]
    /// Tests the validate_user_ident function for being an underscore.
    fn test_validate_user_ident_err_underscore() {
        let ident = Ident::new("_", Span::call_site());
        let res = validate_user_ident(&ident);

        assert!(res.is_err());
        assert!(
            res.err()
                .unwrap()
                .to_string()
                .contains("underscore not allowed as an identifier")
        );
    }

    #[test]
    /// Tests the validate_user_ident function.
    fn test_validate_user_ident_ok() {
        let ident = parse_quote! { my_var };
        let res = validate_user_ident(&ident);

        assert!(res.is_ok_and(|v| v == "my_var"));
    }

    #[test]
    /// Tests parse_ident_from_pat - _ => bail_on!(pat, "not an identifier pattern")
    fn test_parse_ident_from_pat_not_an_ident() {
        let cons = parse_quote!(
            const {
                let var = 10;
            }
        );
        let pat = Pat::Const(cons);

        let res = parse_ident_from_pat(&pat);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "not an identifier pattern\nconst { let var = 10 ; }"
        );
    }

    #[test]
    /// Tests parse_ident_from_pat - bail_if_exists!(by_ref);
    fn test_parse_ident_from_pat_by_ref() {
        let pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: Some(Token![ref](Span::call_site())),
            mutability: None,
            ident: parse_quote!(Boolean),
            subpat: None,
        });

        let res = parse_ident_from_pat(&pat);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "unexpected\nref");
    }

    #[test]
    /// Tests parse_ident_from_pat - bail_if_exists!(mutability);
    fn test_parse_ident_from_pat_mut() {
        let pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: Some(Token![mut](Span::call_site())),
            ident: parse_quote!(Boolean),
            subpat: None,
        });

        let res = parse_ident_from_pat(&pat);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "unexpected\nmut");
    }

    #[test]
    /// Tests parse_ident_from_pat - bail_if_exists!(subpat.as_ref().map(|(at, _)| at));
    fn test_parse_ident_from_pat_sub_pattern() {
        let pat = parse_quote!(a @ Some(b));

        let res = parse_ident_from_pat(&pat);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string(), "unexpected\nSome (b)");
    }

    #[test]
    /// Tests parse_ident_from_pat happy flow
    fn test_parse_ident_from_pat_ok() {
        let pat = parse_quote!(my_var);

        let res = parse_ident_from_pat(&pat);
        assert!(res.is_ok());
    }

    #[test]
    /// Tests the name! macro methods for TypeParamName - as_ref
    fn test_name_macro_type_param_name_as_ref() {
        let var = TypeParamName {
            ident: String::from("example var"),
        };

        let res = var.as_ref();
        assert_eq!(res, "example var");
    }

    #[test]
    /// Tests the name! macro methods for TypeParamName - Display
    fn test_name_macro_type_param_name_display() {
        let var = TypeParamName {
            ident: String::from("example var"),
        };

        let res = format!("{var}");
        assert_eq!(res, "example var");
    }

    #[test]
    /// Tests the name! macro methods for TypeParamName - TryFrom<&Ident>
    fn test_name_macro_type_param_name_try_from_ident() {
        let ident: Ident = parse_quote!(example_var);
        // let res: TypeParamName = ident.try_into(); // this is the same as the next line
        let res = TypeParamName::try_from(&ident);

        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            TypeParamName {
                ident: "example_var".to_string()
            }
        );
    }

    #[test]
    /// Tests the name! macro methods for TypeParamName - TryFrom<&Path>
    fn test_name_macro_type_param_name_try_from_path() {
        let path: Path = parse_quote!(std);
        // let res: TypeParamName = path.try_into(); // this is the same as the next line
        let res = TypeParamName::try_from(&path);

        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            TypeParamName {
                ident: "std".to_string()
            }
        );
    }

    #[test]
    /// Tests the name! macro methods for TypeParamName - TryFrom<&Pat>
    fn test_name_macro_type_param_name_try_from_pat() {
        let pat: Pat = parse_quote!(my_var);
        // let res: TypeParamName = pat.try_into(); // this is the same as the next line
        let res = TypeParamName::try_from(&pat);

        assert!(res.is_ok());
        assert_eq!(
            res.unwrap(),
            TypeParamName {
                ident: "my_var".to_string()
            }
        );
    }
    use crate::ir::name::SmtSortName;
    #[test]
    /// Tests the name! macro methods for TypeParamName - impl From<$child> for $name
    /// > crate::ir::name::SmtSortName
    fn test_name_macro_type_param_name_smt_sort_name() {
        let smt_sort_name = SmtSortName::new_func_param(
            &UsrFuncName {
                ident: "user func name".to_string(),
            },
            &TypeParamName {
                ident: "type param name".to_string(),
            },
        );
        let var1 = TypeParamName::from(smt_sort_name.clone());
        let var2 = TypeParamName::from(&smt_sort_name);

        let res = TypeParamName {
            ident: format!("{}_{}", "user func name", "type param name"),
        };

        assert_eq!(var1, res);
        assert_eq!(var2, res);
    }

    #[test]
    #[should_panic(expected = "not an intrinsic type parameter: A")]
    /// Test the intrinsic method of TypeParamName -  _ => panic!("not an intrinsic type parameter: {}", name),
    fn test_type_param_name_intrinsic_err() {
        let _ = TypeParamName::intrinsic("A");
    }

    #[test]
    /// Test the intrinsic method of TypeParamName - match name { "T" | "K" | "V" => Self { ident: name.to_string() }
    fn test_type_param_name_intrinsic_ok() {
        let var = TypeParamName::intrinsic("T");

        assert_eq!(
            var,
            TypeParamName {
                ident: "T".to_string()
            }
        );
    }

    #[test]
    #[should_panic(expected = "not an intrinsic function: thunk")]
    /// Test the intrinsic method of UsrFuncName - _ => panic!("not an intrinsic function: {}", name),
    fn test_usr_func_name_intrinsic_err() {
        let _ = UsrFuncName::intrinsic("thunk");
    }

    #[test]
    /// Test the intrinsic method of UsrFuncName happy flow
    fn test_usr_func_name_intrinsic_ok() {
        let var = UsrFuncName::intrinsic("not");

        assert_eq!(
            var,
            UsrFuncName {
                ident: "not".to_string()
            }
        );
    }
}

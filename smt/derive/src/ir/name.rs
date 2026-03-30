//! Intermediate representation (IR) names.

/// Utility macro to define a name
macro_rules! name {
    ($(#[$meta:meta])* $name:ident $(: $parent:ty)?) => {
        $(#[$meta])*
        #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
        pub struct $name {
            ident: String,
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.ident
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.ident)
            }
        }

        $(impl From<$parent> for $name {
            fn from(name: $parent) -> Self {
                Self {
                    ident: name.to_string(),
                }
            }
        }

        impl From<&$parent> for $name {
            fn from(name: &$parent) -> Self {
                Self {
                    ident: name.to_string(),
                }
            }
        })?
    };
}

// #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
// pub struct SmtSortName {
//     ident: String,
// }
// if we have let a = SmtSortName { ident: "a".to_string() };
// let b = a.as_ref(); then b is "a" and is of type &str
// println!("{}", a); will print "a"
name! {
    /// Name of a type parameter that implements the SMT trait
    SmtSortName
        : crate::parser::name::TypeParamName
}

impl SmtSortName {
    /// Create an uninterpreted sort for function param
    /// For function `func` and type parameter `param`, the uninterpreted sort is `func_param`
    pub fn new_func_param(
        func: &crate::parser::name::UsrFuncName,
        param: &crate::parser::name::TypeParamName,
    ) -> Self {
        Self {
            ident: format!("{func}_{param}"),
        }
    }

    /// Create an uninterpreted sort for type param
    /// For type `ty` and type parameter `param`, the uninterpreted sort is `ty_param`
    pub fn new_type_param(
        ty: &crate::parser::name::UsrTypeName,
        param: &crate::parser::name::TypeParamName,
    ) -> Self {
        Self {
            ident: format!("{ty}_{param}"),
        }
    }
}

// let a = UsrTypeName { ident: "a".to_string() };
// let b = UsrSortName::from(&a); or let b = UsrSortName::from(a);
// b is of type UsrSortName and b.ident is "a"
name! {
    /// Name of a user-defined type
    UsrSortName
        : crate::parser::name::UsrTypeName
}

name! {
    /// Name of a user-defined function
    UsrFunName
        : crate::parser::name::UsrFuncName
}

name! {
    /// Name of a variable
    Symbol
        : crate::parser::name::VarName
}

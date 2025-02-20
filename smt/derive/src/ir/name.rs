/// Utility macro to define a name
macro_rules! name {
    ($(#[$meta:meta])* $name:ident $(: $parent:ty)?) => {
        $(#[$meta])*
        #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
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

// ONLY for type parameters, user types, user functions (impls and specs), axioms, and variables
// equivalent to:
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
}

impl SmtSortName {
    /// Create an uninterpreted sort for function param
    /// For function `func` and type parameter `param`, the uninterpreted sort is `func_param`
    pub fn new_func_param(
        func: &crate::parser::name::UsrFuncName,
        param: &crate::parser::name::TypeParamName,
    ) -> Self {
        Self {
            ident: format!("{}_{}", func, param),
        }
    }

    /// Create an uninterpreted sort for axiom param
    /// For function `axiom` and type parameter `param`, the uninterpreted sort is `axiom_param`
    pub fn new_axiom_param(
        axiom: &crate::parser::name::AxiomName,
        param: &crate::parser::name::TypeParamName,
    ) -> Self {
        Self {
            ident: format!("{}_{}", axiom, param),
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

name! {
    /// Name of an axiom (user defined function marked with #[smt_axiom])
    UsrAxiomName
        : crate::parser::name::AxiomName
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::name::UsrTypeName;

    #[test]
    fn test_into_from() {
        let a = UsrTypeName {
            ident : String::from("hello")
        };

        let b = UsrSortName::from(a.clone());
        let c : UsrSortName = a.clone().into();
        assert_eq!(b,c);

        let d = b.into();
        let e = UsrTypeName::from(c);
        assert_eq!(a,d);
        assert_eq!(d,e)
    }    
}
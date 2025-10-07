/// Utility macro to define an index
macro_rules! index {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
        pub struct $name {
            pub index: usize,
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.index)
            }
        }
    };
}

// this is equivalent to the following code:
// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
// pub struct UsrSortId {
//     pub index: usize,
// }
// let a = UsrSortId { index: 0 };
// println!("{}", a); // 0
index! {
    /// A unique identifier for user-defined type
    UsrSortId
}

index! {
    /// A unique identifier for user-defined function
    UsrFunId
}

index! {
    /// Index of a variable
    VarId
}

index! {
    /// Index of an expression
    ExpId
}

//! Standard library for SMT types in Rusmart.
//!
//! SMT Types:
//! * `Boolean` - SMT Bool
//! * `Integer` - SMT Int
//! * `Rational` - SMT Real
//! * `Text` - SMT String
//! * `Seq<T>` - SMT Seq
//! * `Set<T>` - SMT Set(Array<T,Bool>)
//! * `Map<K,V>` - SMT Array<K,V>

use crate::smt::SMT;
use crate::smt_impl;
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use paste::paste;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::hash::Hash;

/// ** SMT Bool
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Boolean {
    inner: bool,
}
/// ** SMT Float: A wrapper around the Rust BigRational type
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Rational {
    inner: Intern<BigRational>,
}
/// ** SMT Int: A wrapper around the Rust BigInt type
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Integer {
    inner: Intern<BigInt>,
}
/// ** SMT String
/// The String inside the interns are compared in a lexicographical order when calling the cmp method.
/// For example, "a" < "b" and "aa" < "ab" and "a" < "aa" etc.
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Text {
    inner: Intern<String>,
}
/// ** SMT Seq
/// This is a sequence (list) of SMT values of type T where T is a type that implements the SMT trait.
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Seq<T: SMT> {
    inner: Intern<Vec<SMTWrap<T>>>,
}
/// ** SMT Set (Array<T,Bool>)
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Set<T: SMT> {
    inner: Intern<BTreeSet<SMTWrap<T>>>,
}
/// ** SMT Array
/// This is an array of key type K and value type V where K and V are types that implement the SMT trait.
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Map<K: SMT, V: SMT> {
    inner: Intern<BTreeMap<SMTWrap<K>, SMTWrap<V>>>,
}
/// ** Error state
/// The error state is created by calling the Error::fresh() function.
/// Every time the fresh() method is called, a new error state is created with a unique inner value.
/// The inner values are incremented by one each time a new error state is created.
/// The merge method is used to merge two error states where duplicates are not allowed.
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Error {
    inner: Intern<BTreeSet<usize>>,
}
/// ** `Cloak` is used to prevent cyclic dependencies in Abstract Data Types (ADTs).
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Cloak<T: SMT> {
    inner: Intern<SMTWrap<T>>,
}

pub mod boolean;
pub mod cloak;
pub mod error;
#[macro_use]
pub mod int;
pub mod map;
#[macro_use]
pub mod real;
pub mod seq;
pub mod set;
pub mod smt; // smt constructs
pub mod string;

smt_impl! { A B } // Both A and B need to implement the SMT trait.
smt_impl! { A B C }
smt_impl! { A B C D }
smt_impl! { A B C D E }
smt_impl! { A B C D E F }
smt_impl! { A B C D E F G }
smt_impl! { A B C D E F G H }
smt_impl! { A B C D E F G H I }
smt_impl! { A B C D E F G H I J }
smt_impl! { A B C D E F G H I J K }
smt_impl! { A B C D E F G H I J K L }
smt_impl!(Boolean);
smt_impl!(Integer);
smt_impl!(Rational);
smt_impl!(Text);
smt_impl!(Seq, T);
smt_impl!(Set, T);
smt_impl!(Map, K, V);
smt_impl!(Error);
smt_impl!(Cloak, T);

/// The SMTWrap is a tuple struct that wraps a SMT type for Rust-semantics enrichment.
// In SMTWrap, instead of using #[derive(Eq)] we implement the trait manually to avoid imposing the T: Eq constraint.
// This is because T does not necessarily need to implement the Eq trait as the eq method is a method in the SMT trait.
#[derive(Debug, Clone, Copy, Default)]
struct SMTWrap<T: SMT>(T);
impl<T: SMT> PartialEq for SMTWrap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other.0).inner
    }
}
impl<T: SMT> Eq for SMTWrap<T> {}
// because we manually implement the PartialEq for SMTWrap
// we need to manually implement the Hash trait as well
// see https://rust-lang.github.io/rust-clippy/master/index.html#derived_hash_with_manual_eq
impl<T: SMT> Hash for SMTWrap<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<T: SMT> PartialOrd for SMTWrap<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: SMT> Ord for SMTWrap<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0._cmp(other.0)
    }
}

/// Arithmetic operators
#[macro_export]
macro_rules! arith_operator {
    ($l:ty, $($op: tt),*) => {
        impl $l {
            $(
                #[allow(clippy::should_implement_trait)] // surpress the warning that the trait (Add, Sub, etc.) should be implemented instead of directly implementing the operator
                pub fn $op(self, rhs: Self) -> Self {
                    Self {
                        inner: Intern::new(
                            self.inner.as_ref().$op(rhs.inner.as_ref())
                        )
                    }
                }
            )*
        }
    };
}

/// Order operators
#[macro_export]
macro_rules! order_operator {
    ($l:ty $(,$op: tt)*) => {
        impl $l {
            $(
                pub fn $op(self, rhs: Self) -> crate::Boolean {
                    self.inner.as_ref().$op(rhs.inner.as_ref()).into()
            }
            )*
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use num_traits::cast::ToPrimitive;

    #[test]
    /// This test is for checking the implementation of the the arith_operator macro on Integer types
    fn test_arith_operator_macro_integer() {
        let num1 = Integer::from(7);
        let num2 = Integer::from(2);

        let res = num1.add(num2);
        assert!(*res.eq(Integer::from(9))); // 7 + 2 = 9

        let res = num1.div(num2);
        assert!(*res.eq(Integer::from(3))); // 7 / 2 = 3

        let res = num1.mul(num2);
        assert!(*res.eq(Integer::from(14))); // 7 * 2 = 14

        let res = num1.rem(num2);
        assert!(*res.eq(Integer::from(1))); // 7 % 2 = 1

        let res = num1.sub(num2);
        assert!(*res.eq(Integer::from(5))); // 7 - 2 = 5
    }

    #[test]
    /// This test is for checking the implementation of the the arith_operator macro on Rational types
    fn test_arith_operator_macro_rational() {
        let num1 = Rational::from(7.5);
        let num2 = Rational::from(2);

        let res = num1.add(num2);
        assert!(*res.eq(Rational::from(9.5))); // 7.5 + 2 = 9.5

        let res = num1.div(num2);
        assert!(*res.eq(Rational::from(3.75))); // 7.5 / 2 = 3.75

        let res = num1.mul(num2);
        assert!(*res.eq(Rational::from(15))); // 7.5 * 2 = 15

        let res = num1.sub(num2);
        assert!(*res.eq(Rational::from(5.5))); // 7.5 - 2 = 5.5
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Boolean type
    fn test_smt_impl_macro_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        let res = var1._cmp(var2); // true > false
        assert_eq!(res, Ordering::Greater);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Integer type
    fn test_smt_impl_macro_integer() {
        let var1 = Integer::from(1);
        let var2 = Integer::from(3);

        let res = var1._cmp(var2); // 1 < 3
        assert_eq!(res, Ordering::Less);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Rational type
    fn test_smt_impl_macro_rational() {
        let var1 = Rational::from(1.75);
        let var2 = Rational::from(3);

        let res = var1._cmp(var2); // 1.75 < 3
        assert_eq!(res, Ordering::Less);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Text type
    /// This happens in a lexicographical order
    fn test_smt_impl_macro_text() {
        let var1 = Text::from("more");
        let var2 = Text::from("less");

        let res = var1._cmp(var2); // "more" > "less"
        assert_eq!(res, Ordering::Greater);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Error type
    fn test_smt_impl_macro_error() {
        let var1 = Error::fresh(); // 0
        let var2 = Error::fresh(); // 1
        let var3 = Error::fresh(); // 2

        let res1 = var1._cmp(var2); // 0 < 1
        let res2 = var2._cmp(var3); // 1 < 2
        let res3 = var1._cmp(var3); // 0 < 2

        assert_eq!(res1, Ordering::Less);
        assert_eq!(res2, Ordering::Less);
        assert_eq!(res3, Ordering::Less);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Cloak type
    fn test_smt_impl_macro_cloak() {
        let var1 = Cloak::shield(Integer::from(2));
        let var2 = Cloak::shield(Integer::from(3));

        let res = var1._cmp(var2); // 2 < 3

        assert_eq!(res, Ordering::Less);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Seq type
    /// elements are compared pairwise in order
    fn test_smt_impl_macro_seq() {
        let mut var1: Seq<Integer> = Seq::new();
        var1 = var1.append(Integer::from(1));
        var1 = var1.append(Integer::from(2));
        var1 = var1.append(Integer::from(3));
        var1 = var1.append(Integer::from(4));

        let mut var2: Seq<Integer> = Seq::new();
        var2 = var2.append(Integer::from(10));

        let res1 = var1._cmp(var2); // 1 < 10 (first element comparison)
        assert_eq!(res1, Ordering::Less);
    }

    #[test]
    /// testing the cmp method of the SMT trait on the Set type
    fn test_smt_impl_macro_set() {
        let mut var1 = Set::new();
        var1 = var1.insert(Text::from("hello"));
        var1 = var1.insert(Text::from("world"));

        let mut var2 = Set::new();
        var2 = var2.insert(Text::from("mehrad"));

        let res = var1._cmp(var2); // "hello" > "mehrad"
        assert_eq!(res, Ordering::Less);
    }

    #[test]
    /// The keys are compared in lexicographical order
    fn test_smt_impl_macro_map() {
        let mut var1 = Map::new();

        var1 = var1.put_unchecked(Integer::from(1), Text::from("one"));
        var1 = var1.put_unchecked(Integer::from(2), Text::from("two"));
        var1 = var1.put_unchecked(Integer::from(3), Text::from("three"));

        let mut var2 = Map::new();
        var2 = var2.put_unchecked(Integer::from(0), Text::from("zero"));

        let res = var1._cmp(var2); // 1 > 0
        assert_eq!(res, Ordering::Greater);
    }

    #[test]
    /// This test is for checking the implementation of the the order_operator macro on Integer types
    fn test_order_operator_macro_integer() {
        let var1 = Integer::from(1);
        let var2 = Integer::from(3);

        let res1 = var1.lt(var2); // 1 < 3
        let res2 = var1.le(var2); // 1 <= 3
        let res3 = var1.ge(var2); // 1 >= 3 (false)
        let res4 = Integer::from(3).gt(Integer::from(1).add(Integer::from(1))); // 3 > 1 + 1

        assert!(*res1);
        assert!(*res2);
        assert!(!*res3);
        assert!(*res4);
    }

    #[test]
    /// This test is for checking the implementation of the the order_operator macro on Rational types
    fn test_order_operator_macro_rational() {
        let var1 = Rational::from(1.5);
        let var2 = Rational::from(3);

        let res1 = var1.lt(var2); // 1.5 < 3
        let res2 = var1.le(var2); // 1.5 <= 3

        assert!(*res1);
        assert!(*res2);
    }

    #[test]
    /// This test is for the order_operator macro on Text types
    fn test_order_operator_macro_text() {
        let var1 = Text::from("1.5");
        let var2 = Text::from("9a");
        let var3 = Text::from("apple");

        let res1 = var1.lt(var2); // "1.5" < "9a"
        let res2 = var1.le(var2); // "1.5" <= "9a"
        let res3 = var3.gt(var2); // "apple" > "9a"

        assert!(*res1);
        assert!(*res2);
        assert!(*res3);
    }

    #[test]
    /// Testing the integer_from_literal macro which gives access to the from method for Integer types
    fn test_integer_from_literal() {
        let var1 = Integer::from(1i8);
        let var2 = Integer::from(10u8);

        assert!(var1.inner.to_u8().expect("Failed to convert BigInt to u8") == 1);
        assert!(var2.inner.to_u8().expect("Failed to convert BigInt to u8") == 10);
    }

    #[test]
    /// This test is for checking the implementation of the rational_from_literal_int macro
    fn test_rational_from_literal_int_macro() {
        let a = Rational::from(1i8);
        let b = Rational::from(1u8);
        let c = Rational::from(1i32);
        let d = Rational::from(1i64);
        let e = Rational::from(1i128);
        let f = Rational::from(1isize);
        let g = Rational::from(1f32);

        assert!(
            *a.eq(b)
                .and(b.eq(c).and(c.eq(d).and(d.eq(e).and(e.eq(f).and(f.eq(g))))))
        );
    }

    #[test]
    /// This test is for checking the implementation of the smt_impl macro for tuples
    /// The macro allows the use of the cmp method of the SMT trait on tuples
    fn test_smt_impl_macro() {
        let var1 = (Integer::from(1), Integer::from(2));
        let var2 = (Integer::from(1), Integer::from(2));
        let var3 = (Integer::from(1), Integer::from(3));
        let var4 = (Integer::from(10), Integer::from(1));

        let res1 = var1._cmp(var2); // (1, 2) == (1, 2)
        let res2 = var1._cmp(var3); // (1, 2) < (1, 3)
        let res3 = var3._cmp(var2); // (1, 3) > (1, 2)
        let res4 = var1._cmp(var4); // (1, 2) < (10, 1)
        let res5 = var4._cmp(var3); // (10, 1) > (1, 3)

        assert_eq!(res1, Ordering::Equal);
        assert_eq!(res2, Ordering::Less);
        assert_eq!(res3, Ordering::Greater);
        assert_eq!(res4, Ordering::Less);
        assert_eq!(res5, Ordering::Greater);
    }

    /// This tests the seq! macro
    #[test]
    fn test_seq_macro() {
        use crate::seq;
        let var1 = seq![Integer::from(1), Integer::from(0)];

        let mut var2 = Seq::new();
        var2 = var2.append(Integer::from(1));
        var2 = var2.append(Integer::from(0));

        assert!(*var1.eq(var2)); // [1, 0] == [1, 0]
    }

    /// This tests the set! macro
    #[test]
    fn test_set_macro() {
        use crate::set;
        let var1 = set![Integer::from(1), Integer::from(0)];

        let mut var2 = Set::new();
        var2 = var2.insert(Integer::from(0));
        var2 = var2.insert(Integer::from(1));

        assert!(*var1.eq(var2)); // {0, 1} == {0, 1}
    }

    /// This tests the map! macro
    #[test]
    fn test_map_macro() {
        use crate::map;
        let var1 = map![
            (Integer::from(1), Text::from("Value 1")),
            (Integer::from(2), Text::from("Value 2"))
        ];

        let mut var2 = Map::new();
        var2 = var2.put_unchecked(Integer::from(1), Text::from("Value 1"));
        var2 = var2.put_unchecked(Integer::from(2), Text::from("Value 2"));

        assert!(*var1.eq(var2)); // {1: "Value 1", 2: "Value 2"} == {1: "Value 1", 2: "Value 2"}
    }

    /// testing the three methods (_cmp, eq, ne) of the SMT trait
    #[test]
    fn test_smt_1() {
        let var1 = Integer::from(1);
        let var2 = Integer::from(3);

        assert!(var1._cmp(var2) == Ordering::Less);
        assert!(*Boolean::not(var1.eq(var2))); // equivalent to *var1.ne(var2) or !*var1.eq(var2)
        assert!(*var1.ne(var2));
    }

    #[test]
    fn test_smt_2() {
        let var1 = Text::from("a");
        let mut var2 = Text::from("b");

        assert!(var1._cmp(var2) == Ordering::Less);
        assert!(!*var1.eq(var2));
        assert!(*var1.ne(var2));

        var2 = var1; // after this var1 and var2 should be equal
        assert!(*var1.eq(var2));
    }
    /// testing the deref function of boolean
    #[test]
    fn test_deref_boolean() {
        let var1 = Boolean::from(true);

        assert!(*var1);
    }
    #[test]
    /// testing the from function on Boolean
    fn test_from_boolean() {
        let var1 = Boolean::from(false);

        assert!(!*var1);
    }
    /// testing the boolean operators
    #[test]
    fn test_not_boolean() {
        let mut var1 = Boolean::from(true);
        assert!(*var1);

        var1 = var1.not();
        assert!(!(*var1));
    }
    #[test]
    /// it is true only if both are true
    fn test_and_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(!(*(var1.and(var2)))); // true && false = false
        assert!(!(*(var2.and(var2)))); // false && false = false
        assert!(*(var1.and(var1))); // true && true = true
    }
    #[test]
    /// it is true if at least one is true
    fn test_or_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(*(var1.or(var2))); // true || false = true
        assert!(!(*(var2.or(var2)))); // false || false = false
        assert!(*(var1.or(var1))); // true || true = true
    }
    #[test]
    /// it is true if the two operands do not match
    fn test_xor_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(*var1.xor(var2)); // true xor false = true
        assert!(!(*var2.xor(var2))); // false xor false = false
        assert!(!(*var1.xor(var1))); // true xor true = false
    }
    #[test]
    /// a -> b is valid unless a is true and b is false
    fn test_implies_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(!(*var1.implies(var2))); // true -> false = false
        assert!(*var1.implies(var1)); // true -> true = true
        assert!(*var2.implies(var1)); // false -> true = true
        assert!(*var2.implies(var2)); // false -> false = true
    }

    /// testing the from method of Rational
    #[test]
    fn test_from_rational() {
        let var1 = Rational::from(1.5f32);
        let var2 = Rational::from(1.6f64);

        assert!(*var1.eq(Rational {
            inner: Intern::new(
                BigRational::from_float(1.5).expect("Failed to convert float to BigRational")
            )
        }));
        assert!(*var2.eq(Rational {
            inner: Intern::new(
                BigRational::from_float(1.6).expect("Failed to convert float to BigRational")
            )
        }));
    }

    /// testing the ne/eq/_cmp methods of rational
    #[test]
    fn test_cmp_rational() {
        let var1 = Rational::from(1);
        let var2 = Rational::from(243.3);

        assert_eq!(var1._cmp(var2), Ordering::Less);
    }
    #[test]
    fn test_eq_rational() {
        let var1 = Rational::from(1);
        let var2 = Rational::from(243.3);

        assert!(*var1.eq(var2).eq(false.into())); // equivalent to *var1.ne(var2)
    }
    #[test]
    fn test_ne_rational() {
        let var1 = Rational::from(1);
        let var2 = Rational::from(243.3);

        assert!(*var1.ne(var2));
    }

    #[test]
    /// testing the from method of Text
    fn test_from_text() {
        let var1 = Text::from("value");
        assert!(*var1.eq(Text {
            inner: Intern::new(String::from("value"))
        }));
    }

    #[test]
    /// testing the new method of Error
    fn test_error() {
        let var1 = Error::fresh();
        let var2 = Error::fresh();

        // each newly created error only has one element.
        assert_eq!(var1.inner.len(), 1);
        assert_eq!(var2.inner.len(), 1);

        // the first one is created with a value of zero and each new one is incremented by one.
        assert_eq!(*var1.inner.iter().next().expect("Error is empty"), 0);
        assert_eq!(*var2.inner.iter().next().expect("Error is empty"), 1);

        // in merging, the elements are included in one set, thus {0,1} is inside var3.
        let var3 = var1.merge(var2);
        assert_eq!(var3.inner.len(), 2);

        // the first element is 0 and the second element is 1.
        assert_eq!(*var3.inner.iter().next().expect("Error is empty"), 0);
        assert_eq!(
            *var3
                .inner
                .iter()
                .nth(1)
                .expect("Error does not have 2 elements"),
            1
        );

        // in merging var3 {0,1} and var1 {0} because the inner value is a set the values are not duplicated.
        let var4 = var3.merge(var1);
        assert_eq!(var4.inner.len(), 2);
    }

    #[test]
    /// testing the eq method for SMTWrap
    fn test_eq_smtwrap() {
        let var1 = SMTWrap(Integer::from(1));
        let var2 = SMTWrap(Integer::from(15));

        assert!(!var1.eq(&var2));
        assert!(var1.ne(&var2));
    }

    #[test]
    /// testing the partial_cmp for smt wrap
    fn test_partial_cmp_smtwrap() {
        let x = Cloak::shield(Integer::from(1));
        let y = Cloak::shield(Integer::from(10));
        let var1 = SMTWrap(x);
        let var2 = SMTWrap(y);

        assert_eq!(
            var2.partial_cmp(&var1).expect("Failed to compare"),
            Ordering::Greater
        );
    }

    #[test]
    /// testing the _cmp for smt wrap
    fn test_cmp_smtwrap() {
        let x = Cloak::shield(Integer::from(1));
        let y = Cloak::shield(Integer::from(10));
        let var1 = SMTWrap(x);
        let var2 = SMTWrap(y);

        assert_eq!(var2.cmp(&var1), Ordering::Greater);
    }

    #[test]
    /// Adding SMtWrap to a HashSet
    fn test_hash_smtwrap() {
        let var1 = SMTWrap(Integer::from(1));
        let var2 = SMTWrap(Integer::from(2));
        let var3 = SMTWrap(Integer::from(1));
        let var4 = SMTWrap(Integer::from(2));

        let mut set = std::collections::HashSet::new();
        set.insert(var1);
        set.insert(var2);
        set.insert(var3);
        set.insert(var4);

        assert_eq!(set.len(), 2);
    }

    #[test]
    /// This test is for checking the implementation of the shield and reveal methods on the Cloak type.
    fn test_shield_reveal_cloak() {
        let var1 = Cloak::shield(Integer::from(1));
        let var2 = var1.reveal();
        assert!(*var2.eq(1.into()));
    }

    #[test]
    /// A new sequence has an initial length of 0
    fn test_new_seq() {
        let seq: Seq<Integer> = Seq::new();
        assert!(*seq.length().eq(Integer::from(0)));
    }

    #[test]
    /// The length of the sequence is 3 after appending 3 elements (duplicate elements are allowed)
    fn test_append_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(1));
        assert!(*seq.length().eq(Integer::from(3)));
    }

    #[test]
    /// The value at index 1 is 2
    fn test_at_unchecked_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        assert!(*seq.at_unchecked(Integer::from(1)).eq(Integer::from(2)));
    }

    #[test]
    #[should_panic]
    /// This test checks that the at_unchecked method panics when the index is out of bounds
    fn test_at_unchecked_seq_out_of_bounds() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        seq.at_unchecked(Integer::from(3));
    }

    #[test]
    /// The sequence includes the value 2
    /// The sequence does not include the value 4
    fn test_includes_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        assert!(*seq.includes(Integer::from(2)));
        assert!(!*seq.includes(Integer::from(4)));
    }

    #[test]
    /// The iterator returns the values 0, 1, 2
    fn test_iterator_seq() {
        let seq = Seq::new();
        let seq = seq.append(Text::from("one"));
        let seq = seq.append(Text::from("two"));
        let seq = seq.append(Text::from("three"));
        assert!(*seq.iterator()[0].eq(Integer::from(0)));
        assert!(*seq.iterator()[1].eq(Integer::from(1)));
        assert!(*seq.iterator()[2].eq(Integer::from(2)));
    }

    #[test]
    /// This tests the new method for the Set type
    fn test_new_set() {
        let set = Set::<Integer>::new();
        assert!(*set.length().eq(0.into()));
    }

    #[test]
    /// This tests the length method for the Set type
    fn test_length_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
    }

    #[test]
    /// The set does not contain duplicates
    fn test_length_dup_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(1));
        assert!(*set.length().eq(1.into()));
    }

    #[test]
    /// After removing the element, the length of the set should decrease by 1
    fn test_remove_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
        let set = set.remove(Integer::from(1));
        assert!(*set.length().eq(1.into()));
    }

    #[test]
    /// After removing a non-existent element, the length of the set should not change
    fn test_remove_set2() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
        let set = set.remove(Integer::from(10));
        assert!(*set.length().eq(2.into()));
    }

    #[test]
    /// The set should contain the element that was inserted
    fn test_contains_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.contains(Integer::from(1)));
        assert!(!*set.contains(Integer::from(20)));
    }

    #[test]
    /// when the element is removed, the set should not contain the element anymore
    fn test_contains_set2() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.contains(Integer::from(1)));
        let set = set.remove(Integer::from(1));
        assert!(!*set.contains(Integer::from(1)));
    }

    #[test]
    /// The iterator should return the elements in the set
    fn test_iterator_set() {
        let set = Set::new();
        let set = set.insert(Text::from("one"));
        let set = set.insert(Text::from("two"));
        let set = set.insert(Text::from("three"));

        let iter = set.iterator();
        assert_eq!(iter.len(), 3);
        assert!(*iter[0].eq(Text::from("one")));
        assert!(*iter[1].eq(Text::from("three"))); // it is in lexicographical order
        assert!(*iter[2].eq(Text::from("two")));
    }

    #[test]
    /// A new map is created and the length of the map should be 0
    fn test_map_length() {
        let map: Map<Integer, Integer> = Map::new();
        assert!(*map.length().eq(0.into()));
    }

    #[test]
    /// When adding a key-value pair to the map, the length of the map should increase by 1
    fn test_map_put() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        assert!(*map.length().eq(1.into()));
    }

    #[test]
    /// When adding a key-value pair to the map, the key should exist in the map
    /// and the value should be the same as the one that was added
    fn test_map_get() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        assert!(*map.get_unchecked(Integer::from(1)).eq(Integer::from(2)));
    }

    #[test]
    /// When deleting an existent key from the map, the length of the map should decrease by 1
    fn test_map_del() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        let map = map.del_unchecked(Integer::from(1));
        assert!(*map.length().eq(0.into()));
    }

    #[test]
    /// Deleting a key that does not exist in the map should not change the map.
    fn test_map_del_non_existent() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        assert!(*map.length().eq(1.into()));
        let map = map.del_unchecked(Integer::from(2));
        assert!(*map.length().eq(1.into()));
    }

    #[test]
    #[should_panic]
    /// getting an element that does not exist in the map should panic
    fn test_map_get_non_existent() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        map.get_unchecked(Integer::from(2));
    }

    #[test]
    /// checking if a key exists in the map
    /// a key should only exist if it was added to the map
    fn test_map_contains_key() {
        let map = Map::new().put_unchecked(Integer::from(1), Integer::from(2));
        assert!(*map.contains_key(Integer::from(1)));
    }

    #[test]
    /// getting an iterator over the keys of the map
    fn test_map_iterator() {
        let map = Map::new().put_unchecked(Text::from("one"), Integer::from(1));
        assert!(*map.iterator()[0].eq(Text::from("one")));
    }

    #[test]
    /// the default value of Integer is 0
    fn test_default_integer() {
        let var1 = Integer::default();
        let var2 = Integer::from(0);
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Boolean is false
    fn test_default_boolean() {
        let var1 = Boolean::default();
        let var2 = Boolean::from(false);
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Rational is numerator 0 and denominator 1
    fn test_default_rational() {
        let var1 = Rational::default();
        let var2 = Rational::from(0);
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Text is an empty string
    fn test_default_text() {
        let var1 = Text::default();
        let var2 = Text::from("");
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Error is an empty set
    fn test_default_error() {
        let var1 = Error::default();
        let var2 = Error {
            inner: Intern::new(BTreeSet::new()),
        };
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Cloak is the default value of the inner type
    fn test_default_cloak() {
        let var1 = Cloak::default();
        let var2 = Cloak::shield(Integer::default());
        assert!(*var1.eq(var2));
    }

    #[test]
    /// the default value of Seq is an empty sequence
    fn test_default_seq() {
        let var1 = Seq::<Integer>::default();
        let var2 = Seq {
            inner: Intern::new(Vec::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }

    #[test]
    /// the default value of SMT Set is an empty set
    fn test_default_set() {
        let var1 = Set::<Integer>::default();
        let var2 = Set {
            inner: Intern::new(BTreeSet::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }

    #[test]
    /// the default value of SMT Map is an empty map
    fn test_default_map() {
        let var1 = Map::<Integer, Integer>::default();
        let var2 = Map {
            inner: Intern::new(BTreeMap::<SMTWrap<Integer>, SMTWrap<Integer>>::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }
}

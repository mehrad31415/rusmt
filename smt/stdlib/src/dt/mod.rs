//! Standard library for SMT types.
//!
//! * `Boolean`
//! * `Integer`
//! * `Real`
//! * `Float`
//! * `BitVector`
//! * `String`
//! * `Seq<T>`
//! * `Set<T>` - Array<T,Bool>
//! * `Array<K,V>`

use crate::smt::SMT;
use crate::smt_impl;
use crate::wrap::SMTWrap;
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use ordered_float::OrderedFloat;
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
/// ** SMT Int
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Integer {
    inner: Intern<BigInt>,
}
/// ** SMT Real
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Real {
    inner: Intern<BigRational>,
}
/// ** SMT Float 32
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct F32 {
    inner: OrderedFloat<f32>,
}
/// ** SMT Float 64
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct F64 {
    inner: OrderedFloat<f64>,
}
/// ** SMT BitVector32
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct I32 {
    inner: Intern<i32>,
}
/// ** SMT Unsigned BitVector32
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct U32 {
    inner: Intern<u32>,
}
/// ** SMT BitVector64
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct I64 {
    inner: Intern<i64>,
}
/// ** SMT Unsigned BitVector64
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct U64 {
    inner: Intern<u64>,
}
/// ** SMT String
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct String {
    inner: Intern<std::string::String>,
}
/// ** SMT Seq
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Seq<T: SMT> {
    inner: Intern<Vec<SMTWrap<T>>>,
}
/// ** SMT Set
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Set<T: SMT> {
    inner: Intern<BTreeSet<SMTWrap<T>>>,
}
/// ** SMT Array
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Array<K: SMT, V: SMT> {
    inner: Intern<BTreeMap<SMTWrap<K>, SMTWrap<V>>>,
}
/// ** Path-condition marker
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Path {
    inner: Intern<BTreeSet<usize>>,
}
/// ** Cloak
#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Cloak<T: SMT> {
    inner: Intern<SMTWrap<T>>,
}
/// * array module
pub mod array;
/// * bitvector module
pub mod bitvector;
/// * boolean module
pub mod boolean;
/// * cloak module
pub mod cloak;
/// * float module
pub mod float;
/// * integer module
pub mod int;
/// * path module
pub mod path;
/// * real module
pub mod real;
/// * sequence module
pub mod seq;
/// * set module
pub mod set;
/// * general smt module
pub mod smt;
/// * string module
pub mod string;
/// * smt wrap module
pub mod wrap;

smt_impl!(Boolean);
smt_impl!(Integer);
smt_impl!(Real);
smt_impl!(I32);
smt_impl!(I64);
smt_impl!(U32);
smt_impl!(U64);
smt_impl!(F32);
smt_impl!(F64);
smt_impl!(String);
smt_impl!(Seq, T);
smt_impl!(Set, T);
smt_impl!(Array, K, V);
smt_impl!(Path);
smt_impl!(Cloak, T);
smt_impl! { A B } // Both a and B need to implement the SMT trait. To express (a,b,c) we write (a , (b , c)) or ((a,b), c)

#[cfg(test)]
mod test {
    use super::*;
    use num_traits::cast::ToPrimitive;
    use std::collections::HashSet;

    /// checking the implementation of the the arith_operator macro on Integer types
    #[test]
    fn test_arith_operator_macro_integer() {
        let num1 = Integer::from(7);
        let num2 = Integer::from(2);

        let res = num1.add(num2);
        assert!(*res.eq(Integer::from(9))); // 7 + 2 = 9

        let res = num1.div_trunc(num2);
        assert!(*res.eq(Integer::from(3))); // 7 / 2 = 3

        let res = num1.mul(num2);
        assert!(*res.eq(Integer::from(14))); // 7 * 2 = 14

        let res = num1.rem(num2);
        assert!(*res.eq(Integer::from(1))); // 7 % 2 = 1

        let res = num1.sub(num2);
        assert!(*res.eq(Integer::from(5))); // 7 - 2 = 5
    }

    /// checking the implementation of the the arith_operator macro on Real types
    #[test]
    fn test_arith_operator_macro_real() {
        let num1 = Real::from(7.5);
        let num2 = Real::from(2.0);

        let res = num1.add(num2);
        assert!(*res.eq(Real::from(9.5))); // 7.5 + 2 = 9.5

        let res = num1.div(num2);
        assert!(*res.eq(Real::from(3.75))); // 7.5 / 2 = 3.75

        let res = num1.mul(num2);
        assert!(*res.eq(Real::from(15.0))); // 7.5 * 2 = 15

        let res = num1.sub(num2);
        assert!(*res.eq(Real::from(5.5))); // 7.5 - 2 = 5.5
    }

    /// testing the cmp method of the SMT trait on the Boolean type
    #[test]
    fn test_smt_impl_macro_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        let res = var1._cmp(var2); // true > false
        assert_eq!(res, Ordering::Greater);
    }

    /// testing the cmp method of the SMT trait on the Integer type
    #[test]
    fn test_smt_impl_macro_integer() {
        let var1 = Integer::from(1);
        let var2 = Integer::from(3);

        let res = var1._cmp(var2); // 1 < 3
        assert_eq!(res, Ordering::Less);
    }

    /// testing the cmp method of the SMT trait on the Real type
    #[test]
    fn test_smt_impl_macro_real() {
        let var1 = Real::from(1.75);
        let var2 = Real::from(3);

        let res = var1._cmp(var2); // 1.75 < 3
        assert_eq!(res, Ordering::Less);
    }

    /// testing the cmp method of the SMT trait on the String type
    /// this happens in a lexicographical order
    #[test]
    fn test_smt_impl_macro_string() {
        let var1 = String::from("more");
        let var2 = String::from("less");

        let res = var1._cmp(var2); // "more" > "less"
        assert_eq!(res, Ordering::Greater);
    }

    /// testing the cmp method of the SMT trait on the Cloak type
    #[test]
    fn test_smt_impl_macro_cloak() {
        let var1 = Cloak::shield(Integer::from(2));
        let var2 = Cloak::shield(Integer::from(3));

        let res = var1._cmp(var2); // 2 < 3

        assert_eq!(res, Ordering::Less);
    }

    /// testing the cmp method of the SMT trait on the Seq type
    /// elements are compared pairwise in order
    #[test]
    fn test_smt_impl_macro_seq() {
        let var1: Seq<Integer> = Seq::new();
        let var2 = var1.append(Integer::from(1));
        let var3 = var2.append(Integer::from(2));
        let var4 = var3.append(Integer::from(3));
        let var5 = var4.append(Integer::from(4));

        let var6: Seq<Integer> = Seq::new();
        let var7 = var6.append(Integer::from(10));

        let res1 = var5._cmp(var7); // 1 < 10 (first element comparison)
        assert_eq!(res1, Ordering::Less);
    }

    /// testing the cmp method of the SMT trait on the Set type
    #[test]
    fn test_smt_impl_macro_set() {
        let var1 = Set::new();
        let var2 = var1.insert(String::from("hello"));
        let var3 = var2.insert(String::from("world"));

        let var4 = Set::new();
        let var5 = var4.insert(String::from("mehrad"));

        let res = var3._cmp(var5); // "mehrad" > "hello"
        assert_eq!(res, Ordering::Less);
    }

    /// the keys are compared in lexicographical order
    #[test]
    fn test_smt_impl_macro_array() {
        let var1 = Array::new();

        let var2 = var1.store(Integer::from(1), String::from("one"));
        let var3 = var2.store(Integer::from(2), String::from("two"));
        let var4 = var3.store(Integer::from(3), String::from("three"));

        let var5 = Array::new();
        let var6 = var5.store(Integer::from(0), String::from("zero"));

        let res = var4._cmp(var6); // 1 > 0
        assert_eq!(res, Ordering::Greater);
    }

    /// checking the implementation of the the order_operator macro on Integer types
    #[test]
    fn test_order_operator_macro_integer() {
        let var1 = Integer::from(1);
        let var2 = Integer::from(3);

        let res1 = var1.lt(var2); // 1 < 3
        let res2 = var1.le(var2); // 1 <= 3
        let res3 = var1.ge(var2); // 1 >= 3 (false)
        let res4 = Integer::from(3).gt(Integer::from(1).add(Integer::from(1))); // 3 > 1 + 1

        assert!(*res1);
        assert!(*res2);
        assert!(*res3.not());
        assert!(*res4);
    }

    /// checking the implementation of the the order_operator macro on Real types
    #[test]
    fn test_order_operator_macro_real() {
        let var1 = Real::from(1.5);
        let var2 = Real::from(3.0);

        let res1 = var1.lt(var2); // 1.5 < 3
        let res2 = var1.le(var2); // 1.5 <= 3

        assert!(*res1);
        assert!(*res2);
    }

    /// the order_operator macro on String types
    #[test]
    fn test_order_operator_macro_string() {
        let var1 = String::from("1.5");
        let var2 = String::from("9a");
        let var3 = String::from("apple");

        let res1 = var1.lt(var2); // "1.5" < "9a"
        let res2 = var1.le(var2); // "1.5" <= "9a"
        let res3 = var3.gt(var2); // "apple" > "9a"

        assert!(*res1);
        assert!(*res2);
        assert!(*res3);
    }

    /// testing the integer_from_literal macro which gives access to the from method for Integer types
    #[test]
    fn test_integer_from_literal() {
        let var1 = Integer::from(1i8);
        let var2 = Integer::from(10u8);

        assert!(var1.inner.to_u8().expect("Failed to convert BigInt to u8") == 1);
        assert!(var2.inner.to_u8().expect("Failed to convert BigInt to u8") == 10);
    }

    /// checking the implementation of the real_from_literal_int macro
    #[test]
    fn test_real_from_literal_int_macro() {
        let a = Real::from(1i8);
        let b = Real::from(1u8);
        let c = Real::from(1i32);
        let d = Real::from(1i64);
        let e = Real::from(1i128);
        let f = Real::from(1isize);
        let g = Real::from(1f32);
        let h = Real::from(1f64);

        assert!(
            *a.eq(b).and(
                b.eq(c)
                    .and(c.eq(d).and(d.eq(e).and(e.eq(f).and(f.eq(g).and(g.eq(h))))))
            )
        );
    }

    /// checking the implementation of the smt_impl macro for tuples
    /// the macro allows the use of the cmp method of the SMT trait on tuples
    #[test]
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

    /// this tests the seq! macro
    #[test]
    fn test_seq_macro() {
        use crate::seq;
        let var1 = seq![Integer::from(1), Integer::from(0)];

        let var2 = Seq::new();
        let var3 = var2.append(Integer::from(1));
        let var4 = var3.append(Integer::from(0));

        assert!(*var1.eq(var4)); // [1, 0] == [1, 0]
    }

    /// this tests the set! macro
    #[test]
    fn test_set_macro() {
        use crate::set;
        let var1 = set![Integer::from(1), Integer::from(0)];

        let var2 = Set::new();
        let var3 = var2.insert(Integer::from(0));
        let var4 = var3.insert(Integer::from(1));

        assert!(*var1.eq(var4)); // {0, 1} == {0, 1}
    }

    /// this tests the array! macro
    #[test]
    fn test_array_macro() {
        use crate::array;
        let var1 = array![
            (Integer::from(1), String::from("Value 1")),
            (Integer::from(2), String::from("Value 2"))
        ];

        let var2 = Array::new();
        let var3 = var2.store(Integer::from(1), String::from("Value 1"));
        let var4 = var3.store(Integer::from(2), String::from("Value 2"));

        assert!(*var1.eq(var4)); // {1: "Value 1", 2: "Value 2"} == {1: "Value 1", 2: "Value 2"}
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
        let var1 = String::from("a");
        let var2 = String::from("b");

        assert!(var1._cmp(var2) == Ordering::Less);
        assert!(*var1.eq(var2).not());
        assert!(*var1.ne(var2));

        let var3 = var1;
        assert!(*var1.eq(var3));
    }

    /// testing the deref function of boolean
    #[test]
    fn test_deref_boolean() {
        let var1 = Boolean::from(true);

        assert!(*var1);
    }

    /// testing the from function on Boolean
    #[test]
    fn test_from_boolean() {
        let var1 = Boolean::from(false);

        assert!(*var1.not());
    }

    /// testing the boolean operators
    #[test]
    fn test_not_boolean() {
        let var1 = Boolean::from(true);
        assert!(*var1);

        let var2 = var1.not();
        assert!(!(*var2));
    }

    /// it is true only if both are true
    #[test]
    fn test_and_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(!(*(var1.and(var2)))); // true && false = false
        assert!(!(*(var2.and(var2)))); // false && false = false
        assert!(*(var1.and(var1))); // true && true = true
    }

    /// it is true if at least one is true
    #[test]
    fn test_or_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(*(var1.or(var2))); // true || false = true
        assert!(!(*(var2.or(var2)))); // false || false = false
        assert!(*(var1.or(var1))); // true || true = true
    }

    /// it is true if the two operands do not match
    #[test]
    fn test_xor_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(*var1.xor(var2)); // true xor false = true
        assert!(!(*var2.xor(var2))); // false xor false = false
        assert!(!(*var1.xor(var1))); // true xor true = false
    }

    /// a -> b is valid unless a is true and b is false
    #[test]
    fn test_implies_boolean() {
        let var1 = Boolean::from(true);
        let var2 = Boolean::from(false);

        assert!(!(*var1.implies(var2))); // true -> false = false
        assert!(*var1.implies(var1)); // true -> true = true
        assert!(*var2.implies(var1)); // false -> true = true
        assert!(*var2.implies(var2)); // false -> false = true
    }

    /// testing the from method of Real
    #[test]
    fn test_from_real() {
        let var1 = Real::from(1.5);
        let var2 = Real::from(1.6);

        assert!(*var1.eq(Real {
            inner: Intern::new(
                BigRational::from_float(1.5).expect("Failed to convert float to BigRational")
            )
        }));
        assert!(*var2.eq(Real {
            inner: Intern::new(
                BigRational::from_float(1.6).expect("Failed to convert float to BigRational")
            )
        }));
    }

    /// testing the ne/eq/_cmp methods of real
    #[test]
    fn test_cmp_real() {
        let var1 = Real::from(1.0);
        let var2 = Real::from(243.3);

        assert_eq!(var1._cmp(var2), Ordering::Less);
    }

    #[test]
    fn test_eq_real() {
        let var1 = Real::from(1);
        let var2 = Real::from(243.3);

        assert!(*var1.eq(var2).eq(false.into())); // equivalent to *var1.ne(var2)
    }

    #[test]
    fn test_ne_real() {
        let var1 = Real::from(1);
        let var2 = Real::from(243.3);

        assert!(*var1.ne(var2));
    }

    /// testing the from method of String
    #[test]
    fn test_from_string() {
        let var1 = String::from("value");
        assert!(*var1.eq(String {
            inner: Intern::new(std::string::String::from("value"))
        }));
    }

    /// testing the new method of Path
    #[test]
    fn test_path() {
        let var1 = Path::fresh();
        let var2 = Path::fresh();

        // each newly created path-marker only has one element.
        assert_eq!(var1.inner.len(), 1);
        assert_eq!(var2.inner.len(), 1);

        // the first one is created with a value of zero and each new one is incremented by one.
        assert_eq!(*var1.inner.iter().next().expect("Path is empty"), 0);
        assert_eq!(*var2.inner.iter().next().expect("Path is empty"), 1);

        // in merging, the elements are included in one set, thus {0,1} is inside var3.
        let var3 = var1.merge(var2);
        assert_eq!(var3.inner.len(), 2);

        // the first element is 0 and the second element is 1.
        assert_eq!(*var3.inner.iter().next().expect("Path is empty"), 0);
        assert_eq!(
            *var3
                .inner
                .iter()
                .nth(1)
                .expect("Path does not have 2 elements"),
            1
        );

        // in merging var3 {0,1} and var1 {0} because the inner value is a set the values are not duplicated.
        let var4 = var3.merge(var1);
        assert_eq!(var4.inner.len(), 2);
    }

    /// testing the eq method for SMTWrap
    #[test]
    fn test_eq_smtwrap() {
        let var1 = SMTWrap(Integer::from(1));
        let var2 = SMTWrap(Integer::from(15));

        assert!(!var1.eq(&var2));
        assert!(var1.ne(&var2));
    }

    /// testing the partial_cmp for smt wrap
    #[test]
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

    /// testing the _cmp for smt wrap
    #[test]
    fn test_cmp_smtwrap() {
        let x = Cloak::shield(Integer::from(1));
        let y = Cloak::shield(Integer::from(10));
        let var1 = SMTWrap(x);
        let var2 = SMTWrap(y);

        assert_eq!(var2.cmp(&var1), Ordering::Greater);
    }

    /// adding SMtWrap to a HashSet
    #[test]
    fn test_hash_smtwrap() {
        let var1 = SMTWrap(Integer::from(1));
        let var2 = SMTWrap(Integer::from(2));
        let var3 = SMTWrap(Integer::from(1));
        let var4 = SMTWrap(Integer::from(2));

        let mut set = HashSet::new();
        set.insert(var1);
        set.insert(var2);
        set.insert(var3);
        set.insert(var4);

        assert_eq!(set.len(), 2);
    }

    /// checking the implementation of the shield and reveal methods on the Cloak type.
    #[test]
    fn test_shield_reveal_cloak() {
        let var1 = Cloak::shield(Integer::from(1));
        let var2 = var1.reveal();
        assert!(*var2.eq(1.into()));
    }

    /// a new sequence has an initial length of 0
    #[test]
    fn test_new_seq() {
        let seq: Seq<Integer> = Seq::new();
        assert!(*seq.length().eq(Integer::from(0)));
    }

    /// the length of the sequence is 3 after appending 3 elements (duplicate elements are allowed)
    #[test]
    fn test_append_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(1));
        assert!(*seq.length().eq(Integer::from(3)));
    }

    /// the value at index 1 is 2
    #[test]
    fn test_at_unchecked_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        assert!(*seq.at(Integer::from(1)).eq(Integer::from(2)));
    }

    /// this test checks that the at method gives none when the index is out of bounds
    #[test]
    #[should_panic]
    fn test_at_unchecked_seq_out_of_bounds() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        assert!(*seq.at(Integer::from(3)).eq(Integer::from(10)));
    }

    /// the sequence includes the value 2
    /// the sequence does not include the value 4
    #[test]
    fn test_includes_seq() {
        let seq = Seq::new();
        let seq = seq.append(Integer::from(1));
        let seq = seq.append(Integer::from(2));
        let seq = seq.append(Integer::from(3));
        assert!(*seq.contains(Integer::from(2)));
        assert!(*seq.contains(Integer::from(4)).not());
    }

    /// the iterator returns the values 0, 1, 2
    #[test]
    fn test_iterator_seq() {
        let seq = Seq::new();
        let seq = seq.append(String::from("one"));
        let seq = seq.append(String::from("two"));
        let seq = seq.append(String::from("three"));
        assert!(*seq.iterator()[0].eq(Integer::from(0)));
        assert!(*seq.iterator()[1].eq(Integer::from(1)));
        assert!(*seq.iterator()[2].eq(Integer::from(2)));
    }

    /// this tests the new method for the Set type
    #[test]
    fn test_new_set() {
        let set = Set::<Integer>::new();
        assert!(*set.length().eq(0.into()));
    }

    /// this tests the length method for the Set type
    #[test]
    fn test_length_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
    }

    /// the set does not contain duplicates
    #[test]
    fn test_length_dup_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(1));
        assert!(*set.length().eq(1.into()));
    }

    /// After removing the element, the length of the set should decrease by 1
    #[test]
    fn test_remove_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
        let set = set.remove(Integer::from(1));
        assert!(*set.length().eq(1.into()));
    }

    /// After removing a non-existent element, the length of the set should not change
    #[test]
    fn test_remove_set2() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.length().eq(2.into()));
        let set = set.remove(Integer::from(10));
        assert!(*set.length().eq(2.into()));
    }

    /// the set should contain the element that was inserted
    #[test]
    fn test_contains_set() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.contains(Integer::from(1)));
        assert!(*set.contains(Integer::from(20)).not());
    }

    /// when the element is removed, the set should not contain the element anymore
    #[test]
    fn test_contains_set2() {
        let set = Set::new();
        let set = set.insert(Integer::from(1));
        let set = set.insert(Integer::from(2));
        assert!(*set.contains(Integer::from(1)));
        let set = set.remove(Integer::from(1));
        assert!(*set.contains(Integer::from(1)).not());
    }

    /// the iterator should return the elements in the set
    #[test]
    fn test_iterator_set() {
        let set = Set::new();
        let set = set.insert(String::from("one"));
        let set = set.insert(String::from("two"));
        let set = set.insert(String::from("three"));

        let iter = set.iterator();
        assert_eq!(iter.len(), 3);
        assert!(*iter[0].eq(String::from("one")));
        assert!(*iter[1].eq(String::from("three"))); // it is in lexicographical order
        assert!(*iter[2].eq(String::from("two")));
    }

    /// a new array is created and the length of the array should be 0
    #[test]
    fn test_array_length() {
        let array: Array<Integer, Integer> = Array::new();
        assert!(*array.length().eq(0.into()));
    }

    /// When adding a key-value pair to the array, the length of the array should increase by 1
    #[test]
    fn test_array_put() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        assert!(*array.length().eq(1.into()));
    }

    /// When adding a key-value pair to the array, the key should exist in the array
    /// and the value should be the same as the one that was added
    #[test]
    fn test_array_get() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        assert!(*array.select(Integer::from(1)).eq(Integer::from(2)));
    }

    /// When deleting an existent key from the array, the length of the array should decrease by 1
    #[test]
    fn test_array_del() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        let array = array.del(Integer::from(1));
        assert!(*array.length().eq(0.into()));
    }

    /// Deleting a key that does not exist in the array should not change the array.
    #[test]
    fn test_array_del_non_existent() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        assert!(*array.length().eq(1.into()));
        let array = array.del(Integer::from(2));
        assert!(*array.length().eq(1.into()));
    }

    /// getting an element that does not exist in the array should give None
    #[test]
    #[should_panic]
    fn test_array_get_non_existent() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        assert!(*array.select(Integer::from(2)).eq(Integer::default()));
    }

    /// checking if a key exists in the array
    /// a key should only exist if it was added to the array
    #[test]
    fn test_array_contains_key() {
        let array = Array::new().store(Integer::from(1), Integer::from(2));
        assert!(*array.contains_key(Integer::from(1)));
    }

    /// getting an iterator over the keys of the array
    #[test]
    fn test_array_iterator() {
        let array = Array::new().store(String::from("one"), Integer::from(1));
        assert!(*array.iterator()[0].eq(String::from("one")));
    }

    /// the default value of Integer is 0
    #[test]
    fn test_default_integer() {
        let var1 = Integer::default();
        let var2 = Integer::from(0);
        assert!(*var1.eq(var2));
    }

    /// the default value of Boolean is false
    #[test]
    fn test_default_boolean() {
        let var1 = Boolean::default();
        let var2 = Boolean::from(false);
        assert!(*var1.eq(var2));
    }

    /// the default value of Real is numerator 0 and denominator 1
    #[test]
    fn test_default_real() {
        let var1 = Real::default();
        let var2 = Real::from(0);
        assert!(*var1.eq(var2));
    }

    /// the default value of String is an empty string
    #[test]
    fn test_default_string() {
        let var1 = String::default();
        let var2 = String::from("");
        assert!(*var1.eq(var2));
    }

    /// the default value of Path is an empty set
    #[test]
    fn test_default_path() {
        let var1 = Path::default();
        let var2 = Path {
            inner: Intern::new(BTreeSet::new()),
        };
        assert!(*var1.eq(var2));
    }

    /// the default value of Cloak is the default value of the inner type
    #[test]
    fn test_default_cloak() {
        let var1 = Cloak::default();
        let var2 = Cloak::shield(Integer::default());
        assert!(*var1.eq(var2));
    }

    /// the default value of Seq is an empty sequence
    #[test]
    fn test_default_seq() {
        let var1 = Seq::<Integer>::default();
        let var2 = Seq {
            inner: Intern::new(Vec::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }

    /// the default value of SMT Set is an empty set
    #[test]
    fn test_default_set() {
        let var1 = Set::<Integer>::default();
        let var2 = Set {
            inner: Intern::new(BTreeSet::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }

    /// the default value of SMT Array is an empty array
    #[test]
    fn test_default_array() {
        let var1 = Array::<Integer, Integer>::default();
        let var2 = Array {
            inner: Intern::new(BTreeMap::<SMTWrap<Integer>, SMTWrap<Integer>>::new()),
        };
        assert!(*var1.eq(var2));
        assert!(*var1.is_empty());
    }
}

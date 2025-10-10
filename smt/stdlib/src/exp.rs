//! Standard library for SMT expressions
//!
//! quantified expressions:
//! * `forall`
//! * `exists`
//! * `choose` -- hilbert choice operator (deterministic version)

pub use itertools::iproduct;

/// `forall`
/// There are two patterns:
/// 1) forall!(|x:Integer, y:Integer| x.eq(y))
///   basically initializes the variables (in this case x,y) with their default values (0 for int) and checks the constraint (x.eq(y)) against them.
/// 2) forall!(x in m, y in n => x.gt(y))
///  m and n are values that have the iterator method. For example:
///  let m = set!(Integer::from(1), Integer::from(2))
///  let n = set!(Integer::from(10), Integer::from(20))
///  The iproduct macro creates a cartesian product out of the result of m.iterator() and n.iterator():
///  The result will be: (Integer::from(1), Integer::from(10)), (Integer::from(1), Integer::from(20)), (Integer::from(2), Integer::from(10)), (Integer::from(2), Integer::from(20))
///  The forall macro then iterates over this product and applies the constraint (here x.gt(y)) to each pair of values.
///  If the constraint is true for all pairs, the result is true, otherwise false.
#[macro_export]
macro_rules! forall {
    (|$v0:ident : $t0:ty $(, $vn:ident : $tn:ty)* $(,)?| $constraint:expr) => {
        (|$v0 : $t0 $(, $vn : $tn)*| -> $crate::Boolean {
            $constraint
        })(<$t0>::default() $(, <$tn>::default())*)
    };
    ($v0:ident in $c0:expr $(, $vn:ident in $cn:expr)* => $constraint:expr) => {
        {
            $crate::Boolean::from(
                $crate::iproduct!($c0.iterator() $(, $cn.iterator())*).all(
                    |($v0, $($vn, )*)| *$constraint
                )
            )
        }
    };
}

/// `exists`: same as forall but the result is true if the constraint is true for at least one pair of values.
#[macro_export]
macro_rules! exists {
    (|$v0:ident : $t0:ty $(, $vn:ident : $tn:ty)* $(,)?| $constraint:expr) => {
        (|$v0 : $t0 $(, $vn : $tn)*| -> $crate::Boolean {
            $constraint
        })(<$t0>::default() $(, <$tn>::default())*)
    };
    ($v0:ident in $c0:expr $(, $vn:ident in $cn:expr)* => $constraint:expr) => {
        {
            $crate::Boolean::from(
                $crate::iproduct!($c0.iterator() $(, $cn.iterator())*).any(
                    |($v0, $($vn, )*)| *$constraint
                )
            )
        }
    };
}

/// `choose`: If there `exists` a valid set of values that satisfy the constraint, the values are returned
///    In the first pattern, the default values are returned if the constraint is satisfied
///    In the second pattern, the first pair of values in the product of the collections that satisfies the constraint is returned
///    Equivalent to (exists ((x T)) (P x)) ... (get-model) ...
#[macro_export]
macro_rules! choose {
    (|$v0:ident : $t0:ty $(, $vn:ident : $tn:ty)* $(,)?| $constraint:expr) => {
        if *$crate::exists!(|$v0 : $t0 $(, $vn : $tn)*| $constraint) {
            (<$t0>::default() $(, <$tn>::default())*)
        } else {
            panic!("no valid choice");
        }
    };
    ($v0:ident in $c0:expr $(, $vn:ident in $cn:expr)* => $constraint:expr) => {
        (|| {
            for ($v0, $($vn, )*) in $crate::iproduct!($c0.iterator() $(, $cn.iterator())*) {
                if *$constraint {
                    return ($v0 $(, $vn)*);
                }
            }
            panic!("no valid choice");
        }) ()
    };
}

#[cfg(test)]
mod test {
    use crate::smt::SMT;
    use crate::{array, dt::*, set};

    #[test]
    /// testing the first pattern of the forall macro.
    /// All integers are by default zero so var1 = 0.into(); and var2 = 0.into()
    fn test_pattern_one_forall_one() {
        let v = forall!(|var1: Integer, var2: Integer| var1.eq(var2));
        assert!(*v);
    }

    #[test]
    /// By default a bool is set to false
    fn test_pattern_one_forall_two() {
        let v = forall!(|var1: Boolean| var1);
        assert!(!*v)
    }

    #[test]
    /// the cartesian product will be (1, 10), (2, 10)
    /// 1 < 10 and 2 < 10 so the result should be true
    /// Note that the iterator() method on array returns a list of keys
    /// and the iterator() method on set returns a list of its elements
    fn test_pattern_two_forall_one() {
        let m = array!(
            String::default();
            (Integer::from(1), String::from("one")),
            (Integer::from(2), String::from("two"))
        );
        let s = set!(Integer::from(10));

        let v = forall!(var1 in m, var2 in s => var1.lt(var2));
        assert!(*v);
    }

    #[test]
    /// The functionality of the first pattern of the exists macro is similar to the forall macro
    /// The only difference is the use cases.
    fn test_pattern_one_exists() {
        let v = exists!(|var1: Integer, var2: Integer| var1.eq(var2));
        assert!(*v);
    }

    #[test]
    /// the output of the cartesian product will be (1, 10), (20, 10)
    /// 1 < 10 so the result should be true because at least one pair of values satisfies the constraint
    fn test_pattern_two_exists() {
        let m = array!(
            String::default();
            (Integer::from(1), String::from("one")),
            (Integer::from(20), String::from("twenty"))
        );
        let s = set!(Integer::from(10));

        let v = exists!(var1 in m, var2 in s => var1.lt(var2));
        assert!(*v);
    }

    #[test]
    /// the output of the cartesian product will be (1, 10), (20, 10)
    /// 20 < 10 so the result should be false because not all pairs of values satisfy the constraint
    fn test_pattern_two_forall_two() {
        let m = array!(
            String::default();
            (Integer::from(1), String::from("one")),
            (Integer::from(20), String::from("twenty"))
        );
        let s = set!(Integer::from(10));

        let v = forall!(var1 in m, var2 in s => var1.lt(var2));
        assert!(!*v);
    }

    #[test]
    /// Returns the default of the data type
    fn test_pattern_one_choose() {
        let (v1, v2) = choose!(|var1: Integer, var2: Integer| var1.eq(var2));
        assert!(*v1.eq(v2).and(v2.eq(Integer::from(0))));
    }

    #[test]
    #[should_panic(expected = "no valid choice")]
    /// 0>0 is not valid so the operator will panic
    fn test_pattern_one_choose_panic() {
        let _ = choose!(|var1: Integer, var2: Integer| var1.gt(var2));
    }

    #[test]
    /// the output of the cartesian product will be (1, 10), (20, 10)
    /// 1 < 10 so the result should be (1, 10) because the first pair of values that satisfies the constraint is returned
    fn test_pattern_two_choose() {
        let m = array!(
            String::default();
            (Integer::from(1), String::from("one")),
            (Integer::from(20), String::from("twenty"))
        );
        let s = set!(Integer::from(10));

        let (v1, v2) = choose!(var1 in m, var2 in s => var1.lt(var2));
        assert!(*v1.eq(Integer::from(1)));
        assert!(*v2.eq(Integer::from(10)));
    }

    #[test]
    #[should_panic(expected = "no valid choice")]
    /// the output of the cartesian product will be (10, 10), (20, 10)
    /// no pair of values satisfies the constraint so a panic is thrown because nothing is returned
    fn test_pattern_two_choose_panic() {
        let m = array!(
            String::default();
            (Integer::from(10), String::from("ten")),
            (Integer::from(20), String::from("twenty"))
        );
        let s = set!(Integer::from(10));

        let _ = choose!(var1 in m, var2 in s => var1.lt(var2));
    }

    #[test]
    /// The forall and choose macros have been combined to set the minimum value of a set
    /// fn set_min(set: Set<Value>) -> Value {
    ///     choose!(v in set => forall!(e in set => v.eq(e).or(v.lt(e))))
    /// }
    fn test_set_min() {
        let s = set!(Integer::from(1), Integer::from(2), Integer::from(3));
        let v = choose!(v in s => forall!(e in s => v.eq(e).or(v.lt(e))));
        assert!(*v.eq(Integer::from(1)));
    }

    #[test]
    /// The forall and choose macros have been combined to set the minimum key of a array.
    /// fn array_key_min(array: Array<Value, Value>) -> Value {
    ///    choose!(v in array => forall!(e in array => v.eq(e).or(v.lt(e))))
    /// }
    fn test_array_key_min() {
        let m = array!(
            String::default();
            (Integer::from(1), String::from("one")),
            (Integer::from(2), String::from("two")),
            (Integer::from(3), String::from("three"))
        );
        let v = choose!(v in m => forall!(e in m => v.eq(e).or(v.lt(e))));
        assert!(*v.eq(Integer::from(1)));
    }
}

//! Standard library for SMT expressions
//!
//! quantified expressions:
//! * `forall`
//! * `exists`
//! * `choose` -- hilbert choice operator (deterministic version)

pub use itertools::iproduct;

/// `forall`: universally quantified expression over a collection.
/// Usage: `forall!(x in collection, y in collection => predicate)`
/// Iterates the cartesian product of the collections and checks the predicate for every element.
#[macro_export]
macro_rules! forall {
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

/// `exists`: existentially quantified expression over a collection.
/// Usage: `exists!(x in collection => predicate)`
/// Returns true if the predicate holds for at least one element.
#[macro_export]
macro_rules! exists {
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

/// `choose`: returns the first element in the collection satisfying the predicate.
/// Usage: `choose!(x in collection => predicate)`
/// Panics if no element satisfies the predicate.
/// Equivalent to (exists ((x T)) (P x)) ... (get-model) ...
#[macro_export]
macro_rules! choose {
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
    /// the cartesian product will be (1, 10), (2, 10)
    /// 1 < 10 and 2 < 10 so the result should be true
    /// Note that the iterator() method on array returns a list of keys
    /// and the iterator() method on set returns a list of its elements
    fn test_pattern_two_forall_one() {
        let m = array!(
            (Integer::from(1), String::from("one")),
            (Integer::from(2), String::from("two"))
        );
        let s = set!(Integer::from(10));

        let v = forall!(var1 in m, var2 in s => var1.lt(var2));
        assert!(*v);
    }

    #[test]
    /// the output of the cartesian product will be (1, 10), (20, 10)
    /// 1 < 10 so the result should be true because at least one pair of values satisfies the constraint
    fn test_pattern_two_exists() {
        let m = array!(
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
            (Integer::from(1), String::from("one")),
            (Integer::from(20), String::from("twenty"))
        );
        let s = set!(Integer::from(10));

        let v = forall!(var1 in m, var2 in s => var1.lt(var2));
        assert!(!*v);
    }

    #[test]
    /// the output of the cartesian product will be (1, 10), (20, 10)
    /// 1 < 10 so the result should be (1, 10) because the first pair of values that satisfies the constraint is returned
    fn test_pattern_two_choose() {
        let m = array!(
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
            (Integer::from(1), String::from("one")),
            (Integer::from(2), String::from("two")),
            (Integer::from(3), String::from("three"))
        );
        let v = choose!(v in m => forall!(e in m => v.eq(e).or(v.lt(e))));
        assert!(*v.eq(Integer::from(1)));
    }
}

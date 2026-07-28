//! SMT Trait and Implementations.

use crate::Boolean;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

/// A type that implements the SMT trait must implement the following methods:
///
/// * `_cmp` - Compare two values of the same type
/// * `eq` - Check if two values of the same type are equal
/// * `ne` - Check if two values of the same type are not equal
///
/// The first method does not have a default implementation and must be implemented by the type.
/// The following traits are the supertraits of the SMT trait, thus any type that implements the SMT trait must also implement these traits.
///
/// * `Copy` - The type must be copyable
/// * `Default` - The type must have a default value
/// * `Hash` - The type must be hashable
/// * `Send` - The type must be able to be sent between threads
/// * `Sync` - The type must be able to be access from multiple threads
/// * `Debug` - The type must be able to be formatted using the `{:?}` formatter
/// * `Clone` - The type must be cloneable
pub trait SMT: 'static + Copy + Clone + Default + Hash + Send + Sync + Debug {
    /// Compare two values of the same type
    fn _cmp(self, rhs: Self) -> Ordering;

    /// Equal
    fn eq(self, rhs: Self) -> Boolean {
        (Self::_cmp(self, rhs) == Ordering::Equal).into()
    }

    /// Not equal
    fn ne(self, rhs: Self) -> Boolean {
        (Self::_cmp(self, rhs) != Ordering::Equal).into()
    }
}

/// A macro to implement the SMT trait for the floating-point types.
///
/// Floats cannot use the plain [`smt_impl`] body, because `OrderedFloat`'s
/// ordering treats `+0.0` and `-0.0` as `Equal` while SMT-LIB's `=` — which
/// the backend emits for [`SMT::eq`] — treats them as two distinct values of
/// the `FloatingPoint` sort. Without this override,
/// `F32::pos_zero().eq(F32::neg_zero())` is `true` concretely and `false` in
/// the solver.
///
/// Note the NaN case is deliberately left to `OrderedFloat`, which compares
/// all NaNs `Equal`: SMT-LIB's `FloatingPoint` sort has a *single* NaN value,
/// so `(= NaN NaN)` is `true` regardless of payload. Comparing raw `to_bits()`
/// would match the zeros but break NaN.
#[macro_export]
macro_rules! smt_float_impl {
    ($l:ty, $prim:ty) => {
        impl SMT for $l {
            fn _cmp(self, rhs: Self) -> Ordering {
                let (a, b): ($prim, $prim) = (self.inner.0, rhs.inner.0);
                if a == 0.0 && b == 0.0 {
                    // both zeros: order -0.0 immediately before +0.0
                    return b.is_sign_negative().cmp(&a.is_sign_negative());
                }
                self.inner.cmp(&rhs.inner)
            }
        }
    };
}

/// A macro to implement the SMT trait for a type.
#[macro_export]
macro_rules! smt_impl {
    // single types
    ($l:ty) => {
        // any type in SMT can only be compared with the same type (so float cannot be compared with integer)
        impl SMT for $l {
            fn _cmp(self, rhs: Self) -> Ordering {
                 self.inner.cmp(&rhs.inner)
            }
        }
    };
    // generic types
    ($l:ident, $t0:ident $(, $tn:ident)*) => {
        impl<$t0: SMT $(, $tn: SMT)*> SMT for $l<$t0 $(, $tn)*> {
            fn _cmp(self, rhs: Self) -> Ordering {
                self.inner.cmp(&rhs.inner)
            }
        }
    };
    // tuples
    ( $n0:ident $($nx:ident)+ ) => {
        impl<$n0: SMT $(, $nx: SMT)+> SMT for ($n0 $(, $nx)+) {
            paste!{
                #[allow(non_snake_case)]
                fn _cmp(self, rhs: Self) -> Ordering {
                    let ([<l_ $n0>] $(, [<l_ $nx>])+) = self;
                    let ([<r_ $n0>] $(, [<r_ $nx>])+) = rhs;
                    match $n0::_cmp([<l_ $n0>], [<r_ $n0>]) {
                        Ordering::Less => return Ordering::Less,
                        Ordering::Greater => return Ordering::Greater,
                        Ordering::Equal => {},
                    };
                    $(match $nx::_cmp([<l_ $nx>], [<r_ $nx>]) {
                        Ordering::Less => return Ordering::Less,
                        Ordering::Greater => return Ordering::Greater,
                        Ordering::Equal => {},
                    };)+
                    Ordering::Equal
                }
            }
        }
    };
}

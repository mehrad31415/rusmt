//! SMT Integer type and operations.

use crate::{Boolean, F32, F64, I32, I64, Integer, Real, String, U32, U64};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Euclid;
use num_traits::Num;
use num_traits::Signed;
use num_traits::cast::ToPrimitive;

/// Integer operations
impl Integer {
    /// addition - Result can grow arbitrarily large but is bounded by available system memory
    pub fn add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() + rhs.inner.as_ref()),
        }
    }

    /// multiplication
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() * rhs.inner.as_ref()),
        }
    }

    /// subtraction
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() - rhs.inner.as_ref()),
        }
    }

    /// negation
    pub fn neg(self) -> Self {
        Self {
            inner: Intern::new(-self.inner.as_ref()),
        }
    }

    /// (define-fun div_trunc ((n Int) (d Int)) Int
    ///   (ite (>= n 0)
    ///        (div n d)
    ///        (- (div (- n) d))))
    /// corresponds to Z3: (div_trunc n d)
    pub fn div_trunc(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// Division matching Z3 SMT-LIB `Int` division (Euclidean).
    ///
    /// Z3 ensures: `n = d*q + r` and `0 <= r < |d|` (for `d != 0`).
    /// Corresponds to Z3: `(div n d)`.
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().div_euclid(rhs.inner.as_ref())),
        }
    }

    /// Modulo matching Z3 SMT-LIB (Euclidean)
    /// Result is ALWAYS non-negative: 0 <= r < |rhs|
    /// Corresponds to Z3 native operator: (mod self rhs)
    pub fn modulo(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().rem_euclid(rhs.inner.as_ref())),
        }
    }

    /// remainder (will return 0 or the same sign as the dividend (self))
    /// (define-fun rem_trunc ((a Int) (b Int)) Int
    ///   (- a (* b (div_trunc a b))))
    /// corresponds to Z3: (rem_trunc a b)
    pub fn rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() % rhs.inner.as_ref()),
        }
    }

    /// exponentiation
    /// corresponds to Z3: (^ self exp)
    ///
    /// panics if exp is negative or too large to fit in u32
    pub fn pow(self, exp: Self) -> Self {
        exp.inner
            .to_u32()
            .map(|e| Self {
                inner: Intern::new(self.inner.as_ref().pow(e)),
            })
            .unwrap()
    }

    /// absolute value
    ///
    /// corresponds to Z3: (abs self)
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Checks if self divides rhs (i.e., `rhs mod self == 0`).
    /// (= (mod rhs self) 0)
    /// Returns false if self is zero.
    pub fn divides(self, rhs: Self) -> Boolean {
        (rhs.inner.as_ref() % self.inner.as_ref() == BigInt::from(0)).into()
    }

    /// less than
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() < rhs.inner.as_ref()).into()
    }

    /// less than or equal
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() <= rhs.inner.as_ref()).into()
    }

    /// greater than
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() > rhs.inner.as_ref()).into()
    }

    /// greater than or equal
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() >= rhs.inner.as_ref()).into()
    }

    /// to_real() converts the int to a real type.
    /// (to_real x)
    pub fn to_real(self) -> Real {
        Real {
            inner: Intern::new(BigRational::from((
                self.inner.as_ref().clone(),
                BigInt::from(1),
            ))),
        }
    }

    /// to_i32 converts the int to a i32 type.
    ///
    /// ((_ int2bv 32) x)
    pub fn to_i32(self) -> I32 {
        I32 {
            inner: Intern::new(self.inner.to_i32().unwrap()),
        }
    }

    /// to_i64 converts the int to a i64 type.
    ///
    /// ((_ int2bv 64) x)
    pub fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.to_i64().unwrap()),
        }
    }

    /// to_u32 converts the int to a u32 type.
    ///
    /// ((_ int2bv 32) x)
    pub fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.to_u32().unwrap()),
        }
    }

    /// to_u64 converts the int to a u64 type.
    ///
    /// ((_ int2bv 64) x)
    pub fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.to_u64().unwrap()),
        }
    }

    /// convert to f32
    ///
    /// ((_ to_fp 8 24) RNE (to_real x))
    pub fn to_f32(self) -> F32 {
        F32::from(self.inner.as_ref().to_f32().unwrap())
    }

    /// convert to f64
    ///
    /// ((_ to_fp 11 53) RNE (to_real x))
    pub fn to_f64(self) -> F64 {
        F64::from(self.inner.as_ref().to_f64().unwrap())
    }

    /// Creates an `Integer` from a hexadecimal string.
    /// The string should not include the "0x" prefix nor any underscores.
    ///
    /// corresponds to Z3: (from_hex_str s)
    ///
    // (define-fun hex_char_to_int ((s String)) Int
    //   (if (and (str.<= "0" s) (str.<= s "9")) (- (str.to_code s) 48)       ; '0' is 48
    //   (if (and (str.<= "A" s) (str.<= s "F")) (- (str.to_code s) 55)       ; 'A' is 65 -> 10
    //   (if (and (str.<= "a" s) (str.<= s "f")) (- (str.to_code s) 87)       ; 'a' is 97 -> 10
    //       0)))                                                             ; Error/Default
    // )
    // ; Recursive Hex Parser
    // (define-fun-rec from_hex_str_impl ((s String) (acc Int)) Int
    // (if (= (str.len s) 0)
    //     acc
    //     (from_hex_str_impl
    //       (str.substr s 1 (- (str.len s) 1))
    //       (+ (* acc 16) (hex_char_to_int (str.at s 0)))
    //     )
    // )
    // )
    // ; Wrapper to start recursion
    // (define-fun from_hex_str ((s String)) Int (from_hex_str_impl s 0))
    pub fn from_hex_str(s: String) -> Self {
        Self {
            inner: Intern::new(BigInt::from_str_radix(s.inner.as_ref(), 16).unwrap()),
        }
    }

    /// Creates an `Integer` from an octal string.
    /// The string should not include the "0o" prefix.
    ///
    /// corresponds to Z3: (from_oct_str s)
    ///
    // (define-fun oct_char_to_int ((s String)) Int
    // (if (and (str.<= "0" s) (str.<= s "7")) (- (str.to_code s) 48) 0)
    // )
    // ; Recursive Octal Parser
    // (define-fun-rec from_oct_str_impl ((s String) (acc Int)) Int
    // (if (= (str.len s) 0)
    //     acc
    //     (from_oct_str_impl
    //       (str.substr s 1 (- (str.len s) 1))
    //       (+ (* acc 8) (oct_char_to_int (str.at s 0)))
    //     )
    // )
    // )
    // ; Wrapper
    // (define-fun from_oct_str ((s String)) Int (from_oct_str_impl s 0))
    pub fn from_oct_str(s: String) -> Self {
        Self {
            inner: Intern::new(BigInt::from_str_radix(s.inner.as_ref(), 8).unwrap()),
        }
    }

    /// Creates an `Integer` from a binary string.
    /// The string should not include the "0b" prefix.
    ///
    /// corresponds to Z3: (from_bin_str s)
    // (define-fun-rec from_bin_str_impl ((s String) (acc Int)) Int
    //   (if (= (str.len s) 0)
    //   acc
    //   (from_bin_str_impl
    //     (str.substr s 1 (- (str.len s) 1))
    //     (+ (* acc 2) (if (= (str.at s 0) "1") 1 0))
    //   )
    // )
    // )
    // (define-fun from_bin_str ((s String)) Int (from_bin_str_impl s 0))
    pub fn from_bin_str(s: String) -> Self {
        Self {
            inner: Intern::new(BigInt::from_str_radix(s.inner.as_ref(), 2).unwrap()),
        }
    }

    /// check if greater than i64::MAX
    ///
    /// corresponds to Z3: (> self 9223372036854775807)
    pub fn is_gt_i64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i64::MAX)).into()
    }

    /// check if less than i64::MIN
    ///
    /// corresponds to Z3: (< self -9223372036854775808)
    pub fn is_lt_i64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i64::MIN)).into()
    }

    /// check if greater than u64::MAX
    ///
    /// corresponds to Z3: (> self 18446744073709551615)
    pub fn is_gt_u64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(u64::MAX)).into()
    }

    /// check if less than u64::MIN
    ///
    /// corresponds to Z3: (< self 0)
    pub fn is_lt_u64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u64::MIN)).into()
    }

    /// check if less than i32::MIN
    ///
    /// corresponds to Z3: (< self -2147483648)
    pub fn is_lt_i32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i32::MIN)).into()
    }

    /// check if greater than i32::MAX
    ///
    /// corresponds to Z3: (> self 2147483647)
    pub fn is_gt_i32_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i32::MAX)).into()
    }

    /// check if less than u32::MIN
    ///
    /// corresponds to Z3: (< self 0)
    pub fn is_lt_u32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u32::MIN)).into()
    }

    /// check if greater than u32::MAX
    ///
    /// corresponds to Z3: (> self 4294967295)
    pub fn is_gt_u32_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(u32::MAX)).into()
    }
}

/// Convert to the Integer type from literals
/// let a = Integer::from(1);
/// let a:Integer = 1.into(); // this needs to be annotated
/// let a:Integer = From::from(1); // this needs to be annotated
macro_rules! integer_from_literal {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for Integer {
            fn from(c: $l) -> Self {
                Self {
                    inner: Intern::new(BigInt::from(c)),
                }
            }
        }
        $(impl From<$e> for Integer {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigInt::from(c)),
                }
            }
        })*
    };
}

integer_from_literal!(i8, i16, i32, i64, i128, isize);
integer_from_literal!(u8, u16, u32, u64, u128, usize);

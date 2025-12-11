//! SMT Integer type and operations.

use crate::{Boolean, F32, F64, I32, I64, Integer, Real, String, U32, U64};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Num;
use num_traits::Signed;
use num_traits::Zero;
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

    /// division
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// mod (will return 0 or the same sign as the divisor (rhs))
    pub fn modulo(self, rhs: Self) -> Self {
        let rem = self.inner.as_ref() % rhs.inner.as_ref();
        if rem.is_zero()
            || (rem.is_positive() && rhs.inner.as_ref().is_positive())
            || (rem.is_negative() && rhs.inner.as_ref().is_negative())
        {
            Self {
                inner: Intern::new(rem),
            }
        } else {
            Self {
                inner: Intern::new(rem + rhs.inner.as_ref()),
            }
        }
    }

    /// remainder (will return 0 or the same sign as the dividend (self))
    pub fn rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() % rhs.inner.as_ref()),
        }
    }

    /// exponentiation
    pub fn pow(self, exp: Self) -> Self {
        exp.inner
            .to_u32()
            .map(|e| Self {
                inner: Intern::new(self.inner.as_ref().pow(e)),
            })
            .unwrap()
    }

    /// absolute value
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Checks if self divides rhs (i.e., `rhs % self == 0`).
    /// Returns false if self is zero.
    pub fn divides(self, rhs: Self) -> Boolean {
        if self.inner.is_zero() {
            return false.into();
        }
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
    pub fn to_real(self) -> Real {
        Real {
            inner: Intern::new(BigRational::from((
                self.inner.as_ref().clone(),
                BigInt::from(1),
            ))),
        }
    }

    /// to_i32 converts the int to a i32 type.
    pub fn to_i32(self) -> I32 {
        I32 {
            inner: Intern::new(self.inner.to_i32().unwrap()),
        }
    }

    /// to_i64 converts the int to a i64 type.
    pub fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.to_i64().unwrap()),
        }
    }

    /// to_u32 converts the int to a u32 type.
    pub fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.to_u32().unwrap()),
        }
    }

    /// to_u64 converts the int to a u64 type.
    pub fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.to_u64().unwrap()),
        }
    }

    /// convert to f32
    pub fn to_f32(self) -> F32 {
        F32::from(self.inner.as_ref().to_f32().unwrap())
    }

    /// convert to f64
    pub fn to_f64(self) -> F64 {
        F64::from(self.inner.as_ref().to_f64().unwrap())
    }

    /// Creates an `Integer` from a hexadecimal string.
    /// The string should not include the "0x" prefix nor any underscores.
    pub fn from_hex_str(s: String) -> Self {
        Self {
            inner: Intern::new(
                BigInt::from_str_radix(s.replace("_".into(), "".into()).inner.as_ref(), 16)
                    .unwrap(),
            ),
        }
    }

    /// Creates an `Integer` from an octal string.
    /// The string should not include the "0o" prefix.
    pub fn from_oct_str(s: String) -> Self {
        Self {
            inner: Intern::new(
                BigInt::from_str_radix(s.replace("_".into(), "".into()).inner.as_ref(), 8).unwrap(),
            ),
        }
    }

    /// Creates an `Integer` from a binary string.
    /// The string should not include the "0b" prefix.
    pub fn from_bin_str(s: String) -> Self {
        Self {
            inner: Intern::new(
                BigInt::from_str_radix(s.replace("_".into(), "".into()).inner.as_ref(), 2).unwrap(),
            ),
        }
    }

    /// check if greater than i64::MAX
    pub fn is_gt_i64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i64::MAX)).into()
    }

    /// check if less than i64::MIN
    pub fn is_lt_i64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i64::MIN)).into()
    }

    /// check if greater than u64::MAX
    pub fn is_gt_u64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(u64::MAX)).into()
    }

    /// check if less than u64::MIN
    pub fn is_lt_u64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u64::MIN)).into()
    }

    /// check if less than i32::MIN
    pub fn is_lt_i32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i32::MIN)).into()
    }

    /// check if greater than i32::MAX
    pub fn is_gt_i32_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i32::MAX)).into()
    }

    /// check if less than u32::MIN
    pub fn is_lt_u32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u32::MIN)).into()
    }

    /// check if greater than u32::MAX
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

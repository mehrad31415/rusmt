use crate::{Boolean, F32, F64, Integer, Real, SymbolicBitVec};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::Zero;
use num_traits::cast::ToPrimitive;
use std::marker::PhantomData;

/// arithmetic operations for Integer
impl Integer {
    /// addition
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
    pub fn div(self, rhs: Self) -> Option<Self> {
        if rhs.inner.is_zero() {
            return None;
        }
        Some(Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        })
    }

    /// remainder
    pub fn rem(self, rhs: Self) -> Option<Self> {
        if rhs.inner.is_zero() {
            return None;
        }
        Some(Self {
            inner: Intern::new(self.inner.as_ref() % rhs.inner.as_ref()),
        })
    }

    /// Performs exponentiation.
    ///
    /// It returns `None` if the exponent is negative or too large.
    pub fn pow(self, exp: Self) -> Option<Self> {
        exp.inner.to_u32().map(|e| Self {
            inner: Intern::new(self.inner.as_ref().pow(e)),
        })
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
}

/// comparison operations for Integer
impl Integer {
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
}

/// conversion operations for Integer
impl Integer {
    /// to_real() converts the int to a real type.
    pub fn to_real(self) -> Real {
        Real {
            inner: Intern::new(BigRational::from((
                self.inner.as_ref().clone(),
                BigInt::from(1),
            ))),
        }
    }

    /// to_bitvec() converts the int to a bitvector of size N if the value fits in N bits.
    pub fn to_bitvec<const N: usize>(self) -> Option<SymbolicBitVec<N>> {
        assert!(
            N > 0 && N <= 128,
            "BitVector width must be between 1 and 128"
        );

        // convert the BigInt to i128 type.
        if let Some(val_i128) = self.inner.to_i128() {
            let min_val = if N == 128 {
                i128::MIN
            } else {
                -(1i128 << (N - 1))
            };
            let max_val = if N == 128 {
                i128::MAX
            } else {
                (1i128 << (N - 1)) - 1
            };

            if val_i128 >= min_val && val_i128 <= max_val {
                Some(SymbolicBitVec {
                    inner: val_i128,
                    _phantom: PhantomData,
                })
            } else {
                None
            }
        } else {
            // BigInt is too large to even fit in an i128.
            None
        }
    }

    /// Converts an `Integer` to a 32-bit `SymbolicFloat` (`F32`).
    ///
    /// This is a lossy conversion. Integers with a magnitude greater than 2^24
    /// may lose precision due to rounding. Very large integers will be converted
    /// to `f32::INFINITY` or `f32::NEG_INFINITY`.
    pub fn to_f32(self) -> F32 {
        let bigint_ref = self.inner.as_ref();

        let f32_val = bigint_ref.to_f32().unwrap_or_else(|| {
            if bigint_ref.is_positive() {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        });
        F32::from(f32_val)
    }

    /// Try to convert to f32
    ///
    /// If the integer is too large to fit in f32, return None
    pub fn try_to_f32(self) -> Option<F32> {
        let bigint = self.inner.as_ref();
        let val_f32 = bigint.to_f32()?;

        let rat = BigRational::from_float(val_f32)?;
        if rat.is_integer() && rat.to_integer() == *bigint {
            Some(F32::from(val_f32))
        } else {
            None
        }
    }

    /// Converts an `Integer` to a 64-bit `SymbolicFloat` (`F64`).
    ///
    /// This is a lossy conversion. Integers with a magnitude greater than 2^53
    /// may lose precision due to rounding. Very large integers will be converted
    /// to `f64::INFINITY` or `f64::NEG_INFINITY`.
    pub fn to_f64(self) -> F64 {
        let bigint_ref = self.inner.as_ref();

        let f64_val = bigint_ref.to_f64().unwrap_or_else(|| {
            if bigint_ref.is_positive() {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        });
        F64::from(f64_val)
    }

    /// Try to convert to f64
    ///
    /// If the integer is too large to fit in f64, return None
    pub fn try_to_f64(self) -> Option<F64> {
        let bigint = self.inner.as_ref();
        let val_f64 = bigint.to_f64()?;

        let rat = BigRational::from_float(val_f64)?;
        if rat.is_integer() && rat.to_integer() == *bigint {
            Some(F64::from(val_f64))
        } else {
            None
        }
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

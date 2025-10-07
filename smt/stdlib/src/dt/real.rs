use crate::{Boolean, F32, F64, Integer, Real, SymbolicBitVec};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::Zero;
use num_traits::cast::ToPrimitive;

/// arithmetic operations for Real
impl Real {
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

    /// Performs exponentiation.
    ///
    /// It returns `None` if the exponent is negative or too large.
    pub fn pow(self, exp: Self) -> Option<Self> {
        // Check if exp is an integer
        if !exp.inner.is_integer() {
            return None;
        }
        // Convert exp to i32
        if let Some(e) = exp.inner.to_integer().to_i32() {
            Some(Self {
                inner: Intern::new(self.inner.as_ref().pow(e)),
            })
        } else {
            None
        }
    }

    /// Returns the absolute value of the real number.
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Rounds the real number to the nearest integer.
    pub fn round(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().round().to_integer()),
        }
    }

    /// Floors the real number to the nearest integer less than or equal to the number.
    pub fn floor(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().floor().to_integer()),
        }
    }

    /// Ceils the real number to the nearest integer greater than or equal to the number.
    pub fn ceil(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().ceil().to_integer()),
        }
    }

    /// is integer
    pub fn is_integer(self) -> Boolean {
        self.inner.is_integer().into()
    }
}

/// comparison operations for Real
impl Real {
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

/// conversion operations for Real
impl Real {
    /// Lossless & Fallible: Converts a Real to an Integer if it has no fractional part.
    pub fn to_int(self) -> Option<Integer> {
        if self.inner.fract() != BigRational::from_integer(BigInt::from(0)) {
            return None; // Has a fractional part
        }

        let integer_part = self.inner.to_integer();
        Some(Integer {
            inner: Intern::new(integer_part),
        })
    }

    /// LOSSY (Rounding): Converts a Real to an integer
    pub fn to_int_trunc(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.to_integer()),
        }
    }

    /// Converts a Real to an F32.
    /// If the value is too large or too small, it returns +/- infinity.
    pub fn to_f32(self) -> F32 {
        let f32_val = self.inner.as_ref().to_f32().unwrap_or_else(|| {
            if self.inner.is_positive() {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        });
        F32::from(f32_val)
    }

    /// Try to convert to f32
    ///
    /// If the real number is too large to fit in f32, return None
    pub fn try_to_f32(self) -> Option<F32> {
        let bigrat = self.inner.as_ref();
        let val_f32 = bigrat.to_f32()?;

        let rat = BigRational::from_float(val_f32)?;
        if rat == *bigrat {
            Some(F32::from(val_f32))
        } else {
            None
        }
    }

    /// Converts a Real to an F64.
    /// If the value is too large or too small, it returns +/- infinity.
    pub fn to_f64(self) -> F64 {
        let f64_val = self.inner.as_ref().to_f64().unwrap_or_else(|| {
            if self.inner.is_positive() {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        });
        F64::from(f64_val)
    }

    /// Try to convert to f64
    ///
    /// If the real number is too large to fit in f64, return None
    pub fn try_to_f64(self) -> Option<F64> {
        let bigrat = self.inner.as_ref();
        let val_f64 = bigrat.to_f64()?;

        let rat = BigRational::from_float(val_f64)?;
        if rat == *bigrat {
            Some(F64::from(val_f64))
        } else {
            None
        }
    }

    /// to_bitvec() converts the real to a bitvector of size N if there is no fractional part and the value fits in N bits.
    pub fn to_bitvec<const N: usize>(self) -> Option<SymbolicBitVec<N>> {
        self.to_int().and_then(|int_val| int_val.to_bitvec::<N>())
    }

    /// This allows us to build real numbers from f32
    pub fn try_from_f32(value: f32) -> Option<Self> {
        BigRational::from_float(value).map(|br| Real {
            inner: Intern::new(br),
        })
    }

    /// This allows us to build real numbers from f64
    pub fn try_from_f64(value: f64) -> Option<Self> {
        BigRational::from_float(value).map(|br| Real {
            inner: Intern::new(br),
        })
    }
}

/// Convert to Real from int literals
/// let a = Real::from(1);
/// let a:Real = 1.into(); // this needs to be annotated
/// let a:Real = From::from(1); // this needs to be annotated
macro_rules! real_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for Real {
            fn from(c: $l) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        }
        $(impl From<$e> for Real {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        })*
    };
}

real_from_literal_int!(i8, i16, i32, i64, i128, isize);
real_from_literal_int!(u8, u16, u32, u64, u128, usize);

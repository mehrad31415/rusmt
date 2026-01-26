//! Real datatype and its operations

use crate::{Boolean, F32, F64, Integer, Real};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::cast::ToPrimitive;

/// Real operations
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
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// Exponentiation with integer exponent.
    ///
    /// # Panics
    /// - Panics if exponent is negative or too large to fit in i32
    /// - Only supports integer exponents
    ///
    /// Note: Z3 SMT-LIB does not have native Real exponentiation.
    /// This is provided for interpreter convenience.
    pub fn pow(self, exp: Self) -> Self {
        Self {
            inner: Intern::new(
                self.inner
                    .as_ref()
                    .pow(exp.inner.to_integer().to_i32().unwrap()),
            ),
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

    /// converts a Real to an Integer
    /// (to_int x)
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.to_integer()),
        }
    }

    /// convert to f32
    /// ((_ to_fp 8 24) RNE x) - uses Round to Nearest, ties to Even
    pub fn to_f32(self) -> F32 {
        F32::from(self.inner.as_ref().to_f32().unwrap())
    }

    /// convert to f64
    /// ((_ to_fp 11 53) RNE x) - uses Round to Nearest, ties to Even
    pub fn to_f64(self) -> F64 {
        F64::from(self.inner.as_ref().to_f64().unwrap())
    }
}

/// Convert to Real from int literals
/// let a = Real::from(1);
/// let a:Real = 1.into(); // this needs to be annotated
/// let a:Real = From::from(1); // this needs to be annotated
macro_rules! real_from_literal {
    ($l1:ty, $l2:ty $(,$e:ty)* $(,)?) => {
        impl From<$l1> for Real {
            fn from(c: $l1) -> Self {
                BigRational::from_float(c)
                    .map(|br| Real {
                    inner: Intern::new(br),
                }).expect("DSL Error: Cannot convert NaN or Infinity to SMT Real")
            }
        }
        impl From<$l2> for Real {
            fn from(c: $l2) -> Self {
                BigRational::from_float(c)
                    .map(|br| Real {
                    inner: Intern::new(br),
                }).expect("DSL Error: Cannot convert NaN or Infinity to SMT Real")
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

real_from_literal!(
    f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

// test from Nan
mod tests {
    #[test]
    #[should_panic(expected = "DSL Error: Cannot convert NaN or Infinity to SMT Real")]
    fn test_from_nan() {
        use crate::Real;
        let _nan = Real::from(f64::NAN);
        assert!(true);
    }

    #[test]
    #[should_panic(expected = "DSL Error: Cannot convert NaN or Infinity to SMT Real")]
    fn test_from_infinity() {
        use crate::Real;
        let _infinity = Real::from(f64::INFINITY);
        assert!(true);
    }

    #[test]
    #[should_panic(expected = "DSL Error: Cannot convert NaN or Infinity to SMT Real")]
    fn test_from_negative_infinity() {
        use crate::Real;
        let _negative_infinity = Real::from(f64::NEG_INFINITY);
        assert!(true);
    }
}

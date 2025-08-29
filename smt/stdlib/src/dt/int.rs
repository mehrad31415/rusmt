use crate::{Integer, Rational, arith_operator, order_operator};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::cast::ToPrimitive;
use std::ops::{Add, Div, Mul, Rem, Sub};

impl Integer {
    /// to_rational() converts the int to a real type.
    pub fn to_rational(self) -> Rational {
        Rational {
            inner: Intern::new(BigRational::from((
                self.inner.as_ref().clone(),
                BigInt::from(1),
            ))),
        }
    }

    pub fn pow(self, exp: Self) -> Self {
        Self {
            inner: Intern::new(
                self.inner.as_ref().pow(
                    exp.inner
                        .as_ref()
                        .to_u32()
                        .expect("Exponent out of u32 range"),
                ),
            ),
        }
    }

    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
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
arith_operator!(Integer, add, sub, mul, div, rem);
order_operator!(Integer, lt, le, ge, gt);

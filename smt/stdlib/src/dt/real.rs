use crate::{Integer, Rational, arith_operator, order_operator};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::cast::ToPrimitive;
use std::ops::{Add, Div, Mul, Sub};

impl Rational {
    pub fn pow(self, exp: Self) -> Self {
        Self {
            inner: Intern::new(
                self.inner.as_ref().pow(
                    exp.inner
                        .as_ref()
                        .to_i32()
                        .expect("Exponent out of i32 range"),
                ),
            ),
        }
    }

    pub fn round(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().round().to_integer()),
        }
    }

    pub fn floor(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().floor().to_integer()),
        }
    }

    pub fn ceil(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().ceil().to_integer()),
        }
    }

    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }
}

// These allow us to build rational numbers from f32/f64
impl From<f32> for Rational {
    fn from(value: f32) -> Self {
        Self {
            inner: Intern::new(
                BigRational::from_float(value).expect("Failed to convert float to BigRational"),
            ),
        }
    }
}
impl From<f64> for Rational {
    fn from(value: f64) -> Self {
        Self {
            inner: Intern::new(
                BigRational::from_float(value).expect("Failed to convert float to BigRational"),
            ),
        }
    }
}

/// Convert to Rational from int literals
/// let a = Rational::from(1);
/// let a:Rational = 1.into(); // this needs to be annotated
/// let a:Rational = From::from(1); // this needs to be annotated
macro_rules! rational_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for Rational {
            fn from(c: $l) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        }
        $(impl From<$e> for Rational {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        })*
    };
}

rational_from_literal_int!(i8, i16, i32, i64, i128, isize);
rational_from_literal_int!(u8, u16, u32, u64, u128, usize);
arith_operator!(Rational, add, sub, mul, div);
order_operator!(Rational, lt, le, ge, gt);

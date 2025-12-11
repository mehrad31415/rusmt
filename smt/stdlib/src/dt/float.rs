//! Floating-point types and operations.

use crate::dt::{Boolean, F32, F64, Integer, Real, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::FromPrimitive;
use ordered_float::OrderedFloat;

/// Common trait for all floating-point types
pub trait FloatOps: Sized + SMT {
    /// addition
    fn add(self, rhs: Self) -> Self;
    /// subtraction
    fn sub(self, rhs: Self) -> Self;
    /// multiplication
    fn mul(self, rhs: Self) -> Self;
    /// division
    fn div(self, rhs: Self) -> Self;
    /// negation
    fn neg(self) -> Self;
    /// absolute value
    fn abs(self) -> Self;
    /// Floating-point remainder.
    fn rem(self, rhs: Self) -> Self;
    /// Floating-point square root.
    fn sqrt(self) -> Self;
    /// Minimum of floating-point numbers.
    fn min(self, rhs: Self) -> Self;
    /// Maximum of floating-point numbers.
    fn max(self, rhs: Self) -> Self;
    /// is NaN `(fp.isNaN X)`
    fn is_nan(self) -> Boolean;
    /// is infinite `(fp.isInfinite X)`
    fn is_infinite(self) -> Boolean;
    /// is zero `(fp.isZero X)`
    fn is_zero(self) -> Boolean;
    /// is normal `(fp.isNormal t)`
    fn is_normal(self) -> Boolean;
    /// is subnormal `(fp.isSubnormal X)`
    fn is_subnormal(self) -> Boolean;
    /// is negative `(fp.isNegative X)`
    fn is_negative(self) -> Boolean;
    /// is positive `(fp.isPositive X)`
    fn is_positive(self) -> Boolean;
    /// Less than comparison.
    fn lt(self, rhs: Self) -> Boolean;
    /// Less than or equal comparison.
    fn le(self, rhs: Self) -> Boolean;
    /// Greater than comparison.
    fn gt(self, rhs: Self) -> Boolean;
    /// Greater than or equal comparison.
    fn ge(self, rhs: Self) -> Boolean;
    /// Creates a Not-a-Number (NaN) value.
    fn nan() -> Self;
    /// Creates a positive infinity value.
    fn infinity() -> Self;
    /// Creates a negative infinity value.
    fn neg_infinity() -> Self;
    /// Creates a positive zero value.
    fn pos_zero() -> Self;
    /// Creates a negative zero value.
    fn neg_zero() -> Self;
    /// Converts a float to an Integer `Z3_mk_fpa_round_to_integral`.
    fn to_integer(self) -> Integer;
    /// Converts a float to a Real.
    fn to_real(self) -> Real;
}

/// Operations for F32.
impl FloatOps for F32 {
    fn nan() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f32::NAN)),
        }
    }

    fn infinity() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f32::INFINITY)),
        }
    }

    fn neg_infinity() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f32::NEG_INFINITY)),
        }
    }

    fn pos_zero() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(0.0f32)),
        }
    }

    fn neg_zero() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(-0.0f32)),
        }
    }

    /// The transpiler should generate the `(fp.add roundNearestTiesToEven t1 t2)` SMT-LIB expression.
    fn add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 + rhs.inner.0)),
        }
    }

    /// The transpiler should generate `(fp.sub roundNearestTiesToEven t1 t2)`.
    fn sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 - rhs.inner.0)),
        }
    }

    /// The transpiler should generate `(fp.mul roundNearestTiesToEven t1 t2)`.
    fn mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 * rhs.inner.0)),
        }
    }

    /// The transpiler should generate `(fp.div roundNearestTiesToEven t1 t2)`.
    fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 / rhs.inner.0)),
        }
    }

    fn neg(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(-self.inner.0)),
        }
    }

    fn abs(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.abs())),
        }
    }

    fn rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 % rhs.inner.0)),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.sqrt())),
        }
    }

    fn min(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.min(rhs.inner),
        }
    }

    fn max(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.max(rhs.inner),
        }
    }

    fn lt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    fn le(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    fn gt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    fn ge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    fn is_nan(self) -> Boolean {
        self.inner.is_nan().into()
    }

    fn is_infinite(self) -> Boolean {
        self.inner.is_infinite().into()
    }

    fn is_zero(self) -> Boolean {
        (self.inner.0 == 0.0f32).into()
    }

    fn is_negative(self) -> Boolean {
        self.inner.is_sign_negative().into()
    }

    fn is_positive(self) -> Boolean {
        self.inner.is_sign_positive().into()
    }

    fn is_normal(self) -> Boolean {
        self.inner.is_normal().into()
    }

    fn is_subnormal(self) -> Boolean {
        self.inner.is_subnormal().into()
    }

    fn to_integer(self) -> Integer {
        BigInt::from_f32(self.inner.trunc())
            .map(|bi| Integer {
                inner: Intern::new(bi),
            })
            .unwrap()
    }

    fn to_real(self) -> Real {
        BigRational::from_float(*self.inner)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .unwrap()
    }
}

/// Operations for F64.
impl FloatOps for F64 {
    fn nan() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f64::NAN)),
        }
    }

    fn infinity() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f64::INFINITY)),
        }
    }

    fn neg_infinity() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f64::NEG_INFINITY)),
        }
    }

    fn pos_zero() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(0.0f64)),
        }
    }

    fn neg_zero() -> Self {
        Self {
            inner: Intern::new(OrderedFloat(-0.0f64)),
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 + rhs.inner.0)),
        }
    }

    fn sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 - rhs.inner.0)),
        }
    }

    fn mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 * rhs.inner.0)),
        }
    }

    fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 / rhs.inner.0)),
        }
    }

    fn neg(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(-self.inner.0)),
        }
    }

    fn abs(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.abs())),
        }
    }

    fn rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0 % rhs.inner.0)),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.sqrt())),
        }
    }

    fn min(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.min(rhs.inner),
        }
    }

    fn max(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.max(rhs.inner),
        }
    }

    fn lt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    fn le(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    fn gt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    fn ge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    fn is_nan(self) -> Boolean {
        self.inner.is_nan().into()
    }

    fn is_infinite(self) -> Boolean {
        self.inner.is_infinite().into()
    }

    fn is_zero(self) -> Boolean {
        (self.inner.0 == 0.0f64 || self.inner.0 == -0.0f64).into()
    }

    fn is_negative(self) -> Boolean {
        self.inner.is_sign_negative().into()
    }

    fn is_positive(self) -> Boolean {
        self.inner.is_sign_positive().into()
    }

    fn is_normal(self) -> Boolean {
        self.inner.is_normal().into()
    }

    fn is_subnormal(self) -> Boolean {
        self.inner.is_subnormal().into()
    }

    fn to_integer(self) -> Integer {
        BigInt::from_f64(self.inner.trunc())
            .map(|bi| Integer {
                inner: Intern::new(bi),
            })
            .unwrap()
    }

    fn to_real(self) -> Real {
        BigRational::from_float(*self.inner)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .unwrap()
    }
}

impl From<f32> for F32 {
    /// from_f32() creates a F32 from a f32.
    fn from(f: f32) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f)),
        }
    }
}

impl From<crate::String> for F32 {
    /// from_str() creates a F32 from a string.
    fn from(s: crate::String) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(s.inner.parse::<f32>().unwrap())),
        }
    }
}

impl From<f64> for F64 {
    /// from_f64() creates a F64 from a f64.
    fn from(f: f64) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f)),
        }
    }
}

impl From<crate::String> for F64 {
    /// from_str() creates a F64 from a string.
    fn from(s: crate::String) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(s.inner.parse::<f64>().unwrap())),
        }
    }
}

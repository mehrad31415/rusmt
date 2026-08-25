//! Floating-point types and operations.

use crate::dt::{Boolean, F32, F64, I32, I64, Integer, Real, U32, U64, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::FromPrimitive;
use num_traits::cast::ToPrimitive;
use ordered_float::OrderedFloat;

impl From<f32> for F32 {
    /// from_f32() creates a F32 from a f32.
    /// Corresponds to Z3: ((_ to_fp 8 24) RNE f)
    fn from(f: f32) -> Self {
        Self {
            inner: OrderedFloat(f),
        }
    }
}

impl From<f64> for F64 {
    /// from_f64() creates a F64 from a f64.
    /// Corresponds to Z3: ((_ to_fp 11 53) RNE f)
    fn from(f: f64) -> Self {
        Self {
            inner: OrderedFloat(f),
        }
    }
}

/// Common trait for all floating-point types
pub trait FloatOps: Sized + SMT {
    /// Addition.
    ///
    /// Corresponds to Z3: (fp.add RNE self rhs)
    fn add(self, rhs: Self) -> Self;
    /// subtraction.
    ///
    /// Corresponds to Z3: (fp.sub RNE self rhs)
    fn sub(self, rhs: Self) -> Self;
    /// multiplication.
    ///
    /// Corresponds to Z3: (fp.mul RNE self rhs)
    fn mul(self, rhs: Self) -> Self;
    /// division.
    ///
    /// Corresponds to Z3: (fp.div RNE self rhs)
    fn div(self, rhs: Self) -> Self;
    /// negation
    ///
    /// Corresponds to Z3: (fp.neg self)
    fn neg(self) -> Self;
    /// absolute value
    ///
    /// Corresponds to Z3: (fp.abs self)
    fn abs(self) -> Self;
    /// Floating-point remainder.
    ///
    /// Corresponds to Z3: (fp.rem self rhs)
    fn rem(self, rhs: Self) -> Self;
    /// Floating-point square root.
    ///
    /// Corresponds to Z3: (fp.sqrt RNE self)
    fn sqrt(self) -> Self;
    /// Minimum of floating-point numbers.
    ///
    /// Corresponds to Z3: (fp.min self rhs)
    fn min(self, rhs: Self) -> Self;
    /// Maximum of floating-point numbers.
    ///
    /// Corresponds to Z3: (fp.max self rhs)
    fn max(self, rhs: Self) -> Self;
    /// is NaN.
    ///
    /// Corresponds to Z3: (fp.isNaN self)
    fn is_nan(self) -> Boolean;
    /// is infinite.
    ///
    /// Corresponds to Z3: (fp.isInfinite self)
    fn is_infinite(self) -> Boolean;
    /// is zero.
    ///
    /// Corresponds to Z3: (fp.isZero self)
    fn is_zero(self) -> Boolean;
    /// is normal.
    ///
    /// Corresponds to Z3: (fp.isNormal self)
    fn is_normal(self) -> Boolean;
    /// is subnormal.
    ///
    /// Corresponds to Z3: (fp.isSubnormal self)
    fn is_subnormal(self) -> Boolean;
    /// is negative.
    ///
    /// Corresponds to Z3: (fp.isNegative self)
    fn is_negative(self) -> Boolean;
    /// is positive.
    ///
    /// Corresponds to Z3: (fp.isPositive self)
    fn is_positive(self) -> Boolean;
    /// Less than comparison.
    ///
    /// Corresponds to Z3: (fp.lt self rhs)
    fn lt(self, rhs: Self) -> Boolean;
    /// Less than or equal comparison.
    ///
    /// Corresponds to Z3: (fp.leq self rhs)
    fn le(self, rhs: Self) -> Boolean;
    /// Greater than comparison.
    ///
    /// Corresponds to Z3: (fp.gt self rhs)
    fn gt(self, rhs: Self) -> Boolean;
    /// Greater than or equal comparison.
    ///
    /// Corresponds to Z3: (fp.geq self rhs)
    fn ge(self, rhs: Self) -> Boolean;
    /// Creates a Not-a-Number (NaN) value.
    ///
    /// Corresponds to Z3: (_ NaN 8 24) or (_ NaN 11 53)
    fn nan() -> Self;
    /// Creates a positive infinity value.
    ///
    /// Corresponds to Z3: (_ +oo 8 24) or (_ +oo 11 53)
    fn infinity() -> Self;
    /// Creates a negative infinity value.
    ///
    /// Corresponds to Z3: (_ -oo 8 24) or (_ -oo 11 53)
    fn neg_infinity() -> Self;
    /// Creates a positive zero value.
    ///
    /// Corresponds to Z3: (_ +zero 8 24) or (_ +zero 11 53)
    fn pos_zero() -> Self;
    /// Creates a negative zero value.
    ///
    /// Corresponds to Z3: (_ -zero 8 24) or (_ -zero 11 53)
    fn neg_zero() -> Self;
    /// Converts a float to an Integer.
    ///
    /// Corresponds to Z3: (to_int (fp.to_real self))
    fn to_integer(self) -> Integer;
    /// Converts a float to a Real.
    ///
    /// Corresponds to Z3: (fp.to_real self)
    fn to_real(self) -> Real;
    /// Converts float to UNSIGNED 64-bit integer (Truncated).
    ///
    /// Corresponds to Z3: ((_ fp.to_ubv 64) RTZ self)
    fn to_u64(self) -> U64;
    /// Converts float to UNSIGNED 32-bit integer (Truncated).
    ///
    /// Corresponds to Z3: ((_ fp.to_ubv 32) RTZ self)
    fn to_u32(self) -> U32;
    /// Converts float to signed 64-bit integer (Truncated).
    ///
    /// Corresponds to Z3: ((_ fp.to_sbv 64) RTZ self)
    fn to_i64(self) -> I64;
    /// Converts float to signed 32-bit integer (Truncated/Round-to-Zero).
    ///
    /// Corresponds to Z3: ((_ fp.to_sbv 32) RTZ self)
    fn to_i32(self) -> I32;
    /// Rounds to the nearest integer greater than or equal to `self`.
    ///
    /// Corresponds to Z3: (fp.roundToIntegral RTP self)
    fn ceil(self) -> Self;
    /// Rounds to the nearest integer less than or equal to `self`.
    ///
    /// Corresponds to Z3: (fp.roundToIntegral RTN self)
    fn floor(self) -> Self;
    /// Rounds to the nearest integer towards zero (truncation).
    ///
    /// Corresponds to Z3: (fp.roundToIntegral RTZ self)
    fn trunc(self) -> Self;
    /// Rounds to the nearest integer; ties to even.
    ///
    /// Corresponds to Z3: (fp.roundToIntegral RNE self)
    fn nearest(self) -> Self;
    /// IEEE 754 numeric equality — what `==` means on a machine float.
    ///
    /// Corresponds to Z3: (fp.eq self rhs)
    ///
    /// This is *not* the same as [`SMT::eq`], which the backend emits as
    /// SMT-LIB `(= a b)`: structural equality on the `FloatingPoint` sort,
    /// i.e. "are these the same value?". The two answer different questions
    /// and differ on exactly two situations — NaN vs NaN, and `+0.0` vs
    /// `-0.0`. Everywhere else they coincide:
    ///
    /// |             | `eq` / `(= a b)`            | `fp_eq` / `(fp.eq a b)`   |
    /// |-------------|-----------------------------|---------------------------|
    /// | NaN , NaN   | `true` — one NaN value in the sort (even Nan = -Nan) | `false` — IEEE: NaN ≠ everything |
    /// | +0.0 , -0.0 | `false` — two distinct values | `true` — IEEE: the zeros compare equal |
    ///
    /// Both are faithful to their SMT counterpart, so either is safe in a
    /// spec; pick the one whose question you mean.
    fn fp_eq(self, rhs: Self) -> Boolean;
}

/// Operations for F32.
impl FloatOps for F32 {
    fn nan() -> Self {
        Self {
            inner: OrderedFloat(f32::NAN),
        }
    }

    fn infinity() -> Self {
        Self {
            inner: OrderedFloat(f32::INFINITY),
        }
    }

    fn neg_infinity() -> Self {
        Self {
            inner: OrderedFloat(f32::NEG_INFINITY),
        }
    }

    fn pos_zero() -> Self {
        Self {
            inner: OrderedFloat(0.0f32),
        }
    }

    fn neg_zero() -> Self {
        Self {
            inner: OrderedFloat(-0.0f32),
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 + rhs.inner.0),
        }
    }

    fn sub(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 - rhs.inner.0),
        }
    }

    fn mul(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 * rhs.inner.0),
        }
    }

    fn div(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 / rhs.inner.0),
        }
    }

    fn neg(self) -> Self {
        Self {
            inner: OrderedFloat(-self.inner.0),
        }
    }

    fn abs(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.abs()),
        }
    }

    fn rem(self, rhs: Self) -> Self {
        // Z3's fp.rem is IEEE 754 remainder (rounds quotient to nearest),
        // NOT Rust's % which is fmod (truncation remainder).
        // Use libm::remainderf which computes IEEE 754 remainder.
        Self {
            inner: OrderedFloat(libm::remainderf(self.inner.0, rhs.inner.0)),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.sqrt()),
        }
    }

    fn min(self, rhs: Self) -> Self {
        let (a, b) = (self.inner.0, rhs.inner.0);
        assert!(
            !(a == 0.0 && b == 0.0 && a.is_sign_negative() != b.is_sign_negative()),
            "(fp.min +zero -zero) is unspecified in SMT-LIB, so there is no value to agree on"
        );
        Self {
            inner: OrderedFloat(a.min(b)),
        }
    }

    fn max(self, rhs: Self) -> Self {
        let (a, b) = (self.inner.0, rhs.inner.0);
        assert!(
            !(a == 0.0 && b == 0.0 && a.is_sign_negative() != b.is_sign_negative()),
            "(fp.max +zero -zero) is unspecified in SMT-LIB, so there is no value to agree on"
        );
        Self {
            inner: OrderedFloat(a.max(b)),
        }
    }

    fn lt(self, rhs: Self) -> Boolean {
        (self.inner.0 < rhs.inner.0).into()
    }

    fn le(self, rhs: Self) -> Boolean {
        (self.inner.0 <= rhs.inner.0).into()
    }

    fn gt(self, rhs: Self) -> Boolean {
        (self.inner.0 > rhs.inner.0).into()
    }

    fn ge(self, rhs: Self) -> Boolean {
        (self.inner.0 >= rhs.inner.0).into()
    }

    fn is_nan(self) -> Boolean {
        self.inner.0.is_nan().into()
    }

    fn is_infinite(self) -> Boolean {
        self.inner.0.is_infinite().into()
    }

    fn is_zero(self) -> Boolean {
        (self.inner.0 == 0.0f32).into()
    }

    fn is_negative(self) -> Boolean {
        // Z3's fp.isNegative returns false for NaN (all NaNs).
        // Rust's is_sign_negative returns true for -NaN.
        // Guard with !is_nan to match Z3.
        (!self.inner.0.is_nan() && self.inner.0.is_sign_negative()).into()
    }

    fn is_positive(self) -> Boolean {
        // Z3's fp.isPositive returns false for NaN (all NaNs).
        // Rust's is_sign_positive returns true for +NaN.
        // Guard with !is_nan to match Z3.
        (!self.inner.0.is_nan() && self.inner.0.is_sign_positive()).into()
    }

    fn is_normal(self) -> Boolean {
        self.inner.0.is_normal().into()
    }

    fn is_subnormal(self) -> Boolean {
        self.inner.0.is_subnormal().into()
    }

    fn to_integer(self) -> Integer {
        // Z3: (to_int (fp.to_real self)) applies floor (rounds toward -∞).
        // Must use .floor() not .trunc(): for -1.5, floor=-2 but trunc=-1.
        BigInt::from_f32(self.inner.0.floor())
            .map(|bi| Integer {
                inner: Intern::new(bi),
            })
            .expect("(to_int (fp.to_real x)) is underspecified on NaN and infinity")
    }

    fn to_real(self) -> Real {
        BigRational::from_float(self.inner.0)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .expect("fp.to_real is underspecified on NaN and infinity")
    }

    fn to_i32(self) -> I32 {
        // .to_i32() in Rust performs truncation (RTZ) — matches Z3's ((_ fp.to_sbv 32) RTZ self).
        // Note: Returns None (unwrap panics) on NaN or Overflow.
        // Your interpreter guards should catch those before calling this.
        I32 {
            inner: Intern::new(self.inner.0.to_i32().expect(
                "(to_sbv 32) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.0.to_i64().expect(
                "(to_sbv 64) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.0.to_u32().expect(
                "(to_ubv 32) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.0.to_u64().expect(
                "(to_ubv 64) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn ceil(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.ceil()),
        }
    }

    fn floor(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.floor()),
        }
    }

    fn trunc(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.trunc()),
        }
    }

    fn nearest(self) -> Self {
        // `round_ties_even` is IEEE roundTiesToEven, which keeps the sign of a
        // zero result: RNE(-0.5) is -0.0, not +0.0.
        Self {
            inner: OrderedFloat(self.inner.0.round_ties_even()),
        }
    }

    fn fp_eq(self, rhs: Self) -> Boolean {
        let a: f32 = self.inner.0;
        let b: f32 = rhs.inner.0;
        (a == b).into()
    }
}

/// Operations for F64.
impl FloatOps for F64 {
    fn nan() -> Self {
        Self {
            inner: OrderedFloat(f64::NAN),
        }
    }

    fn infinity() -> Self {
        Self {
            inner: OrderedFloat(f64::INFINITY),
        }
    }

    fn neg_infinity() -> Self {
        Self {
            inner: OrderedFloat(f64::NEG_INFINITY),
        }
    }

    fn pos_zero() -> Self {
        Self {
            inner: OrderedFloat(0.0f64),
        }
    }

    fn neg_zero() -> Self {
        Self {
            inner: OrderedFloat(-0.0f64),
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 + rhs.inner.0),
        }
    }

    fn sub(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 - rhs.inner.0),
        }
    }

    fn mul(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 * rhs.inner.0),
        }
    }

    fn div(self, rhs: Self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0 / rhs.inner.0),
        }
    }

    fn neg(self) -> Self {
        Self {
            inner: OrderedFloat(-self.inner.0),
        }
    }

    fn abs(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.abs()),
        }
    }

    fn rem(self, rhs: Self) -> Self {
        // Z3's fp.rem is IEEE 754 remainder (rounds quotient to nearest),
        // NOT Rust's % which is fmod (truncation remainder).
        // Use libm::remainder which computes IEEE 754 remainder.
        Self {
            inner: OrderedFloat(libm::remainder(self.inner.0, rhs.inner.0)),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.sqrt()),
        }
    }

    fn min(self, rhs: Self) -> Self {
        let (a, b) = (self.inner.0, rhs.inner.0);
        assert!(
            !(a == 0.0 && b == 0.0 && a.is_sign_negative() != b.is_sign_negative()),
            "(fp.min +zero -zero) is unspecified in SMT-LIB, so there is no value to agree on"
        );
        Self {
            inner: OrderedFloat(a.min(b)),
        }
    }

    fn max(self, rhs: Self) -> Self {
        let (a, b) = (self.inner.0, rhs.inner.0);
        assert!(
            !(a == 0.0 && b == 0.0 && a.is_sign_negative() != b.is_sign_negative()),
            "(fp.max +zero -zero) is unspecified in SMT-LIB, so there is no value to agree on"
        );
        Self {
            inner: OrderedFloat(a.max(b)),
        }
    }

    fn lt(self, rhs: Self) -> Boolean {
        (self.inner.0 < rhs.inner.0).into()
    }

    fn le(self, rhs: Self) -> Boolean {
        (self.inner.0 <= rhs.inner.0).into()
    }

    fn gt(self, rhs: Self) -> Boolean {
        (self.inner.0 > rhs.inner.0).into()
    }

    fn ge(self, rhs: Self) -> Boolean {
        (self.inner.0 >= rhs.inner.0).into()
    }

    fn is_nan(self) -> Boolean {
        self.inner.0.is_nan().into()
    }

    fn is_infinite(self) -> Boolean {
        self.inner.0.is_infinite().into()
    }

    fn is_zero(self) -> Boolean {
        (self.inner.0 == 0.0f64 || self.inner.0 == -0.0f64).into()
    }

    fn is_negative(self) -> Boolean {
        // Z3's fp.isNegative returns false for NaN (all NaNs).
        // Rust's is_sign_negative returns true for -NaN.
        // Guard with !is_nan to match Z3.
        (!self.inner.0.is_nan() && self.inner.0.is_sign_negative()).into()
    }

    fn is_positive(self) -> Boolean {
        // Z3's fp.isPositive returns false for NaN (all NaNs).
        // Rust's is_sign_positive returns true for +NaN.
        // Guard with !is_nan to match Z3.
        (!self.inner.0.is_nan() && self.inner.0.is_sign_positive()).into()
    }

    fn is_normal(self) -> Boolean {
        self.inner.0.is_normal().into()
    }

    fn is_subnormal(self) -> Boolean {
        self.inner.0.is_subnormal().into()
    }

    fn to_integer(self) -> Integer {
        // Z3: (to_int (fp.to_real self)) applies floor (rounds toward -∞).
        // Must use .floor() not .trunc(): for -1.5, floor=-2 but trunc=-1.
        BigInt::from_f64(self.inner.0.floor())
            .map(|bi| Integer {
                inner: Intern::new(bi),
            })
            .expect("(to_int (fp.to_real x)) is underspecified on NaN and infinity")
    }

    fn to_real(self) -> Real {
        BigRational::from_float(self.inner.0)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .expect("fp.to_real is underspecified on NaN and infinity")
    }

    fn to_i32(self) -> I32 {
        I32 {
            inner: Intern::new(self.inner.0.to_i32().expect(
                "(to_sbv 32) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.0.to_i64().expect(
                "(to_sbv 64) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.0.to_u32().expect(
                "(to_ubv 32) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.0.to_u64().expect(
                "(to_ubv 64) is underspecified on NaN, infinity and \
                 out-of-range values",
            )),
        }
    }

    fn ceil(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.ceil()),
        }
    }

    fn floor(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.floor()),
        }
    }

    fn trunc(self) -> Self {
        Self {
            inner: OrderedFloat(self.inner.0.trunc()),
        }
    }

    fn nearest(self) -> Self {
        // `round_ties_even` is IEEE roundTiesToEven, which keeps the sign of a
        // zero result: RNE(-0.5) is -0.0, not +0.0.
        Self {
            inner: OrderedFloat(self.inner.0.round_ties_even()),
        }
    }

    fn fp_eq(self, rhs: Self) -> Boolean {
        let a: f64 = self.inner.0;
        let b: f64 = rhs.inner.0;
        (a == b).into()
    }
}

mod tests {
    #[test]
    fn test_float_val() {
        use crate::{F32, F64, float::FloatOps};

        assert!(*F32::from(3.5).fp_eq(F32::from(3.5)));
        assert!(*F32::from(0.0).fp_eq(F32::from(0.0)));
        assert!(*F32::from(1.0).fp_eq(F32::from(1.0)));
        assert!(*F64::from(3.5).fp_eq(F64::from(3.5)));
        assert!(*F64::from(0.0).fp_eq(F64::from(0.0)));
    }

    #[test]
    fn test_float_arithmetic() {
        use crate::{F64, float::FloatOps};

        // add
        assert!(*F64::from(3.0).add(F64::from(4.0)).fp_eq(F64::from(7.0)));
        // addition by infinity
        assert!(*F64::from(3.0).add(F64::infinity()).fp_eq(F64::infinity()));
        // addition by negative infinity
        assert!(
            *F64::from(3.0)
                .add(F64::neg_infinity())
                .fp_eq(F64::neg_infinity())
        );
        // addition by NaN
        assert!(*F64::from(3.0).add(F64::nan()).is_nan());
        // Nan != Nan
        assert!(*F64::nan().fp_eq(F64::nan()).not());
        assert!(*F64::from(10.0).sub(F64::from(3.0)).fp_eq(F64::from(7.0)));
        // subtraction by infinity
        assert!(
            *F64::from(3.0)
                .sub(F64::infinity())
                .fp_eq(F64::neg_infinity())
        );
        // subtraction by negative infinity
        assert!(
            *F64::from(3.0)
                .sub(F64::neg_infinity())
                .fp_eq(F64::infinity())
        );
        // subtraction by NaN
        assert!(*F64::from(3.0).sub(F64::nan()).is_nan());
        assert!(*F64::from(3.0).mul(F64::from(4.0)).fp_eq(F64::from(12.0)));
        // multiplication by infinity
        assert!(*F64::from(3.0).mul(F64::infinity()).fp_eq(F64::infinity()));
        // multiplication by negative infinity
        assert!(
            *F64::from(3.0)
                .mul(F64::neg_infinity())
                .fp_eq(F64::neg_infinity())
        );
        // multiplication by NaN
        assert!(*F64::from(3.0).mul(F64::nan()).is_nan());
        // multiplication by zero
        assert!(*F64::from(3.0).mul(F64::from(0.0)).fp_eq(F64::from(0.0)));
        assert!(*F64::from(7.0).div(F64::from(2.0)).fp_eq(F64::from(3.5)));

        // Div by zero → Infinity
        assert!(*F64::from(1.0).div(F64::from(0.0)).fp_eq(F64::infinity()));
        assert!(
            *F64::from(-1.0)
                .div(F64::from(0.0))
                .fp_eq(F64::neg_infinity())
        );
        // 0/0 → NaN
        assert!(*F64::from(0.0).div(F64::from(0.0)).is_nan());
        // inf / inf → NaN
        assert!(*F64::infinity().div(F64::infinity()).is_nan());
        // Inf - Inf → NaN
        assert!(*F64::infinity().sub(F64::infinity()).is_nan());
        // Inf + Inf → Inf
        assert!(*F64::infinity().add(F64::infinity()).fp_eq(F64::infinity()));
        // Inf * Inf → Inf
        assert!(*F64::infinity().mul(F64::infinity()).fp_eq(F64::infinity()));
        // NaN propagates
        assert!(*F64::nan().add(F64::from(1.0)).is_nan());
        assert!(*F64::from(1.0).mul(F64::nan()).is_nan());
        // Negative * negative = positive
        assert!(*F64::from(-3.0).mul(F64::from(-4.0)).fp_eq(F64::from(12.0)));
        // Negative / negative = positive
        assert!(*F64::from(-6.0).div(F64::from(-2.0)).fp_eq(F64::from(3.0)));
        assert!(
            *F64::from(-7.0)
                .div(F64::from(-3.0))
                .fp_eq(F64::from(2.3333333333333335))
        );
    }

    #[test]
    fn test_float_neg_abs() {
        use crate::{F64, float::FloatOps};

        assert!(*F64::from(3.0).neg().fp_eq(F64::from(-3.0)));
        assert!(*F64::from(-3.0).neg().fp_eq(F64::from(3.0)));
        assert!(*F64::from(0.0).neg().is_zero());
        assert!(*F64::from(0.0).neg().is_negative());
        assert!(*F64::neg_zero().neg().is_positive());
        assert!(*F64::infinity().neg().fp_eq(F64::neg_infinity()));
        assert!(*F64::nan().neg().is_nan());

        assert!(*F64::from(-3.0).abs().fp_eq(F64::from(3.0)));
        assert!(*F64::from(3.0).abs().fp_eq(F64::from(3.0)));
        assert!(*F64::neg_infinity().abs().fp_eq(F64::infinity()));
        assert!(*F64::neg_zero().abs().is_zero());
        assert!(*F64::neg_zero().abs().is_positive());
        assert!(*F64::nan().abs().is_nan());
    }

    #[test]
    fn test_float_rem_sqrt() {
        use crate::{F64, float::FloatOps};

        // rem: IEEE 754 remainder
        assert!(*F64::from(7.0).rem(F64::from(3.0)).fp_eq(F64::from(1.0)));
        assert!(*F64::from(7.0).rem(F64::from(4.0)).fp_eq(F64::from(-1.0))); // rounds to nearest                                                                                  
        assert!(*F64::from(-7.0).rem(F64::from(3.0)).fp_eq(F64::from(-1.0)));
        assert!(*F64::from(7.0).rem(F64::from(-3.0)).fp_eq(F64::from(1.0)));
        assert!(*F64::from(-7.0).rem(F64::from(-3.0)).fp_eq(F64::from(-1.0)));
        assert!(*F64::from(0.0).rem(F64::from(3.0)).fp_eq(F64::from(0.0)));
        assert!(*F64::from(1.0).rem(F64::from(0.0)).is_nan());
        assert!(*F64::infinity().rem(F64::from(3.0)).is_nan());
        assert!(*F64::from(3.0).rem(F64::infinity()).fp_eq(F64::from(3.0)));
        assert!(
            *F64::from(3.0)
                .rem(F64::neg_infinity())
                .fp_eq(F64::from(3.0))
        );
        assert!(*F64::from(3.0).rem(F64::nan()).is_nan());
        assert!(*F64::nan().rem(F64::from(3.0)).is_nan());

        // sqrt
        assert!(*F64::from(4.0).sqrt().fp_eq(F64::from(2.0)));
        assert!(*F64::from(9.0).sqrt().fp_eq(F64::from(3.0)));
        assert!(*F64::from(0.0).sqrt().fp_eq(F64::from(0.0)));
        assert!(*F64::from(-1.0).sqrt().is_nan());
        assert!(*F64::infinity().sqrt().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().sqrt().is_nan());
        assert!(*F64::nan().sqrt().is_nan());
    }

    #[test]
    fn test_float_min_max() {
        use crate::{F64, float::FloatOps};

        assert!(*F64::from(3.0).min(F64::from(5.0)).fp_eq(F64::from(3.0)));
        assert!(*F64::from(5.0).min(F64::from(3.0)).fp_eq(F64::from(3.0)));
        assert!(*F64::from(-1.0).min(F64::from(1.0)).fp_eq(F64::from(-1.0)));
        assert!(*F64::infinity().min(F64::from(3.0)).fp_eq(F64::from(3.0)));
        assert!(
            *F64::neg_infinity()
                .min(F64::from(3.0))
                .fp_eq(F64::neg_infinity())
        );

        assert!(*F64::from(3.0).max(F64::from(5.0)).fp_eq(F64::from(5.0)));
        assert!(*F64::from(5.0).max(F64::from(3.0)).fp_eq(F64::from(5.0)));
        assert!(*F64::from(-1.0).max(F64::from(1.0)).fp_eq(F64::from(1.0)));
        assert!(
            *F64::neg_infinity()
                .max(F64::from(3.0))
                .fp_eq(F64::from(3.0))
        );
        assert!(*F64::infinity().max(F64::from(3.0)).fp_eq(F64::infinity()));
        assert!(*F64::nan().min(F64::from(3.0)).fp_eq(F64::from(3.0)));
        assert!(*F64::nan().max(F64::from(3.0)).fp_eq(F64::from(3.0)));
    }

    #[test]
    fn test_float_comparisons() {
        use crate::{F64, float::FloatOps};

        assert!(*F64::from(3.0).lt(F64::from(5.0)));
        assert!(*F64::from(3.0).le(F64::from(3.0)));
        assert!(*F64::from(5.0).gt(F64::from(3.0)));
        assert!(*F64::from(5.0).ge(F64::from(5.0)));
        assert!(*F64::neg_infinity().lt(F64::from(0.0)));
        assert!(*F64::from(0.0).lt(F64::infinity()));
        assert!(*F64::neg_zero().fp_eq(F64::pos_zero())); // so positive and negative zero are equal

        // NaN comparisons — all false
        assert!(*F64::nan().lt(F64::from(3.0)).not());
        assert!(*F64::nan().le(F64::from(3.0)).not());
        assert!(*F64::nan().gt(F64::from(3.0)).not());
        assert!(*F64::nan().ge(F64::from(3.0)).not());
        assert!(*F64::nan().lt(F64::nan()).not());
        assert!(*F64::nan().ge(F64::nan()).not());
        assert!(*F64::nan().le(F64::infinity()).not());
        assert!(*F64::nan().ge(F64::neg_infinity()).not());
    }

    #[test]
    fn test_float_predicates() {
        use crate::{F64, float::FloatOps};

        // isNaN
        assert!(*F64::nan().is_nan());
        assert!(*F64::from(3.0).is_nan().not());
        assert!(*F64::infinity().is_nan().not());

        // isInfinite
        assert!(*F64::infinity().is_infinite());
        assert!(*F64::neg_infinity().is_infinite());
        assert!(*F64::from(3.0).is_infinite().not());
        assert!(*F64::nan().is_infinite().not());

        // isZero
        assert!(*F64::pos_zero().is_zero());
        assert!(*F64::neg_zero().is_zero());
        assert!(*F64::from(3.0).is_zero().not());
        assert!(*F64::nan().is_zero().not());

        // isNegative
        assert!(*F64::from(-3.0).is_negative());
        assert!(*F64::neg_zero().is_negative());
        assert!(*F64::neg_infinity().is_negative());
        assert!(*F64::from(3.0).is_negative().not());
        assert!(*F64::nan().is_negative().not());

        // isPositive
        assert!(*F64::from(3.0).is_positive());
        assert!(*F64::pos_zero().is_positive());
        assert!(*F64::infinity().is_positive());
        assert!(*F64::from(-3.0).is_positive().not());
        assert!(*F64::nan().is_positive().not());

        // isNormal
        assert!(*F64::from(3.0).is_normal());
        assert!(*F64::from(-3.0).is_normal());
        assert!(*F64::pos_zero().is_normal().not());
        assert!(*F64::infinity().is_normal().not());
        assert!(*F64::nan().is_normal().not());

        // isSubnormal
        assert!(*F64::from(5e-324).is_subnormal());
        assert!(*F64::from(3.0).is_subnormal().not());
        assert!(*F64::pos_zero().is_subnormal().not());
        assert!(*F64::nan().is_subnormal().not());
    }

    #[test]
    fn test_float_rounding_eq() {
        use crate::{F64, float::FloatOps};

        // ceil
        assert!(*F64::from(2.3).ceil().fp_eq(F64::from(3.0)));
        assert!(*F64::from(-2.3).ceil().fp_eq(F64::from(-2.0)));
        assert!(*F64::from(2.0).ceil().fp_eq(F64::from(2.0)));
        assert!(*F64::nan().ceil().is_nan());
        assert!(*F64::infinity().ceil().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().ceil().fp_eq(F64::neg_infinity()));

        // floor
        assert!(*F64::from(2.7).floor().fp_eq(F64::from(2.0)));
        assert!(*F64::from(-2.3).floor().fp_eq(F64::from(-3.0)));
        assert!(*F64::from(2.0).floor().fp_eq(F64::from(2.0)));
        assert!(*F64::nan().floor().is_nan());
        assert!(*F64::infinity().floor().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().floor().fp_eq(F64::neg_infinity()));

        // trunc
        assert!(*F64::from(2.7).trunc().fp_eq(F64::from(2.0)));
        assert!(*F64::from(-2.7).trunc().fp_eq(F64::from(-2.0)));
        assert!(*F64::nan().trunc().is_nan());
        assert!(*F64::infinity().trunc().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().trunc().fp_eq(F64::neg_infinity()));

        // nearest (ties to even)
        assert!(*F64::from(2.5).nearest().fp_eq(F64::from(2.0))); // tie → even (2)
        assert!(*F64::from(3.5).nearest().fp_eq(F64::from(4.0))); // tie → even (4)                                                                                              
        assert!(*F64::from(2.3).nearest().fp_eq(F64::from(2.0)));
        assert!(*F64::from(2.7).nearest().fp_eq(F64::from(3.0)));
        assert!(*F64::from(-2.5).nearest().fp_eq(F64::from(-2.0))); // tie → even (-2)                                                                                             
        assert!(*F64::from(-3.5).nearest().fp_eq(F64::from(-4.0))); // tie → even (-4)                                                                                             
        assert!(*F64::nan().nearest().is_nan());
        assert!(*F64::infinity().nearest().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().nearest().fp_eq(F64::neg_infinity()));

        // fp_eq
        assert!(*F64::from(3.0).fp_eq(F64::from(3.0)));
        assert!(*F64::from(3.0).fp_eq(F64::from(4.0)).not());
        assert!(*F64::neg_zero().fp_eq(F64::pos_zero())); // -0.0 == 0.0                                                                                                       
        assert!(*F64::nan().fp_eq(F64::nan()).not()); // NaN != NaN      
        assert!(*F64::nan().fp_eq(F64::from(3.0)).not()); // NaN != 3.0
        assert!(*F64::infinity().fp_eq(F64::infinity()));
        assert!(*F64::neg_infinity().fp_eq(F64::neg_infinity()));
    }

    #[test]
    fn test_float_conversions() {
        use crate::{F64, I32, I64, Integer, Real, U32, U64, float::FloatOps, smt::SMT};

        // to_integer (floor)
        assert!(*F64::from(3.7).to_integer().eq(Integer::from(3)));
        assert!(*F64::from(-3.7).to_integer().eq(Integer::from(-4)));
        assert!(*F64::from(0.0).to_integer().eq(Integer::from(0)));

        // to_real — 3.5 is exactly representable in binary, so the rational is 7/2.
        // (`Real` has no float literal; see the `dt::real` module docs.)
        assert!(
            *F64::from(3.5)
                .to_real()
                .eq(Real::from(7).div(Real::from(2)))
        );

        // to_i32
        assert!(*F64::from(42.9).to_i32().eq(I32::from(42)));
        assert!(*F64::from(-42.9).to_i32().eq(I32::from(-42)));

        // to_i64
        assert!(*F64::from(42.9).to_i64().eq(I64::from(42)));

        // to_u32
        assert!(*F64::from(42.9).to_u32().eq(U32::from(42u32)));

        // to_u64
        assert!(*F64::from(42.9).to_u64().eq(U64::from(42u64)));
    }

    #[test]
    #[should_panic]
    fn test_float_to_int_nan() {
        use crate::{F64, float::FloatOps};
        F64::nan().to_integer();
    }

    #[test]
    #[should_panic]
    fn test_float_to_real_nan() {
        use crate::{F64, float::FloatOps};
        F64::nan().to_real();
    }

    #[test]
    #[should_panic]
    fn test_float_to_int_inf() {
        use crate::{F64, float::FloatOps};
        F64::infinity().to_integer();
    }

    #[test]
    #[should_panic]
    fn test_float_to_real_inf() {
        use crate::{F64, float::FloatOps};
        F64::infinity().to_real();
    }

    #[test]
    #[should_panic]
    fn test_float_to_u32_negative() {
        use crate::{F64, float::FloatOps};
        F64::from(-1.0).to_u32();
    }

    #[test]
    #[should_panic]
    fn test_float_to_u32_overflow() {
        use crate::{F64, float::FloatOps};
        F64::from(4294967296.0).to_u32();
    }

    #[test]
    #[should_panic]
    fn test_float_to_u32_nan() {
        use crate::{F64, float::FloatOps};
        F64::nan().to_u32();
    }

    #[test]
    #[should_panic]
    fn test_float_to_i32_overflow() {
        use crate::{F64, float::FloatOps};
        F64::from(3e10).to_i32();
    }

    #[test]
    // testing fp_eq
    fn test_float_fp_eq() {
        use crate::{F64, float::FloatOps};

        let a = F64::nan();
        let b = F64::nan().neg();
        let c = F64::pos_zero();
        let d = F64::neg_zero();

        // (simplify (fp.eq (_ NaN 8 24) (_ NaN 8 24)))
        // (simplify (fp.eq (_ NaN 8 24) (fp.neg (_ NaN 8 24))))
        // (simplify (fp.eq (_ NaN 8 24) (_ +zero 8 24)))
        // (simplify (fp.eq (_ +zero 8 24) (_ -zero 8 24)))
        assert!(*a.fp_eq(b).not());
        assert!(*a.fp_eq(a).not());
        assert!(*a.fp_eq(c).not());
        assert!(*c.fp_eq(d));
    }

    #[test]
    // test eq
    fn test_float_eq() {
        use crate::{dt::F64, dt::SMT, float::FloatOps};

        let a = F64::nan();
        let b = F64::nan().neg();
        let c = F64::pos_zero();
        let d = F64::neg_zero();

        // (simplify (= (_ NaN 8 24) (_ NaN 8 24)))
        // (simplify (= (_ NaN 8 24) (fp.neg (_ NaN 8 24))))
        // (simplify (= (_ NaN 8 24) (_ +zero 8 24)))
        // (simplify (= (_ +zero 8 24) (_ -zero 8 24)))
        assert!(*a.eq(b));
        assert!(*a.eq(a));
        assert!(*a.eq(c).not());
        assert!(*c.eq(d).not());
    }
}

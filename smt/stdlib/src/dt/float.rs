//! Floating-point types and operations.

use crate::dt::{Boolean, F32, F64, I32, I64, Integer, Real, String, U32, U64, smt::SMT};
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
            inner: Intern::new(OrderedFloat(f)),
        }
    }
}

impl From<f64> for F64 {
    /// from_f64() creates a F64 from a f64.
    /// Corresponds to Z3: ((_ to_fp 11 53) RNE f)
    fn from(f: f64) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(f)),
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
    /// Equality comparison.
    ///
    /// Corresponds to Z3: (fp.eq self rhs)
    fn fp_eq(self, rhs: Self) -> Boolean;
    /// Creates a float from a hexadecimal string.
    ///
    /// Corresponds to Z3: ((_ to_fp 8 24) ((_ int2bv 32) (from_hex_str s))) or ((_ to_fp 11 53) ((_ int2bv 64) (from_hex_str s)))
    fn from_hex_str(s: String) -> Self;
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
        // Z3's fp.rem is IEEE 754 remainder (rounds quotient to nearest),
        // NOT Rust's % which is fmod (truncation remainder).
        // Use libm::remainderf which computes IEEE 754 remainder.
        Self {
            inner: Intern::new(OrderedFloat(libm::remainderf(self.inner.0, rhs.inner.0))),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.sqrt())),
        }
    }

    fn min(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.min(rhs.inner.0))),
        }
    }

    fn max(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.max(rhs.inner.0))),
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
            .unwrap()
    }

    fn to_real(self) -> Real {
        BigRational::from_float(self.inner.0)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .unwrap()
    }

    fn to_i32(self) -> I32 {
        // .to_i32() in Rust performs truncation (RTZ) — matches Z3's ((_ fp.to_sbv 32) RTZ self).
        // Note: Returns None (unwrap panics) on NaN or Overflow.
        // Your interpreter guards should catch those before calling this.
        I32 {
            inner: Intern::new(self.inner.0.to_i32().unwrap()),
        }
    }

    fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.0.to_i64().unwrap()),
        }
    }

    fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.0.to_u32().unwrap()),
        }
    }

    fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.0.to_u64().unwrap()),
        }
    }

    fn from_hex_str(s: String) -> Self {
        // You MUST use a helper here (e.g. hexf_parse crate)
        // Rust .parse() does not support hex floats.
        let val = hexf_parse::parse_hexf32(&s.inner, false).expect("Invalid Hex Float");
        Self {
            inner: Intern::new(OrderedFloat(val)),
        }
    }

    fn ceil(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.ceil())),
        }
    }

    fn floor(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.floor())),
        }
    }

    fn trunc(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.trunc().into())),
        }
    }

    fn nearest(self) -> Self {
        let val = self.inner.0;
        let r = val.round();
        let res = if (val - r).abs() == 0.5 && (r % 2.0 != 0.0) {
            if val > 0.0 { r - 1.0 } else { r + 1.0 }
        } else {
            r
        };
        Self {
            inner: Intern::new(OrderedFloat(res)),
        }
    }

    fn fp_eq(self, rhs: Self) -> Boolean {                                                                                                                                         
        let a: f32 = self.inner.0.into();                                                                                                                                          
        let b: f32 = rhs.inner.0.into();
        (a == b).into()                                                                                                                                                            
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
        // Z3's fp.rem is IEEE 754 remainder (rounds quotient to nearest),
        // NOT Rust's % which is fmod (truncation remainder).
        // Use libm::remainder which computes IEEE 754 remainder.
        Self {
            inner: Intern::new(OrderedFloat(libm::remainder(self.inner.0, rhs.inner.0))),
        }
    }

    fn sqrt(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.sqrt())),
        }
    }

    fn min(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.min(rhs.inner.0))),
        }
    }

    fn max(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.max(rhs.inner.0))),
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
            .unwrap()
    }

    fn to_real(self) -> Real {
        BigRational::from_float(self.inner.0)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .unwrap()
    }

    fn to_i32(self) -> I32 {
        I32 {
            inner: Intern::new(self.inner.0.to_i32().unwrap()),
        }
    }

    fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(self.inner.0.to_i64().unwrap()),
        }
    }

    fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(self.inner.0.to_u32().unwrap()),
        }
    }

    fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(self.inner.0.to_u64().unwrap()),
        }
    }

    fn from_hex_str(s: String) -> Self {
        let val = hexf_parse::parse_hexf64(&s.inner, false).expect("Invalid Hex Float");
        Self {
            inner: Intern::new(OrderedFloat(val)),
        }
    }

    fn ceil(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.ceil())),
        }
    }

    fn floor(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.floor())),
        }
    }

    fn trunc(self) -> Self {
        Self {
            inner: Intern::new(OrderedFloat(self.inner.0.trunc())),
        }
    }

    fn nearest(self) -> Self {
        let val = self.inner.0;
        let r = val.round();
        let res = if (val - r).abs() == 0.5 && (r % 2.0 != 0.0) {
            if val > 0.0 { r - 1.0 } else { r + 1.0 }
        } else {
            r
        };
        Self {
            inner: Intern::new(OrderedFloat(res)),
        }
    }

    fn fp_eq(self, rhs: Self) -> Boolean {
        let a: f64 = self.inner.0.into();
        let b: f64 = rhs.inner.0.into();
        (a == b).into()
    }
}

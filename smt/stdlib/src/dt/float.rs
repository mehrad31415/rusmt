use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::FromPrimitive;

use crate::{Boolean, F32, F64, Integer, Real, SymbolicBitVec, SymbolicFloat, smt::SMT};
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Rounding modes
#[derive(Debug, Clone, Copy, Hash)]
pub enum RoundingMode {
    /// Round to Nearest, ties to Even (the default for most hardware).
    RNE,
    /// Round to Nearest, ties to Away from zero.
    RNA,
    /// Round Toward Positive infinity.
    RTP,
    /// Round Toward Negative infinity.
    RTN,
    /// Round Toward Zero (truncate).
    RTZ,
}

/// Rounding helper function
fn apply_rounding(val: f64, rm: RoundingMode) -> f64 {
    match rm {
        // Round Nearest, ties to Even is the default behavior of f64 operations.
        RoundingMode::RNE => val.round_ties_even(),
        // Round Toward Zero is truncation.
        RoundingMode::RTZ => val.trunc(),
        // Round Toward Positive Infinity is ceiling.
        RoundingMode::RTP => val.ceil(),
        // Round Toward Negative Infinity is floor.
        RoundingMode::RTN => val.floor(),
        // Round to Nearest, ties Away from zero.
        RoundingMode::RNA => val.round(),
    }
}

/// Constructors for SymbolicFloat.
impl<const EB: usize, const SB: usize> SymbolicFloat<EB, SB> {
    /// Creates a Not-a-Number (NaN) value.
    pub fn nan() -> Self {
        Self {
            inner: f64::NAN,
            ..Default::default()
        }
    }

    /// Creates a positive infinity value.
    pub fn infinity() -> Self {
        Self {
            inner: f64::INFINITY,
            ..Default::default()
        }
    }

    /// Creates a negative infinity value.
    pub fn neg_infinity() -> Self {
        Self {
            inner: f64::NEG_INFINITY,
            ..Default::default()
        }
    }

    /// Creates a positive zero value.
    pub fn pos_zero() -> Self {
        Self {
            inner: 0.0,
            ..Default::default()
        }
    }

    /// Creates a negative zero value.
    pub fn neg_zero() -> Self {
        Self {
            inner: -0.0,
            ..Default::default()
        }
    }
}

/// Arithmetic operations for SymbolicFloat.
impl<const EB: usize, const SB: usize> SymbolicFloat<EB, SB> {
    /// addition
    /// The transpiler should generate the `(fp.add rm t1 t2)` SMT-LIB expression.
    pub fn add(self, rm: RoundingMode, rhs: Self) -> Self {
        Self {
            inner: apply_rounding(self.inner + rhs.inner, rm),
            ..self
        }
    }

    /// subtraction
    /// The transpiler should generate `(fp.sub rm t1 t2)`.
    pub fn sub(self, rm: RoundingMode, rhs: Self) -> Self {
        Self {
            inner: apply_rounding(self.inner - rhs.inner, rm),
            ..self
        }
    }

    /// multiplication
    /// The transpiler should generate `(fp.mul rm t1 t2)`.
    pub fn mul(self, rm: RoundingMode, rhs: Self) -> Self {
        Self {
            inner: apply_rounding(self.inner * rhs.inner, rm),
            ..self
        }
    }

    /// division
    /// The transpiler should generate `(fp.div rm t1 t2)`.
    pub fn div(self, rm: RoundingMode, rhs: Self) -> Self {
        Self {
            inner: apply_rounding(self.inner / rhs.inner, rm),
            ..self
        }
    }

    /// negation
    pub fn neg(self) -> Self {
        Self {
            inner: -self.inner,
            ..self
        }
    }

    /// absolute value
    pub fn abs(self) -> Self {
        Self {
            inner: self.inner.abs(),
            ..self
        }
    }

    /// Floating-point remainder. `(fp.rem t1 t2)`
    pub fn rem(self, rhs: Self) -> Self {
        Self {
            inner: self.inner % rhs.inner,
            ..self
        }
    }

    /// Floating-point square root. `(fp.sqrt rm t)`
    pub fn sqrt(self, rm: RoundingMode) -> Self {
        Self {
            inner: apply_rounding(self.inner.sqrt(), rm),
            ..self
        }
    }

    /// Minimum of floating-point numbers. `(fp.min t1 t2)`
    pub fn min(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.min(rhs.inner),
            ..self
        }
    }

    /// Maximum of floating-point numbers. `(fp.max t1 t2)`
    pub fn max(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.max(rhs.inner),
            ..self
        }
    }
}

/// Comparison operations for SymbolicFloat.
impl<const EB: usize, const SB: usize> SymbolicFloat<EB, SB> {
    /// less than
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    /// less than or equal to
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    /// greater than
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    /// greater than or equal to
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    /// is NaN `(fp.isNaN X)`
    pub fn is_nan(self) -> Boolean {
        self.inner.is_nan().into()
    }

    /// is infinite `(fp.isInfinite X)`
    pub fn is_infinite(self) -> Boolean {
        self.inner.is_infinite().into()
    }

    /// is zero `(fp.isZero X)`
    pub fn is_zero(self) -> Boolean {
        (self.inner == 0.0 || self.inner == -0.0).into()
    }

    /// is negative `(fp.isNegative X)`
    pub fn is_negative(self) -> Boolean {
        self.inner.is_sign_negative().into()
    }

    /// is positive `(fp.isPositive X)`
    pub fn is_positive(self) -> Boolean {
        self.inner.is_sign_positive().into()
    }

    /// is normal `(fp.isNormal t)`
    pub fn is_normal(self) -> Boolean {
        self.inner.is_normal().into()
    }

    /// `(fp.isSubnormal t)`
    pub fn is_subnormal(self) -> Boolean {
        self.inner.is_subnormal().into()
    }
}

impl<const EB: usize, const SB: usize> PartialEq for SymbolicFloat<EB, SB> {
    fn eq(&self, rhs: &Self) -> bool {
        self.inner == rhs.inner
    }
}

impl<const EB: usize, const SB: usize> Eq for SymbolicFloat<EB, SB> {}

impl<const EB: usize, const SB: usize> Hash for SymbolicFloat<EB, SB> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.to_bits().hash(state);
    }
}

impl<const EB: usize, const SB: usize> SMT for SymbolicFloat<EB, SB> {
    fn _cmp(self, rhs: Self) -> Ordering {
        self.inner.total_cmp(&rhs.inner)
    }
}

/// Conversions from SymbolicFloat to other numeric types.
impl<const EB: usize, const SB: usize> SymbolicFloat<EB, SB> {
    /// LOSSY (Truncation) & FALLIBLE (NaN/Infinity): Converts a float to an Integer. `Z3_mk_fpa_round_to_integral`
    pub fn to_integer(self) -> Option<Integer> {
        if !self.inner.is_finite() {
            return None; // Cannot convert NaN or Infinity.
        }

        BigInt::from_f64(self.inner.trunc()).map(|bi| Integer {
            inner: Intern::new(bi),
        })
    }

    /// FALLIBLE (NaN/Infinity): Converts a float to a Real.
    pub fn to_real(self) -> Option<Real> {
        if !self.inner.is_finite() {
            return None; // Cannot convert NaN or Infinity.
        }

        BigRational::from_float(self.inner).map(|br| Real {
            inner: Intern::new(br),
        })
    }

    /// LOSSY (Truncation/Overflow) & FALLIBLE (NaN/Infinity): Converts a float to a BitVector.
    pub fn to_bitvec<const N: usize>(self) -> Option<SymbolicBitVec<N>> {
        // This works by chaining the conversions:
        // 1. Try to convert Float -> Integer (handles NaN/Infinity and truncation).
        // 2. If successful, try to convert Integer -> BitVector (handles overflow).
        self.to_integer().and_then(|int| int.to_bitvec::<N>())
    }
}

impl From<f32> for F32 {
    /// from_f32() creates a F32 from a f32.
    fn from(f: f32) -> Self {
        Self {
            inner: f as f64,
            _phantom: PhantomData,
        }
    }
}

impl From<f64> for F64 {
    /// from_f64() creates a F64 from a f64.
    fn from(f: f64) -> Self {
        Self {
            inner: f,
            _phantom: PhantomData,
        }
    }
}

macro_rules! f32_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for F32 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as f64,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for F32 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as f64,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

f32_from_literal_int!(i8, i16, i32, i64, i128, isize);
f32_from_literal_int!(u8, u16, u32, u64, u128, usize);

macro_rules! f64_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for F64 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as f64,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for F64 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as f64,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

f64_from_literal_int!(i8, i16, i32, i64, i128, isize);
f64_from_literal_int!(u8, u16, u32, u64, u128, usize);

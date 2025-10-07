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

/// Rounding modes for floating-point operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// This is a software simulation, as Rust does not provide native controls.
fn apply_rounding(val: f64, rm: RoundingMode) -> f64 {
    match rm {
        // Round Nearest, ties to Even is the default behavior of f64 operations.
        RoundingMode::RNE => val,
        // Round Toward Zero is truncation.
        RoundingMode::RTZ => val.trunc(),
        // Round Toward Positive Infinity is ceiling.
        RoundingMode::RTP => val.ceil(),
        // Round Toward Negative Infinity is floor.
        RoundingMode::RTN => val.floor(),
        // Round to Nearest, ties Away from zero.
        RoundingMode::RNA => {
            let frac = val.fract();
            if frac.abs() != 0.5 {
                val.round() // .round() is ties-to-even, but correct for non-0.5 fractions.
            } else if val.is_sign_positive() {
                val.ceil() // Tie goes away from zero (up for positive).
            } else {
                val.floor() // Tie goes away from zero (down for negative).
            }
        }
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

    /// is NaN
    pub fn is_nan(self) -> Boolean {
        self.inner.is_nan().into()
    }

    /// is infinite
    pub fn is_infinite(self) -> Boolean {
        self.inner.is_infinite().into()
    }

    /// is zero
    pub fn is_zero(self) -> Boolean {
        (self.inner == 0.0 || self.inner == -0.0).into()
    }

    /// is negative
    pub fn is_negative(self) -> Boolean {
        self.inner.is_sign_negative().into()
    }

    /// is positive
    pub fn is_positive(self) -> Boolean {
        self.inner.is_sign_positive().into()
    }

    /// is normal
    pub fn is_normal(self) -> Boolean {
        self.inner.is_normal().into()
    }

    /// This corresponds to the `(fp.isSubnormal t)` SMT-LIB function.
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
    /// LOSSY (Truncation) & FALLIBLE (NaN/Infinity): Converts a float to an Integer.
    pub fn to_integer(self) -> Option<Integer> {
        if !self.inner.is_finite() {
            return None; // Cannot convert NaN or Infinity.
        }
        // `from_f64` handles the conversion from a float to a BigInt.
        // returns None for non-finite values.
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
        // This works by chaining the fallible conversions:
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

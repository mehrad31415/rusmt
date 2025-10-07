use std::marker::PhantomData;

use crate::{Boolean, F32, F64, I32, I64, Integer, Real, SymbolicBitVec};
use internment::Intern;
use num_bigint::BigInt;

impl From<i32> for I32 {
    fn from(val: i32) -> Self {
        Self {
            inner: val as i128,
            _phantom: PhantomData,
        }
    }
}

impl From<i64> for I64 {
    fn from(val: i64) -> Self {
        Self {
            inner: val as i128,
            _phantom: PhantomData,
        }
    }
}

/// Arithmetic Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// Transpiler Mapping: `(bvadd a b)`
    pub fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_add(rhs.inner),
            ..self
        }
    }

    /// Transpiler Mapping: `(bvsub a b)`
    pub fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_sub(rhs.inner),
            ..self
        }
    }

    /// Transpiler Mapping: `(bvmul a b)`
    pub fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_mul(rhs.inner),
            ..self
        }
    }

    /// Transpiler Mapping: `(bvneg a)`
    pub fn bv_neg(self) -> Self {
        Self {
            inner: self.inner.wrapping_neg(),
            ..self
        }
    }

    /// Transpiler Mapping: `(bvsdiv a b)`
    pub fn bv_sdiv(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_div(rhs.inner)
            .map(|inner| Self { inner, ..self })
    }

    /// Transpiler Mapping: `(bvudiv a b)`
    pub fn bv_udiv(self, rhs: Self) -> Option<Self> {
        (self.inner as u128)
            .checked_div(rhs.inner as u128)
            .map(|inner| Self {
                inner: inner as i128,
                ..self
            })
    }

    /// Transpiler Mapping: `(bvsrem a b)`
    pub fn bv_srem(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_rem(rhs.inner)
            .map(|inner| Self { inner, ..self })
    }

    /// Transpiler Mapping: `(bvurem a b)`
    pub fn bv_urem(self, rhs: Self) -> Option<Self> {
        (self.inner as u128)
            .checked_rem(rhs.inner as u128)
            .map(|inner| Self {
                inner: inner as i128,
                ..self
            })
    }

    /// Transpiler Mapping: `(bvsmod a b)`
    pub fn bv_smod(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_rem_euclid(rhs.inner)
            .map(|inner| Self { inner, ..self })
    }
}

/// Bitwise Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// Transpiler Mapping: `(bvand a b)`
    pub fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: self.inner & rhs.inner,
            ..self
        }
    }

    /// Transpiler Mapping: `(bvor a b)`
    pub fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: self.inner | rhs.inner,
            ..self
        }
    }

    /// Transpiler Mapping: `(bvxor a b)`
    pub fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: self.inner ^ rhs.inner,
            ..self
        }
    }

    /// Transpiler Mapping: `(bvnot a)`
    pub fn bv_not(self) -> Self {
        Self {
            inner: !self.inner,
            ..self
        }
    }

    /// Transpiler Mapping: `(bvnand a b)`
    pub fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    /// Transpiler Mapping: `(bvnor a b)`
    pub fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    /// Transpiler Mapping: `(bvxnor a b)`
    pub fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }
}

/// Shift Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// Transpiler Mapping: `(bvshl a b)`
    pub fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shl(rhs.inner as u32),
            ..self
        }
    }

    /// Transpiler Mapping: `(bvlshr a b)`
    pub fn bv_lshr(self, rhs: Self) -> Self {
        Self {
            inner: ((self.inner as u128).wrapping_shr(rhs.inner as u32)) as i128,
            ..self
        }
    }

    /// Transpiler Mapping: `(bvashr a b)`
    pub fn bv_ashr(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shr(rhs.inner as u32),
            ..self
        }
    }
}

/// Comparison Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// Transpiler Mapping: `(bvslt a b)`
    pub fn bv_slt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    /// Transpiler Mapping: `(bvult a b)`
    pub fn bv_ult(self, rhs: Self) -> Boolean {
        ((self.inner as u128) < (rhs.inner as u128)).into()
    }

    /// Transpiler Mapping: `(bvsle a b)`
    pub fn bv_sle(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    /// Transpiler Mapping: `(bvule a b)`
    pub fn bv_ule(self, rhs: Self) -> Boolean {
        ((self.inner as u128) <= (rhs.inner as u128)).into()
    }

    /// Transpiler Mapping: `(bvsgt a b)`
    pub fn bv_sgt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    /// Transpiler Mapping: `(bvugt a b)`
    pub fn bv_ugt(self, rhs: Self) -> Boolean {
        ((self.inner as u128) > (rhs.inner as u128)).into()
    }

    /// Transpiler Mapping: `(bvsge a b)`
    pub fn bv_sge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    /// Transpiler Mapping: `(bvuge a b)`
    pub fn bv_uge(self, rhs: Self) -> Boolean {
        ((self.inner as u128) >= (rhs.inner as u128)).into()
    }
}

/// Conversion Methods
impl<const N: usize> SymbolicBitVec<N> {
    /// to_int() converts the bitvector to a signed integer type.
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(self.inner)),
        }
    }

    /// to_real() converts the bitvector to a real type.
    pub fn to_real(self) -> Real {
        self.to_int().to_real()
    }

    /// LOSSY (Rounding): Converts a BitVector to an F32.
    /// Large values may lose precision.
    pub fn to_f32(self) -> F32 {
        F32::from(self.inner as f32)
    }

    /// LOSSY (Rounding): Converts a BitVector to an F64.
    /// Large values may lose precision.
    pub fn to_f64(self) -> F64 {
        F64::from(self.inner as f64)
    }
}

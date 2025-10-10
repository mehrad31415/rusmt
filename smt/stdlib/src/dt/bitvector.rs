use std::marker::PhantomData;

use crate::{Boolean, F32, F64, I32, I64, Integer, Real, SymbolicBitVec};
use internment::Intern;
use num_bigint::BigInt;
use num_traits::FromPrimitive;

/// Bitwise Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvnot a)`
    pub fn bv_not(self) -> Self {
        Self {
            inner: !self.inner,
            ..self
        }
    }

    /// `(bvredand a)`
    pub fn bv_redand(self) -> SymbolicBitVec<1> {
        let mask = if N == 128 { -1i128 } else { (1i128 << N) - 1 };

        SymbolicBitVec {
            inner: if (self.inner & mask) == mask { 1 } else { 0 },
            _phantom: PhantomData,
        }
    }

    /// `(bvredor a)`
    pub fn bv_redor(self) -> SymbolicBitVec<1> {
        let mask = if N == 128 { -1i128 } else { (1i128 << N) - 1 };

        SymbolicBitVec {
            inner: if (self.inner & mask) != 0 { 1 } else { 0 },
            _phantom: PhantomData,
        }
    }

    /// `(bvand a b)`
    pub fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: self.inner & rhs.inner,
            ..self
        }
    }

    /// `(bvor a b)`
    pub fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: self.inner | rhs.inner,
            ..self
        }
    }

    /// `(bvxor a b)`
    pub fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: self.inner ^ rhs.inner,
            ..self
        }
    }

    /// `(bvnand a b)`
    pub fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    /// `(bvnor a b)`
    pub fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    /// `(bvxnor a b)`
    pub fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }
}

/// Arithmetic Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvneg a)`
    pub fn bv_neg(self) -> Self {
        Self {
            inner: self.inner.wrapping_neg(),
            ..self
        }
    }

    /// `(bvadd a b)`
    pub fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_add(rhs.inner),
            ..self
        }
    }

    /// `(bvsub a b)`
    pub fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_sub(rhs.inner),
            ..self
        }
    }

    /// `(bvmul a b)`
    pub fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_mul(rhs.inner),
            ..self
        }
    }

    /// `(bvsdiv a b)`
    pub fn bv_sdiv(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_div(rhs.inner)
            .map(|inner| Self { inner, ..self })
    }

    /// `(bvudiv a b)`
    pub fn bv_udiv(self, rhs: Self) -> Option<Self> {
        (self.inner as u128)
            .checked_div(rhs.inner as u128)
            .map(|inner| Self {
                inner: inner as i128,
                ..self
            })
    }

    /// `(bvsrem a b)`
    pub fn bv_srem(self, rhs: Self) -> Option<Self> {
        self.inner
            .checked_rem(rhs.inner)
            .map(|inner| Self { inner, ..self })
    }

    /// `(bvurem a b)`
    pub fn bv_urem(self, rhs: Self) -> Option<Self> {
        (self.inner as u128)
            .checked_rem(rhs.inner as u128)
            .map(|inner| Self {
                inner: inner as i128,
                ..self
            })
    }

    /// `(bvsmod a b)`
    pub fn bv_smod(self, rhs: Self) -> Option<Self> {
        self.inner.checked_rem_euclid(rhs.inner).map(|inner| {
            if rhs.inner < 0 {
                Self {
                    inner: inner + rhs.inner,
                    ..self
                }
            } else {
                Self { inner, ..self }
            }
        })
    }

    /// Z3_mk_bvadd_no_overflow
    pub fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvsub_no_overflow
    pub fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvneg_no_overflow
    pub fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    /// Z3_mk_bvmul_no_overflow
    pub fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvsdiv_no_overflow
    pub fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_div(rhs.inner).is_some().into()
    }
}

/// Shift Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvshl a b)`
    pub fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shl(rhs.inner as u32),
            ..self
        }
    }

    /// `(bvlshr a b)`
    pub fn bv_lshr(self, rhs: Self) -> Self {
        Self {
            inner: ((self.inner as u128).wrapping_shr(rhs.inner as u32)) as i128,
            ..self
        }
    }

    /// `(bvashr a b)`
    pub fn bv_ashr(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shr(rhs.inner as u32),
            ..self
        }
    }

    /// `(rotate_left a b)`
    pub fn bv_rotate_left(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.rotate_left(rhs.inner as u32),
            ..self
        }
    }

    /// `(rotate_right a b)`
    pub fn bv_rotate_right(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.rotate_right(rhs.inner as u32),
            ..self
        }
    }
}

/// Comparison Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvslt a b)`
    pub fn bv_slt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    /// `(bvult a b)`
    pub fn bv_ult(self, rhs: Self) -> Boolean {
        ((self.inner as u128) < (rhs.inner as u128)).into()
    }

    /// `(bvsle a b)`
    pub fn bv_sle(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    /// `(bvule a b)`
    pub fn bv_ule(self, rhs: Self) -> Boolean {
        ((self.inner as u128) <= (rhs.inner as u128)).into()
    }

    /// `(bvsgt a b)`
    pub fn bv_sgt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    /// `(bvugt a b)`
    pub fn bv_ugt(self, rhs: Self) -> Boolean {
        ((self.inner as u128) > (rhs.inner as u128)).into()
    }

    /// `(bvsge a b)`
    pub fn bv_sge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    /// `(bvuge a b)`
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
    pub fn to_f32(self) -> Option<F32> {
        let f32_val = self.inner as f32;

        if let Some(round_tripped_bigint) = BigInt::from_f32(f32_val) {
            if round_tripped_bigint == BigInt::from(self.inner) {
                return Some(F32::from(f32_val));
            }
        }
        None
    }

    /// LOSSY (Rounding): Converts a BitVector to an F64.
    pub fn to_f64(self) -> Option<F64> {
        let f64_val = self.inner as f64;

        if let Some(round_tripped_bigint) = BigInt::from_f64(f64_val) {
            if round_tripped_bigint == BigInt::from(self.inner) {
                return Some(F64::from(f64_val));
            }
        }
        None
    }
}

macro_rules! i32_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for I32 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for I32 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

i32_from_literal_int!(i8, i16, i32, i64, i128, isize);
i32_from_literal_int!(u8, u16, u32, u64, u128, usize);

macro_rules! i64_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for I64 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for I64 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

i64_from_literal_int!(i8, i16, i32, i64, i128, isize);
i64_from_literal_int!(u8, u16, u32, u64, u128, usize);

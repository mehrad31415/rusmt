//! Bitvector datatype and operations

use crate::{Boolean, I32, I64, Integer, U32, U64, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;

/// Common trait for all bitvector operations
pub trait BitvectorOps: Sized + SMT {
    /// `(bvnot a)`
    fn bv_not(self) -> Self;
    /// `(bvredand a)`
    fn bv_redand(self) -> Boolean;
    /// `(bvredor a)`
    fn bv_redor(self) -> Boolean;
    /// `(bvand a b)`
    fn bv_and(self, rhs: Self) -> Self;
    /// `(bvor a b)`
    fn bv_or(self, rhs: Self) -> Self;
    /// `(bvxor a b)`
    fn bv_xor(self, rhs: Self) -> Self;
    /// `(bvnand a b)`
    fn bv_nand(self, rhs: Self) -> Self;
    /// `(bvnor a b)`
    fn bv_nor(self, rhs: Self) -> Self;
    /// `(bvxnor a b)`
    fn bv_xnor(self, rhs: Self) -> Self;

    /// `(bvneg a)`
    fn bv_neg(self) -> Self;
    /// `(bvadd a b)`
    fn bv_add(self, rhs: Self) -> Self;
    /// `(bvsub a b)`
    fn bv_sub(self, rhs: Self) -> Self;
    /// `(bvmul a b)`
    fn bv_mul(self, rhs: Self) -> Self;
    /// `(bvsdiv a b)` or `(bvudiv a b)`
    fn bv_div(self, rhs: Self) -> Self;
    /// `(bvsrem a b)` or `(bvurem a b)`
    fn bv_rem(self, rhs: Self) -> Self;
    /// `(bvsmod a b)`
    fn bv_mod(self, rhs: Self) -> Self;

    /// addition overflow predicate
    fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean;
    /// subtraction overflow predicate
    fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean;
    /// negation overflow predicate
    fn checked_bvneg_no_overflow(self) -> Boolean;
    /// multiplication overflow predicate
    fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean;
    /// division overflow predicate
    fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean;

    /// `(bvshl a b)`
    fn bv_shl(self, rhs: Self) -> Self;
    /// `(bvlshr a b)`
    fn bv_lshr(self, rhs: Self) -> Self;
    /// `(bvashr a b)`
    fn bv_ashr(self, rhs: Self) -> Self;
    /// `(_ rotate_left self) rhs)`
    fn bv_rotate_left(self, rhs: Self) -> Self;
    /// `(_ rotate_right self) rhs)`
    fn bv_rotate_right(self, rhs: Self) -> Self;

    /// `(bvslt a b)` or `(bvult a b)`
    fn bv_lt(self, rhs: Self) -> Boolean;
    /// `(bvsle a b)` or `(bvule a b)`
    fn bv_le(self, rhs: Self) -> Boolean;
    /// `(bvsgt a b)` or `(bvugt a b)`
    fn bv_gt(self, rhs: Self) -> Boolean;
    /// `(bvsge a b)` or `(bvuge a b)`
    fn bv_ge(self, rhs: Self) -> Boolean;

    /// bv2int conversion
    fn to_int(self) -> Integer;
}

impl BitvectorOps for I32 {
    fn bv_not(self) -> Self {
        Self {
            inner: Intern::new(!*self.inner),
        }
    }

    fn bv_redand(self) -> Boolean {
        (*self.inner == -1i32).into()
    }

    fn bv_redor(self) -> Boolean {
        (*self.inner != 0i32).into()
    }

    fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner & *rhs.inner),
        }
    }

    fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner | *rhs.inner),
        }
    }

    fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner ^ *rhs.inner),
        }
    }

    fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }

    fn bv_neg(self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_neg()),
        }
    }

    fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_add(*rhs.inner)),
        }
    }

    fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_sub(*rhs.inner)),
        }
    }

    fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_mul(*rhs.inner)),
        }
    }

    fn bv_div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_div(*rhs.inner)),
        }
    }

    fn bv_rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_rem(*rhs.inner)),
        }
    }

    fn bv_mod(self, rhs: Self) -> Self {
        let a = *self.inner;
        let b = *rhs.inner;
        let rem = a.wrapping_rem(b);
        if rem == 0 || (rem > 0 && b > 0) || (rem < 0 && b < 0) {
            Self {
                inner: Intern::new(rem),
            }
        } else {
            Self {
                inner: Intern::new(rem.wrapping_add(b)),
            }
        }
    }

    fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(*rhs.inner).is_some().into()
    }

    fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(*rhs.inner).is_some().into()
    }

    fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(*rhs.inner).is_some().into()
    }

    fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        if *rhs.inner == 0 {
            false.into()
        } else {
            self.inner.checked_div(*rhs.inner).is_some().into()
        }
    }

    fn bv_shl(self, rhs: Self) -> Self {
        let shift_amt = *rhs.inner as u32;
        Self {
            inner: Intern::new(self.inner.wrapping_shl(shift_amt)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        let shift_amt = *rhs.inner as u32;
        let unsigned_val = *self.inner as u32;
        Self {
            inner: Intern::new(unsigned_val.wrapping_shr(shift_amt) as i32),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        let shift_amt = *rhs.inner as u32;
        Self {
            inner: Intern::new(self.inner.wrapping_shr(shift_amt)),
        }
    }

    fn bv_rotate_left(self, rhs: Self) -> Self {
        let rotation_amt = *rhs.inner as u32;
        let unsigned_val = *self.inner as u32;
        Self {
            inner: Intern::new(unsigned_val.rotate_left(rotation_amt) as i32),
        }
    }

    fn bv_rotate_right(self, rhs: Self) -> Self {
        let rotation_amt = *rhs.inner as u32;
        let unsigned_val = *self.inner as u32;
        Self {
            inner: Intern::new(unsigned_val.rotate_right(rotation_amt) as i32),
        }
    }

    fn bv_lt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner < rhs.inner,
        }
    }

    fn bv_le(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner <= rhs.inner,
        }
    }

    fn bv_gt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner > rhs.inner,
        }
    }

    fn bv_ge(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner >= rhs.inner,
        }
    }

    fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(*self.inner)),
        }
    }
}

impl BitvectorOps for U32 {
    fn bv_not(self) -> Self {
        Self {
            inner: Intern::new(!*self.inner),
        }
    }

    fn bv_redand(self) -> Boolean {
        (*self.inner == u32::MAX).into()
    }

    fn bv_redor(self) -> Boolean {
        (*self.inner != 0u32).into()
    }

    fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner & *rhs.inner),
        }
    }

    fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner | *rhs.inner),
        }
    }

    fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner ^ *rhs.inner),
        }
    }

    fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }

    fn bv_neg(self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_neg()),
        }
    }

    fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_add(*rhs.inner)),
        }
    }

    fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_sub(*rhs.inner)),
        }
    }

    fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_mul(*rhs.inner)),
        }
    }

    fn bv_div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_div(*rhs.inner)),
        }
    }

    fn bv_rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_rem(*rhs.inner)),
        }
    }

    fn bv_mod(self, rhs: Self) -> Self {
        self.bv_rem(rhs)
    }

    fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(*rhs.inner).is_some().into()
    }

    fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(*rhs.inner).is_some().into()
    }

    fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(*rhs.inner).is_some().into()
    }

    fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        if *rhs.inner == 0 {
            false.into()
        } else {
            self.inner.checked_div(*rhs.inner).is_some().into()
        }
    }

    fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_shl(*rhs.inner)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).wrapping_shr(*rhs.inner)),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        let signed_val = *self.inner as i32;
        let shift_amt = *rhs.inner;
        let result = signed_val.wrapping_shr(shift_amt);
        Self {
            inner: Intern::new(result as u32),
        }
    }

    fn bv_rotate_left(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).rotate_left(*rhs.inner)),
        }
    }

    fn bv_rotate_right(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).rotate_right(*rhs.inner)),
        }
    }

    fn bv_lt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner < rhs.inner,
        }
    }

    fn bv_le(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner <= rhs.inner,
        }
    }

    fn bv_gt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner > rhs.inner,
        }
    }

    fn bv_ge(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner >= rhs.inner,
        }
    }

    fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(*self.inner)),
        }
    }
}

impl BitvectorOps for I64 {
    fn bv_not(self) -> Self {
        Self {
            inner: Intern::new(!*self.inner),
        }
    }

    fn bv_redand(self) -> Boolean {
        (*self.inner == -1i64).into()
    }

    fn bv_redor(self) -> Boolean {
        (*self.inner != 0i64).into()
    }

    fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner & *rhs.inner),
        }
    }

    fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner | *rhs.inner),
        }
    }

    fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner ^ *rhs.inner),
        }
    }

    fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }

    fn bv_neg(self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_neg()),
        }
    }

    fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_add(*rhs.inner)),
        }
    }

    fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_sub(*rhs.inner)),
        }
    }

    fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_mul(*rhs.inner)),
        }
    }

    fn bv_div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_div(*rhs.inner)),
        }
    }

    fn bv_rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_rem(*rhs.inner)),
        }
    }

    fn bv_mod(self, rhs: Self) -> Self {
        let a = *self.inner;
        let b = *rhs.inner;
        let rem = a.wrapping_rem(b);
        if rem == 0 || (rem > 0 && b > 0) || (rem < 0 && b < 0) {
            Self {
                inner: Intern::new(rem),
            }
        } else {
            Self {
                inner: Intern::new(rem.wrapping_add(b)),
            }
        }
    }

    fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(*rhs.inner).is_some().into()
    }

    fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(*rhs.inner).is_some().into()
    }

    fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(*rhs.inner).is_some().into()
    }

    fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        if *rhs.inner == 0 {
            false.into()
        } else {
            self.inner.checked_div(*rhs.inner).is_some().into()
        }
    }

    fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_shl(*rhs.inner as u32)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        let unsigned_val = *self.inner as u64;
        let shift_amt = *rhs.inner as u32;
        Self {
            inner: Intern::new(unsigned_val.wrapping_shr(shift_amt) as i64),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_shr(*rhs.inner as u32)),
        }
    }

    fn bv_rotate_left(self, rhs: Self) -> Self {
        let rotation_amt = *rhs.inner as u32;
        let unsigned_val = *self.inner as u64;
        Self {
            inner: Intern::new(unsigned_val.rotate_left(rotation_amt) as i64),
        }
    }

    fn bv_rotate_right(self, rhs: Self) -> Self {
        let rotation_amt = *rhs.inner as u32;
        let unsigned_val = *self.inner as u64;
        Self {
            inner: Intern::new(unsigned_val.rotate_right(rotation_amt) as i64),
        }
    }

    fn bv_lt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner < rhs.inner,
        }
    }

    fn bv_le(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner <= rhs.inner,
        }
    }

    fn bv_gt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner > rhs.inner,
        }
    }

    fn bv_ge(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner >= rhs.inner,
        }
    }

    fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(*self.inner)),
        }
    }
}

impl BitvectorOps for U64 {
    fn bv_not(self) -> Self {
        Self {
            inner: Intern::new(!*self.inner),
        }
    }

    fn bv_redand(self) -> Boolean {
        (*self.inner == u64::MAX).into()
    }

    fn bv_redor(self) -> Boolean {
        (*self.inner != 0u64).into()
    }

    fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner & *rhs.inner),
        }
    }

    fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner | *rhs.inner),
        }
    }

    fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(*self.inner ^ *rhs.inner),
        }
    }

    fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }

    fn bv_neg(self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_neg()),
        }
    }

    fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_add(*rhs.inner)),
        }
    }

    fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_sub(*rhs.inner)),
        }
    }

    fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_mul(*rhs.inner)),
        }
    }

    fn bv_div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_div(*rhs.inner)),
        }
    }

    fn bv_rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_rem(*rhs.inner)),
        }
    }

    fn bv_mod(self, rhs: Self) -> Self {
        self.bv_rem(rhs)
    }

    fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(*rhs.inner).is_some().into()
    }

    fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(*rhs.inner).is_some().into()
    }

    fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(*rhs.inner).is_some().into()
    }

    fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        if *rhs.inner == 0 {
            false.into()
        } else {
            self.inner.checked_div(*rhs.inner).is_some().into()
        }
    }

    fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.wrapping_shl(*rhs.inner as u32)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).wrapping_shr(*rhs.inner as u32)),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        let signed_val = *self.inner as i64;
        let shift_amt = *rhs.inner as u32;
        let result = signed_val.wrapping_shr(shift_amt);
        Self {
            inner: Intern::new(result as u64),
        }
    }

    fn bv_rotate_left(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).rotate_left(*rhs.inner as u32)),
        }
    }

    fn bv_rotate_right(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new((*self.inner).rotate_right(*rhs.inner as u32)),
        }
    }

    fn bv_lt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner < rhs.inner,
        }
    }

    fn bv_le(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner <= rhs.inner,
        }
    }

    fn bv_gt(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner > rhs.inner,
        }
    }

    fn bv_ge(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner >= rhs.inner,
        }
    }

    fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(*self.inner)),
        }
    }
}

impl From<u32> for U32 {
    fn from(value: u32) -> Self {
        Self {
            inner: Intern::new(value),
        }
    }
}

impl From<i32> for I32 {
    fn from(value: i32) -> Self {
        Self {
            inner: Intern::new(value),
        }
    }
}

impl From<u64> for U64 {
    fn from(value: u64) -> Self {
        Self {
            inner: Intern::new(value),
        }
    }
}

impl From<i64> for I64 {
    fn from(value: i64) -> Self {
        Self {
            inner: Intern::new(value),
        }
    }
}

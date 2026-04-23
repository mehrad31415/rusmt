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
    /// bv2int conversion `(bv2int self)`
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

    fn bv_shl(self, rhs: Self) -> Self {
        // Treat shift amount as the unsigned bit-pattern of the BV.
        // Z3's bvshl returns 0 when shift >= bit-width; wrapping_shl would wrap instead.
        let shift_amt = *rhs.inner as u32;
        if shift_amt >= 32 {
            return Self {
                inner: Intern::new(0i32),
            };
        }
        Self {
            inner: Intern::new(self.inner.wrapping_shl(shift_amt)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        // Z3's bvlshr returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner as u32;
        if shift_amt >= 32 {
            return Self {
                inner: Intern::new(0i32),
            };
        }
        let unsigned_val = *self.inner as u32;
        Self {
            inner: Intern::new(unsigned_val.wrapping_shr(shift_amt) as i32),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        // Z3's bvashr with shift >= bit-width fills with the sign bit.
        // Shifting by 31 propagates the sign bit to all positions.
        let shift_amt = *rhs.inner as u32;
        let shift = if shift_amt >= 32 { 31 } else { shift_amt };
        Self {
            inner: Intern::new(self.inner.wrapping_shr(shift)),
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

    fn bv_shl(self, rhs: Self) -> Self {
        // Z3's bvshl returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner;
        if shift_amt >= 32 {
            return Self {
                inner: Intern::new(0u32),
            };
        }
        Self {
            inner: Intern::new(self.inner.wrapping_shl(shift_amt)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        // Z3's bvlshr returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner;
        if shift_amt >= 32 {
            return Self {
                inner: Intern::new(0u32),
            };
        }
        Self {
            inner: Intern::new((*self.inner).wrapping_shr(shift_amt)),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        // Z3's bvashr with shift >= bit-width fills with the sign bit.
        let signed_val = *self.inner as i32;
        let shift_amt = *rhs.inner;
        let shift = if shift_amt >= 32 { 31 } else { shift_amt };
        let result = signed_val.wrapping_shr(shift);
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

    fn bv_shl(self, rhs: Self) -> Self {
        // Interpret shift amount as unsigned (the raw bit-pattern).
        // Z3's bvshl returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner as u64;
        if shift_amt >= 64 {
            return Self {
                inner: Intern::new(0i64),
            };
        }
        Self {
            inner: Intern::new(self.inner.wrapping_shl(shift_amt as u32)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        // Z3's bvlshr returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner as u64;
        if shift_amt >= 64 {
            return Self {
                inner: Intern::new(0i64),
            };
        }
        let unsigned_val = *self.inner as u64;
        Self {
            inner: Intern::new(unsigned_val.wrapping_shr(shift_amt as u32) as i64),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        // Z3's bvashr with shift >= bit-width fills with the sign bit.
        let shift_amt = *rhs.inner as u64;
        let shift = if shift_amt >= 64 {
            63
        } else {
            shift_amt as u32
        };
        Self {
            inner: Intern::new(self.inner.wrapping_shr(shift)),
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

    fn bv_shl(self, rhs: Self) -> Self {
        // Z3's bvshl returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner;
        if shift_amt >= 64 {
            return Self {
                inner: Intern::new(0u64),
            };
        }
        Self {
            inner: Intern::new(self.inner.wrapping_shl(shift_amt as u32)),
        }
    }

    fn bv_lshr(self, rhs: Self) -> Self {
        // Z3's bvlshr returns 0 when shift >= bit-width.
        let shift_amt = *rhs.inner;
        if shift_amt >= 64 {
            return Self {
                inner: Intern::new(0u64),
            };
        }
        Self {
            inner: Intern::new((*self.inner).wrapping_shr(shift_amt as u32)),
        }
    }

    fn bv_ashr(self, rhs: Self) -> Self {
        // Z3's bvashr with shift >= bit-width fills with the sign bit.
        let signed_val = *self.inner as i64;
        let shift_amt = *rhs.inner;
        let shift = if shift_amt >= 64 {
            63
        } else {
            shift_amt as u32
        };
        let result = signed_val.wrapping_shr(shift);
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

mod tests {
    #[test]
    fn test_bv_val() {
        use crate::{I32, U32, smt::SMT};

        assert!(*I32::from(0).eq(I32::from(0)));
        assert!(*I32::from(-1).eq(I32::from(-1)));
        assert!(*I32::from(i32::MAX).eq(I32::from(2147483647)));
        assert!(*I32::from(i32::MIN).eq(I32::from(-2147483648)));
        assert!(*U32::from(0).eq(U32::from(0)));
        assert!(*U32::from(255).eq(U32::from(255)));
        assert!(*U32::from(u32::MAX).eq(U32::from(4294967295)));
    }

    #[test]
    fn test_bv_not() {
        use crate::{I32, bitvector::BitvectorOps, smt::SMT};

        assert!(*I32::from(0).bv_not().eq(I32::from(-1)));
        assert!(*I32::from(-1).bv_not().eq(I32::from(0)));
        assert!(*I32::from(1).bv_not().eq(I32::from(-2)));
    }

    #[test]
    fn test_bv_redand_i32() {
        use crate::{I32, bitvector::BitvectorOps};

        assert!(*I32::from(-1).bv_redand()); // all bits set                                                                                                               
        assert!(*I32::from(0).bv_redand().not()); // no bits set                                                                                                               
        assert!(*I32::from(1).bv_redand().not()); // only one bit set
    }

    #[test]
    fn test_bv_redand_u32() {
        use crate::{U32, bitvector::BitvectorOps};

        assert!(*U32::from(u32::MAX).bv_redand()); // all bits set
        assert!(*U32::from(0u32).bv_redand().not());
    }

    #[test]
    fn test_bv_redand_i64() {
        use crate::{I64, bitvector::BitvectorOps};

        assert!(*I64::from(-1i64).bv_redand());
        assert!(*I64::from(0i64).bv_redand().not());
    }

    #[test]
    fn test_bv_redand_u64() {
        use crate::{U64, bitvector::BitvectorOps};

        assert!(*U64::from(u64::MAX).bv_redand());
        assert!(*U64::from(0u64).bv_redand().not());
    }

    #[test]
    fn test_bv_redor() {
        use crate::{I32, I64, U32, U64, bitvector::BitvectorOps};

        assert!(*I32::from(1).bv_redor());
        assert!(*I32::from(-1).bv_redor());
        assert!(*I32::from(0).bv_redor().not());

        assert!(*U32::from(1u32).bv_redor());
        assert!(*U32::from(0u32).bv_redor().not());

        assert!(*I64::from(1i64).bv_redor());
        assert!(*I64::from(0i64).bv_redor().not());

        assert!(*U64::from(12u64).bv_redor());
        assert!(*U64::from(0u64).bv_redor().not());
    }

    #[test]
    fn test_bv_arithmetic() {
        use crate::{I32, bitvector::BitvectorOps, smt::SMT};

        // and
        assert!(
            *I32::from(0b1100)
                .bv_and(I32::from(0b1010))
                .eq(I32::from(0b1000))
        );
        // or
        assert!(
            *I32::from(0b1100)
                .bv_or(I32::from(0b1010))
                .eq(I32::from(0b1110))
        );
        // xor
        assert!(
            *I32::from(0b1100)
                .bv_xor(I32::from(0b1010))
                .eq(I32::from(0b0110))
        );
        // nand
        assert!(
            *I32::from(0b1100)
                .bv_nand(I32::from(0b1010))
                .eq(I32::from(-9))
        );
        // nor
        assert!(
            *I32::from(0b1100)
                .bv_nor(I32::from(0b1010))
                .eq(I32::from(-15))
        );
        // xnor
        assert!(
            *I32::from(0b1100)
                .bv_xnor(I32::from(0b1010))
                .eq(I32::from(-7))
        );
        //
    }

    #[test]
    fn test_bv_neg_add_sub_mul() {
        use crate::{I32, bitvector::BitvectorOps, smt::SMT};

        // neg
        assert!(*I32::from(1).bv_neg().eq(I32::from(-1)));
        assert!(*I32::from(-1).bv_neg().eq(I32::from(1)));
        assert!(*I32::from(0).bv_neg().eq(I32::from(0)));
        // neg of MIN wraps to MIN (two's complement: -MIN overflows)
        assert!(*I32::from(i32::MIN).bv_neg().eq(I32::from(i32::MIN)));

        // add
        assert!(*I32::from(3).bv_add(I32::from(4)).eq(I32::from(7)));
        // add wrapping
        assert!(
            *I32::from(i32::MAX)
                .bv_add(I32::from(1))
                .eq(I32::from(i32::MIN))
        );

        // sub
        assert!(*I32::from(10).bv_sub(I32::from(3)).eq(I32::from(7)));
        // sub wrapping
        assert!(
            *I32::from(i32::MIN)
                .bv_sub(I32::from(1))
                .eq(I32::from(i32::MAX))
        );

        // mul
        assert!(*I32::from(6).bv_mul(I32::from(7)).eq(I32::from(42)));
        // mul wrapping
        assert!(*I32::from(i32::MAX).bv_mul(I32::from(2)).eq(I32::from(-2)));
    }

    #[test]
    fn test_bv_div() {
        use crate::{I32, U32, bitvector::BitvectorOps, smt::SMT};

        // signed
        assert!(*I32::from(7).bv_div(I32::from(2)).eq(I32::from(3)));
        assert!(*I32::from(-7).bv_div(I32::from(2)).eq(I32::from(-3)));
        assert!(*I32::from(7).bv_div(I32::from(-2)).eq(I32::from(-3)));
        assert!(*I32::from(-7).bv_div(I32::from(-2)).eq(I32::from(3)));

        // unsigned
        assert!(*U32::from(7u32).bv_div(U32::from(2u32)).eq(U32::from(3u32)));
        assert!(*U32::from(10u32).bv_div(U32::from(3u32)).eq(U32::from(3u32)));
    }

    #[test]
    #[should_panic]
    fn test_bv_div_by_zero_i32() {
        use crate::{I32, bitvector::BitvectorOps};
        I32::from(7).bv_div(I32::from(0));
    }

    #[test]
    fn test_bv_div_min_by_neg_one_i32() {
        use crate::{I32, bitvector::BitvectorOps, smt::SMT};
        assert!(
            *I32::from(i32::MIN)
                .bv_div(I32::from(-1))
                .eq(I32::from(i32::MIN))
        );
    }

    #[test]
    fn test_bv_rem_mod() {
        use crate::{I32, U32, bitvector::BitvectorOps, smt::SMT};

        // signed rem: sign follows dividend
        assert!(*I32::from(7).bv_rem(I32::from(3)).eq(I32::from(1)));
        assert!(*I32::from(-7).bv_rem(I32::from(3)).eq(I32::from(-1)));
        assert!(*I32::from(7).bv_rem(I32::from(-3)).eq(I32::from(1)));
        assert!(*I32::from(-7).bv_rem(I32::from(-3)).eq(I32::from(-1)));

        // signed mod: sign follows divisor
        assert!(*I32::from(7).bv_mod(I32::from(3)).eq(I32::from(1)));
        assert!(*I32::from(-7).bv_mod(I32::from(3)).eq(I32::from(2)));
        assert!(*I32::from(7).bv_mod(I32::from(-3)).eq(I32::from(-2)));
        assert!(*I32::from(-7).bv_mod(I32::from(-3)).eq(I32::from(-1)));

        // unsigned rem
        assert!(*U32::from(7u32).bv_rem(U32::from(3u32)).eq(U32::from(1u32)));
    }

    #[test]
    fn test_bv_comparisons() {
        use crate::{I32, U32, bitvector::BitvectorOps};

        // signed: -1 < 1
        assert!(*I32::from(-1).bv_lt(I32::from(1)));
        assert!(*I32::from(-1).bv_le(I32::from(1)));
        assert!(*I32::from(1).bv_gt(I32::from(-1)));
        assert!(*I32::from(1).bv_ge(I32::from(-1)));

        // unsigned: 0xFFFFFFFF > 1 (4294967295 > 1)
        assert!(*U32::from(u32::MAX).bv_gt(U32::from(1u32)));
        assert!(*U32::from(1u32).bv_lt(U32::from(u32::MAX)));
    }

    #[test]
    fn test_bv_to_int() {
        use crate::{I32, Integer, U32, bitvector::BitvectorOps, smt::SMT};

        assert!(*I32::from(5).to_int().eq(Integer::from(5)));
        assert!(*I32::from(-1).to_int().eq(Integer::from(-1)));
        assert!(*I32::from(0).to_int().eq(Integer::from(0)));
        assert!(*I32::from(i32::MIN).to_int().eq(Integer::from(-2147483648)));
        assert!(*I32::from(i32::MAX).to_int().eq(Integer::from(2147483647)));

        assert!(*U32::from(0u32).to_int().eq(Integer::from(0)));
        assert!(*U32::from(255u32).to_int().eq(Integer::from(255)));
        assert!(
            *U32::from(u32::MAX)
                .to_int()
                .eq(Integer::from(4294967295i64))
        );
    }

    #[test]
    fn test_bv_shl_i32() {
        use crate::{I32, bitvector::BitvectorOps, smt::SMT};

        // rhs positive
        assert!(*I32::from(1).bv_shl(I32::from(4)).eq(I32::from(16)));

        // rhs zero
        assert!(*I32::from(5).bv_shl(I32::from(0)).eq(I32::from(5)));

        // rhs = 31 (max valid shift)
        assert!(*I32::from(1).bv_shl(I32::from(31)).eq(I32::from(i32::MIN)));

        // rhs = 32 (>= width → 0)
        assert!(*I32::from(1).bv_shl(I32::from(32)).eq(I32::from(0)));

        // rhs = 100 (way beyond width → 0)
        assert!(*I32::from(1).bv_shl(I32::from(100)).eq(I32::from(0)));

        // rhs negative: -1 as i32 cast to u32 = 4294967295, which is >= 32 → 0
        assert!(*I32::from(1).bv_shl(I32::from(-1)).eq(I32::from(0)));

        // rhs negative: -31 as i32 cast to u32 = 4294967265, which is >= 32 → 0
        assert!(*I32::from(1).bv_shl(I32::from(-31)).eq(I32::from(0)));
    }

    #[test]
    fn test_bv_shl() {
        use crate::{I32, U32, bitvector::BitvectorOps, smt::SMT};

        assert!(*I32::from(1).bv_shl(I32::from(4)).eq(I32::from(16)));
        assert!(*I32::from(1).bv_shl(I32::from(31)).eq(I32::from(i32::MIN)));
        assert!(*I32::from(1).bv_shl(I32::from(32)).eq(I32::from(0)));
        assert!(*I32::from(1).bv_shl(I32::from(100)).eq(I32::from(0)));
        assert!(*U32::from(1u32).bv_shl(U32::from(4u32)).eq(U32::from(16u32)));
        assert!(*U32::from(1u32).bv_shl(U32::from(32u32)).eq(U32::from(0u32)));
    }

    #[test]
    fn test_bv_lshr() {
        use crate::{I32, U32, bitvector::BitvectorOps, smt::SMT};

        assert!(*I32::from(16).bv_lshr(I32::from(2)).eq(I32::from(4)));
        assert!(*I32::from(-1).bv_lshr(I32::from(1)).eq(I32::from(i32::MAX)));
        assert!(*I32::from(-1).bv_lshr(I32::from(32)).eq(I32::from(0)));
        assert!(*I32::from(16).bv_lshr(I32::from(-1)).eq(I32::from(0)));
        assert!(
            *U32::from(16u32)
                .bv_lshr(U32::from(2u32))
                .eq(U32::from(4u32))
        );
        assert!(
            *U32::from(u32::MAX)
                .bv_lshr(U32::from(1u32))
                .eq(U32::from(i32::MAX as u32))
        );
    }

    #[test]
    fn test_bv_rotate_i64() {
        use crate::{I64, bitvector::BitvectorOps, smt::SMT};

        // Simple rotate
        assert!(
            *I64::from(1i64)
                .bv_rotate_left(I64::from(4i64))
                .eq(I64::from(16i64))
        );

        // Rotate top bit around
        assert!(
            *I64::from(i64::MIN)
                .bv_rotate_left(I64::from(1i64))
                .eq(I64::from(1i64))
        );

        // Rotate by 0 — unchanged
        assert!(
            *I64::from(42i64)
                .bv_rotate_left(I64::from(0i64))
                .eq(I64::from(42i64))
        );

        // Rotate by 64 — unchanged (full rotation)
        assert!(
            *I64::from(42i64)
                .bv_rotate_left(I64::from(64i64))
                .eq(I64::from(42i64))
        );

        // rotate -1
        assert!(
            *I64::from(-1i64)
                .bv_rotate_left(I64::from(1i64))
                .eq(I64::from(-1i64))
        );

        // Rotate right
        assert!(
            *I64::from(1i64)
                .bv_rotate_right(I64::from(1i64))
                .eq(I64::from(i64::MIN))
        );

        // Rotate right by 4
        assert!(
            *I64::from(16i64)
                .bv_rotate_right(I64::from(4i64))
                .eq(I64::from(1i64))
        );
    }
}

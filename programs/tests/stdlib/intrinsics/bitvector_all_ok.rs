use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, I32, Integer, U64, bitvector::BitvectorOps};

#[smt_fn]
pub fn bitvector_all(a: I32, b: I32, x: U64, y: U64) -> Boolean {
    // I32 (covers all intrinsic names; U64 below hits 64-bit width too)
    let _not: I32 = a.bv_not();
    let _redand: Boolean = a.bv_redand();
    let _redor: Boolean = a.bv_redor();
    let _and: I32 = a.bv_and(b);
    let _or: I32 = a.bv_or(b);
    let _xor: I32 = a.bv_xor(b);
    let _nand: I32 = a.bv_nand(b);
    let _nor: I32 = a.bv_nor(b);
    let _xnor: I32 = a.bv_xnor(b);
    let _neg: I32 = a.bv_neg();
    let _add: I32 = a.bv_add(b);
    let _sub: I32 = a.bv_sub(b);
    let _mul: I32 = a.bv_mul(b);
    let _div: I32 = a.bv_div(b);
    let _rem: I32 = a.bv_rem(b);
    let _mod: I32 = a.bv_mod(b);
    let _shl: I32 = a.bv_shl(b);
    let _lshr: I32 = a.bv_lshr(b);
    let _ashr: I32 = a.bv_ashr(b);
    let _rotl: I32 = a.bv_rotate_left(b);
    let _rotr: I32 = a.bv_rotate_right(b);
    let _lt: Boolean = a.bv_lt(b);
    let _le: Boolean = a.bv_le(b);
    let _gt: Boolean = a.bv_gt(b);
    let _ge: Boolean = a.bv_ge(b);
    let _to_int: Integer = a.to_int();

    // U64: ensure 64-bit bitvector sorts are exercised too.
    let _u64_rot: U64 = x.bv_rotate_left(y);
    let _u64_to_int: Integer = _u64_rot.to_int();

    Boolean::from(true)
}

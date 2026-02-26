use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::float::FloatOps;
use rusmart_smt_stdlib::{F32, I32, I64, Integer, Real, String, U32, U64};

#[smt_fn]
pub fn f32_to_integer(a: F32) -> Integer {
    F32::to_integer(a)
}

#[smt_fn]
pub fn f32_to_real(a: F32) -> Real {
    F32::to_real(a)
}

#[smt_fn]
pub fn f32_to_bitvectors(a: F32) -> (U32, I32, U64, I64) {
    (
        F32::to_u32(a),
        F32::to_i32(a),
        F32::to_u64(a),
        F32::to_i64(a),
    )
}

#[smt_fn]
pub fn f32_from_hex(s: String) -> F32 {
    F32::from_hex_str(s)
}

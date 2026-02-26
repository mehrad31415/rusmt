use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, Real, String};

#[smt_fn]
pub fn integer_all(x: Integer, y: Integer, z: Integer, s: String) -> Boolean {
    let a = Integer::neg(x);
    let b = Integer::add(a, y);
    let c = Integer::sub(b, z);
    let d = Integer::mul(c, Integer::from(2));
    let e = Integer::div(d, Integer::from(3));
    let f = Integer::div_trunc(e, Integer::from(3));
    let g = Integer::modulo(f, Integer::from(5));
    let h = Integer::rem(g, Integer::from(5));
    let i = Integer::pow(h, Integer::from(2));
    let j = Integer::abs(i);

    let _divides = Integer::divides(Integer::from(2), j);
    let _lt = Integer::lt(j, Integer::from(0));
    let _le = Integer::le(j, Integer::from(0));
    let _gt = Integer::gt(j, Integer::from(0));
    let _ge = Integer::ge(j, Integer::from(0));

    let _to_real: Real = Integer::to_real(j);
    let _to_i32 = Integer::to_i32(j);
    let _to_i64 = Integer::to_i64(j);
    let _to_u32 = Integer::to_u32(j);
    let _to_u64 = Integer::to_u64(j);
    let _to_f32 = Integer::to_f32(j);
    let _to_f64 = Integer::to_f64(j);

    let _from_hex = Integer::from_hex_str(s);
    let _from_oct = Integer::from_oct_str(String::from("77"));
    let _from_bin = Integer::from_bin_str(String::from("1010"));

    let _is_gt_i64_max = Integer::is_gt_i64_max(j);
    let _is_lt_i64_min = Integer::is_lt_i64_min(j);
    let _is_gt_u64_max = Integer::is_gt_u64_max(j);
    let _is_lt_u64_min = Integer::is_lt_u64_min(j);
    let _is_lt_i32_min = Integer::is_lt_i32_min(j);
    let _is_gt_i32_max = Integer::is_gt_i32_max(j);
    let _is_lt_u32_min = Integer::is_lt_u32_min(j);
    let _is_gt_u32_max = Integer::is_gt_u32_max(j);

    Boolean::from(true)
}

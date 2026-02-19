use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, Real};

#[smt_fn]
pub fn real_all(a: Real, b: Real) -> Boolean {
    let n = Real::neg(a);
    let s = Real::add(n, b);
    let d = Real::sub(s, a);
    let p = Real::mul(d, b);
    let q = Real::div(p, Real::from(2));
    let r = Real::pow(q, Real::from(2));
    let abs = Real::abs(r);

    let _round: Integer = Real::round(abs);
    let _floor: Integer = Real::floor(abs);
    let _ceil: Integer = Real::ceil(abs);
    let _is_int = Real::is_integer(abs);

    let _lt = Real::lt(abs, b);
    let _le = Real::le(abs, b);
    let _gt = Real::gt(abs, b);
    let _ge = Real::ge(abs, b);

    let _to_int: Integer = Real::to_int(abs);
    let _to_f32 = Real::to_f32(abs);
    let _to_f64 = Real::to_f64(abs);

    Boolean::from(true)
}


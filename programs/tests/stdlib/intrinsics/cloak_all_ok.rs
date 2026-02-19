use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Cloak, Integer, smt::SMT};

#[smt_fn]
pub fn cloak_all(x: Integer) -> Boolean {
    let c: Cloak<Integer> = Cloak::shield(x);
    let y: Integer = Cloak::reveal(c);
    y.eq(x)
}


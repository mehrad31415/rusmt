use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::Boolean;
use rusmart_smt_stdlib::smt::SMT;

#[smt_fn]
pub fn bool_not(a: Boolean) -> Boolean {
    Boolean::not(a)
}

#[smt_fn]
pub fn bool_and(a: Boolean, b: Boolean) -> Boolean {
    Boolean::and(a, b)
}

#[smt_fn]
pub fn bool_or(a: Boolean, b: Boolean) -> Boolean {
    Boolean::or(a, b)
}

#[smt_fn]
pub fn bool_xor(a: Boolean, b: Boolean) -> Boolean {
    Boolean::xor(a, b)
}

#[smt_fn]
pub fn bool_nand(a: Boolean, b: Boolean) -> Boolean {
    Boolean::nand(a, b)
}

#[smt_fn]
pub fn bool_nor(a: Boolean, b: Boolean) -> Boolean {
    Boolean::nor(a, b)
}

#[smt_fn]
pub fn bool_xnor(a: Boolean, b: Boolean) -> Boolean {
    Boolean::xnor(a, b)
}

#[smt_fn]
pub fn bool_implies(a: Boolean, b: Boolean) -> Boolean {
    Boolean::implies(a, b)
}

#[smt_fn]
pub fn bool_iff(a: Boolean, b: Boolean) -> Boolean {
    Boolean::iff(a, b)
}

#[smt_fn]
pub fn bool_ite<T: SMT>(cond: Boolean, then: T, else_: T) -> T {
    Boolean::ite(cond, then, else_)
}

use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Array, Boolean, Integer, String};

#[smt_fn]
pub fn array_all(m: Array<Integer, String>, k: Integer, v: String) -> Boolean {
    let _new: Array<Integer, String> = Array::<Integer, String>::new();
    let _len: Integer = Array::length(m);
    let m2: Array<Integer, String> = Array::store(m, k, v);
    let _select: String = Array::select(m2, k);
    let _del: Array<Integer, String> = Array::del(m2, k);
    let _contains_key: Boolean = Array::contains_key(
        Array::store(Array::<Integer, String>::new(), k, String::from("x")),
        k,
    );
    let _is_empty: Boolean = Array::is_empty(Array::<Integer, String>::new());

    Boolean::from(true)
}


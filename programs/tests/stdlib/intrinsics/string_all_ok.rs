use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, String, U32};

#[smt_fn]
pub fn string_all(s1: String, s2: String, i: Integer, code: U32) -> Boolean {
    let _new = String::new();
    let _len = String::length(s1);
    let _concat = String::concat(s1, s2);
    let _at = String::at(String::from("abc"), Integer::from(1));
    let _idx = String::index_of(
        String::from("hello world"),
        String::from("world"),
        Integer::from(0),
    );
    let _idx0 = String::index_of_default(String::from("hello world"), String::from("world"));
    let _substr = String::substr(String::from("hello"), Integer::from(1), Integer::from(3));
    let _is_empty = String::is_empty(String::from(""));
    let _contains = String::contains(String::from("hello"), String::from("ell"));
    let _starts = String::starts_with(String::from("hello"), String::from("he"));
    let _ends = String::ends_with(String::from("hello"), String::from("lo"));
    let _is_digit = String::is_digit(String::from("7"));
    let _le = String::le(String::from("a"), String::from("b"));
    let _lt = String::lt(String::from("a"), String::from("b"));
    let _ge = String::ge(String::from("b"), String::from("a"));
    let _gt = String::gt(String::from("b"), String::from("a"));
    let _replace = String::replace(String::from("abab"), String::from("a"), String::from("x"));
    let _replace_all =
        String::replace_all(String::from("abab"), String::from("a"), String::from("x"));
    let _to_int = String::to_int(String::from("123"));
    let _from_int = String::from_int(i);
    let _from_code = String::from_code(code);
    let _to_code = String::to_code(String::from("A"));

    Boolean::from(true)
}

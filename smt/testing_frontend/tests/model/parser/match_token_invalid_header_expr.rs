use rusmart_smt_remark_derive::smt_impl;
use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::SMT;
use rusmart_smt_stdlib::{Boolean, Integer};

// invalid expression Expr::Lit { attrs: [], lit: Lit::Int { token: 1 } }
// 1
// .convert_expr(unifier, elem)?; in Exp::Match in expr.rs
// This is because literals should not be used instead Integer::from(1) should be used

#[smt_impl]
fn foo() -> Boolean {
    match 1 {
        1 => Boolean::from(true),
        _ => Boolean::from(false),
    }
}

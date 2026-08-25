//! Runnable specification for **IMP/WHILE**.
//! Division by zero and undefined variable use are errors (`Store` is `Array<String, I64>`.read-on-miss -> is an error unlike Winskel's convention where it is 0).

use crate::imp::ast::{Aexp, Bexp, Com};
use rusmt_smt_remark_derive::{smt_fn, smt_type};
use rusmt_smt_stdlib::{Array, Boolean, I64, Path, String, bitvector::BitvectorOps, smt::SMT};

/// Abstract syntax.
pub mod ast;
/// Parser and pretty-printer for `.imp` source files.
pub mod parser;
pub mod render;

/// The result of evaluating a [`Com`].
///
/// Big-step IMP fails in two cases:
/// - when a division by zero happens.
/// - when a variable is undefined but used.
#[smt_type]
pub enum EvalResult {
    /// A path-condition marker fired (division by zero or undefined variable).
    Err(Path),
    /// Normal termination with the final store.
    Ok(Array<String, I64>),
}

/// The result of reading a location from the store.
#[smt_type]
pub enum StoreGetResult {
    /// The location is absent from the store (`undefined_variable`).
    Err(Path),
    /// The value stored at the location.
    Ok(I64),
}

/// The result of arithmetic operations.
#[smt_type]
pub enum ArithmeticResult {
    /// A path-condition marker fired while evaluating the expression.
    Err(Path),
    /// The value of the expression.
    Ok(I64),
}

/// The result of boolean operations.
#[smt_type]
pub enum BooleanResult {
    /// A path-condition marker fired while evaluating the expression.
    Err(Path),
    /// The value of the expression.
    Ok(Boolean),
}

/// Read a location from the store. Returns an error if the location is not found.
#[smt_fn]
pub fn store_get(s: Array<String, I64>, x: String) -> StoreGetResult {
    if *s.contains_key(x) {
        StoreGetResult::Ok(s.select(x))
    } else {
        // Named marker: reading a variable absent from the store is an error.
        StoreGetResult::Err(Path::named(String::from("undefined_variable")))
    }
}

/// Big-step evaluation of arithmetic expressions.
///
/// `+`, `-`, `*`, `/` are integer ops on signed 64-bit (`I64`).
/// Evaluation yields `Err` when a variable is undefined (via [`store_get`]) or when the divisor of `/` is `0`;
/// each carries a fresh path-condition marker.
#[smt_fn]
pub fn eval_aexp(a: Aexp, s: Array<String, I64>) -> ArithmeticResult {
    match a {
        Aexp::Num(n) => ArithmeticResult::Ok(n),
        Aexp::Var(x) => match store_get(s, x) {
            StoreGetResult::Err(e) => ArithmeticResult::Err(e),
            StoreGetResult::Ok(v) => ArithmeticResult::Ok(v),
        },
        Aexp::Add(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
                ArithmeticResult::Ok(new_r) => ArithmeticResult::Ok(new_l.bv_add(new_r)),
            },
        },
        Aexp::Sub(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
                ArithmeticResult::Ok(new_r) => ArithmeticResult::Ok(new_l.bv_sub(new_r)),
            },
        },
        Aexp::Mul(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
                ArithmeticResult::Ok(new_r) => ArithmeticResult::Ok(new_l.bv_mul(new_r)),
            },
        },
        Aexp::Div(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => ArithmeticResult::Err(e),
                ArithmeticResult::Ok(new_r) => {
                    if *new_r.eq(I64::from(0)) {
                        // Named marker: division by a zero divisor is an error.
                        ArithmeticResult::Err(Path::named(String::from("division_by_zero")))
                    } else {
                        ArithmeticResult::Ok(new_l.bv_div(new_r))
                    }
                }
            },
        },
    }
}

/// Big-step evaluation of boolean expressions.
///
/// `==` and `<=` compare via [`I64`] (signed). `not`/`and`/`or` are the
/// stdlib `Boolean` operators.
#[smt_fn]
pub fn eval_bexp(b: Bexp, s: Array<String, I64>) -> BooleanResult {
    match b {
        Bexp::True => BooleanResult::Ok(Boolean::from(true)),
        Bexp::False => BooleanResult::Ok(Boolean::from(false)),
        Bexp::Eq(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => BooleanResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => BooleanResult::Err(e),
                ArithmeticResult::Ok(new_r) => BooleanResult::Ok(new_l.eq(new_r)),
            },
        },
        Bexp::Le(l, r) => match eval_aexp(l.reveal(), s) {
            ArithmeticResult::Err(e) => BooleanResult::Err(e),
            ArithmeticResult::Ok(new_l) => match eval_aexp(r.reveal(), s) {
                ArithmeticResult::Err(e) => BooleanResult::Err(e),
                ArithmeticResult::Ok(new_r) => BooleanResult::Ok(new_l.bv_le(new_r)),
            },
        },
        Bexp::Not(b1) => match eval_bexp(b1.reveal(), s) {
            BooleanResult::Err(e) => BooleanResult::Err(e),
            BooleanResult::Ok(new_b) => BooleanResult::Ok(new_b.not()),
        },
        Bexp::And(l, r) => match eval_bexp(l.reveal(), s) {
            BooleanResult::Err(e) => BooleanResult::Err(e),
            BooleanResult::Ok(new_l) => match eval_bexp(r.reveal(), s) {
                BooleanResult::Err(e) => BooleanResult::Err(e),
                BooleanResult::Ok(new_r) => BooleanResult::Ok(new_l.and(new_r)),
            },
        },
        Bexp::Or(l, r) => match eval_bexp(l.reveal(), s) {
            BooleanResult::Err(e) => BooleanResult::Err(e),
            BooleanResult::Ok(new_l) => match eval_bexp(r.reveal(), s) {
                BooleanResult::Err(e) => BooleanResult::Err(e),
                BooleanResult::Ok(new_r) => BooleanResult::Ok(new_l.or(new_r)),
            },
        },
    }
}

/// Big-step evaluation of commands.
#[smt_fn]
pub fn eval_com(c: Com) -> EvalResult {
    let s = Array::new();
    eval_com_inner(c, s)
}

/// Inner function for big-step evaluation of commands.
#[smt_fn]
pub fn eval_com_inner(c: Com, s: Array<String, I64>) -> EvalResult {
    match c {
        Com::Skip => EvalResult::Ok(s),
        Com::Assign(x, a) => {
            let v = eval_aexp(a.reveal(), s);
            match v {
                ArithmeticResult::Err(e) => EvalResult::Err(e),
                ArithmeticResult::Ok(new_v) => EvalResult::Ok(s.store(x, new_v)),
            }
        }
        Com::Seq(c0, c1) => match eval_com_inner(c0.reveal(), s) {
            EvalResult::Err(e) => EvalResult::Err(e), // propagate error
            EvalResult::Ok(s1) => eval_com_inner(c1.reveal(), s1),
        },
        Com::If(b, c0, c1) => match eval_bexp(b.reveal(), s) {
            BooleanResult::Err(e) => EvalResult::Err(e),
            BooleanResult::Ok(new_b) => {
                if *new_b {
                    eval_com_inner(c0.reveal(), s)
                } else {
                    eval_com_inner(c1.reveal(), s)
                }
            }
        },
        Com::While(b, body) => match eval_bexp(b.reveal(), s) {
            BooleanResult::Err(e) => EvalResult::Err(e),
            BooleanResult::Ok(new_b) => {
                if *new_b {
                    match eval_com_inner(body.reveal(), s) {
                        EvalResult::Err(e) => EvalResult::Err(e),
                        EvalResult::Ok(s1) => eval_com_inner(c, s1),
                    }
                } else {
                    EvalResult::Ok(s)
                }
            }
        },
    }
}

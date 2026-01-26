// Test 5: Complex scenario - Expression evaluator
// Tests: Complex type + recursive function + pattern matching + intrinsics

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Cloak, Integer, Real, smt::SMT};

// Expression AST
#[smt_type]
pub enum Expr {
    Const(Integer),
    Add(Cloak<Expr>, Cloak<Expr>),
    Mul(Cloak<Expr>, Cloak<Expr>),
    Sub(Cloak<Expr>, Cloak<Expr>),
    Neg(Cloak<Expr>),
}

// Evaluation result (with Option-like semantics)
#[smt_type]
pub enum EvalResult {
    Ok(Integer),
    Error,
}

// Recursive evaluator
#[smt_fn]
pub fn eval_expr(e: Expr) -> Integer {
    match e {
        Expr::Const(n) => n,
        Expr::Add(left, right) => Integer::add(
            eval_expr(left.reveal()),
            eval_expr(right.reveal()),
        ),
        Expr::Mul(left, right) => Integer::mul(
            eval_expr(left.reveal()),
            eval_expr(right.reveal()),
        ),
        Expr::Sub(left, right) => Integer::sub(
            eval_expr(left.reveal()),
            eval_expr(right.reveal()),
        ),
        Expr::Neg(inner) => Integer::neg(eval_expr(inner.reveal())),
    }
}

// Check if expression is positive
#[smt_fn]
pub fn is_positive_expr(e: Expr) -> Boolean {
    let result = eval_expr(e);
    Integer::gt(result, Integer::from(0))
}

// Depth of expression tree
#[smt_fn]
pub fn expr_depth(e: Expr) -> Integer {
    match e {
        Expr::Const(_) => Integer::from(1),
        Expr::Neg(inner) => {
            Integer::add(Integer::from(1), expr_depth(inner.reveal()))
        }
        Expr::Add(l, r) | Expr::Mul(l, r) | Expr::Sub(l, r) => {
            let left_depth = expr_depth(l.reveal());
            let right_depth = expr_depth(r.reveal());
            let max_depth = {
                let left_is_greater = Integer::gt(left_depth, right_depth);
                if *left_is_greater {
                    left_depth
                } else {
                    right_depth
                }
            };
            Integer::add(Integer::from(1), max_depth)
        }
    }
}

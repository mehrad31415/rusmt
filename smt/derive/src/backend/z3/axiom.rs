//! This module contains the conversion of predicates to SMT-LIB axioms.
//! It has the following functions:
//! - `axiom_in_smt`: Converts a predicate into the corresponding SMT-LIB axiom as a `String`.
//!

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::axiom::Predicate;
use crate::IRContext;

/// Converts a predicate into the corresponding SMT-LIB axiom as a `String`.
/// The predicate is a first-order logic formula that can be used to define
/// the behavior of a function or a data type.
pub fn axiom_in_smt(predicate: &Predicate, ir: &IRContext) -> String {
    // The Generics are already registered in `undef_sorts`.
    let Predicate {
        params,
        body_reg,
        body_exp,
    } = predicate;

    if params.is_empty() {
        // if there are no parameters, we can just return the body
        return format!("(assert {})", expr_to_smt(body_reg, body_exp, ir));
    }

    let field_defs: Vec<String> = params
        .iter()
        .map(|(field_name, sort)| format!("({} {})", field_name, sort_to_smt(sort, ir)))
        .collect();

    let body_expr = expr_to_smt(body_reg, body_exp, ir);

    // (assert <expr>)
    // (forall ( (<symbol> <sort>)+ ) <expr>)
    return format!("(assert (forall ({}) {}))", field_defs.join(" "), body_expr);
}

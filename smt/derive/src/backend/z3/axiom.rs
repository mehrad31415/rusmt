//! This module contains the conversion of predicates to SMT-LIB axioms.
//! It has the following functions:
//! - `axiom_in_smt`: Converts a predicate into the corresponding SMT-LIB axiom as a `String`.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::axiom::Predicate;
use crate::IRContext;
use std::collections::{BTreeMap, BTreeSet};

/// Converts a predicate into the corresponding SMT-LIB axiom as a `String`.
/// The predicate is a first-order logic formula that gives axiomatic semantics of the system.
pub fn axiom_in_smt(predicate: &Predicate, ir: &IRContext) -> String {
    // The Generics are already registered in `undef_sorts`.
    let Predicate {
        params,
        body_reg,
        body_exp,
    } = predicate;
    let mut ret = String::new();
    let mut dependencies = BTreeSet::new();
    let mut mapping_vars = BTreeMap::new();
    let body_expr = expr_to_smt(body_reg, body_exp, ir, &mut dependencies, &mut mapping_vars);

    // add the dependencies
    for dep in dependencies.iter() {
        ret += dep.as_str();
        ret += "\n";
    }

    if params.is_empty() {
        // if there are no parameters, we can just return the body
        ret += format!("(assert {})", body_expr).as_str();
    } else {
        let field_defs: Vec<String> = params
            .iter()
            .map(|(field_name, sort)| format!("({} {})", field_name, sort_to_smt(sort, ir)))
            .collect();

        // (assert <expr>)
        // (forall ( (<symbol> <sort>)+ ) <expr>)
        ret += format!("(assert (forall ({}) {}))", field_defs.join(" "), body_expr).as_str();
    }

    // done
    ret
}

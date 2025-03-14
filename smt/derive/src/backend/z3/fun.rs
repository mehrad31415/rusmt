//! This module contains the logic for converting function definitions to SMT-LIB
//! It has the following functions:
//! - `fundef_in_smt`: Converts a function definition into the corresponding SMT-LIB function definition as a `String`.

use crate::backend::z3::exp::expr_to_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::ir::fun::{FunDef, FunSig};
use crate::ir::name::UsrFunName;
use crate::IRContext;

/// Converts a function definition into the corresponding SMT-LIB function definition as a `String`.
/// The function definition can be either a defined function or an uninterpreted function.
/// The function signature is used to determine the types of the parameters and the return type.
/// The function definition is used to determine the body of the function.
/// The Generics are already registered in `undef_sorts`.
pub fn fundef_in_smt(name: UsrFunName, sig: &FunSig, def: &FunDef, ir: &IRContext) -> String {
    // depending on whether the function is defined or uninterpreted, the function signature is different
    let FunSig { params, ret_ty } = sig;

    let return_type = sort_to_smt(ret_ty, ir);

    match def {
        FunDef::Defined(reg, id) => {
            // convert the function body to SMT-LIB
            let body_expr = expr_to_smt(reg, id, ir);

            let field_defs: Vec<String> = params
                .iter()
                .map(|(field_name, sort)| format!("({} {})", field_name, sort_to_smt(sort, ir)))
                .collect();

            // define the function with define-fun-rec
            return format!(
                "(define-fun-rec {} ({}) {} {})",
                name,
                field_defs.join(" "),
                return_type,
                body_expr
            );
        }
        FunDef::Uninterpreted => {
            let field_defs: Vec<String> = params
                .iter()
                .map(|(_, sort)| format!("{}", sort_to_smt(sort, ir)))
                .collect();

            // declare the function with declare-fun
            return format!(
                "(declare-fun {} ({}) {})",
                name,
                field_defs.join(" "),
                return_type
            );
        }
    }
}

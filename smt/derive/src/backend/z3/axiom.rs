//! This module adds all predicates from the axiom registry into the Z3 solver as universal axioms.

use crate::backend::z3::unimplemented::{CloakManager, MapLengthManager};
use crate::backend::z3::{exp::process_expression, sort::sort_to_z3};
use crate::ir::name::Symbol;
use crate::ir::{
    axiom::Predicate,
    ctxt::IRContext,
    index::{UsrFunId, UsrSortId},
    name::SmtSortName,
};
use std::collections::HashMap;
use z3::{
    Context, DatatypeSort, RecFuncDecl, Solver, Sort,
    ast::{Ast, Bool, Dynamic, forall_const},
};

/// Adds all predicates from the axiom registry into the Z3 solver as universal axioms.
pub fn assert_axioms<'a>(
    ctx: &'a Context,
    ir: &IRContext,
    solver: &Solver,
    predicate: &Predicate,
    fn_map: &HashMap<UsrFunId, RecFuncDecl>,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sort_map: &HashMap<SmtSortName, Sort>,
    cloak_manager: &mut CloakManager<'a>,
    map_length_manager: &mut MapLengthManager,
    axiomatic_parameters: &mut HashMap<String, Dynamic>,
) {
    // destructure the predicate
    let Predicate {
        params,
        body_reg,
        body_exp,
    } = predicate;

    let bound_vars: Vec<(Symbol, Dynamic)> = params
        .iter()
        .map(|(name, sort)| {
            let z3_sort = sort_to_z3(sort, ctx, ir, None, ty_map);
            (
                name.clone(),
                Dynamic::new_const(ctx, name.to_string(), &z3_sort),
            )
        })
        .collect();
    let body_ast: Bool = process_expression(
        ctx,
        solver,
        body_reg,
        *body_exp,
        ir,
        fn_map,
        ty_map,
        sort_map,
        &bound_vars,
        cloak_manager,
        map_length_manager,
        axiomatic_parameters,
    )
    .as_bool()
    .expect("Body of predicate is not a boolean expression");

    // let trigger_terms = extract_trigger_candidates(&body_ast, &bound_vars);
    // let patterns: Vec<Pattern> = trigger_terms
    //     .iter()
    //     .map(|terms| Pattern::new(ctx, &[terms as &dyn Ast]))
    //     .collect();

    let axiom = forall_const(
        ctx,
        &bound_vars
            .iter()
            .map(|(_name, var)| var as &dyn Ast)
            .collect::<Vec<_>>(),
        &[],
        &body_ast,
    );

    solver.assert(&axiom);
}

// /// Extracts trigger candidates from the body of the predicate for pattern matching.
// fn extract_trigger_candidates(
//     body_ast: &Bool,
//     bound_vars: &[(Symbol, Dynamic)],
// ) -> Vec<Dynamic> {
//     let mut candidates = Vec::new();

//     // Traverse AST to collect function applications
//     collect_function_applications(body_ast, bound_vars, &mut candidates);

//     // Filter out trigger killers and rank by quality
//     let filtered = filter_and_rank_candidates(candidates, bound_vars);

//     filtered
// }

// /// Filters out trigger killers and ranks candidates based on the number of bound variables they contain.
// fn filter_and_rank_candidates(
//     candidates: Vec<Dynamic>,
//     bound_vars: &[(Symbol, Dynamic)],
// ) -> Vec<Dynamic> {
//     let mut filtered = candidates
//         .into_iter()
//         .filter(|ast| !is_trigger_killer(ast))
//         .collect::<Vec<_>>();

//     // Sort by the number of bound variables they contain
//     filtered.sort_by_key(|ast| {
//         // use -(count as isize) to get descending order
//         -(ast
//             .children()
//             .iter()
//             .filter(|child| bound_vars.iter().any(|(_name, var)| var == *child))
//             .count() as isize)
//     });

//     filtered
// }

// /// Checks if the AST node is a function application that is a trigger killer.
// fn collect_function_applications(
//     ast: &dyn Ast,
//     bound_vars: &[(Symbol, Dynamic)],
//     candidates: &mut Vec<Dynamic>,
// ) {
//     // on nested quantifiers, we do not want to collect function applications
//     if ast.kind() == AstKind::Quantifier {
//         return;
//     }

//     // If the AST is a function application and not a trigger killer,
//     // add it to the candidates if it contains bound variables.
//     if ast.is_app() && !is_trigger_killer(ast) {
//         if ast
//             .children()
//             .iter()
//             .any(|child| bound_vars.iter().any(|(_name, var)| var == child))
//         {
//             candidates.push(Dynamic::from_ast(ast));
//         }
//     }

//     // Recursively examine children
//     for i in 0..ast.num_children() {
//         if let Some(child) = ast.nth_child(i) {
//             collect_function_applications(&child, bound_vars, candidates);
//         }
//     }
// }

// /// Triggers killers are common operators that should not be used as triggers in quantifiers.
// fn is_trigger_killer(ast: &dyn Ast) -> bool {
//     if let Ok(decl) = ast.safe_decl() {
//         matches!(
//             decl.name().to_string().as_str(),
//             "+" | "-"
//                 | "*"
//                 | "/"
//                 | "mod"
//                 | "and"
//                 | "or"
//                 | "not"
//                 | "=>"
//                 | "="
//                 | "<"
//                 | "<="
//                 | ">"
//                 | ">="
//                 | "ite"
//                 | "distinct"
//         )
//     } else {
//         false
//     }
// }

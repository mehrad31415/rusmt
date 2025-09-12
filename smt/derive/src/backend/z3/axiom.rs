//! This module adds all predicates from the axiom registry into the Z3 solver as universal axioms.

use crate::backend::z3::unimplemented::{CloakManager, MapLengthManager};
use crate::backend::z3::{exp::process_expression, sort::sort_to_z3};
use crate::ir::exp::ExpRegistry;
use crate::ir::index::{ExpId, UsrAxiomId};
use crate::ir::name::Symbol;
use crate::ir::sort::Sort;
use crate::ir::{
    ctxt::IRContext,
    index::{UsrFunId, UsrSortId},
    name::SmtSortName,
};
use std::collections::HashMap;
use z3::{
    DatatypeSort, FuncDecl,
    ast::{Ast, Bool, Dynamic},
};

// /// Adds all predicates from the axiom registry into the Z3 solver as universal axioms.
// pub fn assert_axioms<'a>(
//     ctx: &'a Context,
//     ir: &IRContext,
//     solver: &Solver,
//     predicate: &Predicate,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, Sort>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, Dynamic>,
// ) {
//     // destructure the predicate
//     let Predicate {
//         params,
//         body_reg,
//         body_exp,
//     } = predicate;

//     let bound_vars: Vec<(Symbol, Dynamic)> = params
//         .iter()
//         .map(|(name, sort)| {
//             let z3_sort = sort_to_z3(sort, ctx, ir, None, ty_map);
//             (
//                 name.clone(),
//                 Dynamic::new_const(ctx, name.to_string(), &z3_sort),
//             )
//         })
//         .collect();

//     let body_ast: Bool = process_expression(
//         ctx,
//         solver,
//         body_reg,
//         *body_exp,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         &bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     )
//     .as_bool()
//     .expect("Body of predicate is not a boolean expression");

//     let axiom = forall_const(
//         ctx,
//         &bound_vars
//             .iter()
//             .map(|(_name, var)| var as &dyn Ast)
//             .collect::<Vec<_>>(),
//         &[],
//         &body_ast,
//     );

//     solver.assert(&axiom);
// }

pub fn process_axiom_body<'a>(
    ctx: &'a z3::Context,
    solver: &z3::Solver,
    ir: &IRContext,
    axiom_id: UsrAxiomId,
    params: &Vec<(Symbol, Sort)>,
    exp_registry: &ExpRegistry,
    root_exp_id: ExpId,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sort_map: &HashMap<SmtSortName, z3::Sort>,
    cloak_manager: &mut CloakManager<'a>,
    map_length_manager: &mut MapLengthManager,
    axiomatic_parameters: &mut HashMap<String, Dynamic>,
    axiom_map: &HashMap<UsrAxiomId, FuncDecl>,
    fn_map: &HashMap<UsrFunId, FuncDecl>,
) {
    // Build formal parameters as Z3 consts (just like you do for functions)
    let param_vars: Vec<(Symbol, Dynamic)> = params
        .iter()
        .map(|(name, sort)| {
            let srt = sort_to_z3(sort, ctx, ir, None, ty_map);
            (
                name.clone(),
                Dynamic::fresh_const(ctx, name.to_string().as_str(), &srt),
            )
        })
        .collect();

    // Compute axiom body
    let body: Bool = process_expression(
        ctx,
        solver,
        exp_registry,
        root_exp_id,
        ir,
        fn_map,
        ty_map,
        sort_map,
        &param_vars,
        cloak_manager,
        map_length_manager,
        axiomatic_parameters,
    )
    .as_bool()
    .expect("axiom body must be Bool");

    // Look up the single declaration we made earlier
    let decl = axiom_map
        .get(&axiom_id)
        .expect("axiom declaration not found");

    // Define axiom predicate: axiom(params) = body
    let args: Vec<&dyn Ast> = param_vars.iter().map(|(_, d)| d as &dyn Ast).collect();
    let pred_app = decl.apply(&args);
    let axiom_assertion = pred_app._eq(body);
    solver.assert(&axiom_assertion);
}

//! This module contains functions for working with Z3 function declarations and definitions.

use crate::backend::z3::unimplemented::{CloakManager, MapLengthManager};
use crate::backend::z3::{exp::process_expression, sort::sort_to_z3};
use crate::ir::name::Symbol;
use crate::ir::{
    ctxt::IRContext,
    exp::ExpRegistry,
    fun::FunSig,
    index::{ExpId, UsrFunId, UsrSortId},
    name::{SmtSortName, UsrFunName},
    sort::Sort,
};
use log::debug;
use std::collections::HashMap;
use z3::ast::Ast;
use z3::{Context, DatatypeVariant, RecFuncDecl, Solver, ast::Dynamic};

/// Creates a Z3 function declaration from the function signature
pub fn create_function_declaration(
    ctx: &Context,
    fn_name: &UsrFunName,
    generics: &[Sort],
    sig: &FunSig,
    ir: &IRContext,
    ty_map: &HashMap<UsrSortId, (z3::Sort, Vec<DatatypeVariant>)>,
    sort_map: &mut HashMap<SmtSortName, z3::Sort>,
) -> RecFuncDecl {
    // destructure the function signature
    let FunSig { params, ret_ty } = sig;
    // convert parameter sorts to Z3 sorts
    let param_sorts: Vec<z3::Sort> = params
        .iter()
        .map(|(_, sort)| sort_to_z3(sort, ctx, ir, None, ty_map))
        .collect();

    // convert return sort to Z3 sort
    let ret_sort = sort_to_z3(ret_ty, ctx, ir, None, ty_map);
    let fn_name_str = fn_name.to_string();

    for generic in generics {
        if let Sort::Uninterpreted(smt_sort_name) = generic {
            // If the generic is an uninterpreted sort, we need to ensure it is defined
            if let Some(_z3_sort) = sort_map.get(smt_sort_name) {
                // If the sort is already defined, we can use it
                debug!("Using existing sort: {smt_sort_name}");
            } else {
                // it should have already been defined in the IR so panic
                panic!(
                    "Uninterpreted sort {smt_sort_name} not found in sort map. It should be defined in the IR."
                );
            }
        } else {
            panic!("Generic sort must be an uninterpreted sort, found: {generic:?}");
        }
    }
    // Create the function declaration
    RecFuncDecl::new(
        ctx,
        fn_name_str,
        &param_sorts.iter().collect::<Vec<_>>(),
        &ret_sort,
    )
}

/// processes the body of a defined function
pub fn process_function_body<'a>(
    ctx: &'a Context,
    solver: &Solver,
    fn_id: UsrFunId,
    exp_registry: &ExpRegistry,
    root_exp_id: ExpId,
    ir: &IRContext,
    fn_map: &HashMap<UsrFunId, RecFuncDecl>,
    ty_map: &HashMap<UsrSortId, (z3::Sort, Vec<DatatypeVariant>)>,
    sort_map: &HashMap<SmtSortName, z3::Sort>,
    cloak_manager: &mut CloakManager<'a>,
    map_length_manager: &mut MapLengthManager,
    axiomatic_parameters: &mut HashMap<String, Dynamic>,
) {
    // Get the function signature
    let sig = ir.fn_registry.retrieve_sig(fn_id);

    // Create parameter variables
    let param_vars: Vec<(Symbol, Dynamic)> = sig
        .params
        .iter()
        .map(|(param_name, param_sort)| {
            let z3_sort = sort_to_z3(param_sort, ctx, ir, None, ty_map);
            (
                param_name.clone(),
                Dynamic::new_const(ctx, param_name.to_string(), &z3_sort),
            )
        })
        .collect();

    // Process the expression tree to create Z3 AST
    let body_ast = process_expression(
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
    );

    // Get the function declaration
    let fn_decl = fn_map.get(&fn_id).expect("Function declaration not found");

    // add def
    let param_refs: Vec<&dyn Ast> = param_vars.iter().map(|(_s, d)| d as &dyn Ast).collect();
    fn_decl.add_def(&param_refs, &body_ast);
}

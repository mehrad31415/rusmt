//! This module contains the conversion of Rusmart intrinsics to SMT-LIB format.

use crate::{
    backend::z3::{
        exp::process_expression,
        sort::sort_to_z3,
        unimplemented::{
            CloakManager, MapLengthManager, StringCompareOp, compare_string_asts, empty_map,
            mk_seq_empty, not_present, seq_contains,
        },
    },
    ir::{
        ctxt::IRContext,
        exp::ExpRegistry,
        index::{UsrFunId, UsrSortId},
        intrinsics::Intrinsic,
        name::{SmtSortName, Symbol},
        sort::Sort,
    },
};
use std::collections::HashMap;
use z3::{Context, DatatypeSort, RecFuncDecl, Solver, ast, ast::Ast};

pub fn process_intrinsic<'ctx>(
    ctx: &'ctx Context,
    solver: &Solver,
    exp_registry: &ExpRegistry,
    intrinsic: &Intrinsic,
    ir: &IRContext,
    fn_map: &HashMap<UsrFunId, RecFuncDecl>,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sort_map: &HashMap<SmtSortName, z3::Sort>,
    bound_vars: &Vec<(Symbol, ast::Dynamic)>,
    cloak_manager: &mut CloakManager<'ctx>,
    map_length_manager: &mut MapLengthManager,
    axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
) -> ast::Dynamic {
    use crate::ir::intrinsics::Intrinsic::*;
    match intrinsic {
        // --- Boolean ---
        // `Boolean::from`
        BoolVal(b) => ast::Bool::from_bool(ctx, *b).into(),
        // `Boolean::not`
        BoolNot { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::not(&val_ast).into()
        }
        // `Boolean::and`
        BoolAnd { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::and(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Boolean::or`
        BoolOr { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::or(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Boolean::xor`
        BoolXor { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::xor(&lhs_ast, &rhs_ast).into()
        }
        // `Boolean::implies`
        BoolImplies { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::implies(&lhs_ast, &rhs_ast).into()
        }
        // `Boolean::iff`
        BoolIff { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_bool()
            .expect("Expected a boolean AST");
            ast::Bool::iff(&lhs_ast, &rhs_ast).into()
        }
        // --- Integer ---
        // `Integer::from`
        IntVal(i) => ast::Int::from_i64(ctx, *i).into(),
        // `Integer::lt`
        IntLt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::lt(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::le`
        IntLe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::le(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::ge`
        IntGe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::ge(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::gt`
        IntGt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::gt(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::add`
        IntAdd { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::add(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Integer::sub`
        IntSub { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::sub(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Integer::mul`
        IntMul { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::mul(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Integer::div`
        IntDiv { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::div(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::rem`
        IntRem { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::rem(&lhs_ast, &rhs_ast).into()
        }
        // `Integer::to_rational`
        IntToRational { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::to_real(&val_ast).into()
        }
        // `Integer::pow`
        IntPow { base, exp } => {
            let base_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *base,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let exp_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *exp,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            ast::Int::power(&base_ast, &exp_ast).into()
        }
        // `Integer::abs` -- there are no direct support for absolute value in Z3 for integers.
        IntAbs { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            let zero = ast::Int::from_i64(ctx, 0);
            let cond = val_ast.ge(&zero); // val_ast >= 0
            cond.ite(&val_ast, &val_ast.unary_minus()).into()
        }
        // --- Rational ---
        // `Rational::from`
        NumVal(i) => ast::Real::from_int(&ast::Int::from_i64(ctx, *i)).into(),
        // `Rational::lt`
        NumLt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::lt(&lhs_ast, &rhs_ast).into()
        }
        // `Rational::le`
        NumLe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::le(&lhs_ast, &rhs_ast).into()
        }
        // `Rational::ge`
        NumGe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::ge(&lhs_ast, &rhs_ast).into()
        }
        // `Rational::gt`
        NumGt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::gt(&lhs_ast, &rhs_ast).into()
        }
        // `Rational::add`
        NumAdd { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::add(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Rational::sub`
        NumSub { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::sub(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Rational::mul`
        NumMul { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::mul(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Rational::div`
        NumDiv { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            ast::Real::div(&lhs_ast, &rhs_ast).into()
        }
        // `Num::pow`
        NumPow { base, exp } => {
            let base_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *base,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let exp_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *exp,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected an integer AST");
            ast::Real::power(&base_ast, &exp_ast).into()
        }
        // `Num::abs`
        NumAbs { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let zero = ast::Real::from_int(&ast::Int::from_i64(ctx, 0));
            let cond = val_ast.ge(&zero); // val_ast >= 0
            cond.ite(&val_ast, &val_ast.unary_minus()).into()
        }
        // `Num::round`
        NumRound { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let i = val_ast.approx_f64().round() as i64;
            ast::Int::from_i64(ctx, i).into()
        }
        // `Num::floor`
        NumFloor { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let i = val_ast.approx_f64().floor() as i64;
            ast::Int::from_i64(ctx, i).into()
        }
        // `Num::ceil`
        NumCeil { val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_real()
            .expect("Expected a rational AST");
            let i = val_ast.approx_f64().ceil() as i64;
            ast::Int::from_i64(ctx, i).into()
        }
        // --- Text ---
        // `Text::from`
        StrVal(s) => ast::String::from_str(ctx, s)
            .expect("Failed to create string AST")
            .into(),
        // `Text::lt` - lexicographic string comparison
        StrLt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let result = compare_string_asts(&lhs_ast, &rhs_ast, StringCompareOp::Lt)
                .expect("Failed to compare string ASTs");
            result.into()
        }
        // `Text::le`
        StrLe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let result = compare_string_asts(&lhs_ast, &rhs_ast, StringCompareOp::Le)
                .expect("Failed to compare string ASTs");
            result.into()
        }
        // `Text::ge`
        StrGe { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let result = compare_string_asts(&lhs_ast, &rhs_ast, StringCompareOp::Ge)
                .expect("Failed to compare string ASTs");
            result.into()
        }
        // `Text::gt`
        StrGt { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let result = compare_string_asts(&lhs_ast, &rhs_ast, StringCompareOp::Gt)
                .expect("Failed to compare string ASTs");
            result.into()
        }
        // `Text::concat`
        StrConcat { lhs, rhs } => {
            let left = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            let right = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            ast::String::concat(ctx, &[&left, &right]).into()
        }
        // `Text::at_index`
        StrAt { seq, idx } => {
            let text_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            let index_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *idx,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            text_ast.at(&index_ast).into()
        }
        // `Text::length`
        StrLength { seq } => {
            let text_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            text_ast.length().into()
        }
        // `Text::contains`
        StrIncludes { seq, item } => {
            let text_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            let item_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            text_ast.contains(&item_ast).into()
        }
        // `Text::starts_with`
        StrStartsWith { seq, item } => {
            let text_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            let item_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            item_ast.prefix(&text_ast).into()
        }
        // `Text::ends_with`
        StrEndsWith { seq, item } => {
            let text_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            let item_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_string()
            .expect("Expected a string AST");
            item_ast.suffix(&text_ast).into()
        }
        // --- Cloak (box) ---
        // `Cloak::shield`
        BoxShield { t, val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let base_sort = sort_to_z3(t, ctx, ir, None, ty_map);

            let (shield_decl, _reveal_decl) =
                cloak_manager.get_or_create_cloak_for_type(solver, &base_sort);

            shield_decl.apply(&[&val_ast])
        }
        // `Cloak::reveal`
        BoxReveal { t, val } => {
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let base_sort = sort_to_z3(t, ctx, ir, None, ty_map);

            let (_shield_decl, reveal_decl) =
                cloak_manager.get_or_create_cloak_for_type(solver, &base_sort);

            reveal_decl.apply(&[&val_ast])
        }
        // --- Sequence ---
        // `Seq::empty`
        SeqEmpty { t } => {
            let elem_sort = sort_to_z3(t, ctx, ir, None, ty_map);
            mk_seq_empty(ctx, &elem_sort)
        }
        // `Seq::length`
        SeqLength { t: _, seq } => {
            let seq_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_seq()
            .expect("Expected a sequence AST");
            seq_ast.length().into()
        }
        // `Seq::append`
        SeqAppend { t: _, seq, item } => {
            let seq_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_seq()
            .expect("Expected a sequence AST");
            let item_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let item_ast = ast::Seq::unit(ctx, &item_ast);
            debug_assert_eq!(seq_ast.get_sort(), item_ast.get_sort());
            ast::Seq::concat(ctx, &[&seq_ast, &item_ast]).into()
        }
        // `Seq::at_unchecked`
        SeqAt { t: _, seq, idx } => {
            let seq_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_seq()
            .expect("Expected a sequence AST");
            let index_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *idx,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            seq_ast.nth(&index_ast)
        }
        // `Seq::includes`
        SeqIncludes { t: _, seq, item } => {
            let seq_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_seq()
            .expect("Expected a sequence AST");
            let item_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let item_ast = ast::Seq::unit(ctx, &item_ast);
            seq_contains(ctx, &seq_ast, &item_ast).into()
        }
        // `Seq::is_empty`
        SeqIsEmpty { t: _, seq } => {
            let seq_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *seq,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_seq()
            .expect("Expected a sequence AST");
            seq_ast.length()._eq(&ast::Int::from_i64(ctx, 0)).into()
        }
        // --- Set ---
        // `Set::empty`
        SetEmpty { t } => {
            let elem_sort = sort_to_z3(t, ctx, ir, None, ty_map);
            ast::Set::empty(ctx, &elem_sort).into()
        }
        // `Set::length`
        SetLength { t, set } => {
            let m = MapLength {
                k: t.clone(),
                v: Sort::Boolean,
                map: *set,
            };
            process_intrinsic(
                ctx,
                solver,
                exp_registry,
                &m,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
        }
        // `Set::insert`
        SetInsert { t: _, set, item } => {
            let set_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *set,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let item_ast: ast::Dynamic = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            ast::Set::add(&set_ast, &item_ast).into()
        }
        // `Set::remove`
        SetRemove { t: _, set, item } => {
            let set_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *set,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let item_ast: ast::Dynamic = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            ast::Set::del(&set_ast, &item_ast).into()
        }
        // `Set::contains`
        SetContains { t: _, set, item } => {
            let set_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *set,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let item_ast: ast::Dynamic = process_expression(
                ctx,
                solver,
                exp_registry,
                *item,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            ast::Set::member(&set_ast, &item_ast).into()
        }
        // `Set::intersection`
        SetIntersection { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            ast::Set::intersect(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Set::union`
        SetUnion { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            ast::Set::set_union(ctx, &[&lhs_ast, &rhs_ast]).into()
        }
        // `Set::difference`
        SetDifference { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            ast::Set::difference(&lhs_ast, &rhs_ast).into()
        }
        // `Set::is_subset`
        SetIsSubset { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_set()
            .expect("Expected a set AST");
            ast::Set::set_subset(&lhs_ast, &rhs_ast).into()
        }
        // `Set::is_empty`
        SetIsEmpty { t, set } => {
            let m = MapLength {
                k: t.clone(),
                v: Sort::Boolean,
                map: *set,
            };
            process_intrinsic(
                ctx,
                solver,
                exp_registry,
                &m,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .unwrap()
            ._eq(&ast::Int::from_i64(ctx, 0))
            .into()
        }
        // --- Map ---
        // `Map::empty`
        MapEmpty { k, v } => {
            let key_sort = sort_to_z3(k, ctx, ir, None, ty_map);
            let val_sort = sort_to_z3(v, ctx, ir, None, ty_map);
            map_length_manager.populate(solver, &key_sort, &val_sort);
            empty_map(ctx, &key_sort, &val_sort)
        }
        // `Map::length`
        MapLength { k, v, map } => {
            let key_sort = sort_to_z3(k, ctx, ir, None, ty_map);
            let val_sort = sort_to_z3(v, ctx, ir, None, ty_map);
            let map_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *map,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_array()
            .expect("Expected a map AST");
            map_length_manager
                .get_map_length(&map_ast, &key_sort, &val_sort)
                .into()
        }
        // `Map::put_unchecked`
        MapPut {
            k: _,
            v: _,
            map,
            key,
            val,
        } => {
            let map_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *map,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_array()
            .expect("Expected a map AST");
            let key_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *key,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let val_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *val,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            ast::Array::store(&map_ast, &key_ast, &val_ast).into()
        }
        // `Map::get_unchecked`
        MapGet {
            k: _,
            v: _,
            map,
            key,
        } => {
            let map_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *map,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_array()
            .expect("Expected a map AST");
            let key_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *key,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            ast::Array::select(&map_ast, &key_ast)
        }
        // `Map::del_unchecked`
        MapDel { k: _, v, map, key } => {
            let map_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *map,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_array()
            .expect("Expected a map AST");
            let key_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *key,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let v_ast = sort_to_z3(v, ctx, ir, None, ty_map);
            let not_present_v = not_present(ctx, &v_ast);
            ast::Array::store(&map_ast, &key_ast, &not_present_v).into()
        }
        // `Map::contains_key`
        MapContainsKey { k: _, v, map, key } => {
            let map_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *map,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_array()
            .expect("Expected a map AST");
            let key_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *key,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let v_ast = sort_to_z3(v, ctx, ir, None, ty_map);
            let not_present_v = not_present(ctx, &v_ast);
            let has: ast::Dynamic = ast::Array::select(&map_ast, &key_ast);
            has._eq(&not_present_v).not().into()
        }
        // `Map::is_empty`
        MapIsEmpty { k, v, map } => {
            let m = MapLength {
                k: k.clone(),
                v: v.clone(),
                map: *map,
            };
            let len = process_intrinsic(
                ctx,
                solver,
                exp_registry,
                &m,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            )
            .as_int()
            .expect("Expected an integer AST");
            len._eq(&ast::Int::from_i64(ctx, 0)).into()
        }
        // --- Error ---
        // `Error::fresh`
        ErrFresh => {
            panic!("Unexpected Error::fresh expression in Z3 backend");
        }
        // `Error::merge`
        ErrMerge { lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            panic!("Unexpected Error::merge expression in Z3 backend: {lhs_ast:?} and {rhs_ast:?}");
        }
        // --- Generic eq/ne ---
        // `<any-smt-type>::eq`
        SmtEq { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            lhs_ast._eq(&rhs_ast).into()
        }
        // `<any-smt-type>::ne`
        SmtNe { t: _, lhs, rhs } => {
            let lhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *lhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            let rhs_ast = process_expression(
                ctx,
                solver,
                exp_registry,
                *rhs,
                ir,
                fn_map,
                ty_map,
                sort_map,
                bound_vars,
                cloak_manager,
                map_length_manager,
                axiomatic_parameters,
            );
            lhs_ast._eq(&rhs_ast).not().into()
        }
    }
}

//! Translate IR expressions to Z3 AST objects.

use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::sort::resolve_type_name;
use crate::backend::z3_api::context::Z3ApiContext;
use crate::backend::z3_api::intrinsics::translate_intrinsic;
use crate::backend::z3_api::mk_string_symbol;
use crate::backend::z3_api::Z3Ast;
use crate::ir::exp::{EnumSelector, ExpRegistry, Expression, VarKind, VariantCtor};
use crate::ir::index::{ExpId, UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort};
use std::collections::{BTreeSet, HashMap};

/// Translate an IR expression to a Z3 AST.
pub fn translate_expression<'ctx>(
    api_ctx: &mut Z3ApiContext<'ctx>,
    exp_registry: &ExpRegistry,
    exp_id: ExpId,
    var_map: &HashMap<String, z3_sys::Z3_ast>,
    scc_fids: &BTreeSet<UsrFunId>,
) -> Z3Ast<'ctx> {
    let ctx = api_ctx.ctx;
    let exp = exp_registry.lookup_exp(&exp_id);

    match exp {
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.kind {
                VarKind::Param | VarKind::Quant | VarKind::Axiom => {
                    let name = var.name.to_string();
                    if let Some(&ast) = var_map.get(&name) {
                        unsafe { Z3Ast::new(ctx, ast) }
                    } else {
                        panic!("Variable '{}' not found in var_map", name);
                    }
                }
                VarKind::Bound { bind } => {
                    translate_expression(api_ctx, exp_registry, *bind, var_map, scc_fids)
                }
                VarKind::Match {
                    head,
                    sort,
                    branch,
                    selector,
                } => {
                    let head_ast =
                        translate_expression(api_ctx, exp_registry, *head, var_map, scc_fids);
                    let field_idx = match selector {
                        EnumSelector::Tuple(idx) => *idx,
                        EnumSelector::Record(field_name) => {
                            // Look up the field index in the variant definition
                            let dt = api_ctx.ir.ty_registry.retrieve(*sort);
                            match dt {
                                DataType::Enum(variants) => {
                                    let variant = variants.get(branch).expect("variant not found");
                                    match variant {
                                        crate::ir::sort::Variant::Record(fields) => {
                                            fields
                                                .keys()
                                                .position(|k| k == field_name)
                                                .expect("field not found in record variant")
                                        }
                                        _ => panic!("Record selector on non-record variant"),
                                    }
                                }
                                _ => panic!("Record selector on non-enum type"),
                            }
                        }
                    };
                    let accessor = api_ctx.get_accessor(*sort, branch, field_idx);
                    unsafe {
                        let result = z3_sys::Z3_mk_app(ctx, accessor, 1, [head_ast.raw()].as_ptr()).expect("Z3_mk_app");
                        Z3Ast::new(ctx, result)
                    }
                }
            }
        }

        Expression::Pack { sort, elems } | Expression::Tuple { sort, slots: elems } => {
            let type_name = resolve_type_name(api_ctx.ir, *sort);
            let ctor_name = format!("mk-{}", type_name);
            let ctor = api_ctx.get_constructor(*sort, &ctor_name);
            let args: Vec<z3_sys::Z3_ast> = elems
                .iter()
                .map(|e| {
                    translate_expression(api_ctx, exp_registry, *e, var_map, scc_fids).raw()
                })
                .collect();
            unsafe {
                let result =
                    z3_sys::Z3_mk_app(ctx, ctor, args.len() as u32, args.as_ptr()).expect("Z3_mk_app");
                Z3Ast::new(ctx, result)
            }
        }

        Expression::Record { sort, fields } => {
            let type_name = resolve_type_name(api_ctx.ir, *sort);
            let ctor_name = format!("mk-{}", type_name);
            let ctor = api_ctx.get_constructor(*sort, &ctor_name);

            // Order fields according to type definition
            let dt = api_ctx.ir.ty_registry.retrieve(*sort);
            let args: Vec<z3_sys::Z3_ast> = if let DataType::Record(type_fields) = dt {
                type_fields
                    .keys()
                    .map(|type_field_name| {
                        let exp_id = fields.get(type_field_name).expect("field missing");
                        translate_expression(api_ctx, exp_registry, *exp_id, var_map, scc_fids)
                            .raw()
                    })
                    .collect()
            } else {
                fields
                    .values()
                    .map(|exp_id| {
                        translate_expression(api_ctx, exp_registry, *exp_id, var_map, scc_fids)
                            .raw()
                    })
                    .collect()
            };

            unsafe {
                let result =
                    z3_sys::Z3_mk_app(ctx, ctor, args.len() as u32, args.as_ptr()).expect("Z3_mk_app");
                Z3Ast::new(ctx, result)
            }
        }

        Expression::Enum {
            sort,
            branch,
            variant,
        } => {
            let ctor = api_ctx.get_constructor(*sort, branch);
            match variant {
                VariantCtor::Unit => unsafe {
                    let result = z3_sys::Z3_mk_app(ctx, ctor, 0, std::ptr::null()).expect("Z3_mk_app");
                    Z3Ast::new(ctx, result)
                },
                VariantCtor::Tuple(elems) => {
                    let args: Vec<z3_sys::Z3_ast> = elems
                        .iter()
                        .map(|e| {
                            translate_expression(api_ctx, exp_registry, *e, var_map, scc_fids)
                                .raw()
                        })
                        .collect();
                    unsafe {
                        let result = z3_sys::Z3_mk_app(
                            ctx,
                            ctor,
                            args.len() as u32,
                            args.as_ptr(),
                        ).expect("Z3_mk_app");
                        Z3Ast::new(ctx, result)
                    }
                }
                VariantCtor::Record(fields) => {
                    // Order fields according to variant definition
                    let dt = api_ctx.ir.ty_registry.retrieve(*sort);
                    let args: Vec<z3_sys::Z3_ast> = if let DataType::Enum(variants) = dt {
                        if let Some(vdef) = variants.get(branch) {
                            if let crate::ir::sort::Variant::Record(type_fields) = vdef {
                                type_fields
                                    .keys()
                                    .map(|type_field_name| {
                                        let exp_id =
                                            fields.get(type_field_name).expect("field missing");
                                        translate_expression(
                                            api_ctx,
                                            exp_registry,
                                            *exp_id,
                                            var_map,
                                            scc_fids,
                                        )
                                        .raw()
                                    })
                                    .collect()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    unsafe {
                        let result = z3_sys::Z3_mk_app(
                            ctx,
                            ctor,
                            args.len() as u32,
                            args.as_ptr(),
                        ).expect("Z3_mk_app");
                        Z3Ast::new(ctx, result)
                    }
                }
            }
        }

        Expression::AccessSlot { base, slot } => {
            let base_ast =
                translate_expression(api_ctx, exp_registry, *base, var_map, scc_fids);
            let base_sort = get_user_sort_of_base(exp_registry, *base, api_ctx);
            let type_name = resolve_type_name(api_ctx.ir, base_sort);
            let ctor_name = format!("mk-{}", type_name);
            let accessor = api_ctx.get_accessor(base_sort, &ctor_name, *slot);
            unsafe {
                let result =
                    z3_sys::Z3_mk_app(ctx, accessor, 1, [base_ast.raw()].as_ptr()).expect("Z3_mk_app");
                Z3Ast::new(ctx, result)
            }
        }

        Expression::AccessField { base, field } => {
            let base_ast =
                translate_expression(api_ctx, exp_registry, *base, var_map, scc_fids);
            let (base_sort, variant_name) =
                get_user_sort_and_variant(exp_registry, *base, api_ctx);
            let type_name = resolve_type_name(api_ctx.ir, base_sort);

            // Find the field index
            let dt = api_ctx.ir.ty_registry.retrieve(base_sort);
            let (variant_key, field_idx) = if let Some(ref vname) = variant_name {
                match dt {
                    DataType::Enum(variants) => {
                        let vdef = variants.get(vname).expect("variant not found");
                        match vdef {
                            crate::ir::sort::Variant::Record(fields_map) => {
                                let idx = fields_map
                                    .keys()
                                    .position(|k| k == field)
                                    .expect("field not found");
                                (vname.clone(), idx)
                            }
                            _ => panic!("AccessField on non-record variant"),
                        }
                    }
                    _ => panic!("AccessField with variant on non-enum"),
                }
            } else {
                match dt {
                    DataType::Record(fields_map) => {
                        let idx = fields_map
                            .keys()
                            .position(|k| k == field)
                            .expect("field not found");
                        (format!("mk-{}", type_name), idx)
                    }
                    _ => panic!("AccessField on non-record type"),
                }
            };

            let accessor = api_ctx.get_accessor(base_sort, &variant_key, field_idx);
            unsafe {
                let result =
                    z3_sys::Z3_mk_app(ctx, accessor, 1, [base_ast.raw()].as_ptr()).expect("Z3_mk_app");
                Z3Ast::new(ctx, result)
            }
        }

        Expression::Match { cases } => {
            if cases.is_empty() {
                unsafe {
                    let result = z3_sys::Z3_mk_true(ctx).expect("Z3_mk_true");
                    return Z3Ast::new(ctx, result);
                }
            }

            // Build nested ite from last to first
            let mut result =
                translate_expression(api_ctx, exp_registry, cases.last().unwrap().body, var_map, scc_fids);

            for case in cases.iter().rev().skip(1) {
                let condition = if case.atoms.len() == 1 {
                    let atom = &case.atoms[0];
                    let head_ast = translate_expression(
                        api_ctx,
                        exp_registry,
                        atom.head,
                        var_map,
                        scc_fids,
                    );
                    let tester = api_ctx.get_tester(atom.sort, &atom.branch);
                    unsafe {
                        let test =
                            z3_sys::Z3_mk_app(ctx, tester, 1, [head_ast.raw()].as_ptr()).expect("Z3_mk_app");
                        Z3Ast::new(ctx, test)
                    }
                } else {
                    let conditions: Vec<z3_sys::Z3_ast> = case
                        .atoms
                        .iter()
                        .map(|atom| {
                            let head_ast = translate_expression(
                                api_ctx,
                                exp_registry,
                                atom.head,
                                var_map,
                                scc_fids,
                            );
                            let tester = api_ctx.get_tester(atom.sort, &atom.branch);
                            unsafe {
                                z3_sys::Z3_mk_app(ctx, tester, 1, [head_ast.raw()].as_ptr()).expect("Z3_mk_app")
                            }
                        })
                        .collect();
                    unsafe {
                        let and = z3_sys::Z3_mk_and(
                            ctx,
                            conditions.len() as u32,
                            conditions.as_ptr(),
                        ).expect("Z3_mk_and");
                        Z3Ast::new(ctx, and)
                    }
                };

                let body =
                    translate_expression(api_ctx, exp_registry, case.body, var_map, scc_fids);
                unsafe {
                    let ite =
                        z3_sys::Z3_mk_ite(ctx, condition.raw(), body.raw(), result.raw()).expect("Z3_mk_ite");
                    result = Z3Ast::new(ctx, ite);
                }
            }
            result
        }

        Expression::Phi { cases, default } => {
            let mut result =
                translate_expression(api_ctx, exp_registry, *default, var_map, scc_fids);
            for case in cases.iter().rev() {
                let cond =
                    translate_expression(api_ctx, exp_registry, case.cond, var_map, scc_fids);
                let body =
                    translate_expression(api_ctx, exp_registry, case.body, var_map, scc_fids);
                unsafe {
                    let ite = z3_sys::Z3_mk_ite(ctx, cond.raw(), body.raw(), result.raw()).expect("Z3_mk_ite");
                    result = Z3Ast::new(ctx, ite);
                }
            }
            result
        }

        Expression::IterForall { vars, body } => {
            // Create bound variables and translate body
            let mut new_var_map = var_map.clone();
            let mut bound_consts: Vec<z3_sys::Z3_ast> = Vec::new();
            // NOTE: The `_coll_exp_id` is not used for Z3 quantifiers (it's for iteration semantics)

            for (var_id, _coll_exp_id) in vars {
                let var = exp_registry.lookup_var(var_id);
                let z3_sort = api_ctx.translate_sort(&var.sort);
                unsafe {
                    let sym = mk_string_symbol(ctx, &var.name.to_string());
                    let bound = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("Z3_mk_const");
                    z3_sys::Z3_inc_ref(ctx, bound);
                    new_var_map.insert(var.name.to_string(), bound);
                    bound_consts.push(bound);
                }
            }

            let body_ast =
                translate_expression(api_ctx, exp_registry, *body, &new_var_map, scc_fids);
            let bound_apps: Vec<z3_sys::Z3_app> = bound_consts
                .iter()
                .map(|&c| unsafe { z3_sys::Z3_to_app(ctx, c).expect("Z3_to_app") })
                .collect();

            unsafe {
                let forall = z3_sys::Z3_mk_forall_const(
                    ctx,
                    0, // weight
                    bound_apps.len() as u32,
                    bound_apps.as_ptr(),
                    0,                // num_patterns
                    std::ptr::null(), // patterns
                    body_ast.raw(),
                ).expect("Z3_mk_forall_const");
                Z3Ast::new(ctx, forall)
            }
        }

        Expression::IterExists { vars, body } | Expression::IterChoose { vars, body, .. } => {
            let mut new_var_map = var_map.clone();
            let mut bound_consts: Vec<z3_sys::Z3_ast> = Vec::new();

            for (var_id, _coll_exp_id) in vars {
                let var = exp_registry.lookup_var(var_id);
                let z3_sort = api_ctx.translate_sort(&var.sort);
                unsafe {
                    let sym = mk_string_symbol(ctx, &var.name.to_string());
                    let bound = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("Z3_mk_const");
                    z3_sys::Z3_inc_ref(ctx, bound);
                    new_var_map.insert(var.name.to_string(), bound);
                    bound_consts.push(bound);
                }
            }

            let body_ast =
                translate_expression(api_ctx, exp_registry, *body, &new_var_map, scc_fids);
            let bound_apps: Vec<z3_sys::Z3_app> = bound_consts
                .iter()
                .map(|&c| unsafe { z3_sys::Z3_to_app(ctx, c).expect("Z3_to_app") })
                .collect();

            unsafe {
                let exists = z3_sys::Z3_mk_exists_const(
                    ctx,
                    0,
                    bound_apps.len() as u32,
                    bound_apps.as_ptr(),
                    0,
                    std::ptr::null(),
                    body_ast.raw(),
                ).expect("Z3_mk_exists_const");
                Z3Ast::new(ctx, exists)
            }
        }

        Expression::Procedure { callee, args } => {
            let decl = api_ctx.get_func_decl(*callee);
            let arg_asts: Vec<z3_sys::Z3_ast> = args
                .iter()
                .map(|a| {
                    translate_expression(api_ctx, exp_registry, *a, var_map, scc_fids).raw()
                })
                .collect();
            unsafe {
                let result = z3_sys::Z3_mk_app(
                    ctx,
                    decl,
                    arg_asts.len() as u32,
                    arg_asts.as_ptr(),
                ).expect("Z3_mk_app");
                Z3Ast::new(ctx, result)
            }
        }

        Expression::Intrinsic(intrinsic) => {
            translate_intrinsic(api_ctx, intrinsic, exp_registry, var_map, scc_fids)
        }
    }
}

/// Get the UsrSortId of an expression's base (for AccessSlot).
fn get_user_sort_of_base(
    exp_registry: &ExpRegistry,
    base: ExpId,
    api_ctx: &Z3ApiContext,
) -> UsrSortId {
    let base_exp = exp_registry.lookup_exp(&base);
    match base_exp {
        Expression::Record { sort, .. }
        | Expression::Enum { sort, .. }
        | Expression::Pack { sort, .. }
        | Expression::Tuple { sort, .. } => *sort,
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.sort {
                Sort::User(sid) => *sid,
                s => panic!("Access base must have user sort, got {s}"),
            }
        }
        Expression::Procedure { callee, .. } => {
            let ret = api_ctx.ir.fn_registry.retrieve_sig(*callee).ret_ty.clone();
            match ret {
                Sort::User(sid) => sid,
                s => panic!("Access base must return user sort, got {s}"),
            }
        }
        other => panic!("Access base must be user-typed, got {other:?}"),
    }
}

/// Get the UsrSortId and optional variant name of an expression's base (for AccessField).
fn get_user_sort_and_variant(
    exp_registry: &ExpRegistry,
    base: ExpId,
    api_ctx: &Z3ApiContext,
) -> (UsrSortId, Option<String>) {
    let base_exp = exp_registry.lookup_exp(&base);
    match base_exp {
        Expression::Record { sort, .. } => (*sort, None),
        Expression::Enum { sort, branch, .. } => (*sort, Some(branch.clone())),
        Expression::Pack { sort, .. } | Expression::Tuple { sort, .. } => (*sort, None),
        Expression::Var(var_id) => {
            let var = exp_registry.lookup_var(var_id);
            match &var.sort {
                Sort::User(sid) => (*sid, None),
                s => panic!("Access base must have user sort, got {s}"),
            }
        }
        Expression::Procedure { callee, .. } => {
            let ret = api_ctx.ir.fn_registry.retrieve_sig(*callee).ret_ty.clone();
            match ret {
                Sort::User(sid) => (sid, None),
                s => panic!("Access base must return user sort, got {s}"),
            }
        }
        other => panic!("Access base must be user-typed, got {other:?}"),
    }
}

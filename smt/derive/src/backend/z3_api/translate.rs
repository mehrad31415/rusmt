//! Translate IR expressions to Z3 AST nodes via the Z3 API.

use crate::backend::z3::sort::resolve_type_name;
use crate::backend::z3_api::Z3Ast;
use crate::backend::z3_api::context::Z3ApiContext;
use crate::backend::z3_api::intrinsics::translate_intrinsic;
use crate::backend::z3_api::mk_string_symbol;
use crate::ir::exp::{EnumSelector, ExpRegistry, Expression, VarKind, VariantCtor};
use crate::ir::index::{ExpId, UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort, Variant};
use std::collections::{BTreeMap, HashMap};

/// Translate an IR `Expression` to a Z3 AST.
///
/// `var_map` maps variable names (as they appear in `Variable::name`) to their
/// Z3 AST. Parameters, quantified iterator variables, axiomatized choose!
/// variables, and match-bound locals are looked up here.
///
/// `rename` overrides per-call function decls during bounded-recursion
/// unrolling: any `Procedure` call to a fid in `rename` resolves to the
/// supplied decl instead of `api.get_func_decl(fid)`. Pass an empty map for
/// regular (non-unrolled) translation.
pub fn translate_expression<'ctx>(
    api: &mut Z3ApiContext<'ctx>,
    reg: &ExpRegistry,
    exp_id: ExpId,
    var_map: &HashMap<String, z3_sys::Z3_ast>,
    rename: &BTreeMap<UsrFunId, z3_sys::Z3_func_decl>,
) -> Z3Ast<'ctx> {
    let ctx = api.ctx;
    match reg.lookup_exp(&exp_id) {
        Expression::Var(vid) => {
            let var = reg.lookup_var(vid);
            match &var.kind {
                VarKind::Param | VarKind::Quant | VarKind::Axiom => {
                    let name = var.name.to_string();
                    let ast = *var_map
                        .get(&name)
                        .unwrap_or_else(|| panic!("variable '{}' not in var_map", name));
                    unsafe { Z3Ast::new(ctx, ast) }
                }
                VarKind::Bound { bind } => translate_expression(api, reg, *bind, var_map, rename),
                VarKind::Match {
                    head,
                    sort,
                    branch,
                    selector,
                } => {
                    let head_ast = translate_expression(api, reg, *head, var_map, rename);
                    let idx = match selector {
                        EnumSelector::Tuple(i) => *i,
                        EnumSelector::Record(fname) => {
                            record_field_index(api, *sort, branch, fname)
                        }
                    };
                    let acc = api.get_accessor(*sort, branch, idx);
                    unsafe {
                        let r = z3_sys::Z3_mk_app(ctx, acc, 1, [head_ast.raw()].as_ptr())
                            .expect("mk_app");
                        Z3Ast::new(ctx, r)
                    }
                }
            }
        }

        Expression::Pack { sort, elems } | Expression::Tuple { sort, slots: elems } => {
            let tname = resolve_type_name(api.ir, *sort);
            let ctor_branch = format!("mk-{}", tname);
            let ctor = api.get_constructor(*sort, &ctor_branch);
            let args: Vec<z3_sys::Z3_ast> = elems
                .iter()
                .map(|e| translate_expression(api, reg, *e, var_map, rename).raw())
                .collect();
            unsafe {
                let r =
                    z3_sys::Z3_mk_app(ctx, ctor, args.len() as u32, args.as_ptr()).expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::Record { sort, fields } => {
            let tname = resolve_type_name(api.ir, *sort);
            let ctor_branch = format!("mk-{}", tname);
            let ctor = api.get_constructor(*sort, &ctor_branch);
            // Order by type-definition field order.
            let dt = api.ir.ty_registry.retrieve(*sort);
            let args: Vec<z3_sys::Z3_ast> = match dt {
                DataType::Record(type_fields) => type_fields
                    .keys()
                    .map(|k| {
                        let e = fields
                            .get(k)
                            .unwrap_or_else(|| panic!("field {} missing", k));
                        translate_expression(api, reg, *e, var_map, rename).raw()
                    })
                    .collect(),
                _ => panic!("Record expression on non-record sort"),
            };
            unsafe {
                let r =
                    z3_sys::Z3_mk_app(ctx, ctor, args.len() as u32, args.as_ptr()).expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::Enum {
            sort,
            branch,
            variant,
        } => {
            let ctor = api.get_constructor(*sort, branch);
            let args: Vec<z3_sys::Z3_ast> = match variant {
                VariantCtor::Unit => Vec::new(),
                VariantCtor::Tuple(elems) => elems
                    .iter()
                    .map(|e| translate_expression(api, reg, *e, var_map, rename).raw())
                    .collect(),
                VariantCtor::Record(fields) => {
                    let dt = api.ir.ty_registry.retrieve(*sort);
                    match dt {
                        DataType::Enum(variants) => match variants.get(branch) {
                            Some(Variant::Record(type_fields)) => type_fields
                                .keys()
                                .map(|k| {
                                    let e = fields
                                        .get(k)
                                        .unwrap_or_else(|| panic!("field {} missing", k));
                                    translate_expression(api, reg, *e, var_map, rename).raw()
                                })
                                .collect(),
                            _ => panic!("enum variant shape mismatch"),
                        },
                        _ => panic!("Enum expression on non-enum sort"),
                    }
                }
            };
            unsafe {
                let r =
                    z3_sys::Z3_mk_app(ctx, ctor, args.len() as u32, args.as_ptr()).expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::AccessSlot { base, slot } => {
            let base_ast = translate_expression(api, reg, *base, var_map, rename);
            let base_sid = user_sort_of(reg, *base, api);
            let tname = resolve_type_name(api.ir, base_sid);
            let ctor_branch = format!("mk-{}", tname);
            let acc = api.get_accessor(base_sid, &ctor_branch, *slot);
            unsafe {
                let r = z3_sys::Z3_mk_app(ctx, acc, 1, [base_ast.raw()].as_ptr()).expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::AccessField { base, field } => {
            let base_ast = translate_expression(api, reg, *base, var_map, rename);
            let base_sid = user_sort_of(reg, *base, api);
            let dt = api.ir.ty_registry.retrieve(base_sid);
            let (branch, idx) = match dt {
                DataType::Record(fmap) => {
                    let tname = resolve_type_name(api.ir, base_sid);
                    let branch = format!("mk-{}", tname);
                    let idx = fmap
                        .keys()
                        .position(|k| k == field)
                        .unwrap_or_else(|| panic!("no field {}", field));
                    (branch, idx)
                }
                _ => panic!("AccessField on non-record sort"),
            };
            let acc = api.get_accessor(base_sid, &branch, idx);
            unsafe {
                let r = z3_sys::Z3_mk_app(ctx, acc, 1, [base_ast.raw()].as_ptr()).expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::Match { cases } => {
            if cases.is_empty() {
                unsafe {
                    let t = z3_sys::Z3_mk_true(ctx).expect("mk_true");
                    return Z3Ast::new(ctx, t);
                }
            }
            // Right-fold: last case body is the base, each earlier case is
            // (ite condition body result).
            let mut result =
                translate_expression(api, reg, cases.last().unwrap().body, var_map, rename);
            for case in cases.iter().rev().skip(1) {
                let cond = if case.atoms.len() == 1 {
                    let atom = &case.atoms[0];
                    let head = translate_expression(api, reg, atom.head, var_map, rename);
                    let tester = api.get_tester(atom.sort, &atom.branch);
                    unsafe {
                        let r = z3_sys::Z3_mk_app(ctx, tester, 1, [head.raw()].as_ptr())
                            .expect("mk_app");
                        Z3Ast::new(ctx, r)
                    }
                } else {
                    let conjs: Vec<z3_sys::Z3_ast> = case
                        .atoms
                        .iter()
                        .map(|atom| {
                            let head = translate_expression(api, reg, atom.head, var_map, rename);
                            let tester = api.get_tester(atom.sort, &atom.branch);
                            unsafe {
                                z3_sys::Z3_mk_app(ctx, tester, 1, [head.raw()].as_ptr())
                                    .expect("mk_app")
                            }
                        })
                        .collect();
                    unsafe {
                        let r = z3_sys::Z3_mk_and(ctx, conjs.len() as u32, conjs.as_ptr())
                            .expect("mk_and");
                        Z3Ast::new(ctx, r)
                    }
                };
                let body = translate_expression(api, reg, case.body, var_map, rename);
                unsafe {
                    let r = z3_sys::Z3_mk_ite(ctx, cond.raw(), body.raw(), result.raw())
                        .expect("mk_ite");
                    result = Z3Ast::new(ctx, r);
                }
            }
            result
        }

        Expression::Phi { cases, default } => {
            let mut result = translate_expression(api, reg, *default, var_map, rename);
            for case in cases.iter().rev() {
                let cond = translate_expression(api, reg, case.cond, var_map, rename);
                let body = translate_expression(api, reg, case.body, var_map, rename);
                unsafe {
                    let r = z3_sys::Z3_mk_ite(ctx, cond.raw(), body.raw(), result.raw())
                        .expect("mk_ite");
                    result = Z3Ast::new(ctx, r);
                }
            }
            result
        }

        Expression::IterForall { vars, body } => {
            let (bound_apps, new_var_map) = bind_iter_vars(api, reg, vars, var_map);
            let body_ast = translate_expression(api, reg, *body, &new_var_map, rename);
            let guard = build_iter_guard(api, reg, vars, &new_var_map, rename);
            let implication = match guard {
                Some(g) => unsafe {
                    z3_sys::Z3_mk_implies(ctx, g, body_ast.raw()).expect("implies")
                },
                None => body_ast.raw(),
            };
            unsafe {
                let r = z3_sys::Z3_mk_forall_const(
                    ctx,
                    0,
                    bound_apps.len() as u32,
                    bound_apps.as_ptr(),
                    0,
                    std::ptr::null(),
                    implication,
                )
                .expect("forall_const");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::IterExists { vars, body } => {
            let (bound_apps, new_var_map) = bind_iter_vars(api, reg, vars, var_map);
            let body_ast = translate_expression(api, reg, *body, &new_var_map, rename);
            let guard = build_iter_guard(api, reg, vars, &new_var_map, rename);
            let combined = match guard {
                Some(g) => unsafe {
                    z3_sys::Z3_mk_and(ctx, 2, [g, body_ast.raw()].as_ptr()).expect("and")
                },
                None => body_ast.raw(),
            };
            unsafe {
                let r = z3_sys::Z3_mk_exists_const(
                    ctx,
                    0,
                    bound_apps.len() as u32,
                    bound_apps.as_ptr(),
                    0,
                    std::ptr::null(),
                    combined,
                )
                .expect("exists_const");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::IterChoose { .. } => {
            // IterChoose at the root of a function body is handled by the context's
            // `build_choose_axiom`; it should never appear as a sub-expression.
            panic!("choose! must be the sole expression in a function body")
        }

        Expression::Procedure { callee, args } => {
            let decl = match rename.get(callee) {
                Some(&renamed) => renamed,
                None => api.get_func_decl(*callee),
            };
            let arg_asts: Vec<z3_sys::Z3_ast> = args
                .iter()
                .map(|a| translate_expression(api, reg, *a, var_map, rename).raw())
                .collect();
            unsafe {
                let r = z3_sys::Z3_mk_app(ctx, decl, arg_asts.len() as u32, arg_asts.as_ptr())
                    .expect("mk_app");
                Z3Ast::new(ctx, r)
            }
        }

        Expression::Intrinsic(intrinsic) => {
            translate_intrinsic(api, intrinsic, reg, var_map, rename)
        }
    }
}

fn bind_iter_vars<'ctx>(
    api: &mut Z3ApiContext<'ctx>,
    reg: &ExpRegistry,
    vars: &std::collections::BTreeMap<crate::ir::index::VarId, ExpId>,
    var_map: &HashMap<String, z3_sys::Z3_ast>,
) -> (Vec<z3_sys::Z3_app>, HashMap<String, z3_sys::Z3_ast>) {
    let ctx = api.ctx;
    let mut new_map = var_map.clone();
    let mut apps = Vec::new();
    for (vid, _) in vars {
        let var = reg.lookup_var(vid);
        let z3_sort = api.translate_sort(&var.sort);
        unsafe {
            let sym = mk_string_symbol(ctx, &var.name.to_string());
            let c = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("mk_const");
            z3_sys::Z3_inc_ref(ctx, c);
            new_map.insert(var.name.to_string(), c);
            apps.push(z3_sys::Z3_to_app(ctx, c).expect("to_app"));
        }
    }
    (apps, new_map)
}

/// Build the membership guard for a forall/exists over a collection.
///
/// - `Set T`  : (set.member v C)
/// - `Array K V`: select(C, v) ≠ null_V
/// - `Seq T`  : 0 ≤ v < seq.len(C)
fn build_iter_guard<'ctx>(
    api: &mut Z3ApiContext<'ctx>,
    reg: &ExpRegistry,
    vars: &std::collections::BTreeMap<crate::ir::index::VarId, ExpId>,
    var_map: &HashMap<String, z3_sys::Z3_ast>,
    rename: &BTreeMap<UsrFunId, z3_sys::Z3_func_decl>,
) -> Option<z3_sys::Z3_ast> {
    let ctx = api.ctx;
    let mut guards: Vec<z3_sys::Z3_ast> = Vec::new();
    for (vid, coll_eid) in vars {
        let var = reg.lookup_var(vid);
        let coll_sort = match reg.lookup_exp(coll_eid) {
            Expression::Var(cv) => reg.lookup_var(cv).sort.clone(),
            _ => panic!("iterator collection must be a plain variable"),
        };
        let coll_ast = translate_expression(api, reg, *coll_eid, var_map, rename).raw();
        let var_ast = *var_map
            .get(&var.name.to_string())
            .expect("bound iter var missing");

        let g = unsafe {
            match &coll_sort {
                Sort::Set(_) => {
                    z3_sys::Z3_mk_set_member(ctx, var_ast, coll_ast).expect("set_member")
                }
                Sort::Array(_, val_sort) => {
                    let null = api.null_for_sort(val_sort);
                    let sel = z3_sys::Z3_mk_select(ctx, coll_ast, var_ast).expect("select");
                    let eq = z3_sys::Z3_mk_eq(ctx, sel, null).expect("eq");
                    z3_sys::Z3_mk_not(ctx, eq).expect("not")
                }
                Sort::Seq(_) => {
                    let len = z3_sys::Z3_mk_seq_length(ctx, coll_ast).expect("len");
                    let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
                    let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("zero");
                    let ge = z3_sys::Z3_mk_ge(ctx, var_ast, zero).expect("ge");
                    let lt = z3_sys::Z3_mk_lt(ctx, var_ast, len).expect("lt");
                    z3_sys::Z3_mk_and(ctx, 2, [ge, lt].as_ptr()).expect("and")
                }
                other => panic!("cannot iterate over {:?}", other),
            }
        };
        guards.push(g);
    }
    match guards.len() {
        0 => None,
        1 => Some(guards[0]),
        _ => Some(unsafe {
            z3_sys::Z3_mk_and(ctx, guards.len() as u32, guards.as_ptr()).expect("and")
        }),
    }
}

fn record_field_index(api: &Z3ApiContext, sid: UsrSortId, branch: &str, field: &str) -> usize {
    let dt = api.ir.ty_registry.retrieve(sid);
    match dt {
        DataType::Enum(variants) => match variants.get(branch) {
            Some(Variant::Record(fields)) => fields
                .keys()
                .position(|k| k == field)
                .unwrap_or_else(|| panic!("field {} missing", field)),
            _ => panic!("record selector on non-record variant"),
        },
        DataType::Record(fields) => fields
            .keys()
            .position(|k| k == field)
            .unwrap_or_else(|| panic!("field {} missing", field)),
        _ => panic!("record selector on non-record type"),
    }
}

/// Resolve the user sort of an expression's base. Uses `ExpRegistry::derive_type`
/// so chains like `foo.bar.baz` work without re-implementing sort inference.
fn user_sort_of(reg: &ExpRegistry, base: ExpId, api: &Z3ApiContext) -> UsrSortId {
    match reg.derive_type(base, api.ir) {
        Sort::User(sid) => sid,
        s => panic!("expected user sort, got {s}"),
    }
}

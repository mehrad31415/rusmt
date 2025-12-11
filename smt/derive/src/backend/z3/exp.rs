// //! This module contains the conversion of expressions to SMT-LIB format.
// //! An expression is what constitutes the body of a function or an axiom.

// use crate::backend::z3::intrinsics::process_intrinsic;
// use crate::backend::z3::sort::{derive_type, sort_to_z3};
// use crate::ir::ctxt::IRContext;
// use crate::ir::exp::{
//     EnumSelector, ExpRegistry, Expression, MatchAtom, MatchCase, PhiCase, VarKind, VariantCtor,
//     VariantDtor,
// };
// use crate::ir::index::{ExpId, UsrFunId, UsrSortId, VarId};
// use crate::ir::name::{SmtSortName, Symbol, UsrSortName};
// use crate::ir::sort::Sort;
// use core::panic;
// use std::collections::{BTreeMap, HashMap};
// /// Converts an expression to a Z3 AST.
// pub fn process_expression<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     exp_id: ExpId,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     // destruct ExpRegistry
//     let ExpRegistry { vars: _, exps } = exp_registry;

//     let exp = exps.get(&exp_id).expect("expression not found in registry");
//     process_expression_rec(
//         ctx,
//         solver,
//         exp_registry,
//         exp,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     )
// }

// /// Recursively processes an expression to create a Z3 AST.
// fn process_expression_rec<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     exp: &Expression,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     match exp {
//         Expression::Intrinsic(intrinsic) => process_intrinsic(
//             ctx,
//             solver,
//             exp_registry,
//             intrinsic,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Var(var_id) => {
//             let var = exp_registry
//                 .vars
//                 .get(var_id)
//                 .expect("variable not found in registry");
//             let varname = &var.name;
//             let varkind = &var.kind;
//             match varkind {
//                 VarKind::Param => bound_vars
//                     .iter()
//                     .find(|(name, _)| name == varname)
//                     .map_or_else(
//                         || panic!("Parameter {} not found in bound variables", varname),
//                         |(_, var)| var.clone(),
//                     ),
//                 VarKind::Axiom => axiomatic_parameters
//                     .get(&varname.to_string())
//                     .cloned()
//                     .unwrap_or_else(|| {
//                         panic!(
//                             "Axiomatic parameter {} not found in axiomatic parameters",
//                             varname
//                         )
//                     }),
//                 VarKind::Quant => axiomatic_parameters
//                     .get(&varname.to_string())
//                     .cloned()
//                     .unwrap_or_else(|| {
//                         panic!(
//                             "Axiomatic parameter {} not found in axiomatic parameters",
//                             varname
//                         )
//                     }),
//                 VarKind::Match {
//                     head,
//                     sort,
//                     branch,
//                     selector,
//                 } => {
//                     // Process the head expression first
//                     let head_ast = process_expression(
//                         ctx,
//                         solver,
//                         exp_registry,
//                         *head,
//                         ir,
//                         fn_map,
//                         ty_map,
//                         sort_map,
//                         bound_vars,
//                         cloak_manager,
//                         map_length_manager,
//                         axiomatic_parameters,
//                     );

//                     let dt_variants = &ty_map
//                         .get(sort)
//                         .expect("sort not found in type map")
//                         .variants;
//                     let (sort_name, _) = ir.ty_registry.reverse_lookup(*sort);
//                     let sort_name = sort_name.expect("sort name not found");

//                     // Select the appropriate accessor based on the selector type
//                     let accessor_fn = match selector {
//                         EnumSelector::Tuple(field_idx) => {
//                             let variant = dt_variants
//                                 .iter()
//                                 .enumerate()
//                                 .find(|(_, variant)| {
//                                     // Extract variant name from constructor declaration name
//                                     let constructor_name = variant.constructor.name().to_string();
//                                     constructor_name
//                                         == format!(
//                                             "field_{}_{}_{}_",
//                                             sort_name,
//                                             branch,
//                                             field_idx + 1
//                                         )
//                                 })
//                                 .map(|(_, variant)| variant)
//                                 .unwrap_or_else(|| {
//                                     panic!(
//                                         "Branch '{}' not found in datatype variants for sort '{}'",
//                                         branch, sort_name
//                                     )
//                                 });

//                             if field_idx >= &variant.accessors.len() {
//                                 panic!(
//                                     "Accessor index {} out of bounds for variant '{}' (has {} accessors)",
//                                     field_idx,
//                                     branch,
//                                     variant.accessors.len()
//                                 );
//                             }
//                             &variant.accessors[*field_idx]
//                         }
//                         EnumSelector::Record(field_name) => {
//                             let variant = dt_variants
//                                 .iter()
//                                 .enumerate()
//                                 .find(|(_, variant)| {
//                                     // Extract variant name from constructor declaration name
//                                     let constructor_name = variant.constructor.name().to_string();
//                                     constructor_name
//                                         == format!("record_{sort_name}_{branch}_{field_name}_")
//                                 })
//                                 .map(|(_, variant)| variant)
//                                 .unwrap_or_else(|| {
//                                     panic!(
//                                         "Branch '{}' not found in datatype variants for sort '{}'",
//                                         branch, sort_name
//                                     )
//                                 });
//                             let index = variant
//                                     .accessors
//                                     .iter()
//                                     .position(|accessor| {
//                                         accessor.name()
//                                             == format!("record_{sort_name}_{branch}_{field_name}_")
//                                     })
//                                     .unwrap_or_else(|| panic!("Record field '{}' not found in variant '{}' of sort '{}'. Available accessors: {:?}",
//                                         field_name,
//                                         branch,
//                                         sort_name,
//                                         variant.accessors
//                                             .iter()
//                                             .map(|acc| acc.name().to_string())
//                                             .collect::<Vec<_>>()));

//                             &variant.accessors[index]
//                         }
//                     };

//                     // Get the accessor function and apply it to the head
//                     accessor_fn.apply(&[&head_ast])
//                 }
//                 VarKind::Bound { bind } => process_expression(
//                     ctx,
//                     solver,
//                     exp_registry,
//                     *bind,
//                     ir,
//                     fn_map,
//                     ty_map,
//                     sort_map,
//                     bound_vars,
//                     cloak_manager,
//                     map_length_manager,
//                     axiomatic_parameters,
//                 ),
//             }
//         }
//         Expression::Pack { sort, elems } => pack_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             *sort,
//             elems,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Tuple { sort, slots } => tuple_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             *sort,
//             slots,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Record { sort, fields } => record_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             sort,
//             fields,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Enum {
//             sort,
//             branch,
//             variant,
//         } => enum_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             *sort,
//             branch,
//             variant,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::AccessSlot { base, slot } => access_slot(
//             ctx,
//             solver,
//             exp_registry,
//             *base,
//             *slot,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::AccessField { base, field } => access_field(
//             ctx,
//             solver,
//             exp_registry,
//             *base,
//             field,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Match { cases } => match_to_ite(
//             ctx,
//             solver,
//             exp_registry,
//             cases,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Phi { cases, default } => phi_to_ite(
//             ctx,
//             solver,
//             exp_registry,
//             cases,
//             *default,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::Forall { vars, body } => {
//             let mut bound_constants = Vec::new();
//             let mut local_axiom_params = axiomatic_parameters.clone();

//             for (var_id, var_sort) in vars {
//                 let var_name = exp_registry
//                     .vars
//                     .get(var_id)
//                     .expect("variable not found in registry")
//                     .name
//                     .to_string();
//                 let s = sort_to_z3(var_sort, ctx, ir, None, ty_map);
//                 let bound_var = ast::Dynamic::new_const(ctx, var_name.clone(), &s);

//                 bound_constants.push(bound_var.clone());
//                 local_axiom_params.insert(var_name, bound_var);
//             }

//             // Process the body with local axiomatic parameters
//             let body_ast = process_expression(
//                 ctx,
//                 solver,
//                 exp_registry,
//                 *body,
//                 ir,
//                 fn_map,
//                 ty_map,
//                 sort_map,
//                 bound_vars,
//                 cloak_manager,
//                 map_length_manager,
//                 &mut local_axiom_params,
//             );

//             // Convert body to Bool
//             let body_bool = body_ast
//                 .as_bool()
//                 .expect("Forall body must be a Boolean expression");

//             let bound_refs: Vec<&dyn Ast> = bound_constants.iter().map(|v| v as &dyn Ast).collect();
//             let forall_expr = ast::forall_const(ctx, &bound_refs, &[], &body_bool);
//             forall_expr.into()
//         }
//         Expression::Exists { vars, body } => {
//             let mut bound_constants = Vec::new();
//             let mut local_axiom_params = axiomatic_parameters.clone();

//             for (var_id, var_sort) in vars {
//                 let var_name = exp_registry
//                     .vars
//                     .get(var_id)
//                     .expect("variable not found in registry")
//                     .name
//                     .to_string();
//                 let s = sort_to_z3(var_sort, ctx, ir, None, ty_map);
//                 let bound_var = ast::Dynamic::new_const(ctx, var_name.clone(), &s);

//                 bound_constants.push(bound_var.clone());
//                 local_axiom_params.insert(var_name, bound_var);
//             }

//             // Process the body with local axiomatic parameters
//             let body_ast = process_expression(
//                 ctx,
//                 solver,
//                 exp_registry,
//                 *body,
//                 ir,
//                 fn_map,
//                 ty_map,
//                 sort_map,
//                 bound_vars,
//                 cloak_manager,
//                 map_length_manager,
//                 &mut local_axiom_params,
//             );

//             // Convert body to Bool
//             let body_bool = body_ast
//                 .as_bool()
//                 .expect("Exists body must be a Boolean expression");

//             let bound_refs: Vec<&dyn Ast> = bound_constants.iter().map(|v| v as &dyn Ast).collect();
//             let exists_expr = ast::exists_const(ctx, &bound_refs, &[], &body_bool);
//             exists_expr.into()
//         }
//         Expression::Choose {
//             vars: _,
//             body: _,
//             rets: _,
//         } => {
//             unimplemented!()
//         }
//         Expression::IterForall { vars, body } => iter_forall_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             vars,
//             *body,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::IterExists { vars, body } => iter_exists_to_z3(
//             ctx,
//             solver,
//             exp_registry,
//             vars,
//             *body,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         ),
//         Expression::IterChoose {
//             vars: _,
//             body: _,
//             rets: _,
//         } => {
//             unimplemented!()
//         }
//         Expression::Procedure { callee, args } => {
//             let def = fn_map.get(callee).expect("function not found");
//             let arg_exprs: Vec<_> = args
//                 .iter()
//                 .map(|arg| {
//                     process_expression(
//                         ctx,
//                         solver,
//                         exp_registry,
//                         *arg,
//                         ir,
//                         fn_map,
//                         ty_map,
//                         sort_map,
//                         bound_vars,
//                         cloak_manager,
//                         map_length_manager,
//                         axiomatic_parameters,
//                     )
//                 })
//                 .collect();
//             def.apply(
//                 arg_exprs
//                     .iter()
//                     .map(|a| a as &dyn Ast)
//                     .collect::<Vec<_>>()
//                     .as_slice(),
//             )
//         }
//     }
// }

// /// Converts a Phi expression to an if-then-else chain.
// fn phi_to_ite<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     cases: &[PhiCase],
//     default: ExpId,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     if cases.is_empty() {
//         panic!("Phi expression must have at least one case");
//     }
//     let mut acc = process_expression(
//         ctx,
//         solver,
//         exp_registry,
//         default,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     );

//     for PhiCase { cond, body } in cases.iter().rev() {
//         let c = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             *cond,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         )
//         .as_bool()
//         .expect("Condition must be a boolean expression");

//         let t = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             *body,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );

//         acc = c.ite(&t, &acc);
//     }
//     acc
// }

// /// Accesses a field in a struct or a record.
// fn access_field<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     base: ExpId,
//     field: &str,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let ExpRegistry { vars: _, exps } = exp_registry;
//     let type_exp = exps.get(&base).expect("expression not found in registry");
//     if let Expression::Record { sort, fields: _ } = type_exp {
//         let variants = &ty_map
//             .get(sort)
//             .expect("sort not found in type map")
//             .variants;
//         let type_name = ir
//             .ty_registry
//             .reverse_lookup(*sort)
//             .0
//             .expect("sort name not found");
//         let field_accessor = variants[0]
//             .accessors
//             .iter()
//             .find(|accessor| accessor.name() == format!("record_{type_name}_{field}_"))
//             .expect("field accessor not found");
//         let base_ast = process_expression_rec(
//             ctx,
//             solver,
//             exp_registry,
//             type_exp,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//         let field_ast = field_accessor.apply(&[&base_ast]);
//         field_ast
//     } else {
//         panic!("Base expression must be a record, found: {:?}", type_exp);
//     }
// }

// /// Accesses a slot in a struct tuple.
// fn access_slot<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     base: ExpId,
//     slot: usize,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let ExpRegistry { vars: _, exps } = exp_registry;
//     let type_exp = exps.get(&base).expect("expression not found in registry");
//     if let Expression::Tuple { sort, slots: _ } = type_exp {
//         let variants = &ty_map
//             .get(sort)
//             .expect("sort not found in type map")
//             .variants;
//         let type_name = ir
//             .ty_registry
//             .reverse_lookup(*sort)
//             .0
//             .expect("sort name not found");
//         let slot_accessor = variants[0]
//             .accessors
//             .iter()
//             .find(|accessor| accessor.name() == format!("field_{}_{}", type_name, slot + 1))
//             .expect("slot accessor not found");
//         let base_ast = process_expression_rec(
//             ctx,
//             solver,
//             exp_registry,
//             type_exp,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//         let slot_ast = slot_accessor.apply(&[&base_ast]);
//         slot_ast
//     } else {
//         panic!("Base expression must be a tuple, found: {:?}", type_exp);
//     }
// }

// /// Packs a set of expressions into a tuple.
// fn pack_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     sort: UsrSortId,
//     elems: &[ExpId],
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let (sort_name, _ty_args) = ir.ty_registry.reverse_lookup(sort);
//     if sort_name.is_some() {
//         panic!("tuples are unnamed types");
//     }
//     let variants = &ty_map
//         .get(&sort)
//         .expect("sort not found in type map for Pack")
//         .variants;
//     assert!(
//         variants.len() == 1,
//         "Tuple sort must have exactly one variant"
//     );
//     let variant = &variants[0];

//     let mut arg_asts = Vec::with_capacity(elems.len());
//     for &e in elems {
//         let e_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             e,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//         arg_asts.push(e_ast);
//     }

//     // sanity check for the number of arguments
//     let accessors = &variant.accessors;
//     assert!(
//         accessors.len() == arg_asts.len(),
//         "Tuple arity mismatch: expected {} fields but got {}",
//         accessors.len(),
//         arg_asts.len()
//     );

//     let args_ref: Vec<&dyn Ast> = arg_asts.iter().map(|a| a as &dyn Ast).collect();
//     variant.constructor.apply(&args_ref)
// }

// fn tuple_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     sort: UsrSortId,
//     slots: &[ExpId],
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let (sort_name, _ty_args) = ir.ty_registry.reverse_lookup(sort);
//     let _sort_name = sort_name.expect("struct tuple sort name not found");

//     let variants = &ty_map
//         .get(&sort)
//         .expect("sort not found in type map for Tuple")
//         .variants;
//     assert!(
//         variants.len() == 1,
//         "Tuple sort must have exactly one variant"
//     );
//     let variant = &variants[0];

//     let mut arg_asts = Vec::with_capacity(slots.len());
//     for &s in slots {
//         let s_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             s,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//         arg_asts.push(s_ast);
//     }

//     // sanity check for the number of arguments
//     let accessors = &variant.accessors;
//     assert!(
//         accessors.len() == arg_asts.len(),
//         "Tuple arity mismatch: expected {} fields but got {}",
//         accessors.len(),
//         arg_asts.len()
//     );

//     let args_ref: Vec<&dyn Ast> = arg_asts.iter().map(|a| a as &dyn Ast).collect();
//     variant.constructor.apply(&args_ref)
// }

// /// Converts a record expression to a Z3 AST.
// fn record_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     sort: &UsrSortId,
//     fields: &BTreeMap<String, ExpId>,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let (sort_name, _ty_args) = ir.ty_registry.reverse_lookup(*sort);
//     let sort_name = sort_name.expect("record sort name not found");

//     let variants = &ty_map
//         .get(sort)
//         .expect("sort not found in type map for Record")
//         .variants;
//     assert!(
//         variants.len() == 1,
//         "Record sort must have exactly one variant"
//     );
//     let variant = &variants[0];

//     let mut field_asts: Vec<ast::Dynamic> = Vec::with_capacity(fields.len());
//     let mut field_names: Vec<String> = Vec::with_capacity(fields.len());
//     for (field_name, field_exp_id) in fields {
//         let field_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             *field_exp_id,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//         field_names.push(field_name.clone());
//         field_asts.push(field_ast);
//     }
//     // sanity check for the number of arguments
//     let accessors = &variant.accessors;
//     assert!(
//         accessors.len() == field_asts.len(),
//         "Record arity mismatch: expected {} fields but got {}",
//         accessors.len(),
//         field_asts.len()
//     );

//     let mut applied_fields: Vec<ast::Dynamic> = Vec::with_capacity(field_asts.len());
//     for (field_name, field_ast) in field_names.iter().zip(field_asts.iter()) {
//         let accessor = accessors
//             .iter()
//             .find(|accessor| accessor.name() == format!("record_{sort_name}_{field_name}_"))
//             .unwrap_or_else(|| {
//                 panic!(
//                     "Field '{}' not found in record variant '{}'. Available accessors: {:?}",
//                     field_name,
//                     sort_name,
//                     accessors
//                         .iter()
//                         .map(|acc| acc.name().to_string())
//                         .collect::<Vec<_>>()
//                 )
//             });
//         let applied = accessor.apply(&[field_ast]);
//         applied_fields.push(applied);
//     }
//     let args_ref: Vec<&dyn Ast> = applied_fields.iter().map(|a| a as &dyn Ast).collect();
//     variant.constructor.apply(&args_ref)
// }

// fn enum_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     sort: UsrSortId,
//     branch: &str,
//     variant: &VariantCtor,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let (sort_name, _ty_args) = ir.ty_registry.reverse_lookup(sort);
//     let sort_name = sort_name.expect("enum sort name not found");

//     let variants = &ty_map
//         .get(&sort)
//         .expect("sort not found in type map for Enum")
//         .variants;

//     // Find the variant that matches the branch name
//     let matching_variant = variants
//         .iter()
//         .find(|dt_variant| {
//             let constructor_name = dt_variant.constructor.name().to_string();
//             constructor_name == *branch
//         })
//         .unwrap_or_else(|| {
//             panic!(
//                 "Branch '{}' not found in enum variants for sort '{}'. Available variants: {:?}",
//                 branch,
//                 sort_name,
//                 variants
//                     .iter()
//                     .map(|v| v.constructor.name().to_string())
//                     .collect::<Vec<_>>()
//             )
//         });

//     // Get the constructor for this variant
//     let constructor = &matching_variant.constructor;

//     // Handle different variant types based on VariantCtor
//     match variant {
//         VariantCtor::Unit => {
//             // Unit variants have no arguments - just apply the constructor with empty args
//             constructor.apply(&[])
//         }

//         VariantCtor::Tuple(arg_exprs) => {
//             // Process each argument expression
//             let mut arg_asts = Vec::with_capacity(arg_exprs.len());
//             for &expr_id in arg_exprs {
//                 let arg_ast = process_expression(
//                     ctx,
//                     solver,
//                     exp_registry,
//                     expr_id,
//                     ir,
//                     fn_map,
//                     ty_map,
//                     sort_map,
//                     bound_vars,
//                     cloak_manager,
//                     map_length_manager,
//                     axiomatic_parameters,
//                 );
//                 arg_asts.push(arg_ast);
//             }

//             // Verify argument count matches accessor count
//             let expected_argc = matching_variant.accessors.len();
//             if arg_asts.len() != expected_argc {
//                 panic!(
//                     "Argument count mismatch for tuple variant '{}': expected {} but got {}",
//                     branch,
//                     expected_argc,
//                     arg_asts.len()
//                 );
//             }

//             // Apply constructor to arguments
//             let args_ref: Vec<&dyn Ast> = arg_asts.iter().map(|a| a as &dyn Ast).collect();
//             constructor.apply(&args_ref)
//         }

//         VariantCtor::Record(field_map) => {
//             let expected_argc = matching_variant.accessors.len();
//             let mut arg_asts = vec![None; expected_argc];

//             // Process each field in the record
//             for (field_name, expr_id) in field_map {
//                 // Find the accessor index for this field name
//                 let field_index = find_record_field_index_for_enum(
//                     sort_name,
//                     branch,
//                     field_name,
//                     matching_variant,
//                 );

//                 // Process the expression for this field
//                 let field_ast = process_expression(
//                     ctx,
//                     solver,
//                     exp_registry,
//                     *expr_id,
//                     ir,
//                     fn_map,
//                     ty_map,
//                     sort_map,
//                     bound_vars,
//                     cloak_manager,
//                     map_length_manager,
//                     axiomatic_parameters,
//                 );

//                 arg_asts[field_index] = Some(field_ast);
//             }

//             // Ensure all fields were provided
//             let final_args: Result<Vec<_>, _> = arg_asts
//                 .into_iter()
//                 .enumerate()
//                 .map(|(i, opt_ast)| {
//                     opt_ast.ok_or_else(|| {
//                         format!("Missing field at position {i} for record variant '{branch}'")
//                     })
//                 })
//                 .collect();

//             let final_args = final_args.unwrap_or_else(|err| panic!("{}", err));

//             // Apply constructor to arguments
//             let args_ref: Vec<&dyn Ast> = final_args.iter().map(|a| a as &dyn Ast).collect();
//             constructor.apply(&args_ref)
//         }
//     }
// }

// fn find_record_field_index_for_enum(
//     sort_name: &UsrSortName,
//     branch: &str,
//     field_name: &str,
//     variant: &DatatypeVariant,
// ) -> usize {
//     // The field names follow the pattern: "record_{sort_name}_{branch}_{field_name}_"
//     let expected_field_name = format!("record_{sort_name}_{branch}_{field_name}_");

//     // Find the accessor with matching name
//     for (idx, accessor) in variant.accessors.iter().enumerate() {
//         let accessor_name = accessor.name().to_string();
//         if accessor_name == expected_field_name {
//             return idx;
//         }
//     }

//     panic!(
//         "Record field '{}' not found in variant '{}' of sort '{}'. Available accessors: {:?}",
//         field_name,
//         branch,
//         sort_name,
//         variant
//             .accessors
//             .iter()
//             .map(|acc| acc.name().to_string())
//             .collect::<Vec<_>>()
//     )
// }

// fn match_to_ite<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     cases: &Vec<MatchCase>,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     if cases.is_empty() {
//         panic!("no cases in match");
//     }

//     let first_case = &cases[0];
//     let remaining_cases: Vec<MatchCase> = cases.iter().skip(1).cloned().collect();

//     // only has one case
//     if remaining_cases.is_empty() {
//         // Extend bound variables with pattern bindings and process body
//         let extended_bound_vars = extend_bound_vars_with_patterns(
//             ctx,
//             solver,
//             exp_registry,
//             &first_case.atoms,
//             bound_vars,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );

//         return process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             first_case.body,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             &extended_bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );
//     }

//     // Build the condition for the first case
//     let condition = build_match_condition(
//         ctx,
//         solver,
//         exp_registry,
//         &first_case.atoms,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     );

//     // Process the then branch (first case body with extended bindings)
//     let extended_bound_vars = extend_bound_vars_with_patterns(
//         ctx,
//         solver,
//         exp_registry,
//         &first_case.atoms,
//         bound_vars,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     );

//     let then_branch = process_expression(
//         ctx,
//         solver,
//         exp_registry,
//         first_case.body,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         &extended_bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     );

//     // Process the else branch (recursive call on remaining cases)
//     let else_branch = match_to_ite(
//         ctx,
//         solver,
//         exp_registry,
//         &remaining_cases,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_vars,
//         cloak_manager,
//         map_length_manager,
//         axiomatic_parameters,
//     );

//     // Create the if-then-else expression
//     condition.ite(&then_branch, &else_branch)
// }

// fn build_match_condition<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     atoms: &[MatchAtom],
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Bool {
//     let mut conditions = Vec::new();

//     for atom in atoms {
//         // Process the head expression
//         let head_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             atom.head,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );

//         // Get the datatype variants for this sort
//         let variants = &ty_map
//             .get(&atom.sort)
//             .expect("sort not found in type map")
//             .variants;
//         let (sort_name, _) = ir.ty_registry.reverse_lookup(atom.sort);
//         let _sort_name = sort_name.expect("sort name not found");

//         // Find the matching variant for this branch
//         let matching_variant = variants
//             .iter()
//             .find(|variant| {
//                 let constructor_name = variant.constructor.name().to_string();
//                 constructor_name == atom.branch
//             })
//             .unwrap_or_else(|| panic!("Branch '{}' not found in datatype variants", atom.branch));

//         // Create the tester condition using the variant's tester function
//         let tester_condition = matching_variant
//             .tester
//             .apply(&[&head_ast])
//             .as_bool()
//             .unwrap();
//         conditions.push(tester_condition);
//     }

//     // Combine all conditions with AND
//     if conditions.len() == 1 {
//         conditions.into_iter().next().unwrap()
//     } else {
//         ast::Bool::and(ctx, &conditions.iter().collect::<Vec<_>>())
//     }
// }

// fn extend_bound_vars_with_patterns<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     atoms: &[MatchAtom],
//     bound_vars: &Vec<(Symbol, ast::Dynamic)>,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_parameters: &mut HashMap<String, ast::Dynamic>,
// ) -> Vec<(Symbol, ast::Dynamic)> {
//     let mut extended_vars = bound_vars.clone();

//     for atom in atoms {
//         // Process the head expression
//         let head_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             atom.head,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_vars,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_parameters,
//         );

//         // Get the datatype variants for this sort
//         let variants = &ty_map
//             .get(&atom.sort)
//             .expect("sort not found in type map")
//             .variants;

//         // Find the matching variant for this branch
//         let matching_variant = variants
//             .iter()
//             .find(|variant| {
//                 let constructor_name = variant.constructor.name().to_string();
//                 constructor_name == atom.branch
//             })
//             .unwrap_or_else(|| panic!("Branch '{}' not found in datatype variants", atom.branch));

//         // Extract variables based on the destructor pattern
//         match &atom.variant {
//             VariantDtor::Unit => {
//                 // No variables to bind for unit variants
//             }

//             VariantDtor::Tuple(var_opts) => {
//                 // Bind variables positionally for tuple variants
//                 for (idx, var_opt) in var_opts.iter().enumerate() {
//                     if let Some(var_id) = var_opt {
//                         if idx < matching_variant.accessors.len() {
//                             let accessor = &matching_variant.accessors[idx];
//                             let field_value = accessor.apply(&[&head_ast]);

//                             // Get variable name from registry
//                             let var = exp_registry
//                                 .vars
//                                 .get(var_id)
//                                 .expect("Variable not found in registry");
//                             let var_symbol = var.name.clone();

//                             extended_vars.push((var_symbol, field_value));
//                         }
//                     }
//                 }
//             }

//             VariantDtor::Record(field_map) => {
//                 // Bind variables by field name for record variants
//                 let (sort_name, _) = ir.ty_registry.reverse_lookup(atom.sort);
//                 let sort_name = sort_name.expect("sort name not found");

//                 for (field_name, var_opt) in field_map {
//                     if let Some(var_id) = var_opt {
//                         // Find accessor index by field name pattern
//                         let expected_accessor_name =
//                             format!("record_{}_{}_{}_", sort_name, atom.branch, field_name);

//                         let accessor_idx = matching_variant
//                             .accessors
//                             .iter()
//                             .enumerate()
//                             .find(|(_, accessor)| accessor.name() == expected_accessor_name)
//                             .map(|(idx, _)| idx)
//                             .unwrap_or_else(|| {
//                                 panic!("Accessor '{expected_accessor_name}' not found in variant")
//                             });

//                         let accessor = &matching_variant.accessors[accessor_idx];
//                         let field_value = accessor.apply(&[&head_ast]);

//                         // Get variable name from registry
//                         let var = exp_registry
//                             .vars
//                             .get(var_id)
//                             .expect("Variable not found in registry");
//                         let var_symbol = var.name.clone();

//                         extended_vars.push((var_symbol, field_value));
//                     }
//                 }
//             }
//         }
//     }

//     extended_vars
// }

// fn iter_forall_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     vars: &BTreeMap<VarId, ExpId>,
//     body: ExpId,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_env: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_params: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let mut local_axiom_params = axiomatic_params.clone();

//     let mut z3_vars = Vec::new();
//     let mut z3_colls = Vec::new();
//     for (vid, coll_eid) in vars {
//         let vinfo = exp_registry.vars.get(vid).unwrap();
//         let z3_v = ast::Dynamic::fresh_const(
//             ctx,
//             vinfo.name.as_ref(),
//             &sort_to_z3(&vinfo.sort, ctx, ir, None, ty_map),
//         );
//         z3_vars.push(((vid, vinfo), z3_v.clone()));

//         let coll_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             *coll_eid,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_env,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_params,
//         );
//         z3_colls.push(coll_ast);
//     }

//     for ((_vid, vinfo), z3_v) in &z3_vars {
//         local_axiom_params.insert(vinfo.name.to_string(), z3_v.clone());
//     }

//     let body_bool = process_expression(
//         ctx,
//         solver,
//         exp_registry,
//         body,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_env,
//         cloak_manager,
//         map_length_manager,
//         &mut local_axiom_params,
//     )
//     .as_bool()
//     .expect("IterForall body must be boolean");

//     let mut guards = Vec::new();
//     for (((vid, _vinfo), z3_v), coll_ast) in z3_vars.iter().zip(z3_colls.iter()) {
//         let coll_sort = derive_type(exp_registry, ir, vars.get_key_value(vid).unwrap().1);
//         let guard = match coll_sort {
//             Sort::Set(_) => coll_ast.as_set().unwrap().member(z3_v),
//             Sort::Map(_, v) => {
//                 let sel = coll_ast.as_array().unwrap().select(z3_v);
//                 sel._eq(&not_present(ctx, &sort_to_z3(&v, ctx, ir, None, ty_map)))
//                     .not()
//             }
//             Sort::Seq(_) => {
//                 let idx = z3_v.as_int().expect("Expected integer");
//                 let seq_ast = coll_ast.as_seq().unwrap();
//                 let len_ast = seq_ast.length();

//                 ast::Bool::and(
//                     ctx,
//                     &[&idx.ge(&ast::Int::from_i64(ctx, 0)), &idx.lt(&len_ast)],
//                 )
//             }
//             _ => panic!("iterator must be over Set, Seq, or Map"),
//         };
//         guards.push(guard);
//     }
//     let guard_conj = ast::Bool::and(ctx, &guards.iter().collect::<Vec<_>>());

//     let refs: Vec<&dyn Ast> = z3_vars.iter().map(|(_, v)| v as &dyn Ast).collect();
//     let final_body = if guards.is_empty() {
//         body_bool
//     } else {
//         ast::Bool::implies(&guard_conj, &body_bool)
//     };
//     ast::forall_const(ctx, &refs, &[], &final_body).into()
// }

// fn iter_exists_to_z3<'a>(
//     ctx: &'a Context,
//     solver: &Solver,
//     exp_registry: &ExpRegistry,
//     vars: &BTreeMap<VarId, ExpId>,
//     body: ExpId,
//     ir: &IRContext,
//     fn_map: &HashMap<UsrFunId, FuncDecl>,
//     ty_map: &HashMap<UsrSortId, DatatypeSort>,
//     sort_map: &HashMap<SmtSortName, z3::Sort>,
//     bound_env: &Vec<(Symbol, ast::Dynamic)>,
//     cloak_manager: &mut CloakManager<'a>,
//     map_length_manager: &mut MapLengthManager,
//     axiomatic_params: &mut HashMap<String, ast::Dynamic>,
// ) -> ast::Dynamic {
//     let mut local_axiom_params = axiomatic_params.clone();

//     let mut z3_vars = Vec::new();
//     let mut z3_colls = Vec::new();
//     for (vid, coll_eid) in vars {
//         let vinfo = exp_registry.vars.get(vid).unwrap();
//         let z3_v = ast::Dynamic::fresh_const(
//             ctx,
//             vinfo.name.as_ref(),
//             &sort_to_z3(&vinfo.sort, ctx, ir, None, ty_map),
//         );
//         z3_vars.push(((vid, vinfo), z3_v.clone()));

//         let coll_ast = process_expression(
//             ctx,
//             solver,
//             exp_registry,
//             *coll_eid,
//             ir,
//             fn_map,
//             ty_map,
//             sort_map,
//             bound_env,
//             cloak_manager,
//             map_length_manager,
//             axiomatic_params,
//         );
//         z3_colls.push(coll_ast);
//     }

//     for ((_vid, vinfo), z3_v) in &z3_vars {
//         local_axiom_params.insert(vinfo.name.to_string(), z3_v.clone());
//     }

//     let body_bool = process_expression(
//         ctx,
//         solver,
//         exp_registry,
//         body,
//         ir,
//         fn_map,
//         ty_map,
//         sort_map,
//         bound_env,
//         cloak_manager,
//         map_length_manager,
//         &mut local_axiom_params,
//     )
//     .as_bool()
//     .expect("IterExists body must be boolean");

//     let mut guards = Vec::new();
//     for (((vid, _vinfo), z3_v), coll_ast) in z3_vars.iter().zip(z3_colls.iter()) {
//         let coll_sort = derive_type(exp_registry, ir, vars.get_key_value(vid).unwrap().1);
//         let guard = match coll_sort {
//             Sort::Set(_) => coll_ast.as_set().unwrap().member(z3_v),
//             Sort::Map(_, v) => {
//                 let sel = coll_ast.as_array().unwrap().select(z3_v);
//                 sel._eq(&not_present(ctx, &sort_to_z3(&v, ctx, ir, None, ty_map)))
//                     .not()
//             }
//             Sort::Seq(_) => {
//                 let idx = z3_v.as_int().expect("Expected integer");
//                 let seq_ast = coll_ast.as_seq().unwrap();
//                 let len_ast = seq_ast.length();
//                 ast::Bool::and(
//                     ctx,
//                     &[&idx.ge(&ast::Int::from_i64(ctx, 0)), &idx.lt(&len_ast)],
//                 )
//             }
//             _ => panic!("iterator must be over Set, Seq, or Map"),
//         };
//         guards.push(guard);
//     }
//     let guard_conj = ast::Bool::and(ctx, &guards.iter().collect::<Vec<_>>());
//     let refs: Vec<&dyn Ast> = z3_vars.iter().map(|(_, v)| v as &dyn Ast).collect();

//     let final_body = if guards.is_empty() {
//         body_bool
//     } else {
//         ast::Bool::implies(&guard_conj, &body_bool)
//     };
//     ast::exists_const(ctx, &refs, &[], &final_body).into()
// }

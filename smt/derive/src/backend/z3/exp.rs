//! This module contains the conversion of expressions to SMT-LIB format.
//! An expression is the body of a function or an axiom.

use crate::backend::z3::intrinsics::intrinsics_to_smt;
use crate::ir::exp::{ExpRegistry, Expression, MatchAtom, VariantCtor};
use crate::ir::index::ExpId;
use crate::IRContext;

/// Converts an expression into the corresponding SMT-LIB as a `String`.
/// This function takes an expression registry, an expression ID, and an IR context.
/// It recursively converts the expression and its components into SMT-LIB format.
pub fn expr_to_smt(exp_registry: &ExpRegistry, id: &ExpId, ir: &IRContext) -> String {
    // destruct ExpRegistry
    let ExpRegistry { vars, exps } = exp_registry;

    let exp = exps.get(id).expect("expression not found in registry");

    match exp {
        Expression::Var(var_id) => {
            let var_name = vars.get(var_id).expect("variable not found in registry");
            var_name.name.to_string()
        }
        Expression::Pack { sort, elems } => {
            let (sort_name, ty_args) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(_) = sort_name {
                panic!("tuples are unnamed types");
            }
            let tuple_name = format!(
                "Tuple_{}",
                ty_args // for tuples it is the elements list
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );
            let constructor_name = format!("mk-{}", tuple_name);
            let elems = elems
                .iter()
                .map(|e| format!("{}", expr_to_smt(exp_registry, e, ir)))
                .collect::<Vec<_>>();
            format!("({} {})", constructor_name, elems.join(" "))
        }
        Expression::Tuple { sort, slots } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = slots
                    .iter()
                    .map(|e| format!("{}", expr_to_smt(exp_registry, e, ir)))
                    .collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("tuple has no name")
            }
        }
        Expression::Record { sort, fields } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = fields
                    .iter()
                    .map(|(_, e)| format!("{}", expr_to_smt(exp_registry, e, ir)))
                    .collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("record has no name")
            }
        }
        Expression::Enum {
            sort,
            branch,
            variant,
        } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(_) = ty {
                let constructor_name = format!("{}", branch);
                match variant {
                    VariantCtor::Unit => format!("({})", constructor_name),
                    VariantCtor::Tuple(t) => {
                        let elems = t
                            .iter()
                            .map(|e| format!("{}", expr_to_smt(exp_registry, e, ir)))
                            .collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                    VariantCtor::Record(r) => {
                        let elems = r
                            .iter()
                            .map(|(_, e)| format!("{}", expr_to_smt(exp_registry, e, ir)))
                            .collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                }
            } else {
                panic!("enum has no name")
            }
        }
        Expression::AccessSlot { base, slot } => {
            let base_smt = expr_to_smt(exp_registry, base, ir);
            let field_name = format!("field{}_", slot + 1);
            format!("({} {})", field_name, base_smt)
        }
        Expression::AccessField { base, field } => {
            let base_smt = expr_to_smt(exp_registry, base, ir);
            format!("({} {})", field, base_smt)
        }
        Expression::Match { cases } => {
            for case in cases {
                // let MatchCase { atoms, body } = case;
                let atoms = &case.atoms;
                let body = case.body;
                for atom in atoms {
                    let MatchAtom {
                        head,
                        sort,
                        branch,
                        variant,
                    } = atom;
                }
            }
            return "s".to_string();
        }
        Expression::Phi { cases, default } => {
            let first = cases.iter().next();
            if first.is_none() {
                panic!("no cases in phi");
            }
            let first_case = first.unwrap();
            // let PhiCase { cond, body } = first_case;
            let cond = first_case.cond;
            let body = first_case.body;
            let cond_smt = expr_to_smt(exp_registry, &cond, ir);
            let body_smt = expr_to_smt(exp_registry, &body, ir);
            let default = expr_to_smt(exp_registry, default, ir);
            format!("ite ({}) ({}) ({})", cond_smt, body_smt, default)
        }
        Expression::Intrinsic(intrinsic) => intrinsics_to_smt(intrinsic, exp_registry, ir),
        Expression::Procedure { callee, args } => {
            let callee_smt = ir.fn_registry.get_name(callee);
            let args_smt = args
                .iter()
                .map(|e| format!("{}", expr_to_smt(exp_registry, e, ir)))
                .collect::<Vec<_>>();
            format!("({} {})", callee_smt, args_smt.join(" "))
        }
        _ => panic!("expression not supported: {:?}", exp),
    }
}

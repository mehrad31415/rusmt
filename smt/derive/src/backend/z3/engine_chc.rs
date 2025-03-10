use crate::backend::codegen::{l, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::common::BackendZ3;
use crate::ir::axiom::Predicate;
use crate::ir::exp::ExpRegistry;
use crate::ir::fun::{FunDef, FunSig};
use crate::ir::index::{ExpId, UsrSortId};
use crate::ir::name::{UsrAxiomName, UsrFunName};
use crate::ir::sort::{DataType, Sort, Variant};
use crate::IRContext;
use crate::ir::exp::Expression;
use crate::ir::exp::VariantCtor;

/// BackendZ3CHC is a backend designed for Z3's CHC (Constrained Horn Clause) engine.
pub struct BackendZ3CHC {}

impl BackendZ3CHC {
    pub fn new() -> Self {
        Self {}
    }
}

impl BackendZ3 for BackendZ3CHC {
    fn name(&self) -> String {
        "z3_chc".to_string()
    }

    /// Generate backend SMT-LIB code for the CHC engine based on the given `IRContext`.
    fn process(&self, ir: &IRContext) -> BackendResult<String> {
        let mut x = ContentBuilder::new();

        l!(x, "; verification of impl-spec pair: {}", ir.desc);
        l!(x, "(set-option :print-success false)");
        l!(x, "(set-option :produce-models true)");
        l!(x, "(set-logic ALL)");
        l!(x); // add new line

        // write the type parameters
        l!(x, "; Define Type Parameters:");
        for sort in &ir.undef_sorts {
            l!(x, "(declare-sort {} 0)", sort);
        }
        l!(x); // add new line

        // write the user-defined types
        l!(x, "; Define user-defined types:");
        for sid in ir.ty_registry.data_types().keys() {
            println!("sort: {}", sid);
            let s = user_defined_types(*sid, ir, false);
            l!(x, "{}", s);
        }
        l!(x);

        // // write the functions
        // for (name, generics_id) in &ir.fn_registry.lookup {
        //     println!("name: {}", name);
        //     for (_, id) in generics_id {
        //         let sig = ir.fn_registry.retrieve_sig(*id);
        //         let def = ir.fn_registry.retrieve_def(*id);
        //         let s = user_defined_functions(name.clone(), sig, def, ir);
        //         l!(x, "{}", s);
        //     }
        // }
        // l!(x);

        // // write the axioms
        // for (name, generics_id) in &ir.axiom_registry.lookup {
        //     for (_, id) in generics_id {
        //         // generics are already registered
        //         let predicate = ir.axiom_registry.retrieve(*id);
        //         let s = user_defined_axioms(name.clone(), predicate, ir);
        //         l!(x, "{}", s);
        //     }
        // }
        // l!(x);

        // exit
        l!(x, "(exit)");
        // done
        Ok(x.build())
    }
}

pub fn user_defined_types(sid: UsrSortId, ir: &IRContext, call: bool) -> String {
    let ret;

    // get the data type
    let (type_name, gen_or_elem, dt) = {
        let dt = ir.ty_registry.retrieve(sid);
        let (type_name, gen_or_elem) = ir.ty_registry.reverse_lookup(sid);
        (type_name, gen_or_elem, dt)
    };

    if type_name.is_none() {
        // then it is a tuple like (a, b, c)
        if let DataType::Tuple(_) = dt {
            // unique name for tuple
            let tuple_name = format!(
                "Tuple_{}",
                gen_or_elem // for tuples it is the elements list type
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );
            let constructor_name = format!("mk-{}", tuple_name);

            // Generate field names: field1_, field2_, ...
            let field_names: Vec<String> = (0..gen_or_elem.len())
                .map(|i| format!("field{}_", i + 1))
                .collect();

            // Combine fields with their respective sorts
            let field_defs: Vec<String> = gen_or_elem
                .iter()
                .zip(field_names.iter())
                .map(|(sort, field_name)| format!("({} {})", field_name, to_smt_sort(sort, ir)))
                .collect();

            // formulate the declaration
            if call {
                // for tuple (Integer, Bool):
                // Tuple_Integer_Bool, basically gives the sort name
                ret = format!("{}", tuple_name);
            } else {
                // for tuple (Integer, Bool):
                // (declare-datatypes () ((Tuple_Integer_Bool (mk-Tuple_Integer_Bool (field1_ Int) (field2_ Bool)))))
                ret = format!(
                    "(declare-datatypes () (({} ({} {}))))",
                    tuple_name,
                    constructor_name,
                    field_defs.join(" ")
                );
            }
            return ret;
        } else {
            panic!("not a tuple: {}", sid);
        }
    }

    // now it has a name
    let type_name = type_name.expect("should have a name");
    let constructor_name = format!("mk-{}", type_name);
    match dt {
        DataType::Tuple(elems) => {
            // Generate field names
            let field_names: Vec<String> = (0..elems.len())
                .map(|i| format!("field{}_", i + 1))
                .collect();

            // Combine field names with their respective sorts (types)
            let field_defs: Vec<String> = elems
                .iter()
                .zip(field_names.iter())
                .map(|(sort, field_name)| format!("({} {})", field_name, to_smt_sort(sort, ir)))
                .collect();
            if call {
                // use the type name: MyStruct
                ret = format!("{}", type_name);
            } else {
                if gen_or_elem.is_empty() {
                    ret = format!(
                        "(declare-datatypes () (({} ({} {}))))",
                        type_name,
                        constructor_name,
                        field_defs.join(" ")
                    );
                } else {
                    ret = format!(
                        "(declare-datatypes (({})) (({} ({} {}))))",
                        gen_or_elem
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                        type_name,
                        constructor_name,
                        field_defs.join(" ")
                    );
                }
            }
            return ret;
        }
        DataType::Record(recs) => {
            let field_defs: Vec<String> = recs
                .iter()
                .map(|(field_name, sort)| format!("({} {})", field_name, to_smt_sort(sort, ir)))
                .collect();

            if call {
                ret = format!("{}", type_name);
            } else {
                if gen_or_elem.is_empty() {
                    ret = format!(
                        "(declare-datatypes () (({} ({} {}))))",
                        type_name,
                        constructor_name,
                        field_defs.join(" ")
                    );
                } else {
                    ret = format!(
                        "(declare-datatypes (({})) (({} ({} {}))))",
                        gen_or_elem
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                        type_name,
                        constructor_name,
                        field_defs.join(" ")
                    );
                }
            }
            return ret;
        }
        DataType::Enum(vars) => {
            let mut variants = Vec::new();
            for (variant_name, variant_df) in vars {
                match variant_df {
                    Variant::Unit => {
                        variants.push(format!("({})", variant_name.clone()));
                    }
                    Variant::Tuple(t) => {
                        if t.is_empty() {
                            panic!("slots in tuple is empty");
                        }

                        let field_names: Vec<String> =
                            (0..t.len()).map(|i| format!("field{}_", i + 1)).collect();

                        // Combine field names with their respective sorts (types)
                        let field_defs: Vec<String> = t
                            .iter()
                            .zip(field_names.iter())
                            .map(|(sort, field_name)| {
                                format!("({} {})", field_name, to_smt_sort(sort, ir))
                            })
                            .collect();

                        variants.push(format!("({} {})", variant_name, field_defs.join(" ")));
                    }
                    Variant::Record(r) => {
                        if r.is_empty() {
                            panic!("slots in record is empty");
                        }

                        let field_defs: Vec<String> = r
                            .iter()
                            .map(|(field_name, sort)| {
                                format!("({} {})", field_name, to_smt_sort(sort, ir))
                            })
                            .collect();

                        variants.push(format!("({} {})", variant_name, field_defs.join(" ")));
                    }
                }
            }

            if call {
                ret = format!("{}", type_name);
            } else {
                if gen_or_elem.is_empty() {
                    ret = format!(
                        "(declare-datatypes () (({} {})))",
                        type_name,
                        variants.join(" ")
                    );
                } else {
                    ret = format!(
                        "(declare-datatypes (({})) (({} {})))",
                        gen_or_elem
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                        type_name,
                        variants.join(" ")
                    );
                }
            }
            return ret;
        }
    }
}

/// Converts a Rust `Sort` into the corresponding SMT-LIB sort as a `String`
pub fn to_smt_sort(s: &Sort, ir: &IRContext) -> String {
    match s {
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Rational => "Real".to_string(),
        Sort::Text => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", to_smt_sort(inner, ir)),
        Sort::Set(inner) => format!("(Set {})", to_smt_sort(inner, ir)),
        Sort::Map(key, value) => {
            format!(
                "(Array {} {})",
                to_smt_sort(key, ir),
                to_smt_sort(value, ir)
            )
        }
        Sort::Error => "(undefined_function)".to_string(), // triggers an undefined function which leads to a crash assuming the function is not defined!
        Sort::User(usr_sort_id) => user_defined_types(*usr_sort_id, ir, true),
        Sort::Uninterpreted(name) => format!("{}", name),
    }
}

pub fn user_defined_functions(
    name: UsrFunName,
    sig: &FunSig,
    def: &FunDef,
    ir: &IRContext,
) -> String {
    let ret;
    let FunSig { params, ret_ty } = sig;

    let return_type = to_smt_sort(ret_ty, ir);

    match def {
        FunDef::Defined(reg, id) => {
            let body_expr = expr_to_smt(reg, id, ir);

            let field_defs: Vec<String> = params
            .iter()
            .map(|(field_name, sort)| format!("({} {})", field_name, to_smt_sort(sort, ir)))
            .collect();

            ret = format!(
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
            .map(|(_, sort)| format!("{}", to_smt_sort(sort, ir)))
            .collect();

            ret = format!(
                "(declare-fun {} ({}) {})",
                name,
                field_defs.join(" "),
                return_type
            );
        }
    }
    return ret;
}

pub fn user_defined_axioms(name: UsrAxiomName, predicate: &Predicate, ir: &IRContext) -> String {
    let ret;
    let Predicate {
        params,
        body_reg,
        body_exp,
    } = predicate;

    let field_defs: Vec<String> = params
        .iter()
        .map(|(field_name, sort)| format!("({} {})", field_name, to_smt_sort(sort, ir)))
        .collect();

    let fields: Vec<String> = params
        .iter()
        .map(|(_, sort)| format!("{}", to_smt_sort(sort, ir)))
        .collect();

    let symbols: Vec<String> = params
        .iter()
        .map(|(field_name, _)| field_name.to_string())
        .collect();

    let body_expr = expr_to_smt(body_reg, body_exp, ir);

    ret = format!("(declare-fun {} ({}) Bool)\n
                   (assert (forall ({}) (=> ({} {}) ({}))))\n
                   ; Universal negation check: If there exists (lhs, rhs) where add_axiom does NOT hold\n
                   (assert (exists ({}) (not ({} {}))))\n
                   (check-sat)\n",
            name, 
            fields.join(" "), 
            field_defs.join(" "),
            name,
            symbols.join(" "),
            body_expr,
            field_defs.join(" "),
            name,
            symbols.join(" "), );
        
    return ret;
}

pub fn expr_to_smt(exp_registry: &ExpRegistry, id: &ExpId, ir: &IRContext) -> String {
    // destruct ExpRegistry
    let ExpRegistry {
        vars,
        exps
    } = exp_registry;

    let exp = exps.get(id).expect("expression not found in registry");

    match exp {
        Expression::Var(var_id) => {
            let var_name = vars.get(var_id).expect("variable not found in registry");
            var_name.name.to_string()
        }
        Expression::Pack { sort, elems} => {
            let (_, ty_args) = ir.ty_registry.reverse_lookup(*sort);
            let tuple_name = format!(
                "Tuple_{}",
                ty_args // for tuples it is the elements list
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );
            let constructor_name = format!("mk-{}", tuple_name);
            let elems = elems.iter().map(|e| expr_to_smt(exp_registry, e, ir)).collect::<Vec<_>>();
            format!("({} {})", constructor_name, elems.join(" "))
        },
        Expression::Tuple { sort, slots} => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = slots.iter().map(|e| expr_to_smt(exp_registry, e, ir)).collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("tuple has no name")
            }
        },
        Expression::Record { sort, fields } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(ty) = ty {
                let constructor_name = format!("mk-{}", ty);
                let elems = fields.iter().map(|(k, v)| format!("({} {})", k, expr_to_smt(exp_registry, v, ir))).collect::<Vec<_>>();
                format!("({} {})", constructor_name, elems.join(" "))
            } else {
                panic!("record has no name")
            }
        },
        Expression::Enum { sort, branch, variant } => {
            let (ty, _) = ir.ty_registry.reverse_lookup(*sort);
            if let Some(_) = ty {
                let constructor_name = format!("{}", branch);
                match variant {
                    VariantCtor::Unit => format!("({})", constructor_name),
                    VariantCtor::Tuple(t) => {
                        let elems = t.iter().map(|e| expr_to_smt(exp_registry, e, ir)).collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                    VariantCtor::Record(r) => {
                        let elems = r.iter().map(|(k, v)| format!("({} {})", k, expr_to_smt(exp_registry, v, ir))).collect::<Vec<_>>();
                        format!("({} {})", constructor_name, elems.join(" "))
                    }
                }
            } else {
                panic!("enum has no name")
            }
        },
        Expression::AccessSlot { base, slot } => {
            let base_smt = expr_to_smt(exp_registry, base, ir);
            let field_name = format!("field{}_", slot + 1);
            format!("({} {})", field_name, base_smt)
        },
        Expression::AccessField { base, field } => {
            let base_smt = expr_to_smt(exp_registry, base, ir);
            format!("({} {})", field, base_smt)
        }, 
        // Expression::Match { cases } => {
        //     if cases.is_empty() {
        //         panic!("match expression must have at least one case");
        //     }
        
        //     let mut iter = cases.iter();
        //     let first_case = iter.next().unwrap();
        //     let MatchCase {
        //         atoms,
        //         body,
        //     } = first_case;
        
        //     let mut result = expr_to_smt(exp_registry, body, ir);
        
        //     for case in iter {
        //         let cond = format!(
        //             "(= {} {})",
        //             case.atoms.branch,
        //             expr_to_smt(exp_registry, &case.body, ir),
        //         );
        //         let body = expr_to_smt(exp_registry, &case.body, ir);
        //         result = format!("(ite {} {} {})", cond, body, result);
        //     }
        
        //     result
        // },        
        _ => format!("{}", id)
    }

}

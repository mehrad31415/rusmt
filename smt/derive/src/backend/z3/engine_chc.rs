use crate::backend::codegen::{l, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::common::BackendZ3;
use crate::backend::z3::session::to_smt_sort_func_sig;
use crate::backend::z3::session::user_defined_types;
use crate::ir::axiom::Predicate;
use crate::ir::ctxt::IRContext;
use crate::ir::fun::FunDef;
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

        l!(
            x,
            "; verification of the following impl-spec pair: {}",
            ir.desc
        );
        l!(x); // add new line

        for sort in &ir.undef_sorts {
            l!(x, "(declare-sort {} 0)", sort);
        }

        for sid in ir.ty_registry.data_types().keys() {
            let s = user_defined_types(*sid, ir);
            l!(x, "{}", s);
            l!(x);
        }

        // write the functions
        for (name, generics_id) in &ir.fn_registry.lookup {
            for (_, id) in generics_id {
                let sig = ir.fn_registry.retrieve_sig(*id);
                let def = ir.fn_registry.retrieve_def(*id);

                match def {
                    FunDef::Defined(reg, id) => {
                        // Extract argument types from (name, sort) pairs
                        // println!("{:?}", sig);
                        let params: Vec<String> = sig
                            .params
                            .iter()
                            .map(|(var, sort)| {
                                format!("({} {})", var, to_smt_sort_func_sig(sort, ir))
                            })
                            .collect();

                        let return_type = to_smt_sort_func_sig(&sig.ret_ty, ir);

                        let body_expr = reg.expr_to_smt(*id);

                        l!(
                            x,
                            "(define-fun {} ({}) {} {})",
                            name,
                            params.join(" "),
                            return_type,
                            body_expr
                        );
                        l!(x);
                    }
                    FunDef::Uninterpreted => {
                        // Extract argument types from (name, sort) pairs
                        let arg_types: Vec<String> = sig
                            .params
                            .iter()
                            .map(|(_, sort)| to_smt_sort_func_sig(sort, ir))
                            .collect();
                        let return_type = to_smt_sort_func_sig(&sig.ret_ty, ir);

                        // Generate SMT function declaration (the generics of the function have already been defined)
                        l!(
                            x,
                            "(declare-fun {} ({}) {})",
                            name,
                            arg_types.join(" "),
                            return_type
                        );
                        l!(x);
                    }
                }
            }
        }

        l!(x);

        // println!("axioms: {:?}", ir.axiom_registry.lookup);
        // write the axioms
        for (name, generics_id) in &ir.axiom_registry.lookup {
            for (_, id) in generics_id {
                // generics are already registered
                let predicate = ir.axiom_registry.retrieve(*id);

                let Predicate {
                    params,
                    body_reg,
                    body_exp,
                } = predicate;

                let param_list: Vec<String> = params
                    .iter()
                    .map(|(_, sort)| {
                        format!("{}", to_smt_sort_func_sig(sort, ir))
                    })
                    .collect();

                let param_names: Vec<String> = params
                    .iter()
                    .map(|(symbol, _)| symbol.to_string())
                    .collect();

                let body_expr = body_reg.expr_to_smt(*body_exp);

                // Declare the axiom function
                l!(
                    x,
                    "(declare-fun {} ({} Bool))",
                    name,
                    param_list.join(" ")
                );
                l!(x);
                l!(
                    x,
                    "(assert (forall ({}) (= ({} {}) {})))",
                    param_list.join(" "),
                    name,
                    param_names.join(" "),
                    body_expr
                );
                l!(x);
            }
        }

        // done
        Ok(x.build())
    }
}

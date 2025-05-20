//! Root module for Z3 smtlib code generation.

use crate::backend::codegen::{l, ApiResult, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::axiom_in_smt;
use crate::backend::z3::common::BackendZ3;
use crate::backend::z3::fun::{fundef_in_smt, group_dependent_funcs};
use crate::backend::z3::sort::sort_to_smt;
use crate::backend::z3::ty::tydef_in_smt;
use crate::ir::index::{UsrFunId, UsrSortId, VarId};
use crate::ir::name::{SmtSortName, UsrFunName};
use crate::ir::sort::Sort;
use crate::IRContext;
use std::collections::{BTreeMap, BTreeSet};

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
        // create a new content builder for writing the SMT-LIB code
        let mut x = ContentBuilder::new();

        // destructure the IRContext
        let IRContext {
            desc,
            undef_sorts,
            ty_registry,
            fn_registry,
            axiom_registry,
        } = ir;

        // The set-option command is used to configure Z3.
        l!(x, "; verification of impl-spec pair: {}", desc);
        // disable success messages
        l!(x, "(set-option :print-success false)");
        // enable model generation in case of satisfiability for debugging
        l!(x, "(set-option :produce-models true)");
        // set the string solver to be the z3str3 solver for string constraints
        l!(x, "(set-option :smt.string_solver z3str3)");
        // set the string solver to be the seq solver (default) for sequence constraints
        l!(x, "(set-option :smt.string_solver seq)");
        // mbqi (model-based quantifier instantiation) is a technique used by Z3 to instantiate quantifiers
        // in a model-based way. It is used to improve the performance of the solver.
        // l!(x, "(set-option :mbqi true)");
        // allow all available theories: l!(x, "(set-logic ALL)"); Z3 will automatically detect the logic
        l!(x); // add new line

        // write the type parameters
        if !&undef_sorts.is_empty() {
            l!(x, "; Define Type Parameters of Function Signatures:");
            for sort in undef_sorts {
                l!(x, "(declare-sort {} 0)", sort);
            }
            l!(x); // add new line
        }

        // write the user-defined types
        if !ty_registry.data_types().is_empty() {
            l!(x, "; Define user-defined types:");
            for sid in ty_registry.data_types().keys() {
                l!(x, "{}", tydef_in_smt(*sid, ir));
            }
            l!(x); // add new line
        }

        // this set is used for mutually recursive functions
        let mut func_names: Vec<Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>> =
            Vec::new();

        for (name, generics_id) in &fn_registry.lookup {
            group_dependent_funcs(name, generics_id, ir, &mut func_names);
        }

        let mut dependencies = Vec::new();
        let mut mapping_vars: BTreeMap<VarId, String> = BTreeMap::new();

        let mut funcs_smt: Vec<String> = Vec::new();
        // get the function definitions
        for funcs in func_names.iter() {
            funcs_smt.push(fundef_in_smt(
                ir,
                &funcs,
                &mut dependencies,
                &mut mapping_vars,
            ));
        }

        let mut axioms_smt: Vec<String> = Vec::new();
        // get the axioms
        // we don't need to write the name. The axiom is registered as forall<axiom params> {axiom body}
        for (name, generics_id) in &axiom_registry.lookup {
            // generics are already registered in undef_sorts
            for (_generics, id) in generics_id {
                let predicate = axiom_registry.retrieve(*id);
                axioms_smt.push(axiom_in_smt(
                    name,
                    predicate,
                    ir,
                    &mut dependencies,
                    &mut mapping_vars,
                ));
            }
        }

        if !dependencies.is_empty() {
            l!(x, "; Define dependencies:");
            // add the dependencies
            for dep in dependencies.iter() {
                l!(x, "{}", dep.as_str());
            }
            l!(x); // add new line
        }

        if !&fn_registry.lookup.is_empty() {
            l!(x, "; Define user functions:");
            // add the function definitions
            for func in funcs_smt.iter() {
                l!(x, "{}", func.as_str());
            }
            l!(x); // add new line
        }

        if !&axiom_registry.lookup.is_empty() {
            l!(x, "; Define axioms:");
            // add the axioms
            for axiom in axioms_smt.iter() {
                l!(x, "{}", axiom.as_str());
            }
            l!(x); // add new line
        }

        // prove the `validity` of the fact that the operational (smt_impl) and denotational semantics (smt_spec) are equivalent
        // To prove: negate the equivalence and check for unsatisfiability
        l!(
            x,
            "; Prove the equivalence of the operational and denotational semantics:"
        );

        // get the impl and spec names
        let impl_name = &desc.fn_impl.clone();
        let spec_name = &desc.fn_spec.clone();

        // get the id of the spec and the impl
        let impl_id = ir
            .fn_registry
            .lookup
            .get(&UsrFunName::from(impl_name))
            .expect("Function not found");
        if impl_id.len() != 1 {
            panic!("Multiple implementations found for the same function");
        }
        let impl_id = impl_id.first_key_value().expect("error").1;
        let spec_id = ir
            .fn_registry
            .lookup
            .get(&UsrFunName::from(spec_name))
            .expect("Function not found");
        if spec_id.len() != 1 {
            panic!("Multiple specifications found for the same function");
        }
        let spec_id = spec_id.first_key_value().expect("error").1;
        // get the signature of the impl and spec
        let impl_sig = fn_registry.retrieve_sig(*impl_id);
        let spec_sig = fn_registry.retrieve_sig(*spec_id);

        // get the var + sort pair
        let impl_input = impl_sig
            .params
            .iter()
            .map(|(sym, sort)| format!("{}_{} {}", impl_name, sym, sort_to_smt(sort, ir, None)))
            .collect::<BTreeSet<_>>();
        let spec_input = spec_sig
            .params
            .iter()
            .map(|(sym, sort)| format!("{} {}", sym, sort_to_smt(sort, ir, None)))
            .collect::<BTreeSet<_>>();

        // union of the impl and spec inputs
        let all_sym_sort = impl_input
            .union(&spec_input)
            .clone()
            .collect::<BTreeSet<_>>();

        // (declare-const lhs Point) (declare-const rhs Point)
        // declare the variables for the params of the impl and spec
        if !all_sym_sort.is_empty() {
            l!(x, "; Define parameters of function signatures:");
            for s in all_sym_sort.clone() {
                l!(x, "(declare-const {})", s)
            }
            l!(x); // add new line
        }

        // get the symbols for the spec and impl
        let impl_syms = impl_sig.get_params();
        let spec_syms = spec_sig.get_params();
        // sanity check
        if impl_syms.len() != spec_syms.len() {
            panic!("specification and implementation must have the same number of params")
        }

        let constraint = if impl_syms.is_empty() {
            "".to_string()
        } else if impl_syms.len() == 1 {
            let is = impl_syms.first().expect("impl must have one element");
            let ss = spec_syms.first().expect("spec must have one element");
            format!("(= {} {})", is, ss)
        } else {
            format!(
                "(and {})",
                impl_syms
                    .iter()
                    .zip(spec_syms.iter())
                    .map(|(is, ss)| format!("(= {} {})", is, ss))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        // ; exists (lhs Point) (rhs Point) (= (add_spec lhs rhs) (add lhs rhs))
        l!(
            x,
            "; (exists ({}) (=> {} (= ({} {}) ({} {}))))",
            all_sym_sort
                .iter()
                .map(|s| format!("({})", s.as_str()))
                .collect::<Vec<_>>()
                .join(" "),
            constraint,
            impl_name,
            impl_syms
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            spec_name,
            spec_syms
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );

        // the variables of the impl and spec need to be the same even if they have different names
        for (i, s) in impl_syms.iter().zip(&spec_syms) {
            l!(x, "(assert (= {} {}))", i, s)
        }

        if impl_syms.is_empty() {
            // (assert (= (add_spec lhs rhs) (add lhs rhs)))
            l!(x, "(assert (= {} {}))", impl_name, spec_name);
            l!(x); // add new line
        } else {
            // (assert (!= (add_spec lhs rhs) (add lhs rhs)))
            l!(
                x,
                "(assert (= ({} {}) ({} {})))",
                impl_name,
                impl_syms
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                spec_name,
                spec_syms
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            l!(x); // add new line
        }

        // check for satisfiability - if it is satisfiable, then the spec soundly specifies the impl
        l!(x, "(check-sat)");
        // exit
        l!(x, "(exit)");
        // done
        Ok(x.build())
    }

    fn call_z3_api(&self, ir: &IRContext) -> BackendResult<ApiResult<'_>> {
        unimplemented!()
    }
}

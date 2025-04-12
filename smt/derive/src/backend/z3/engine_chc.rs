//! Root module for Z3 smtlib code generation.

use crate::backend::codegen::{l, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::axiom_in_smt;
use crate::backend::z3::common::BackendZ3;
use crate::backend::z3::fun::{fundef_in_smt, group_dependent_funcs};
use crate::backend::z3::sort::sort_to_smt;
use crate::backend::z3::ty::tydef_in_smt;
use crate::ir::index::UsrFunId;
use crate::ir::name::UsrFunName;
use crate::ir::sort::Sort;
use crate::parser::name::UsrFuncName;
use crate::IRContext;
use proc_macro2::{Ident, Span};
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

        // before anything else, declare the parameters of the function signatures of impl and spec
        // this is because assertions that use the parameters of the impl and spec need to be declared outside of the function
        let (impl_name, spec_name) = desc.split_once(" ~> ").expect("Invalid description");
        let impl_name = Ident::new(impl_name, Span::call_site());
        let spec_name = Ident::new(spec_name, Span::call_site());

        // get the id of the spec and the impl
        let impl_id = ir
            .fn_registry
            .lookup
            .get(&UsrFunName::from(
                UsrFuncName::try_from(&impl_name).expect("impl name invalid"),
            ))
            .expect("Function not found")
            .first_key_value()
            .expect("Function not found")
            .1;
        let spec_id = ir
            .fn_registry
            .lookup
            .get(&UsrFunName::from(
                UsrFuncName::try_from(&spec_name).expect("spec name invalid"),
            ))
            .expect("Function not found")
            .first_key_value()
            .expect("Function not found")
            .1;

        // get the signature of the impl and spec
        let impl_sig = fn_registry.retrieve_sig(*impl_id);
        let spec_sig = fn_registry.retrieve_sig(*spec_id);

        // get the symbols for the spec and impl
        let impl_syms = impl_sig
            .params
            .iter()
            .map(|(sym, _)| sym.clone())
            .collect::<BTreeSet<_>>();
        let spec_syms = spec_sig
            .params
            .iter()
            .map(|(sym, _)| sym.clone())
            .collect::<BTreeSet<_>>();

        // sanity check
        if impl_syms.len() != spec_syms.len() {
            panic!("specification and implementation must have the same number of params")
        }

        // get the var + sort pair
        let set1 = impl_sig
            .params
            .iter()
            .map(|(sym, sort)| format!("{} {}", sym, sort_to_smt(sort, ir)))
            .collect::<BTreeSet<_>>();
        let set2 = spec_sig
            .params
            .iter()
            .map(|(sym, sort)| format!("{} {}", sym, sort_to_smt(sort, ir)))
            .collect::<BTreeSet<_>>();
        let all_sym_sort = set1.union(&set2).clone().collect::<BTreeSet<_>>();

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

        // (declare-const lhs Point) (declare-const rhs Point)
        // declare the variables for the params of the impl and spec
        for s in all_sym_sort.clone() {
            l!(x, "(declare-const {})", s)
        }

        // this set is used for mutually recursive functions
        let mut func_names: Vec<Vec<(UsrFunName, Option<BTreeMap<Vec<Sort>, UsrFunId>>)>> =
            Vec::new();

        for (name, generics_id) in &fn_registry.lookup {
            group_dependent_funcs(name, generics_id, ir, &mut func_names);
        }

        // write the functions
        if !&fn_registry.lookup.is_empty() {
            l!(x, "; Define user functions:");
            for funcs in func_names.iter() {
                l!(x, "{}", fundef_in_smt(ir, &funcs));
                l!(x); // add new line
            }
        }

        // write the axioms
        if !&axiom_registry.lookup.is_empty() {
            l!(x, "; Define axioms:");
            // we don't need to write the name. The axiom is registered as forall<axiom params> {axiom body}
            for (_name, generics_id) in &axiom_registry.lookup {
                // generics are already registered in undef_sorts
                for (_generics, id) in generics_id {
                    let predicate = axiom_registry.retrieve(*id);
                    l!(x, "{}", axiom_in_smt(predicate, ir));
                }
            }
            l!(x); // add new line
        }

        // prove the `validity` of the fact that the operational (smt_impl) and denotational semantics (smt_spec) are equivalent
        // To prove: negate the equivalence and check for unsatisfiability
        l!(
            x,
            "; Prove the equivalence of the operational and denotational semantics:"
        );

        // ; forall (lhs Point) (rhs Point) (= (add_spec lhs rhs) (add lhs rhs))
        l!(
            x,
            "; (assert (forall ({}) (=> {} (= ({} {}) ({} {})))))",
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
            // (assert (!= (add_spec lhs rhs) (add lhs rhs)))
            l!(x, "(assert (not (= {} {})))", impl_name, spec_name);
        } else {
            // (assert (!= (add_spec lhs rhs) (add lhs rhs)))
            l!(
                x,
                "(assert (not (= ({} {}) ({} {}))))",
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
        }

        l!(x); // add new line

        // check for satisfiability - if it is unsatisfiable, then the impl and spec are equivalent (valid)
        l!(x, "(check-sat)");
        // exit
        l!(x, "(exit)");
        // done
        Ok(x.build())
    }
}

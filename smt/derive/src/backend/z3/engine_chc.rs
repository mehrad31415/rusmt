use std::collections::BTreeSet;

use crate::backend::codegen::{l, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::axiom_in_smt;
use crate::backend::z3::common::BackendZ3;
use crate::backend::z3::fun::fundef_in_smt;
use crate::backend::z3::sort::sort_to_smt;
use crate::backend::z3::ty::tydef_in_smt;
use crate::ir::name::UsrFunName;
use crate::parser::name::UsrFuncName;
use crate::IRContext;
use proc_macro2::Ident;
use proc_macro2::Span;

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

        // The set-option command is used to configure Z3.
        l!(x, "; verification of impl-spec pair: {}", ir.desc);
        l!(
            x,
            "(set-option :print-success false) ; disable success messages"
        );
        l!(
            x,
            "(set-option :produce-models true) ; enable model generation"
        );
        l!(
            x,
            "(set-option :smt.string_solver z3str3) ; set the string solver to be the z3str3 solver"
        );
        l!(
            x,
            "(set-option :smt.string_solver seq)    ; set the string solver to be the seq solver (default)"
        );
        l!(x, "(set-logic ALL)");
        l!(x); // add new line

        // write the type parameters
        l!(x, "; Define Type Parameters of Function Signatures:");
        for sort in &ir.undef_sorts {
            l!(x, "(declare-sort {} 0)", sort);
        }
        l!(x); // add new line

        // write the user-defined types
        l!(x, "; Define user-defined types:");
        for sid in ir.ty_registry.data_types().keys() {
            l!(x, "{}", tydef_in_smt(*sid, ir));
        }
        l!(x); // add new line

        // write the functions
        l!(x, "; Define user functions:");
        for (name, generics_id) in &ir.fn_registry.lookup {
            // generics are already registered in undef_sorts
            for (_generics, id) in generics_id {
                let sig = ir.fn_registry.retrieve_sig(*id);
                let def = ir.fn_registry.retrieve_def(*id);
                l!(x, "{}", fundef_in_smt(name.clone(), sig, def, ir));
            }
        }
        l!(x); // add new line

        // write the axioms
        l!(x, "; Define axioms:");
        // we don't need to write the name. The axiom is registered as forall<axiom params> {axiom body}
        for (_name, generics_id) in &ir.axiom_registry.lookup {
            // generics are already registered in undef_sorts
            for (_generics, id) in generics_id {
                let predicate = ir.axiom_registry.retrieve(*id);
                l!(x, "{}", axiom_in_smt(predicate, ir));
            }
        }
        l!(x); // add new line

        // prove the `validity` of the fact that the operational (smt_impl) and denotational semantics (smt_spec) are equivalent
        // To prove: negate the equivalence and check for unsatisfiability
        l!(
            x,
            "; Prove the equivalence of the operational and denotational semantics:"
        );
        let impl_name = ir.desc.split_once(" ~> ").expect("Invalid description").0;
        let impl_name = Ident::new(impl_name, Span::call_site());
        let spec_name = ir.desc.split_once(" ~> ").expect("Invalid description").1;
        let spec_name = Ident::new(spec_name, Span::call_site());

        // get the signature of the impl
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

        let impl_sig = ir.fn_registry.retrieve_sig(*impl_id);
        let spec_sig = ir.fn_registry.retrieve_sig(*spec_id);

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

        // forall (lhs Point) (rhs Point) (= (add_spec lhs rhs) (add lhs rhs))
        l!(
            x,
            "; (assert (forall ({}) (= ({} {}) ({} {}))))",
            all_sym_sort
                .iter()
                .map(|s| format!("({})", s.as_str()))
                .collect::<Vec<_>>()
                .join(" "),
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

        // (declare-var lhs Point) (declare-var rhs Point)
        for s in all_sym_sort {
            l!(x, "(declare-const {})", s)
        }

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
        l!(x); // add new line

        // check for satisfiability
        l!(x, "(check-sat)");
        // exit
        l!(x, "(exit)");
        // done
        Ok(x.build())
    }
}

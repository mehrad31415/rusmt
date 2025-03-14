use crate::backend::codegen::{l, ContentBuilder};
use crate::backend::error::BackendResult;
use crate::backend::z3::axiom::axiom_in_smt;
use crate::backend::z3::common::BackendZ3;
use crate::backend::z3::fun::fundef_in_smt;
use crate::backend::z3::ty::tydef_in_smt;
use crate::IRContext;

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

        // first parse the description to get the name of the functions:

        // exit
        l!(x, "(exit)");
        // done
        Ok(x.build())
    }
}

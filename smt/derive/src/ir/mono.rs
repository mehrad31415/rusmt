use crate::parser::generics::{Generics, GenericsInstPartial, Monomorphization, PartialInst};
use crate::parser::infer::{TIError, TypeRef, TypeUnifier};
use std::collections::{BTreeSet, VecDeque};

/// Unify two (partial) instantiations and see if new instantiations appear.
fn self_interference(
    generics: &Generics, // the generics comes from the generics of an axiom (which is a function) in the definition of the axiom.
    lhs: &Monomorphization,
    rhs: &Monomorphization,
) -> Option<Monomorphization> {
    // unify the arguments in an asymmetric setting
    let mut unifier = TypeUnifier::new();

    let base_lhs = GenericsInstPartial::new_with_mono(generics, lhs).complete(&mut unifier);
    let base_rhs = GenericsInstPartial::new_with_mono(generics, rhs).complete(&mut unifier);
    let inst_lhs = base_lhs.vec();
    let inst_rhs = base_rhs.vec();

    let mut unifies = true;
    for (lhs, rhs) in inst_lhs.iter().zip(inst_rhs.iter()) {
        match unifier.unify(lhs, rhs) {
            Ok(None) => {
                unifies = false;
                break;
            }
            Ok(_) => (),
            Err(TIError::CyclicUnification) => {
                panic!("type unification error: cyclic type unification")
            }
        }
    }
    if !unifies {
        return None;
    }

    // collect the unified results from both sides
    let ty_to_inst = |ty| {
        let refreshed = unifier.refresh_type(&ty); // Refreshes a type by replacing any type variables with their inferred types.
        match refreshed.reverse() {
            //reverse Converts the `TypeRef` back into a `TypeTag`, if possible.
            None => {
                let var = match refreshed {
                    TypeRef::Var(v) => v,
                    _ => panic!("type parameter must be either assigned or variadic"),
                };
                let tp_name = match base_lhs.reverse(&var).or_else(|| base_rhs.reverse(&var)) {
                    None => panic!("unable to find the origin of type var {var}"),
                    Some((n, _)) => n.clone(),
                };
                PartialInst::Unassigned(tp_name)
            }
            Some(tag) => PartialInst::Assigned(tag),
        }
    };

    let refreshed_lhs: Vec<_> = inst_lhs.into_iter().map(ty_to_inst).collect();
    let refreshed_rhs: Vec<_> = inst_rhs.into_iter().map(ty_to_inst).collect();

    // sanity check before returning the new instance
    if refreshed_lhs != refreshed_rhs {
        panic!("monomorphization of two partial instantiations yields different results");
    }
    Some(Monomorphization {
        args: refreshed_lhs,
    })
}

/// Probe for additional instantiations to add via self‐interference.
///
/// This function iterates over each already existing monomorphization and attempts to unify it
/// with the provided `addition`. If unification yields a new monomorphization that is not yet
/// in `existing` or already queued in `extended`, it is appended to the queue.
fn probe_instantiations(
    generics: &Generics,
    existing: &BTreeSet<Monomorphization>,
    addition: &Monomorphization,
    extended: &mut VecDeque<Monomorphization>,
) {
    for inst in existing {
        match self_interference(generics, addition, inst) {
            None => continue,
            Some(mono) => {
                if !existing.contains(&mono) && !extended.contains(&mono) {
                    extended.push_back(mono);
                }
            }
        }
    }
}

/// Add a new (partial) instantiation to the set of existing instantiations.
///
/// This function will return a list of new instantiations that were added to the set of existing
pub fn add_instantiation(
    generics: &Generics, // the generics comes from the generics of an axiom (which is a function) in the definition of the axiom.
    existing: &mut BTreeSet<Monomorphization>,
    addition: Monomorphization,
) -> Vec<Monomorphization> {
    // nothing to add if this mono is already processed
    // if the existing set already contains the new monomorphization, return an empty vector as there is nothing to add. and the existing set is not modified.
    if existing.contains(&addition) {
        return vec![];
    }

    let mut incremental = vec![];

    // create extended queue and add the new mono
    let mut extended = VecDeque::new();
    extended.push_back(addition); // the extended only contains the new monomorphization at the beginning.

    // loop until nothing left in the queue
    while !extended.is_empty() {
        let inst = extended.pop_front().unwrap(); // pop the first element from the queue.
        probe_instantiations(generics, existing, &inst, &mut extended);
        existing.insert(inst.clone()); // insert the monomorphization into the set of existing instantiations.
        incremental.push(inst);
    }
    incremental
}

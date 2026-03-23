//! This module is only used in the Expr module and provides utilities for analyzing match expressions.

use crate::parser::expr::{CtxtForExpr, Expr, MatchCombo, MatchVariant, Unpack};
use crate::parser::generics::GenericsInstFull;
use crate::parser::infer::{TypeRef, TypeUnifier, ti_unify};
use crate::parser::name::{UsrTypeName, VarName};
use crate::parser::path::{ADTBranch, ADTPath};
use crate::parser::ty::EnumVariant;
use crate::{bail_if_exists, bail_on};
use itertools::Itertools;
use std::collections::BTreeMap;
use syn::{ExprMatch, ExprPath, FieldPat, Member, Pat, PatStruct, PatTupleStruct, Result};

/// An atom for a specific variable in the match head
/// For example in match (x, y) { (1, 2) => ... } one MatchAtom is for x and the other is for y
/// MatchAtom::Default is for the wildcard pattern for example in match (x, y) { (_, 2) => ... } the first atom is MatchAtom::Default
pub enum MatchAtom {
    Default,
    Binding(ADTBranch, Unpack),
}

/// A full match arm for all variables in the match head
struct MatchArm {
    atoms: Vec<MatchAtom>, // the atoms for each variable in the match head
    body: Expr,            // the body of the match arm (the expression to be executed)
}

/// An analyzer for match expression
pub struct MatchAnalyzer;

impl MatchAnalyzer {
    /// Analyze a pattern for: match arm -> head -> case -> binding
    fn analyze_pat_match_binding(pat: &Pat) -> Result<Option<VarName>> {
        let binding = match pat {
            Pat::Wild(_) => None,
            // we cannot have a pattern liek Some(Some(1))...
            _ => Some(pat.try_into()?), // it will give an error if the pattern has a subpattern, is a ref, or is mutable. Otherwise, the ident is extracted. If the identifier is not a reserved keyword or underscore, the varname is returned.
        };
        Ok(binding)
    }

    /// Analyze a pattern
    /// For example, in match (some, thing) { (1, 2) => ... } the case for the first arm is (1, 2)
    /// for each arm: each of the (some, 1) and (thing, 2) are made in the expr.rs module and passed to the analyze_pat_match_head one by one
    /// in (some, 1), the `1` pattern is analyzed in this function
    /// The pattern can only be an enum, tuple struct, or record struct
    fn analyze_pat_match_case<T: CtxtForExpr>(
        ctxt: &T,
        unifier: &mut TypeUnifier,
        pat: &Pat,
    ) -> Result<(
        ADTBranch,
        GenericsInstFull,
        Unpack,
        BTreeMap<VarName, TypeRef>,
    )> {
        // create the bindings for the pattern
        let mut bindings = BTreeMap::new();

        let (branch, inst, unpack) = match pat {
            // matching against enum variants that are unit structs like MyEnum::MyVariant
            Pat::Path(pat_path) => {
                let ExprPath {
                    attrs: _,
                    qself,
                    path,
                } = pat_path;
                // the qself should not be there (for example this is invalid: <T>::A)
                bail_if_exists!(qself.as_ref().map(|q| &q.ty));

                // so the path should be an enum variant and can only have two segments.
                let adt = ADTPath::from_path(ctxt, path)?;
                // branch contains the enum type name and variant name
                // inst contains the type parameters matched with the arguments. Through the complete, the arguments are converted from Option(TypeTag) to TypeRef where None is converted to TypeRef::TypeVar and the rest is straightforward. None for the type parameters means that there are no arguments or the argument is _ (wildcard).
                let (branch, inst) = adt.complete(unifier); // unit variant enum::<i32>::A
                let variant = match ctxt.get_adt_variant_details(&branch) {
                    None => bail_on!(path, "not a valid enum branch"), // it will never happen as this is already checked in the ADTPath::from_path
                    Some(def) => def,
                };

                // unnecessary check
                match variant {
                    // the variant can only be a unit variant (if it was a record, it is Pat::Struct & for tuple struct, it is Pat::TupleStruct)
                    // so it would not be a Pat::Path if not a unit variant
                    EnumVariant::Unit => (),
                    _ => bail_on!(pat, "unexpected pattern"), // thus this pattern will never be reached
                }
                // branch contains the enum type name and variant name
                // inst contains the type parameters matched with the arguments
                (branch, inst, Unpack::Unit)
            }
            // A tuple struct or tuple variant pattern: `Variant(x, y, .., z)`
            Pat::TupleStruct(pat_tuple) => {
                let PatTupleStruct {
                    attrs: _,
                    qself,
                    path,
                    paren_token: _,
                    elems,
                } = pat_tuple;
                // the qself should not be there (for example this is invalid: <T>::A)
                bail_if_exists!(qself.as_ref().map(|q| &q.ty));

                // it should definitely be an enum variant pattern (so matching can only happen against tuple variants). If the pattern was a struct like struct MyStruct(i32); it will give an error on this line
                let adt = ADTPath::from_path(ctxt, path)?;
                let (branch, inst) = adt.complete(unifier);
                let variant = match ctxt.get_adt_variant_details(&branch) {
                    None => bail_on!(path, "not a valid enum branch"), // it will never happen as this is already checked in the ADTPath::from_path
                    Some(def) => def,
                };

                match variant {
                    // in the definition, it should be a tuple variant
                    EnumVariant::Tuple(def_tuple) => {
                        let slots = &def_tuple.slots;
                        // if the number of the arguments in the calling place of the tuple struct and the number of the arguments in the definition of the tuple struct do not match, then give an error
                        if elems.len() != slots.len() {
                            bail_on!(elems, "number of slots mismatch");
                        }

                        let mut unpack = BTreeMap::new();
                        // in the match ... { MyEnum::MyVariant(a,b)=> ... } a and b are the elems.
                        // the slots will be the type of a and b in the definition of the tuple variant
                        // for each of the a and b, we will check if it is a binding or not
                        for (i, (elem, slot)) in elems.iter().zip(slots.iter()).enumerate() {
                            match Self::analyze_pat_match_binding(elem)? {
                                None => (), // for wildcard, no binding is created
                                Some(var) => {
                                    // conver the typetag to typeref. For all cases it is straightforward.
                                    // the `inst` is a list of type parameters in the definition of the type to the arguments in the calling place
                                    // for type parameters (slot is a type parameter), inst.instantiate(slot) finds the corresponding argument
                                    let ty_substitute = match inst.instantiate(slot) {
                                        None => bail_on!(elem, "no such type parameter"), // this will never happen as the inst is formed using the type parameters of the definition and slot is taken from the definition as well
                                        Some(instantiated) => instantiated,
                                    };
                                    match bindings.insert(var.clone(), ty_substitute) {
                                        None => (),
                                        Some(_) => {
                                            bail_on!(elem, "duplicated name");
                                        }
                                    }
                                    unpack.insert(i, var);
                                }
                            }
                        }
                        (branch, inst, Unpack::Tuple(unpack))
                    }
                    // the variant can only be a tuple variant as the pattern is used as a tuple variant
                    _ => bail_on!(pat, "unexpected pattern"),
                }
            }
            // A struct or struct variant pattern: `Variant { x, y, .. }`.
            Pat::Struct(pat_struct) => {
                let PatStruct {
                    attrs: _,
                    qself,
                    path,
                    brace_token: _,
                    fields,
                    rest,
                } = pat_struct;
                // the qself should not be there (for example this is invalid: <T>::A)
                bail_if_exists!(qself.as_ref().map(|q| &q.ty));
                // the rest should not be there (for example this is invalid: MyEnum::MyVariant { a, .. } )
                bail_if_exists!(rest);

                // it should definitely be a record variant pattern (so matching can only happen against record variants). If the pattern was a struct like struct MyStruct { a: i32 }; it will give an error on this line
                let adt = ADTPath::from_path(ctxt, path)?;
                let (branch, inst) = adt.complete(unifier);
                let variant = match ctxt.get_adt_variant_details(&branch) {
                    None => bail_on!(path, "not a valid enum branch"), // it will never happen as this is already checked in the ADTPath::from_path
                    Some(def) => def,
                };

                match variant {
                    EnumVariant::Record(def_record) => {
                        let records = &def_record.fields;
                        // if the number of the arguments in the calling place of the record struct and the number of the arguments in the definition of the record struct do not match, then give an error
                        if fields.len() != records.len() {
                            bail_on!(fields, "number of fields mismatch");
                        }

                        let mut unpack = BTreeMap::new();
                        // in the calling place
                        for field in fields {
                            let FieldPat {
                                attrs: _,
                                member,
                                colon_token: _,
                                pat,
                            } = field;
                            // the member should be named for record struct
                            let field_name = match member {
                                Member::Named(name) => name.to_string(),
                                Member::Unnamed(_) => bail_on!(member, "unnamed field"),
                            };

                            // the field used in the call should be in the definition of the record struct
                            let field_type = match records.get(&field_name) {
                                None => bail_on!(member, "no such field"),
                                Some(t) => t,
                            };

                            // instantiate the type reference
                            let ty_substitute = match inst.instantiate(field_type) {
                                None => bail_on!(member, "no such type parameter"), // this will never happen as the inst is formed using the type parameters of the definition and field_type is taken from the definition as well
                                Some(instantiated) => instantiated,
                            };

                            match Self::analyze_pat_match_binding(pat)? {
                                None => (),
                                // only varname is allowed as a binding
                                Some(var) => {
                                    match bindings.insert(var.clone(), ty_substitute) {
                                        None => (),
                                        Some(_) => {
                                            bail_on!(pat, "duplicated name");
                                        }
                                    }
                                    unpack.insert(field_name, var); // only varname is allowed as a binding
                                }
                            }
                        }
                        (branch, inst, Unpack::Record(unpack))
                    }
                    // the variant can only be a record variant
                    _ => bail_on!(pat, "unexpected pattern"),
                }
            }
            // the case pattern can only be an enum, variant record, or variant tuple
            _ => bail_on!(pat, "invalid case pattern"),
        };

        Ok((branch, inst, unpack, bindings))
    }

    /// Analyze a pattern for: match head -> { pat -> body }
    /// For example, in match (some, thing) { (1, 2) => ... } the head is (some, thing) and the case for the first arm is (1, 2)
    /// for each arm in the match body, the pattern is taken (for example (1, 2)). Each of the (some, 1) and (thing, 2) are made and passed to the analyze_pat_match_head one by one
    pub fn analyze_pat_match_head<T: CtxtForExpr>(
        ctxt: &T,
        unifier: &mut TypeUnifier,
        head: &Expr,
        pat: &Pat,
    ) -> Result<(MatchAtom, BTreeMap<VarName, TypeRef>)> {
        // receive the TypeRef of the header
        let ety = head.ty();

        let (atom, bindings) = match pat {
            // A pattern that matches any value: `_`.
            // if the pattern is `_`, then it is a default pattern and no bindings are there
            Pat::Wild(_) => (MatchAtom::Default, BTreeMap::new()),
            // other than a wildcard, the pattern can only be an enum, tuple struct, or record struct
            _ => {
                // branch is the enum type name and variant name of the pattern
                // every pattern must be a variant of an enum type whether it is a unit, tuple, or record variant.
                // inst is the type parameters matched with the arguments of the calling place
                // unpack is either unit for a unit variant, tuple for a tuple variant, or record for a record variant.
                // it maps a slot index or a field name to the corresponding variable name.
                // bindings is the map of the bindings (contains the variable name and the type reference)
                // bindings lists the elements inside the variant tuple or record
                let (branch, inst, unpack, bindings) =
                    Self::analyze_pat_match_case(ctxt, unifier, pat)?;

                // unify the type (the type of the header is the expected type of the pattern)
                // ty_ref is the type of the pattern (TypeRef::User(ty_name of branch, arguments in call of branch))
                let ty_ref = inst.make_ty(branch.ty_name.clone());
                ti_unify!(unifier, ety, &ty_ref, pat);

                // done
                (MatchAtom::Binding(branch, unpack), bindings)
            }
        };

        // done
        Ok((atom, bindings))
    }
}

/// An organizer for the match arms
/// Encapsulates the list of match arms for a match expression
pub struct MatchOrganizer {
    arms: Vec<MatchArm>,
}

impl MatchOrganizer {
    /// Create a new organizer with no arms
    pub fn new() -> Self {
        Self { arms: vec![] }
    }

    /// Add a match arm
    pub fn add_arm(&mut self, atoms: Vec<MatchAtom>, body: Expr) {
        self.arms.push(MatchArm { atoms, body })
    }

    /// Organize the arms into per-combo-by-permutation format
    pub fn into_organized(
        self,             // holds Vec<MatchArm>, each MatchArm holds Vec<MatchAtom> and Expr. The unpacks here are populated in the analyze_pat_match_case function.
        expr: &ExprMatch, // the match expression
        heads: &[(UsrTypeName, BTreeMap<String, Unpack>)], // this holds the type name and all the variants of the type for all the variables in the match head
    ) -> Result<Vec<MatchCombo>> {
        // utility enum to indicate whether a match combo is abstract or concrete
        enum MatchComboStatus {
            None, // no match found yet
            Abstract(Vec<(usize, MatchCombo)>), // at least one position matched by _ wildcard
            Concrete(usize, MatchCombo), // all positions matched by specific enum::variant
        }

        // tracks how many combo are mapped to each arm
        let mut map_arms = BTreeMap::new();

        // sanity check, plus initialize the tracking
        // For each arm, verify atom count matches head count, and that variant names belong to the right enum type. Also initializes map_arms to track how many combos map to each arm (for the "unused arm" check later)
        for (i, arm) in self.arms.iter().enumerate() {
            map_arms.insert(i, 0_usize);
            if arm.atoms.len() != heads.len() {
                bail_on!(expr, "atoms and heads number mismatch");
            }
            for (atom, (adt_name, adt_variants)) in arm.atoms.iter().zip(heads.iter()) {
                match atom {
                    MatchAtom::Default => (),
                    MatchAtom::Binding(branch, _unpack) => {
                        if adt_name != &branch.ty_name {
                            bail_on!(expr, "atoms and heads ADT name mismatch");
                        }
                        if !adt_variants.contains_key(&branch.variant) {
                            bail_on!(expr, "atoms and heads ADT variant mismatch");
                        }
                    }
                }
            }
        }

        // list all the combo
        let mut all_combinations = vec![];
        // combo on each run is the vector of a random combination of the variants of the type for all the variables in the match head
        for combo in heads
            .iter()
            .map(|(_, names)| names.iter())
            .multi_cartesian_product()
        {
            // sanity check
            assert_eq!(combo.len(), heads.len());

            // conversion
            // combo_as_branch is a random combination of the type::variant for all the variables in the match head along with the default unpacking where the tuple or struct variables are empty
            let combo_as_branch: Vec<_> = combo
                .into_iter()
                .zip(heads.iter())
                .map(|((variant_name, variant_default), (type_name, _))| {
                    let branch = ADTBranch {
                        ty_name: type_name.clone(),
                        variant: variant_name.clone(),
                    };
                    (branch, variant_default)
                })
                .collect();

            // go over each arm and check which is the match
            let mut found = MatchComboStatus::None;
            // in this loop it is run for each random combination of the enum::variant for all the variables in the match head along with the default unpacking where the tuple or struct variables are empty against each arm
            for (i, arm) in self.arms.iter().enumerate() {
                let mut variants = vec![];
                let mut is_matched = true;
                let mut is_abstract = false;
                // foreach of the arms match the atoms with the enum::variant.
                for ((combo_branch, default_unpack), arm_atom) in
                    combo_as_branch.iter().zip(arm.atoms.iter())
                {
                    let combo_unpack = match arm_atom {
                        MatchAtom::Default => {
                            is_abstract = true;
                            *default_unpack // if a pattern is _, then the default unpacking is used because there is no binding
                        }
                        MatchAtom::Binding(branch, unpack) => if branch != combo_branch {
                            is_matched = false; // here the match head for example is MyEnum::MyVariant1(a) and the pattern is MyEnum::MyVariant2(b) so the type name of the head and the pattern match but the variant name does not match. If the type did not match it woul be caught earlier
                            break;
                        } else {
                            unpack
                        },
                    };
                    let variant = MatchVariant {
                        branch: combo_branch.clone(),
                        unpack: combo_unpack.clone(),
                    };
                    // for each arm variants is the list of enum::variant and the unpack (whether it is default or binding). Unpack has the variables used as the pattern in the match arm
                    variants.push(variant);
                }

                // check if everything matches
                if !is_matched {
                    continue;
                }

                // assign the combo
                let combo = MatchCombo {
                    variants,
                    body: arm.body.clone(),
                };
                if is_abstract {
                    match found {
                        MatchComboStatus::None => {
                            found = MatchComboStatus::Abstract(vec![(i, combo)]);
                        }
                        MatchComboStatus::Abstract(existing) => {
                            found = MatchComboStatus::Abstract(
                                existing
                                    .into_iter()
                                    .chain(std::iter::once((i, combo)))
                                    .collect(),
                            )
                        }
                        MatchComboStatus::Concrete(..) => {
                            // do nothing, a concrete match takes priority
                        }
                    }
                } else {
                    // if two concrete arms match to the same combo, raise an error
                    // this is because two arms have the same pattern and the same combination of the enum::variant for all the variables in the match head
                    if matches!(found, MatchComboStatus::Concrete(..)) {
                        bail_on!(expr, "two concrete match arms handles the same combination");
                    }
                    found = MatchComboStatus::Concrete(i, combo);
                }
            }

            // ensure that each combo is handled by one and only one match arm
            let (i, combo) = match found {
                MatchComboStatus::None => bail_on!(expr, "no match arms handles a combination"), // the match arms need to be exhaustive
                MatchComboStatus::Abstract(candidates) => {
                    // cannot have multile wild card match arms that match the same combination
                    if candidates.len() != 1 {
                        bail_on!(expr, "ambiguous abstract match arms");
                    }
                    candidates.into_iter().next().unwrap()
                }
                MatchComboStatus::Concrete(i, combo) => (i, combo),
            };

            all_combinations.push(combo);
            map_arms.entry(i).and_modify(|c| *c += 1);
        }

        // check that every match arm is useful
        if map_arms.values().any(|v| *v == 0) {
            bail_on!(expr, "unused match arms"); // one match arm might match multiple combinations but it should match at least one
        }

        Ok(all_combinations)
    }
}

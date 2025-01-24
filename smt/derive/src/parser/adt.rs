//! This module is only used in the Expr module.
//! This module provide utilities for analyzing match expressions.

use std::collections::BTreeMap;

use itertools::Itertools;
use syn::{ExprMatch, ExprPath, FieldPat, Member, Pat, PatOr, PatStruct, PatTupleStruct, Result};

use crate::{bail_if_exists, bail_if_missing, bail_on};
use crate::parser::expr::{CtxtForExpr, Expr, MatchCombo, MatchVariant, Unpack};
use crate::parser::generics::GenericsInstFull;
use crate::parser::infer::{ti_unify, TypeRef, TypeUnifier};
use crate::parser::name::{UsrTypeName, VarName};
use crate::parser::path::{ADTBranch, ADTPath};
use crate::parser::ty::EnumVariant;

/// An atom for a specific variable in the match head
/// For example in match (x, y) { (1, 2) => ... } one MatchAtom is for x and the other is for y
pub enum MatchAtom {
    Default,
    Binding(BTreeMap<ADTBranch, Unpack>),
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
            _ => Some(pat.try_into()?), // it will give an error if the pattern has a subpattern, is a ref, or is mutable. Otherwise, the ident is extracted. If the identifier is not a reserved keyword or underscore, the varname is returned.
        };
        Ok(binding)
    }

    /// Analyze a pattern for: match arm -> head -> case
    /// For example, in match (1, 2) { (some, thing) => ... } the case for the first arm is (some, thing)
    /// for each arm: each of the (some, 1) and (thing, 2) are made in the expr.rs module and passed to the analyze_pat_match_head one by one
    /// in (some, 1), the `some` pattern is analyzed in this function
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
            // matching against enum variants
            Pat::Path(pat_path) => {
                let ExprPath {
                    attrs: _,
                    qself,
                    path,
                } = pat_path;
                // the qself should not be there (for example this is invalid: <T>::A)
                bail_if_exists!(qself.as_ref().map(|q| &q.ty));

                // so the path should be an enum variant and can only have two segments.
                // match 1 {
                //     1 => println!("Hello, world!"), // 1 is a Pat::Lit and is not allowed. This is because we use the Integer record struct defined in the dt.rs of the stdlib. So normal numbers are not allowed.
                //     x => println!("{}", x), // x is a Pat::Ident and only has one segment and is not allowed. this is because x plays the role of a wildcard. So there is no need to have a binding for it.
                // } not allowed
                // in let x = 1; match 1 { x => println!("{}", x) } the inner x is different from the outer x. So again no binding is created
                let adt = ADTPath::from_path(ctxt, path)?;
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

                // it should definitely be a tuple variant pattern (so matching can only happen against tuple variants). If the pattern was a struct like struct MyStruct(i32); it will give an error on this line
                let adt = ADTPath::from_path(ctxt, path)?;
                let (branch, inst) = adt.complete(unifier);
                let variant = match ctxt.get_adt_variant_details(&branch) {
                    None => bail_on!(path, "not a valid enum branch"), // it will never happen as this is already checked in the ADTPath::from_path
                    Some(def) => def,
                };

                match variant {
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
                    // the variant can only be a tuple variant
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
                                None => bail_on!(member, "no such type parameter"),
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
                                    unpack.insert(field_name, var);
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

    /// Analyze a pattern for: match arm -> head
    /// For example, in match (1, 2) { (some, thing) => ... } the head is (1, 2) and the case for the first arm is (some, thing)
    /// for each arm in the match body, the pattern is taken (for example (some, thing)). Each of the (some, 1) and (thing, 2) are made and passed to the analyze_pat_match_head one by one
    /// ctxt: is the ExprParserRoot (this will always remain the same because we are parsing only inside one function - so the context, the kind, and the signature will remain the same)
    pub fn analyze_pat_match_head<T: CtxtForExpr>(
        ctxt: &T,
        unifier: &mut TypeUnifier,
        pat: &Pat,
        head: &Expr,
    ) -> Result<(MatchAtom, BTreeMap<VarName, TypeRef>)> {
        // receive the TypeRef of the header
        let ety = head.ty();

        let (atom, bindings) = match pat {
            // A pattern that matches any value: `_`.
            // if the pattern is `_`, then it is a default pattern and no bindings are there
            Pat::Wild(_) => (MatchAtom::Default, BTreeMap::new()),
            // A pattern that matches any one of a set of cases
            // for example match x { 1 | 2 => ... }
            Pat::Or(pat_or) => {
                let PatOr {
                    attrs: _,
                    leading_vert,
                    cases,
                } = pat_or;
                // the leading `|` should not be there
                bail_if_exists!(leading_vert);

                let mut variants = BTreeMap::new();
                let mut iter = cases.iter(); // iterator over the cases
                                             // there should be at least one case
                let pat_case = bail_if_missing!(iter.next(), pat_or, "empty or case pattern");

                // analyze the bindings
                // analyze_pat_match_case receives the pattern, the unifier, and the context
                let (branch, inst, unpack, ref_bindings) =
                    Self::analyze_pat_match_case(ctxt, unifier, pat_case)?;

                // unify the type
                let ty_ref = inst.make_ty(branch.ty_name.clone());
                ti_unify!(unifier, ety, &ty_ref, pat_case);

                // save it
                if variants.insert(branch, unpack).is_some() {
                    bail_on!(cases, "duplicated adt variant");
                }

                for pat_case in iter.by_ref() {
                    // analyze the bindings
                    let (branch, inst, unpack, new_bindings) =
                        Self::analyze_pat_match_case(ctxt, unifier, pat_case)?;

                    // unify the type
                    let ty_ref = inst.make_ty(branch.ty_name.clone());
                    ti_unify!(unifier, ety, &ty_ref, pat_case);

                    // check binding consistency
                    if ref_bindings != new_bindings {
                        bail_on!(pat_case, "case patterns do not bind the same variable set");
                    }
                    // save it
                    if variants.insert(branch, unpack).is_some() {
                        bail_on!(cases, "duplicated adt variant");
                    }
                }

                // done
                (MatchAtom::Binding(variants), ref_bindings)
            }
            // other than a wildcard or a or pattern, the pattern can only be enum unit, enum tuple, or enum record
            _ => {
                // analyze the bindings
                // branch is the enum type name and variant name of the pattern (every pattern is a variant of an enum)
                // inst is the type parameters matched with the arguments of the calling place
                // unpack is the unpacked form of the pattern (unit, tuple, or record)
                // bindings is the map of the bindings (contains the variable name and the type reference)
                // bindings lists the elements inside the variant tuple or record
                let (branch, inst, unpack, bindings) =
                    Self::analyze_pat_match_case(ctxt, unifier, pat)?;

                // unify the type (the type of the header is the expected type of the pattern)
                // ety is the type of the header
                // ty_ref is the type of the pattern (TypeRef::User<ty_name of branch, arguments>
                let ty_ref = inst.make_ty(branch.ty_name.clone());
                ti_unify!(unifier, ety, &ty_ref, pat);

                // done
                let variants = std::iter::once((branch, unpack)).collect();
                (MatchAtom::Binding(variants), bindings)
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
        self,
        expr: &ExprMatch,
        heads: &[(UsrTypeName, BTreeMap<String, Unpack>)],
    ) -> Result<Vec<MatchCombo>> {
        // utility enum to indicate whether a match combo is concrete or abstract
        enum MatchComboStatus {
            None,
            Abstract(Vec<(usize, MatchCombo)>),
            Concrete(usize, MatchCombo),
        }

        // tracks how many combo are mapped to each arm
        let mut map_arms = BTreeMap::new();

        // sanity check, plus initialize the tracking
        for (i, arm) in self.arms.iter().enumerate() {
            map_arms.insert(i, 0_usize);
            if arm.atoms.len() != heads.len() {
                bail_on!(expr, "atoms and heads number mismatch");
            }
            for (atom, (adt_name, adt_variants)) in arm.atoms.iter().zip(heads.iter()) {
                match atom {
                    MatchAtom::Default => (),
                    MatchAtom::Binding(binding) => {
                        for branch in binding.keys() {
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
        }

        // list all the combo
        let mut all_combinations = vec![];
        for combo in heads
            .iter()
            .map(|(_, names)| names.iter())
            .multi_cartesian_product()
        {
            // sanity check
            assert_eq!(combo.len(), heads.len());

            // conversion
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
            for (i, arm) in self.arms.iter().enumerate() {
                let mut variants = vec![];
                let mut is_matched = true;
                let mut is_abstract = false;
                for ((combo_branch, default_unpack), arm_atom) in
                    combo_as_branch.iter().zip(arm.atoms.iter())
                {
                    let combo_unpack = match arm_atom {
                        MatchAtom::Default => {
                            is_abstract = true;
                            *default_unpack
                        }
                        MatchAtom::Binding(binding) => match binding.get(combo_branch) {
                            None => {
                                is_matched = false;
                                break;
                            }
                            Some(unpack) => unpack,
                        },
                    };
                    let variant = MatchVariant {
                        branch: combo_branch.clone(),
                        unpack: combo_unpack.clone(),
                    };
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
                    if matches!(found, MatchComboStatus::Concrete(..)) {
                        bail_on!(expr, "two concrete match arms handles the same combination");
                    }
                    found = MatchComboStatus::Concrete(i, combo);
                }
            }

            // ensure that each combo is handled by one and only one match arm
            let (i, combo) = match found {
                MatchComboStatus::None => bail_on!(expr, "no match arms handles a combination"),
                MatchComboStatus::Abstract(candidates) => {
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
            bail_on!(expr, "unused match arms");
        }

        Ok(all_combinations)
    }
}

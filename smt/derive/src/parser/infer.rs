//! Type inference and unification for the SMT parser.

use crate::parser::name::{TypeParamName, UsrTypeName};
use crate::parser::ty::TypeTag;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

/// An error for type inference
pub enum TIError {
    /// A cyclic unification error.
    CyclicUnification,
}

/// It is only used in this module in the form of TIResult<Option<TypeRef>>.
type TIResult<T> = Result<T, TIError>;

/// This macro simplifies error handling during type unification.
/// - If the result is `Ok(Some(value))`, it retrieves the value.
/// - If the result is `Ok(None)`, it returns `Ok(None)` immediately.
/// - If the result is `Err(err)`, where `err` is `TIError::CyclicUnification`, it bails with an error message.
macro_rules! ti_unwrap {
    ($item:expr) => {
        match ($item)? {
            None => return Ok(None),
            Some(__v) => __v,
        }
    };
}

/// This macro simplifies the unification process and error handling.
macro_rules! ti_unify {
    ($unifier:expr, $lhs:expr, $rhs:expr, $spanned:expr) => {
        match $unifier.unify($lhs, $rhs) {
            Err($crate::parser::infer::TIError::CyclicUnification) => {
                $crate::bail_on!($spanned, "cyclic type unification");
            }
            Ok(None) => {
                // Refresh types to get their current state before reporting error
                let lhs_refreshed = $unifier.refresh_type($lhs);
                let rhs_refreshed = $unifier.refresh_type($rhs);
                $crate::bail_on!(
                    $spanned,
                    "type mismatch: cannot unify {} with {}",
                    lhs_refreshed,
                    rhs_refreshed
                );
            }
            Ok(Some(__v)) => __v, // __v is of type TypeRef and is the result of the unification.
        }
    };
}
pub(crate) use ti_unify; // this makes the ti_unify! macro available to other modules in the crate. But it is not available to external crates that depend on this crate. To make it available to external crates, #[macro_export] should be added before the macro definition.

/// Represents a type variable used in type unification.
///
/// Each `TypeVar` is identified by a unique `usize` index.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct TypeVar(usize);

impl Display for TypeVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "?{}", self.0) // the display is ?<index> where index is the usize value of the TypeVar.
    }
}

/// Represents a type reference in the type unification process.
///
/// Like TypeTag, but allows type variable for undetermined types.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum TypeRef {
    /// variable
    Var(TypeVar),
    /// boolean
    Boolean,
    /// integer (unlimited precision)
    Integer,
    /// real numbers (unlimited precision)
    Real,
    /// floating point 32-bit
    F32,
    /// floating point 64-bit
    F64,
    /// signed 32-bit integer
    I32,
    /// signed 64-bit integer
    I64,
    /// unsigned 32-bit integer
    U32,
    /// unsigned 64-bit integer
    U64,
    /// string
    String,
    /// inductively defined type
    Cloak(Box<TypeRef>),
    /// SMT-sequence
    Seq(Box<TypeRef>),
    /// SMT-set
    Set(Box<TypeRef>),
    /// SMT-array
    Array(Box<TypeRef>, Box<TypeRef>),
    /// path-condition marker type
    Path,
    /// user-defined type
    User(UsrTypeName, Vec<TypeRef>),
    /// a tuple of types
    Pack(Vec<TypeRef>),
    /// parameter like generics
    Parameter(TypeParamName),
}

impl From<&TypeTag> for TypeRef {
    /// Converts a `TypeTag` reference into a `TypeRef`.
    ///
    /// This allows for converting concrete types into types that can participate in unification.
    fn from(ty: &TypeTag) -> Self {
        match ty {
            TypeTag::Boolean => Self::Boolean,
            TypeTag::Integer => Self::Integer,
            TypeTag::Real => Self::Real,
            TypeTag::F32 => Self::F32,
            TypeTag::F64 => Self::F64,
            TypeTag::U32 => Self::U32,
            TypeTag::U64 => Self::U64,
            TypeTag::I32 => Self::I32,
            TypeTag::I64 => Self::I64,
            TypeTag::String => Self::String,
            TypeTag::Cloak(sub) => Self::Cloak(Box::new(sub.as_ref().into())), // as_ref() dereferences the `sub` twice and adds a pointer. The first dereference eliminates the & reference, and the second dereference eliminates the Box reference. Therefore, sub.as_ref() gives &TypeTag, and sub.as_ref().into() gives TypeRef. The into() method is the same as the From trait. so sub.as_ref().into() is the same as TypeRef::from(sub.as_ref()).
            TypeTag::Seq(sub) => Self::Seq(Box::new(sub.as_ref().into())),
            TypeTag::Set(sub) => Self::Set(Box::new(sub.as_ref().into())),
            TypeTag::Array(key, val) => {
                Self::Array(Box::new(key.as_ref().into()), Box::new(val.as_ref().into()))
            }
            TypeTag::Path => Self::Path,
            TypeTag::User(name, tags) => {
                Self::User(name.clone(), tags.iter().map(|t| t.into()).collect())
            }
            TypeTag::Pack(elems) => Self::Pack(elems.iter().map(|t| t.into()).collect()),
            TypeTag::Parameter(name) => Self::Parameter(name.clone()),
        }
    }
}

impl TypeRef {
    /// Validates whether the type is complete (i.e., contains no type variables).
    ///
    /// Returns `true` if the type contains no `Var` variants; `false` otherwise.
    pub fn validate(&self) -> bool {
        match self {
            Self::Var(_) => false,
            Self::Boolean
            | Self::Integer
            | Self::Real
            | Self::F32
            | Self::F64
            | Self::I32
            | Self::I64
            | Self::U32
            | Self::U64
            | Self::String
            | Self::Path
            | Self::Parameter(_) => true,
            Self::Cloak(sub) | Self::Seq(sub) | Self::Set(sub) => sub.validate(),
            Self::Array(key, val) => key.validate() && val.validate(),
            Self::Pack(elems) => elems.iter().all(|t| t.validate()), // all() returns true if all elements of the iterator return true when passed to the closure.
            Self::User(_, args) => args.iter().all(|t| t.validate()),
        }
    }
}

impl Display for TypeRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(var) => var.fmt(f), // like TypeVar, the display is ?<index> where index is the usize value of the TypeVar.
            Self::Boolean => write!(f, "Boolean"),
            Self::Integer => write!(f, "Integer"),
            Self::Real => write!(f, "Real"),
            Self::F32 => write!(f, "F32"),
            Self::F64 => write!(f, "F64"),
            Self::I32 => write!(f, "I32"),
            Self::I64 => write!(f, "I64"),
            Self::U32 => write!(f, "U32"),
            Self::U64 => write!(f, "U64"),
            Self::String => write!(f, "String"),
            Self::Cloak(sub) => write!(f, "Cloak<{sub}>"),
            Self::Seq(sub) => write!(f, "Seq<{sub}>"),
            Self::Set(sub) => write!(f, "Set<{sub}>"),
            Self::Array(key, val) => write!(f, "Array<{key},{val}>"),
            Self::Path => write!(f, "Path"),
            Self::User(name, args) => {
                if args.is_empty() {
                    name.fmt(f) // if there are no type arguments, just print the name.
                } else {
                    write!(f, "{}<{}>", name, args.iter().format(",")) // if there are type arguments, print the name followed by the type arguments separated by commas.
                }
            }
            Self::Pack(elems) => {
                write!(f, "({})", elems.iter().format(",")) // print the elements of the tuple separated by commas.
            }
            Self::Parameter(name) => name.fmt(f), // invokes the Display trait for TypeParamName (write!(f, "{}", name)).
        }
    }
}

/// An equivalence group of type variables.
///
/// This struct is not public because it is only used internally by the Typing struct.
#[derive(Clone, Debug)]
struct TypeEquivGroup {
    /// Set of type variable indices that are equivalent.
    vars: BTreeSet<usize>, // Example: {0, 4, 7} means "Variable 0, Variable 4, and Variable 7 are all the same type.
    /// The reference type assigned to this equivalence group, if any.
    sort: Option<TypeRef>,
    // Example: Some(Integer). This means "Variables 0, 4, and 7 are ALL Integers."
    // Example: None. This means "We know 0, 4, and 7 are the same, but we still don't know what type that is."
}

impl TypeEquivGroup {
    /// Represent this group with a type variable at the minimum index and returns a `TypeRef::Var`.
    ///
    /// example if vars = BTreeSet::from([4,2,3,4]), then the minimum index is 2.
    fn var(&self) -> TypeRef {
        let var = *self
            .vars
            .first()
            .expect("at least one type variable is expected in the equivalence group");
        TypeRef::Var(TypeVar(var))
    }

    /// Extracts the representative type of this group.
    ///
    /// "What is the current best name for this group?"
    pub fn repr(&self) -> TypeRef {
        match self.sort.as_ref() {
            // as_ref() Converts from `&Option<T>` to `Option<&T>` to not take ownership of TypeRef.
            None => self.var(),
            Some(t) => t.clone(),
        }
    }
}

/// Represents a type unification instance.
/// This struct is not public because it is only used internally by the TypeUnifier struct.
#[derive(Clone)]
struct Typing {
    /// A key-value map where the key is the index of a type variable (like 0 in TypeVar(0usize)) and the value is the index of the equivalence group of the type variable.
    params: BTreeMap<usize, usize>, // "Where is the file for variable #5?"
    /// Holds the equivalence groups.
    groups: Vec<TypeEquivGroup>,
}

impl Typing {
    /// Create an empty type unification context
    fn new() -> Self {
        Self {
            params: BTreeMap::new(),
            groups: vec![],
        }
    }

    /// Creates a new type variable and returns it.
    ///
    /// This is the only method which adds elements to the params and groups fields of the Typing struct.
    fn mk_var(&mut self) -> TypeVar {
        let var_id = self.params.len(); // a freshly new index as the params length is increased by 1 on each call. A new type variable with this index will be created.
        let group_index = self.groups.len(); // a freshly new index as the groups length is increased by 1 on each call.

        // Register the param
        let existing = self.params.insert(var_id, group_index); // (0,0), (1,1), (2,2), ...

        // this will never happen! because the var_id is unique at each call.
        if existing.is_some() {
            panic!("Type variable already exists");
        }

        // Assign a fresh equivalence group to the type variable.
        let group = TypeEquivGroup {
            vars: std::iter::once(var_id).collect(),
            sort: None, // initially, there is no concrete type assigned to the type variable.
        };
        // Register the group (the index of this group is group_index which is the value of var_id key in the params BTreeMap).
        self.groups.push(group);

        // Return the new type variable.
        TypeVar(var_id)
    }

    /// Merge the type constraints
    ///
    /// l has the lower index (i.e., l.0 < h.0).
    fn merge_group(
        &mut self,
        l: &TypeVar,
        h: &TypeVar,
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeRef>> {
        // Obtain group indices.
        // obtain the equivalence group index for the left type variable.
        let idx_l = *self
            .params
            .get(&l.0)
            .expect("type variable not found for l in merge_group");

        // obtain the equivalence group index for the right type variable.
        let idx_h = *self
            .params
            .get(&h.0)
            .expect("type variable not found for h in merge_group");

        // Obtain groups.
        // group_l is the equivalence group of the type variable at the lower index.
        let mut group_l = self
            .groups
            .get(idx_l)
            .expect("equivalence group not found for l in merge_group")
            .clone();

        // group_h is the equivalence group of the type variable at the higher index.
        let group_h = self
            .groups
            .get(idx_h)
            .expect("equivalence group not found for h in merge_group")
            .clone();

        // Nothing to do if they belong to the same group. This can happen if they have been unified before in the merge_group method (only method to update the index key of the params BTreeMap) => *self.params.get_mut(&h.0).unwrap() = idx_l;
        if idx_l == idx_h {
            return Ok(Some(group_l.repr())); // return the representative type of the equivalence group.
        }

        // Check for cyclic unification.
        // a new involved will be created for every expression.
        if !involved.is_disjoint(&group_l.vars) {
            return Err(TIError::CyclicUnification);
        }
        if !involved.is_disjoint(&group_h.vars) {
            return Err(TIError::CyclicUnification);
        }

        // Prevent recursive typing.
        involved.extend(group_l.vars.iter().copied());
        involved.extend(group_h.vars.iter().copied());

        // Unify the equivalence set, after a sanity checking.
        if !group_l.vars.is_disjoint(&group_h.vars) {
            panic!("Non-disjoint equivalence sets");
        }
        group_l.vars.extend(group_h.clone().vars);

        // check whether they unify to the same type, if any
        // this match basically updates the sort of the lower type variable equivalence group.
        match (group_l.sort.as_ref(), group_h.sort.as_ref()) {
            (None, None) => {
                // Neither group has a concrete type; nothing to unify.
            }
            (Some(_), None) => {
                // The lower group already has candidates; keep it.
            }
            (None, Some(sort_h)) => {
                // propagate the type candidates to the lower group
                group_l.sort = Some(sort_h.clone());
            }
            // both groups have concrete types
            (Some(sort_l), Some(sort_h)) => {
                // further unity (refine) the types, also check for mismatches
                let unified = ti_unwrap!(self.unify(sort_l, sort_h, involved));
                group_l.sort = Some(unified);
            }
        };

        // update the equivalence group of the lower type variable.
        *self.groups.get_mut(idx_l).unwrap() = group_l.clone();
        // update the equivalence group of the higher type variable to the equivalence group of the lower type variable. So both the type variables l and h point to the same equivalence group (have the same group index).
        for var_id in &group_h.vars {
            *self
                .params
                .get_mut(var_id)
                .expect("Var in group but not params") = idx_l;
        }

        // Pre-calculate the inferred type.
        let inferred = group_l.repr();
        // Return the inferred type.
        Ok(Some(inferred))
    }

    /// Assign the constraint (this is only used when one of the types is a type variable and the other is a concrete type).
    ///
    /// Updates the type of the type variable `v` with the concrete type `t`.
    fn update_group(
        &mut self,
        v: &TypeVar,
        t: &TypeRef, // t is not TypeRef::Var(TypeVar).
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeRef>> {
        // Obtain the group index and group.
        let idx = *self.params.get(&v.0).unwrap();
        let mut group = self.groups.get(idx).unwrap().clone();

        // Decide whether further unification is needed.
        let inferred = match group.sort.as_ref() {
            // if the group does not have a concrete type, assign `t` as the sort.
            None => {
                // propagate the type to the group; assign `t`.
                t.clone()
            }
            // if the group has a concrete type, unify `t` with the sort (if possible).
            Some(e) => {
                // further unity (refine) the types, also check for mismatches
                ti_unwrap!(self.unify(e, t, involved))
            }
        };

        // update the type in this group
        group.sort = Some(inferred.clone());

        // update the group (basically, the sort is updated if it is None or unified with the concrete type `t`).
        *self.groups.get_mut(idx).unwrap() = group;

        // Return the inferred type.
        Ok(Some(inferred))
    }

    /// Unifies two types and returns the unified type.
    ///
    /// This is the core unification algorithm.
    ///
    /// # Arguments
    ///
    /// * `lhs` - The left-hand side type marking the returned type.
    /// * `rhs` - The right-hand side type marking the expected type.
    /// * `involved` - A set of type variable indices involved in unification to detect cycles.
    ///
    /// # Errors
    ///
    /// Returns `TIError::CyclicUnification` if a cyclic unification is detected.
    fn unify(
        &mut self,
        lhs: &TypeRef,
        rhs: &TypeRef,
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeRef>> {
        use TypeRef::*; // Import all variants of TypeRef for convenience.

        let inferred = match (lhs, rhs) {
            // Both are type variables.
            (Var(l), Var(r)) => match Ord::cmp(&l.0, &r.0) {
                // same variables as they have same ids
                Ordering::Equal => {
                    // if the indexes of the type variables are equal, return the type variable itself. In this case, the type variables are the same. They are the same variable.
                    // no knowledge gain in this case
                    Var(l.clone())
                }
                Ordering::Less => ti_unwrap!(self.merge_group(l, r, involved)), // l is the lower index (l.0 < r.0) that is l was created before r.
                Ordering::Greater => ti_unwrap!(self.merge_group(r, l, involved)), // r is the lower index (r.0 < l.0) that is r was created before l.
            },
            // One is a variable.
            (Var(l), _) => ti_unwrap!(self.update_group(l, rhs, involved)),
            (_, Var(r)) => ti_unwrap!(self.update_group(r, lhs, involved)),

            // Both are concrete types.
            (Boolean, Boolean) => Boolean,
            (Integer, Integer) => Integer,
            (Real, Real) => Real,
            (F32, F32) => F32,
            (F64, F64) => F64,
            (I32, I32) => I32,
            (I64, I64) => I64,
            (U32, U32) => U32,
            (U64, U64) => U64,
            (String, String) => String,
            (Path, Path) => Path,

            // variadic types
            (Cloak(sub_lhs), Cloak(sub_rhs)) => {
                Cloak(ti_unwrap!(self.unify(sub_lhs, sub_rhs, involved)).into())
                // self.unify(sub_lhs, sub_rhs, involved) returns TIResult<Option<TypeRef>>. ti_unwrap! macro unwraps the result and returns the TypeRef. The into() method converts the TypeRef into a Box<TypeRef>. Nonetheless, if the unify is an error, ti_unwrap! returns the error (TIError::CyclicUnification). and if it is Ok(None), it returns Ok(None).
            }
            (Seq(sub_lhs), Seq(sub_rhs)) => {
                Seq(ti_unwrap!(self.unify(sub_lhs, sub_rhs, involved)).into())
            }
            (Set(sub_lhs), Set(sub_rhs)) => {
                Set(ti_unwrap!(self.unify(sub_lhs, sub_rhs, involved)).into())
            }
            (Array(key_lhs, val_lhs), Array(key_rhs, val_rhs)) => Array(
                ti_unwrap!(self.unify(key_lhs, key_rhs, involved)).into(),
                ti_unwrap!(self.unify(val_lhs, val_rhs, involved)).into(),
            ),

            // Unify user-defined types.
            (User(name_lhs, args_lhs), User(name_rhs, args_rhs)) => {
                // If names are different, types cannot be unified.
                if name_lhs != name_rhs {
                    return Ok(None);
                }

                // invariant checking (the number of type arguments must match to unify) - this will never happen! because we cannot have different numbers of type arguments for the same user-defined type.
                if args_lhs.len() != args_rhs.len() {
                    panic!("Type argument number mismatch");
                }

                // try to unify the type arguments
                let mut new_args = vec![];
                for (v_lhs, v_rhs) in args_lhs.iter().zip(args_rhs) {
                    new_args.push(ti_unwrap!(self.unify(v_lhs, v_rhs, involved)));
                }
                User(name_lhs.clone(), new_args)
            }
            // packs (i.e., type tuples)
            (Pack(pack_lhs), Pack(pack_rhs)) => {
                // If tuple sizes differ, cannot unify.
                if pack_lhs.len() != pack_rhs.len() {
                    panic!("Tuple size mismatch");
                }

                // Unify each element.
                let mut new_elems = vec![];
                for (v_lhs, v_rhs) in pack_lhs.iter().zip(pack_rhs) {
                    new_elems.push(ti_unwrap!(self.unify(v_lhs, v_rhs, involved)));
                }
                Pack(new_elems)
            }
            // Unify type parameters.
            (Parameter(name_lhs), Parameter(name_rhs)) => {
                // If names are different, types cannot be unified (for example, T and U cannot be unified).
                if name_lhs != name_rhs {
                    return Ok(None);
                }
                Parameter(name_lhs.clone())
            }
            // All other cases are considered a type mismatch.
            _ => return Ok(None),
        };

        // Return the inferred type.
        Ok(Some(inferred))
    }

    /// Retrieve the type behind the type variable
    fn retrieve_type(&self, var: &TypeVar) -> TypeRef {
        let idx = *self
            .params
            .get(&var.0)
            .expect("type var should have an equivalence group"); // get the index of the equivalence group of the TypeVar.
        self.groups.get(idx).unwrap().repr()
    }
}

/// Context manager for type unification (this is the API for the type unification that other modules will use).
///
/// let mut unifier = TypeUnifier::new();
/// let var = TypeRef::Var(unifier.mk_var()); Do not directly create like TypeRef::Var(TypeVar(0usize)). Instead, use the mk_var method of the TypeUnifier struct. This is because the TypeVar is added to the params and groups of the Typing struct.
/// let int_type = TypeRef::Integer;
/// let result = unifier.unify(&var, &int_type).
#[derive(Clone)]
pub struct TypeUnifier {
    /// The internal typing worker that performs unification.
    typing: Typing,
}

impl TypeUnifier {
    /// Create a new type unifier.
    pub fn new() -> Self {
        Self {
            typing: Typing::new(), // Typing::new() creates a new Typing instance. like Typing { params: BTreeMap::new(), groups: vec![] }. So the params and groups are empty at the beginning.
        }
    }

    /// Creates a new type variable.
    ///
    /// # Returns
    /// A new `TypeVar` instance.
    ///
    /// Basically if an explicit type is not declared, we call TypeRef::Var(unifier.mk_var()) to create a new type variable.
    pub fn mk_var(&mut self) -> TypeVar {
        self.typing.mk_var() // calls the mk_var method of the Typing struct. The new TypeVar is also added to the params and groups of the Typing struct.
    }

    /// Unifies two types and returns the unified type.
    ///
    /// # Parameters
    /// - `lhs`: The left-hand side type marking the returned type for example right-hand side of the assignment.
    /// - `rhs`: The right-hand side type marking the expected type for example left hand side of the assignment.
    ///
    /// # Returns
    /// A `TIResult` containing the unified type or `None` if unification fails.
    ///
    /// This is called in the ti_unify! macro and is also used in other modules.
    pub fn unify(&mut self, lhs: &TypeRef, rhs: &TypeRef) -> TIResult<Option<TypeRef>> {
        let mut involved = BTreeSet::new(); // initialize the involved set.
        self.typing.unify(lhs, rhs, &mut involved)
    }

    /// Retrieve either an assigned type or the variable itself (if multiple options available)
    ///
    /// # Parameters
    /// - `var`: The type variable to retrieve the type for.
    ///
    /// # Returns
    /// The `TypeRef` representing the type of the variable.
    ///
    /// Only used in TypeUnifier::refresh_type.
    /// self.typing.retrieve_type(var) calls the retrieve_type method of the Typing struct.
    /// The retrieve_type method of the Typing struct checks the equivalence group of the TypeVar and returns the representative type of the group (that can be from the sort field of the group and if it is None, it returns the minimum index of the vars field of the group).
    fn retrieve_type(&self, var: &TypeVar) -> TypeRef {
        self.typing.retrieve_type(var)
    }

    /// Refreshes a type by replacing any type variables with their inferred types.
    ///
    /// # Parameters
    /// - `ty`: The type to refresh.
    ///
    /// # Returns
    /// A new `TypeRef` with type variables replaced.
    ///
    /// Try to instantiate a type when needed
    pub fn refresh_type(&self, ty: &TypeRef) -> TypeRef {
        match ty {
            TypeRef::Var(var) => self.retrieve_type(var), // calls the TypeUnifier::retrieve_type method which in turn calls the Typing::retrieve_type method. It retrieves the equivalence group of the TypeVar and returns the representative type of the group (that can be from the sort field of the group and if it is None, it returns the minimum index of the vars field of the group like TypeRef::Var(TypeVar(0usize)) etc.).
            TypeRef::Boolean => TypeRef::Boolean,
            TypeRef::Integer => TypeRef::Integer,
            TypeRef::Real => TypeRef::Real,
            TypeRef::F32 => TypeRef::F32,
            TypeRef::F64 => TypeRef::F64,
            TypeRef::I32 => TypeRef::I32,
            TypeRef::I64 => TypeRef::I64,
            TypeRef::U32 => TypeRef::U32,
            TypeRef::U64 => TypeRef::U64,
            TypeRef::String => TypeRef::String,
            TypeRef::Cloak(sub) => TypeRef::Cloak(self.refresh_type(sub).into()), // the into() method converts the TypeRef into a Box<TypeRef>.
            TypeRef::Seq(sub) => TypeRef::Seq(self.refresh_type(sub).into()),
            TypeRef::Set(sub) => TypeRef::Set(self.refresh_type(sub).into()),
            TypeRef::Array(key, val) => {
                TypeRef::Array(self.refresh_type(key).into(), self.refresh_type(val).into())
            }
            TypeRef::Path => TypeRef::Path,
            TypeRef::User(name, args) => TypeRef::User(
                name.clone(),
                args.iter().map(|t| self.refresh_type(t)).collect(),
            ),
            TypeRef::Pack(elems) => {
                TypeRef::Pack(elems.iter().map(|t| self.refresh_type(t)).collect())
            }
            TypeRef::Parameter(name) => TypeRef::Parameter(name.clone()),
        }
    }
}

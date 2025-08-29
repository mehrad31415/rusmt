//! Type inference and unification for the SMT parser.

use crate::parser::name::{TypeParamName, UsrTypeName};
use crate::parser::ty::TypeTag;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

/// An error for type inference
pub enum TIError {
    /// Error indicating a cyclic unification was detected.
    /// This occurs when a type variable is unified with a type that contains itself. for example, "Type A is equal to a list of Type A." example: type A = [A]
    /// Cyclic unification errors usually mean that the type relationships are self-referential and cannot be resolved, leading to an infinite loop.
    CyclicUnification,
}

/// A specialized `Result` type for type inference operations, where the error type is `TIError` and the success type is a generic `T`.
/// It is only used in this module in the form of TIResult<Option<TypeRef>>.
type TIResult<T> = Result<T, TIError>;

/// A declarative macro that unwraps a `TIResult<Option<T>>`. item? first checks if `item` is an Err(TIError::CyclicUnification) or Ok(Option<T>). If it is an Err, it returns the TIError::CyclicUnification. If it is not, then `Ok` is unwrapped and Option<T> is checked. If it is None, it early returns Ok(None). If it is Some(value), the value is used. This macro can only be used inside functions which have a return type of TIResult<Option<T>>.
///
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

/// Try to unify the two types, bail on the spanned element if not unified.
///
/// If unification fails due to a cyclic unification, it bails with an error message.
/// If unification yields `None`, it bails with an error message.
/// Otherwise, it returns the unified type.
///
/// # Arguments
///
/// * `$unifier` - The type unifier to use, it is of type `&mut TypeUnifier`.
/// * `$lhs` - The left-hand side type marking the returned type, it is of type `&TypeRef`.
/// * `$rhs` - The right-hand side type marking the expected type, it is of type `&TypeRef`.
/// * `$spanned` - The spanned element (e.g., for error reporting).
///
/// The return type of `unify` is `TIResult<Option<TypeRef>>`.
macro_rules! ti_unify {
    ($unifier:expr, $lhs:expr, $rhs:expr, $spanned:expr) => {
        match $unifier.unify($lhs, $rhs) {
            Err($crate::parser::infer::TIError::CyclicUnification) => {
                $crate::bail_on!($spanned, "cyclic type unification");
            }
            Ok(None) => {
                $crate::bail_on!($spanned, "no viable type");
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
pub struct TypeVar(usize); // usize can only store non-negative values (it cannot represent negative numbers).
// The exact size of usize depends on whether the system is 32-bit or 64-bit.

impl Display for TypeVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "?{}", self.0) // the display is ?<index> where index is the usize value of the TypeVar.
    }
}

/// Represents a type reference in the type unification process.
///
/// Like TypeTag, but allows type variable for undetermined types.
/// # Examples
/// let var = TypeVar(0usize);
/// let type_ref = TypeRef::Var(var);
/// println!("Type reference: {}", type_ref); // Type reference: ?0
/// TypeRef is a superset of TypeTag. It can represent all types that TypeTag can represent, plus type variables, which are used to represent types that are not yet explicitly determined.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum TypeRef {
    /// variable
    Var(TypeVar),
    /// boolean
    Boolean,
    /// integer (unlimited precision)
    Integer,
    /// rational numbers (unlimited precision)
    Rational,
    /// string
    Text,
    /// inductively defined type
    Cloak(Box<TypeRef>),
    /// SMT-sequence
    Seq(Box<TypeRef>),
    /// SMT-set
    Set(Box<TypeRef>),
    /// SMT-array
    Map(Box<TypeRef>, Box<TypeRef>),
    /// dynamic error type
    Error,
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
            TypeTag::Rational => Self::Rational,
            TypeTag::Text => Self::Text,
            TypeTag::Cloak(sub) => Self::Cloak(Box::new(sub.as_ref().into())), // as_ref() dereferences the `sub` twice and adds a pointer. The first dereference eliminates the & reference, and the second dereference eliminates the Box reference. Therefore, sub.as_ref() gives &TypeTag, and sub.as_ref().into() gives TypeRef. The into() method is the same as the From trait. so sub.as_ref().into() is the same as TypeRef::from(sub.as_ref()).
            TypeTag::Seq(sub) => Self::Seq(Box::new(sub.as_ref().into())),
            TypeTag::Set(sub) => Self::Set(Box::new(sub.as_ref().into())),
            TypeTag::Map(key, val) => {
                Self::Map(Box::new(key.as_ref().into()), Box::new(val.as_ref().into()))
            }
            TypeTag::Error => Self::Error,
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
    ///
    /// # Examples
    /// let int_type = TypeRef::Integer;
    /// assert!(int_type.validate());
    ///
    /// let var = TypeVar(0usize);
    /// let var_type = TypeRef::Var(var);
    /// assert!(!var_type.validate());
    pub fn validate(&self) -> bool {
        match self {
            Self::Var(_) => false,
            Self::Boolean
            | Self::Integer
            | Self::Rational
            | Self::Text
            | Self::Error
            | Self::Parameter(_) => true,
            Self::Cloak(sub) | Self::Seq(sub) | Self::Set(sub) => sub.validate(),
            Self::Map(key, val) => key.validate() && val.validate(),
            Self::Pack(elems) => elems.iter().all(|t| t.validate()), // all() returns true if all elements of the iterator return true when passed to the closure.
            Self::User(_, args) => args.iter().all(|t| t.validate()),
        }
    }

    /// Converts the `TypeRef` back into a `TypeTag`, if possible.
    ///
    /// Returns `Some(TypeTag)` if the type contains no type variables; `None` otherwise.
    /// This is because the Var(TypeVar) variant cannot be converted back into a TypeTag.
    ///
    /// # Examples
    /// let int_type = TypeRef::Integer;
    /// let type_tag = int_type.reverse();
    /// assert_eq!(type_tag, Some(TypeTag::Integer));
    pub fn reverse(&self) -> Option<TypeTag> {
        let reversed = match self {
            Self::Var(_) => return None, // Cannot reverse a type variable into a concrete type.
            Self::Boolean => TypeTag::Boolean,
            Self::Integer => TypeTag::Integer,
            Self::Rational => TypeTag::Rational,
            Self::Text => TypeTag::Text,
            Self::Error => TypeTag::Error,
            Self::Parameter(name) => TypeTag::Parameter(name.clone()),
            Self::Cloak(sub) => TypeTag::Cloak(Box::new(sub.as_ref().reverse()?)), // as_ref() dereferences the `sub` twice and adds a pointer. The first dereference eliminates the & reference, and the second dereference eliminates the Box reference. Therefore, sub.as_ref() gives &TypeRef, and sub.as_ref().reverse() gives Option<TypeTag>. The ? operator unwraps the Option and returns the value inside the Some variant or returns None.
            Self::Seq(sub) => TypeTag::Seq(Box::new(sub.as_ref().reverse()?)),
            Self::Set(sub) => TypeTag::Set(Box::new(sub.as_ref().reverse()?)),
            Self::Map(key, val) => TypeTag::Map(
                Box::new(key.as_ref().reverse()?),
                Box::new(val.as_ref().reverse()?),
            ),
            Self::Pack(elems) => {
                TypeTag::Pack(elems.iter().map(|t| t.reverse()).collect::<Option<_>>()?)
                // collect() is a method provided by Rust's Iterator trait that transforms an iterator into a collection, such as a Vec, HashMap, an Option, etc.
                // The ::<Option<_>> part is a type hint that tells the compiler to collect the iterator into an <Option<Vec<TypeRef>>> type. The ? operator then unwraps the Option, returning the value inside if it is Some, or returning None if it is None.
                // The Option<_> type means that collect() will return Some(collection) if all elements of the iterator successfully produce Some(value). None if any element produces None.
                // So each element is moved out of the Option (t.reverse() gives Option<TypeRef>) but the whole collection is moved into the outer Option.
            }
            Self::User(name, args) => TypeTag::User(
                name.clone(),
                args.iter().map(|t| t.reverse()).collect::<Option<_>>()?,
            ),
        };
        Some(reversed)
    }
}

impl Display for TypeRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(var) => var.fmt(f), // like TypeVar, the display is ?<index> where index is the usize value of the TypeVar.
            Self::Boolean => write!(f, "Boolean"),
            Self::Integer => write!(f, "Integer"),
            Self::Rational => write!(f, "Rational"),
            Self::Text => write!(f, "Text"),
            Self::Cloak(sub) => write!(f, "Cloak<{sub}>"),
            Self::Seq(sub) => write!(f, "Seq<{sub}>"),
            Self::Set(sub) => write!(f, "Set<{sub}>"),
            Self::Map(key, val) => write!(f, "Map<{key},{val}>"),
            Self::Error => write!(f, "Error"),
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
/// Types that are unified together are stored in an equivalence group.
/// The `vars` field contains the indices of the type variables in the group.
/// The `sort` field contains the reference type assigned to the group which is the inferred type (if any).
/// This struct is not public because it is only used internally by the Typing struct.
#[derive(Clone, Debug)]
struct TypeEquivGroup {
    /// Set of type variable indices that are equivalent.
    vars: BTreeSet<usize>,
    /// The reference type assigned to this equivalence group, if any.
    sort: Option<TypeRef>,
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
    /// If a concrete type is assigned (`sort` is `Some`), returns that type.
    /// Otherwise, returns a `TypeRef::Var` using the minimum type variable index.
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
    params: BTreeMap<usize, usize>,
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
    /// Unifies the groups of `l` and `h`, updating their types accordingly.
    /// Returns the inferred type after merging.
    ///
    /// # Errors
    ///
    /// Returns `TIError::CyclicUnification` if a cyclic unification is detected.
    ///
    /// This is only called in the unification process when both types are type variables.
    /// Here l was created before h so h is merged into l.
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
        group_l.vars.extend(group_h.vars);

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
        *self.params.get_mut(&h.0).unwrap() = idx_l;

        // Pre-calculate the inferred type.
        let inferred = group_l.repr();
        // Return the inferred type.
        Ok(Some(inferred))
    }

    /// Assign the constraint (this is only used when one of the types is a type variable and the other is a concrete type).
    ///
    /// Updates the type of the type variable `v` with the concrete type `t`.
    /// The way this is done is by retrieving the equivalence group of the type variable and assigning the concrete type to the group if it does not have a concrete type already. If the group has a concrete type, the concrete type is unified with the concrete type `t`. This only updates the sort field of the equivalence group.
    ///
    /// # Errors
    ///
    /// Returns `TIError::CyclicUnification` if a cyclic unification is detected.
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
                // Ord::cmp returns Ordering::Less, Ordering::Equal, or Ordering::Greater. It is used to compare two values and it is part of the standard library (does not need to be imported). &l.0.cmp(&r.0) = Ord::cmp(&l.0, &r.0). Ordering needs to be imported (use std::cmp::Ordering).
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
            (Rational, Rational) => Rational,
            (Text, Text) => Text,
            (Error, Error) => Error,

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
            (Map(key_lhs, val_lhs), Map(key_rhs, val_rhs)) => Map(
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
    ///
    /// Returns the concrete type assigned to the variable's equivalence group, or the variable itself.
    ///
    /// When we have a TypeVar, it is like TypeVar(usize). var.0 gives the usize of the TypeVar.
    /// In this case, self.params.get(&var.0) returns the `usize value` of the corresponding TypeVar in the `params` BTreeMap. This is because each TypeVar is stored in the params BTreeMap with its usize as the key. Look at mk_var method for more clarification. The `usize value` is the index of the equivalence group of the TypeVar (idx).
    /// self.groups.get(idx).unwrap() returns the equivalence group of the TypeVar.
    /// the repr() method of the TypeEquivGroup struct returns the representative type of the equivalence group (that can be from the sort field of the group and if it is None, it returns the minimum index of the vars field of the group like TypeRef::Var(TypeVar(0usize)) etc.).
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
            TypeRef::Rational => TypeRef::Rational,
            TypeRef::Text => TypeRef::Text,
            TypeRef::Cloak(sub) => TypeRef::Cloak(self.refresh_type(sub).into()), // the into() method converts the TypeRef into a Box<TypeRef>.
            TypeRef::Seq(sub) => TypeRef::Seq(self.refresh_type(sub).into()),
            TypeRef::Set(sub) => TypeRef::Set(self.refresh_type(sub).into()),
            TypeRef::Map(key, val) => {
                TypeRef::Map(self.refresh_type(key).into(), self.refresh_type(val).into())
            }
            TypeRef::Error => TypeRef::Error,
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

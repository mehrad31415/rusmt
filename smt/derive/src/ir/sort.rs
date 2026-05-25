//! Intermediate representation (IR) types.

use crate::ir::ctxt::IRBuilder;
use crate::ir::index::UsrSortId;
use crate::ir::name::{SmtSortName, UsrSortName};
use crate::parser::infer::TypeRef;
use crate::parser::name::UsrTypeName;
use crate::parser::ty::{EnumVariant, TypeBody, TypeTag};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// A unique and complete reference to an SMT sort
/// In the IR, types have been fully resolved (e.g. type variables should have been unified and substituted with its representation).
/// This is basically the IR representation of a typetag in the parser
/// Tuple types have been treated as user-defined types with no name so they fall under the Sort::User variant (so there is no Pack variant)
/// The TypeRef::Parameter variant is converted to Sort::Uninterpreted
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub enum Sort {
    /// boolean
    Boolean,
    /// integer (unlimited precision)
    Integer,
    /// real
    Real,
    /// F32
    F32,
    /// F64
    F64,
    /// I32
    I32,
    /// I64
    I64,
    /// U32
    U32,
    /// U64
    U64,
    /// string
    String,
    /// SMT-sequence
    Seq(Box<Sort>),
    /// SMT-set
    Set(Box<Sort>),
    /// SMT-array
    Array(Box<Sort>, Box<Sort>),
    /// path-condition marker type
    Path,
    /// user-defined type (including pack-defined type tuple)
    User(UsrSortId),
    /// uninterpreted (used for type parameters that implement the SMT trait)
    Uninterpreted(SmtSortName),
}

impl Display for Sort {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Self::Seq(sub) => write!(f, "Seq<{sub}>"),
            Self::Set(sub) => write!(f, "Set<{sub}>"),
            Self::Array(key, val) => write!(f, "Array<{key}, {val}>"),
            Self::Path => write!(f, "Path"),
            Self::User(sid) => write!(f, "${sid}"),
            Self::Uninterpreted(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Debug)]
/// A helper enum representing a variant definition for algebraic data types (ADTs).
pub enum Variant {
    /// A unit variant like MyVariant   
    Unit,
    /// A tuple variant like MyVariant(1,2,3)
    Tuple(Vec<Sort>),
    /// A record variant like MyVariant{x:1,y:2,z:3}
    Record(BTreeMap<String, Sort>),
}

impl Display for Variant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => Ok(()),
            Self::Tuple(slots) => {
                let content: Vec<_> = slots.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", content.join(","))
            }
            Self::Record(fields) => {
                let content: Vec<_> = fields.iter().map(|(k, v)| format!("{k}:{v}")).collect();
                write!(f, "{{{}}}", content.join(","))
            }
        }
    }
}

#[derive(Debug)]
/// Complete definition of a user-defined data type.
/// It can be a tuple type, a record type, or an enum (ADT) with multiple variants.
pub enum DataType {
    /// A tuple type like MyStruct(i32, i32, i32) or (i32, i32, i32) depending on whether the tuple has a name or not
    Tuple(Vec<Sort>),
    /// A record type like MyStruct{x: i32, y: i32, z: i32}
    Record(BTreeMap<String, Sort>),
    /// An enum type like MyEnum::MyVariant{x:1,y:2,z:3}
    Enum(BTreeMap<String, Variant>),
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tuple(slots) => {
                let content: Vec<_> = slots.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", content.join(","))
            }
            Self::Record(fields) => {
                let content: Vec<_> = fields.iter().map(|(k, v)| format!("{k}:{v}")).collect();
                write!(f, "{{{}}}", content.join(","))
            }
            Self::Enum(variants) => {
                let content: Vec<_> = variants
                    .iter()
                    .map(|(k, v)| match v {
                        Variant::Unit => k.to_string(),
                        _ => format!("{k}:{v}"),
                    })
                    .collect();
                write!(f, "{{{}}}", content.join(" | "))
            }
        }
    }
}

/// A registry that tracks all data types (sorts) in the IR context.
///
/// It maintains two kinds of indices:
/// - **Named types:** user-defined types that have a name.
/// - **Unnamed tuple types:** tuple types that do not have an explicit name.
/// It also stores the complete definitions (DataType) associated with each unique sort.
#[derive(Default, Debug)]
pub struct TypeRegistry {
    /// a map from user-defined type and type parameters to sort id
    //   idx_named = {
    //       "Option" => {
    //         [Sort::Uninterpreted("Option_T")] => UsrSortId(0),  // generic definition
    //         [Sort::I32] => UsrSortId(3),                         // concrete instantiation
    //         [Sort::Boolean] => UsrSortId(5),                     // another instantiation
    //     }
    // }
    /// The outer map groups by type name, the inner map distinguishes instantiations.
    pub idx_named: BTreeMap<UsrSortName, BTreeMap<Vec<Sort>, UsrSortId>>,
    /// a map from unnamed type tuple (still user-defined) to sort id
    pub idx_tuple: BTreeMap<Vec<Sort>, UsrSortId>,
    /// the actual type definitions
    defs: BTreeMap<UsrSortId, DataType>,
}

impl TypeRegistry {
    /// Initialize an empty registry
    pub fn new() -> Self {
        Self {
            idx_named: BTreeMap::new(),
            idx_tuple: BTreeMap::new(),
            defs: BTreeMap::new(),
        }
    }

    /// Get the unique sort identifier given an optional type name and its instantiation.
    ///
    /// If `name` is `None`, the lookup is done in the anonymous tuple mapping and inst is the type of tuple elements.
    /// If `name` is provided, the lookup is done in the named types mapping and inst is the type parameters (generics).
    pub fn get_index(&self, name: Option<&UsrSortName>, inst: &[Sort]) -> Option<UsrSortId> {
        let idx = match name {
            None => self.idx_tuple.get(inst)?,
            Some(n) => self.idx_named.get(n)?.get(inst)?,
        };
        Some(*idx)
    }

    /// Create a new type signature (i.e. register a type instance) in the registry.
    pub(crate) fn create(&mut self, name: Option<UsrSortName>, inst: Vec<Sort>) -> UsrSortId {
        // This gives the number of registered types so far.
        let idx = UsrSortId {
            index: self.idx_tuple.len() + self.idx_named.values().map(|v| v.len()).sum::<usize>(),
        };
        // existing is the UsrSortId that is already registered
        let existing = match name {
            None => self.idx_tuple.insert(inst, idx),
            Some(n) => self.idx_named.entry(n).or_default().insert(inst, idx),
        };
        if existing.is_some() {
            panic!("type instance already registered {existing:#?}");
        }
        idx
    }

    /// Register a definition to the registry
    pub(crate) fn register_def(&mut self, idx: UsrSortId, def: DataType) {
        let existing = self.defs.insert(idx, def);
        if existing.is_some() {
            panic!("type definition already registered");
        }
    }

    /// Retrieve the data type definition
    pub fn retrieve(&self, idx: UsrSortId) -> &DataType {
        self.defs.get(&idx).expect("no such sort id")
    }

    /// Reverse lookup the sort id to type name and instantiation
    pub fn reverse_lookup(&self, idx: UsrSortId) -> (Option<&UsrSortName>, &[Sort]) {
        for (name, val) in &self.idx_named {
            for (inst, sid) in val {
                if *sid == idx {
                    return (Some(name), inst); // all ids are unique so we can return the first one we find knowing that the id is not present in the idx_tuple
                }
            }
        }
        for (inst, sid) in &self.idx_tuple {
            if *sid == idx {
                return (None, inst);
            }
        }
        panic!("no such sort id");
    }

    /// Retrieve all user-defined data types
    pub fn data_types(&self) -> &BTreeMap<UsrSortId, DataType> {
        &self.defs
    }
}

/// Sort-related functions in the IR builder
impl<'a, 'ctx: 'a> IRBuilder<'a, 'ctx> {
    /// Resolve a parser-level type reference into an IR-level sort.
    /// This function “unwraps” cloaked types, ensures that no type variables remain, and
    /// registers any user-defined types with the type registry if needed.
    pub fn resolve_type(&mut self, ty: &TypeRef) -> Sort {
        let sort = match ty {
            TypeRef::Var(_) => panic!("incomplete type inference in the IR builder"), // you cannot have type variables in the IR so all types should be resolved at the parser level that is why the unification is done.
            TypeRef::Boolean => Sort::Boolean,
            TypeRef::Integer => Sort::Integer,
            TypeRef::Real => Sort::Real,
            TypeRef::F32 => Sort::F32,
            TypeRef::F64 => Sort::F64,
            TypeRef::I32 => Sort::I32,
            TypeRef::I64 => Sort::I64,
            TypeRef::U32 => Sort::U32,
            TypeRef::U64 => Sort::U64,
            TypeRef::String => Sort::String,
            TypeRef::Cloak(sub) => self.resolve_type(sub.as_ref()),
            TypeRef::Seq(sub) => Sort::Seq(self.resolve_type(sub.as_ref()).into()),
            TypeRef::Set(sub) => Sort::Set(self.resolve_type(sub.as_ref()).into()),
            TypeRef::Array(key, val) => Sort::Array(
                self.resolve_type(key.as_ref()).into(),
                self.resolve_type(val.as_ref()).into(),
            ),
            TypeRef::Path => Sort::Path,
            // For user-defined types, register the type and get its unique sort id. inst is the type parameters
            TypeRef::User(name, inst) => Sort::User(self.register_type(Some(name), inst)),
            // For pack-defined types (anonymous tuple types), register them without a name.
            TypeRef::Pack(elems) => Sort::User(self.register_type(None, elems)),
            // basically for a TypeRef::Parameter(TypeParamName { ident : name.clone()}), it is Sort::Uninterpreted(SmtSortName { ident: name.clone() })
            TypeRef::Parameter(name) => self
                .ty_inst
                .get(name) // name is the type parameter from the definition (generics). This gives the corresponding type argument in the call in the Sort IR representation.
                .unwrap_or_else(|| panic!("no such type parameter {name}"))
                .clone(),
        };
        sort
    }

    /// Utility: resolve a vec of type refs
    pub fn resolve_type_ref_vec(&mut self, tys: &[TypeRef]) -> Vec<Sort> {
        tys.iter().map(|e| self.resolve_type(e)).collect()
    }

    /// Utility: resolve a vec of type tags (first the type tags are converted to type refs and then to sorts)
    fn resolve_type_tag_vec(&mut self, tys: &[TypeTag]) -> Vec<Sort> {
        tys.iter().map(|e| self.resolve_type(&e.into())).collect()
    }

    /// Utility: resolve a map of type tags
    fn resolve_type_tag_map(&mut self, tys: &BTreeMap<String, TypeTag>) -> BTreeMap<String, Sort> {
        tys.iter()
            .map(|(key, val)| (key.clone(), self.resolve_type(&val.into())))
            .collect()
    }

    /// Register a user-defined type in the IR.
    pub fn register_type(
        &mut self,
        ty_name: Option<&UsrTypeName>,
        ty_args: &[TypeRef], // for tuples these are the elements of the tuple and for user-defined types these are the type parameters (both in the definition)
    ) -> UsrSortId {
        // UsrSortName is the name of a user-defined type in the IR
        let name = ty_name.map(|n| n.into());
        // Resolve type arguments: TypeRef::Parameter(T) -> Sort::Uninterpreted(`typename`_T) (using ty_inst)
        // For type definitions, ty_inst should already have bindings created before calling this
        let ty_args = self.resolve_type_ref_vec(ty_args);

        // check if we have already processed the data type
        match self.ir.ty_registry.get_index(name.as_ref(), &ty_args) {
            None => (),
            Some(idx) => return idx,
        }

        // create the instance and get the index
        let idx = self.ir.ty_registry.create(name, ty_args.clone());

        // process the definition
        let def = match ty_name {
            None => {
                // construct a tuple without conversion needed
                DataType::Tuple(ty_args)
            }
            Some(n) => {
                // get the type definition from the ASTContext
                let def = self.ctxt.get_type(n);

                // prepare the builder for definition processing
                // create a mapping from the generics in the definition to the arguments
                let mut builder = self.derive(&def.head, ty_args);

                // parse the definition with the newly contextualized holder
                match &def.body {
                    TypeBody::Tuple(tuple) => {
                        DataType::Tuple(builder.resolve_type_tag_vec(&tuple.slots))
                    }
                    TypeBody::Record(record) => {
                        DataType::Record(builder.resolve_type_tag_map(&record.fields))
                    }
                    TypeBody::Enum(adt) => {
                        let mut variants = BTreeMap::new();
                        for (key, val) in &adt.variants {
                            let variant = match val {
                                EnumVariant::Unit => Variant::Unit,
                                EnumVariant::Tuple(tuple) => {
                                    Variant::Tuple(builder.resolve_type_tag_vec(&tuple.slots))
                                }
                                EnumVariant::Record(record) => {
                                    Variant::Record(builder.resolve_type_tag_map(&record.fields))
                                }
                            };
                            variants.insert(key.clone(), variant);
                        }
                        DataType::Enum(variants)
                    }
                }
            }
        };

        // register the definition
        self.ir.ty_registry.register_def(idx, def);

        // return the sort
        idx
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter}; // for implementing the Display trait on `Refinement` and `NamedItem` enums

use log::trace; // For logging very fine-grained information
use std::fs; // for filesystem operations; for reading a single file in process_file(&mut self, path: &Path) -> Result<()> method
use std::path::Path; // for path handling

use syn::{File, Ident, Item, ItemEnum, ItemFn, ItemMod, ItemStruct, Result, Stmt}; // Import syn crate types for parsing Rust code

use walkdir::WalkDir; // For walking through directories recursively (in  Context::new<P: AsRef<Path>>(path_input: P) -> Result<Self>)

use crate::parser::apply::{ApplyDatabase, Kind};
use crate::parser::attr::{ImplMark, Mark, SpecMark};
use crate::{bail_if_exists, bail_on, bail_on_with_note};
use crate::parser::expr::{Expr, ExprParserRoot, Op};
use crate::parser::func::{Axiom, FuncDef, FuncSig, ImplFuncDef, SpecFuncDef};
use crate::parser::generics::{Generics, GenericsInstPartial, Monomorphization, PartialInst};
use crate::parser::infer::{TIError, TypeRef, TypeUnifier};
use crate::parser::name::{AxiomName, UsrFuncName, UsrTypeName};
use crate::parser::ty::{TypeBody, TypeDef, TypeTag};

#[derive(Debug)]
/// SMT-marked type
pub enum MarkedType {
    Enum(ItemEnum),     // a wrapper for the syn enum type
    Struct(ItemStruct), // a wrapper for the syn struct type
}

impl MarkedType {
    /// Retrieve the name of the marked type
    pub fn name(&self) -> &Ident {
        match self {
            Self::Enum(item) => &item.ident,
            Self::Struct(item) => &item.ident,
        }
    }
}

#[derive(Debug)]
/// SMT-marked function as impl
/// #[smt_impl(method = my_method, specs = [spec1, spec2])]
/// fn my_fn() { ... }
pub struct MarkedImpl {
    item: ItemFn,   // the function that is marked as smt_impl (here fn my_fn { ... })
    mark: ImplMark, // the mark that is associated with the function
}

impl MarkedImpl {
    /// Retrieve the name of the function marked as impl
    pub fn name(&self) -> &Ident {
        &self.item.sig.ident
    }
}

#[derive(Debug)]
/// SMT-marked function as spec
/// #[smt_spec(method = my_method, impls = [impl1, impl2])]
/// fn my_fn() { ... }
pub struct MarkedSpec {
    item: ItemFn,   // the function that is marked as smt_spec (here fn my_fn { ... })
    mark: SpecMark, // the mark that is associated with the function
}

impl MarkedSpec {
    /// Retrieve the name of the function marked as spec
    pub fn name(&self) -> &Ident {
        &self.item.sig.ident
    }
}

#[derive(Debug)]
/// SMT-marked const as axiom
/// #[smt_axiom] // no additional attributes
pub struct MarkedAxiom {
    item: ItemFn, // the function that is marked as smt_axiom
}

impl MarkedAxiom {
    /// Retrieve the name of the function marked as axiom
    pub fn name(&self) -> &Ident {
        &self.item.sig.ident
    }
}

/// A helper enum to resolve naming conflicts in sanity check
enum NamedItem {
    Type,
    Impl,
    Spec,
    Axiom,
}

impl Display for NamedItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Impl => write!(f, "impl"),
            Self::Spec => write!(f, "spec"),
            Self::Axiom => write!(f, "axiom"),
        }
    }
}

/// A refinement relation
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Refinement {
    pub fn_impl: UsrFuncName,
    pub fn_spec: UsrFuncName,
}

impl Display for Refinement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ~> {}", self.fn_impl, self.fn_spec)
    }
}

#[derive(Debug)]
/// Context manager for holding marked items in rust code
pub struct Context {
    types: BTreeMap<UsrTypeName, MarkedType>,
    impls: BTreeMap<UsrFuncName, MarkedImpl>,
    specs: BTreeMap<UsrFuncName, MarkedSpec>,
    axioms: BTreeMap<AxiomName, MarkedAxiom>,
}

impl Context {
    /// Build a context for crate
    // `P` can be any type that has an internal reference to a Path. for example: &str, String, PathBuf, &Path ...
    pub fn new<P: AsRef<Path>>(path_input: P) -> Result<Self> {
        // create fresh context
        let mut ctxt = Self {
            types: BTreeMap::new(),
            impls: BTreeMap::new(),
            specs: BTreeMap::new(),
            axioms: BTreeMap::new(),
        };

        // scan over the code base
        let path_input = path_input.as_ref();

        // check whether the input is a file or a directory
        if path_input.is_file() {
            // process the file
            ctxt.process_file(path_input)?;
        } else {
            // create a WalkDir iterator starting from path_input and recursively walk through all subdirectories.
            // process all files in the directory
            for entry in WalkDir::new(path_input) {
                let entry = entry.unwrap_or_else(|err| {
                    panic!(
                        "unable to walk the directory {}: {}",
                        path_input.to_string_lossy(),
                        err
                    )
                });
                ctxt.process_file(entry.path())?;
            }
        }

        // post-collection checking
        ctxt.sanity_check()?;

        // return a collection of items as derivation context
        Ok(ctxt)
    }

    /// Process a single file
    fn process_file(&mut self, path: &Path) -> Result<()> {
        // if the path does not have a file extension or the file extension is not "rs", do nothing and return Ok(())
        if path.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(path).unwrap_or_else(|err| {
                panic!(
                    "unable to read source file {}: {}",
                    path.to_string_lossy(),
                    err
                )
            });
            // parse the content of the file and process the syntax if rs file
            self.process_syntax(syn::parse_file(&content)?)?;
        }
        Ok(())
    }

    /// Process the content (i.e., syntax) of the entire file
    pub fn process_syntax(&mut self, file: File) -> Result<()> {
        // ignore the sheband and inner attributes (e.g., #![allow(unused_imports)])
        let File {
            shebang: _,
            attrs: _,
            items,
        } = file;
        // process the items in the file
        self.process_items(items)
    }

    /// Process a list of items extracted from a file
    fn process_items(&mut self, items: Vec<Item>) -> Result<()> {
        // identify items marked with smt-related attributes
        for item in items {
            match item {
                // an enum type can be only marked with #[smt_type]
                Item::Enum(syntax) => match Mark::parse_attrs(&syntax.attrs)? {
                    None => continue, // the enum is not marked with any smt-related attributes
                    Some(Mark::Type) => self.add_type(MarkedType::Enum(syntax))?,
                    _ => bail_on!(
                        &syntax,
                        "invalid annotation\nonly #[smt_type] is allowed for enum {}",
                        &syntax.ident
                    ),
                },
                // a struct type can be only marked with #[smt_type]
                Item::Struct(syntax) => match Mark::parse_attrs(&syntax.attrs)? {
                    None => continue, // the struct is not marked with any smt-related attributes
                    Some(Mark::Type) => self.add_type(MarkedType::Struct(syntax))?,
                    _ => bail_on!(
                        &syntax,
                        "invalid annotation\nonly #[smt_type] is allowed for struct {}",
                        &syntax.ident
                    ),
                },
                // a function can be marked with #[smt_impl], #[smt_spec], or #[smt_axiom]
                Item::Fn(syntax) => match Mark::parse_attrs(&syntax.attrs)? {
                    None => continue, // the function is not marked with any smt-related attributes
                    Some(Mark::Impl(mark)) => self.add_impl(MarkedImpl { item: syntax, mark })?,
                    Some(Mark::Spec(mark)) => self.add_spec(MarkedSpec { item: syntax, mark })?,
                    Some(Mark::Axiom) => self.add_axiom(MarkedAxiom { item: syntax })?,
                    _ => bail_on!(
                        syntax,
                        "invalid annotation\n#[smt_type] is not allowed for fn"
                    ),
                },
                // recursively process the items in the module
                Item::Mod(syntax) => {
                    let ItemMod {
                        attrs: _,
                        vis: _,
                        unsafety,
                        mod_token: _,
                        ident: _,
                        content,
                        semi: _,
                    } = syntax;
                    // a module cannot be unsafe
                    bail_if_exists!(unsafety);
                    match content {
                        None => (),
                        Some((_, items)) => self.process_items(items)?,
                    }
                }
                // ignore other items
                _ => (),
            }
        }
        Ok(())
    }

    /// Add a type to the context
    fn add_type(&mut self, item: MarkedType) -> Result<()> {
        // the ident of the struct or enum is extracted
        // if the ident is a reserved keyword or underscore, return an error
        let name = item.name().try_into()?;

        // if the type name is already in the context, return an error (duplicated type name)
        if let Some(prev) = self.types.get(&name) {
            bail_on_with_note!(
                prev.name(),
                "previously defined here",
                item.name(),
                "duplicated type name"
            );
        }
        trace!("type found: {}", name);
        self.types.insert(name, item);
        Ok(())
    }

    /// Add an impl function to the context
    fn add_impl(&mut self, item: MarkedImpl) -> Result<()> {
        // check if the function name is a reserved keyword or underscore, return an error if so
        let name = item.name().try_into()?;

        // check for duplicated impl name, return an error if so
        if let Some(prev) = self.impls.get(&name) {
            bail_on_with_note!(
                prev.name(),
                "previously defined here",
                item.name(),
                "duplicated impl name"
            );
        }
        trace!("impl found: {}", name);
        self.impls.insert(name, item);
        Ok(())
    }

    /// Add a spec function to the context
    fn add_spec(&mut self, item: MarkedSpec) -> Result<()> {
        // check if the function name is a reserved keyword or underscore, return an error if so
        let name = item.name().try_into()?;

        // check for duplicated spec name, return an error if so
        if let Some(prev) = self.specs.get(&name) {
            bail_on_with_note!(
                prev.name(),
                "previously defined here",
                item.name(),
                "duplicated spec name"
            );
        }
        trace!("spec found: {}", name);
        self.specs.insert(name, item);
        Ok(())
    }

    /// Add an axiom to the context
    fn add_axiom(&mut self, item: MarkedAxiom) -> Result<()> {
        // check if the function name is a reserved keyword or underscore, return an error if so
        let name = item.name().try_into()?;

        // check for duplicated axiom name, return an error if so
        if let Some(prev) = self.axioms.get(&name) {
            bail_on_with_note!(
                prev.name(),
                "previously defined here",
                item.name(),
                "duplicated axiom name"
            );
        }
        trace!("axiom found: {}", name);
        self.axioms.insert(name, item);
        Ok(())
    }

    /// Check whether the marks declared are consistent or not
    /// This function checks if the names of types, impls, specs, and axioms are unique in the context.
    /// In other words, avoid naming conflict between different smt-related items.
    /// It also checks if the impl and spec pairs are valid, meaning that every spec target used inside an impl needs to be defined, and every impl target used inside a spec needs to be defined.
    pub fn sanity_check(&self) -> Result<()> {
        // create a map to store the names of types, impls, specs, and axioms
        let mut names = BTreeMap::new();

        // k.as_ref() & v.name() both return the name of the type, impl, spec, or axiom
        // The names are stored in the value as well so that they can be used to display the error message and the span location of the error
        // NamedItem::Type, NamedItem::Impl, NamedItem::Spec, NamedItem::Axiom are used to identify the type of the item where the name conflict occurs. They are defined because BTreeMap is homogeneous.
        for (key, (kind, ident)) in self
            .types
            .iter()
            .map(|(k, v)| (k.as_ref(), (NamedItem::Type, v.name())))
            .chain(
                self.impls
                    .iter()
                    .map(|(k, v)| (k.as_ref(), (NamedItem::Impl, v.name()))),
            )
            .chain(
                self.specs
                    .iter()
                    .map(|(k, v)| (k.as_ref(), (NamedItem::Spec, v.name()))),
            )
            .chain(
                self.axioms
                    .iter()
                    .map(|(k, v)| (k.as_ref(), (NamedItem::Axiom, v.name()))),
            )
        {
            if let Some((prev_kind, prev_ident)) = names.get(key) {
                bail_on_with_note!(
                    prev_ident,
                    "previously defined here",
                    ident,
                    "naming conflict between {} and {}",
                    kind,
                    prev_kind,
                );
            }
            names.insert(key, (kind, ident));
        }

        // impl and spec pairs
        for marked in self.impls.values() {
            let MarkedImpl { item, mark } = marked;
            for target in &mark.specs {
                if !self.specs.contains_key(target) {
                    bail_on!(item, "invalid spec target: {}", target);
                }
            }
        }
        for marked in self.specs.values() {
            let MarkedSpec { item, mark } = marked;
            for target in &mark.impls {
                if !self.impls.contains_key(target) {
                    bail_on!(item, "invalid impl target: {}", target);
                }
            }
        }
        Ok(())
    }

    /// Parse the generics declarations
    /// The only difference between Context and ContextWithGenerics is that ContextWithGenerics is the types field in the struct.
    /// In ContextWithGenerics, the list of the generics are stored as well.
    pub fn parse_generics(self) -> Result<ContextWithGenerics> {
        let mut types = BTreeMap::new();
        for (name, marked) in self.types {
            // from_marked_type will check that the generics are of the form <T: SM, U: SMT...> and return the parsed generics. No duplicate generics, extra trait bounds, lifetime bounds, or const bounds, etc. are allowed.
            let parsed = Generics::from_marked_type(&marked)?;
            types.insert(name, (parsed, marked));
        }

        Ok(ContextWithGenerics {
            types,
            impls: self.impls,
            specs: self.specs,
            axioms: self.axioms,
        })
    }
}

#[derive(Debug)]
/// Context manager after analyzing for generics
pub struct ContextWithGenerics {
    types: BTreeMap<UsrTypeName, (Generics, MarkedType)>,
    impls: BTreeMap<UsrFuncName, MarkedImpl>,
    specs: BTreeMap<UsrFuncName, MarkedSpec>,
    axioms: BTreeMap<AxiomName, MarkedAxiom>,
}

impl ContextWithGenerics {
    /// Get the generics declaration for a given type name
    pub fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.types.get(name).map(|(generics, _)| generics)
    }

    /// Parse types
    /// This function will convert a ContextWithGenerics struct into a ContextWithType struct.
    /// The only difference between the two structs is that the `types` field in the struct is different.
    /// In ContextWithGenerics, `types` is a BTreeMap<UsrTypeName, (Generics, MarkedType)> (mapping from `type name` to a tuple of the `generics` the type has and the `MarkedType` which declares whether the type is an enum or a struct).
    /// In ContextWithType, `types` is a BTreeMap<UsrTypeName, TypeDef> (mapping from type name to the TypeDef struct).
    /// The `TypeDef` struct is defined in the ty.rs file with Generics as the head and the TypeBody as the body. So, the same Generics are encapsulated in the TypeDef struct.
    /// The main difference thus is that `MarkedType` is converted into `TypeBody`.
    /// The `TypeBody` struct is defined in the ty.rs file as well. It is as follows:
    /// pub enum TypeBody {
    ///     Tuple(TypeTuple),
    ///     Record(TypeRecord),
    ///     Enum(TypeEnum),
    /// }
    /// If the MarkedType is an MarkedType::Enum(ItemEnum), then the TypeBody will be TypeBody::Enum(TypeEnum). So the only change is that the syn enum type is converted into the TypeEnum.
    /// If the MarkedType is an MarkedType::Struct(ItemStruct), then the TypeBody will be TypeBody::Record(TypeRecord) or TypeBody::Tuple(TypeTuple) depending on the fields of the struct.
    /// The process of converting MarkedType into TypeBody is done in the TypeBody::from_marked function.
    pub fn parse_types(self) -> Result<ContextWithType> {
        // map to store the new types
        let mut new_types = BTreeMap::new();

        // iterate over the types
        for (name, (generics, marked)) in &self.types {
            trace!("handling type: {}", name);
            // from_marked tries to convert the MarkedType into TypeBody
            let body = TypeBody::from_marked(&self, generics, marked)?;
            let def = TypeDef {
                head: generics.clone(),
                body,
            };
            trace!("type analyzed: {}", name);
            new_types.insert(name.clone(), def);
        }

        // re-packing
        let Self {
            types: _,
            impls,
            specs,
            axioms,
        } = self;

        Ok(ContextWithType {
            types: new_types,
            impls,
            specs,
            axioms,
        })
    }
}

#[derive(Debug)]
/// Context manager after type analysis is done
/// The finalized `types` is a container of all the types defined by the user which are marked with #[smt_type].
/// It contains a mapping from the type name to the TypeDef.
/// TypeDef is an encapsulation of the details of the type.
/// TypeDef has a header which lists all the generics used in the type.
/// TypeDef has a body of type TypeBody; where it contains the details about whether the type is an enum or struct.
/// Also the variants and/or fields are stored in the TypeBody.
pub struct ContextWithType {
    types: BTreeMap<UsrTypeName, TypeDef>,
    impls: BTreeMap<UsrFuncName, MarkedImpl>,
    specs: BTreeMap<UsrFuncName, MarkedSpec>,
    axioms: BTreeMap<AxiomName, MarkedAxiom>,
}

impl ContextWithType {
    /// Get the generics declaration for a type
    /// used for finding if a user defined type exists in the context
    /// If return value is None, then the type does not exist in the context
    pub fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.types.get(name).map(|def: &TypeDef| &def.head)
    }

    /// Parse function signatures
    /// At this point the types are already parsed and finalized. 
    /// The next step is to parse the function signatures. A function can be marked as an impl, spec, or axiom.
    /// parse_func_sigs converts a ContextWithType struct into a ContextWithSig if successful.
    /// The difference between the two structs is that the `impls`, `specs`, and `axioms` fields in the structs are different. 
    /// So only the `types` field is the same in both structs.
    /// In ContextWithType, `impls` is a BTreeMap<UsrFuncName, MarkedImpl>, `specs` is a BTreeMap<UsrFuncName, MarkedSpec>, and `axioms` is a BTreeMap<AxiomName, MarkedAxiom>.
    /// In ContextWithSig, `impls` is a BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)>, `specs` is a BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)>, and `axioms` is a BTreeMap<AxiomName, (FuncSig, Vec<Stmt>)>.
    /// So `MarkedImpl`, `MarkedSpec`, and `MarkedAxiom` all are converted into `(FuncSig, Vec<Stmt>)`.
    /// MarkedImpl for example encapsulates the item function, an optional method name, and the list of spec names. The FuncSig and Vec<Stmt> are the function signature and the function body respectively that are extracted from the item function. Same goes for MarkedSpec and MarkedAxiom.
    /// FuncSig is an ADT which encapsulates the generics, the input parameters, and the return type of the function signature.
    pub fn parse_func_sigs(self) -> Result<ContextWithSig> {
        // impl (extracting the function signature)
        let mut sig_impls = BTreeMap::new();
        for (name, marked) in &self.impls {
            let ItemFn {
                attrs: _,
                vis: _,
                sig,
                block: _, // handled later
            } = &marked.item; // function item

            trace!("handling impl sig: {}", name);
            // convert the function signature into a FuncSig struct (abstract syntax tree for function signatures)
            let parsed = FuncSig::from_sig(&self, sig)?;
            trace!("impl sig analyzed: {}", name);
            // we pass on the sig to bail_on the signature in case they are not compatible in spec and impl
            sig_impls.insert(name.clone(), (parsed, sig.clone()));
        }

        // spec (extracting the function signature)
        let mut sig_specs = BTreeMap::new();
        for (name, marked) in &self.specs {
            let ItemFn {
                attrs: _,
                vis: _,
                sig,
                block: _, // handled later
            } = &marked.item; // function item

            trace!("handling spec sig: {}", name);
            // convert the function signature into a FuncSig struct (abstract syntax tree for function signatures)
            let parsed = FuncSig::from_sig(&self, sig)?;
            trace!("spec sig analyzed: {}", name);
            // we pass on the sig to bail_on the signature in case they are not compatible in spec and impl
            sig_specs.insert(name.clone(), (parsed, sig.clone()));
        }

        // axiom (extracting the function signature and body)
        let mut unpacked_axioms: BTreeMap<AxiomName, (FuncSig, Vec<Stmt>)> = BTreeMap::new();
        for (name, marked) in &self.axioms {
            let ItemFn {
                attrs: _,
                vis: _,
                sig,
                block, // handled later
            } = &marked.item;

            trace!("handling axiom sig: {}", name);

            // convert the function signature into a FuncSig struct (abstract syntax tree for function signatures)
            let head = FuncSig::from_sig(&self, sig)?;
            // the axiom return type must be Boolean
            if !matches!(head.ret_ty, TypeTag::Boolean) {
                bail_on!(&sig, "expect Boolean as axiom return type");
            }
            // extract the body
            let body = block.stmts.clone();

            trace!("axiom analyzed sig: {}", name);
            unpacked_axioms.insert(name.clone(), (head, body));
        }

        // populate the databases
        let mut vc_db = BTreeSet::new(); // a database for verification conditions
        let mut fn_db = ApplyDatabase::with_intrinsics(); // a database intialized with the system functions

        for (name, (sig, raw)) in sig_impls.iter() {
            // this will never throw an error and the `expect` is only to unwrap the MarkedImpl
            let mark = &self.impls.get(name).expect("impl").mark;
            // check signature
            for spec_name in &mark.specs {
                // a spec used in the list of impl must be defined.
                // for example, if the impl is #[smt_impl(method = my_method, specs = [spec1, spec2])], then spec1 and spec2 must be defined like #[smt_spec(method = ..., impls = [...])] fn spec1() { ... } and #[smt_spec(method = ..., impls = [...])] fn spec2() { ... }
                // otherwise an error is thrown
                let (spec_sig, spec_raw) = sig_specs.get(spec_name).expect("spec");
                // if the signature of the spec and impl are not compatible, then an error is thrown
                if !spec_sig.is_compatible(sig) {
                    bail_on_with_note!(raw, "signature mismatch", spec_raw, "spec signature here");
                }
                // otherwise, the refinement relation is added to the vc_db
                // the refinement relation is a struct that contains the name of the impl and the name of the corresponding spec
                // As vc_db is a set, it will not add the same refinement relation twice
                vc_db.insert(Refinement {
                    fn_impl: name.clone(),
                    fn_spec: spec_name.clone(),
                });
            }

            // register to type db
            // register_user_func is a function that registers a user-defined function to `fn_db`, which is a database for functions
            // the function takes the name of the function, the signature of the function, the method name, and the kind of the function (impl or spec)
            match fn_db.register_user_func(name, sig, mark.method.as_ref(), Kind::Impl) {
                Ok(()) => (),
                Err(e) => bail_on!(raw, "{}", e),
            }
        }

        // sig_specs is a BTreeMap<UsrFuncName, (FuncSig, syn::Signature)> where the syn::Signature is the raw signature of the function. It contains all the specs that are defined in the context.
        for (name, (sig, raw)) in sig_specs.iter() {
            // this will never throw an error and the `expect` is only to unwrap the MarkedSpec
            // this is because the sig_specs is constructed from the self.specs and they both contain all the specs that are defined in the context
            let mark = &self.specs.get(name).expect("spec").mark;
            // check signature for each impl in the list of impls that is defined in the spec
            // for example, if the spec is #[smt_spec(method = my_method, impls = [impl1, impl2])], then impl1 and impl2 are checked
            for impl_name in &mark.impls {
                // retrieve the signature of the impl 
                // if an impl inside #[smt_spec(impls = [.....])] is not defined, then an error is thrown
                let (impl_sig, impl_raw) = sig_impls.get(impl_name).expect("impl");
                // if the signature of the impl and spec are not compatible, then an error is thrown
                if !impl_sig.is_compatible(sig) {
                    bail_on_with_note!(raw, "signature mismatch", impl_raw, "impl signature here");
                }
                // otherwise, the refinement relation is added to the vc_db
                // So if we have `spec1` and `impl1`, and spec1 is marked with #[smt_spec(method = my_method, impls = [impl1])], then the refinement relation is added to the vc_db. That is, there is no need to write #[smt_impl(method = my_method, specs = [spec1])] fn impl1() { ... } and we can write #[smt_impl(method = my_method)] fn impl1() { ... } instead. Although it is not necessary to write the impl, it is still possible to write it.
                vc_db.insert(Refinement {
                    fn_impl: impl_name.clone(),
                    fn_spec: name.clone(),
                });
            }

            // register to type db
            match fn_db.register_user_func(name, sig, mark.method.as_ref(), Kind::Spec) {
                Ok(()) => (),
                Err(e) => bail_on!(raw, "{}", e),
            }
        }
        trace!("databases constructed");

        // re-packing
        let Self {
            types,
            impls,
            specs,
            axioms: _,
        } = self;

        // extract the func sig and body from the impls
        let unpacked_impls: BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)> = impls
            .into_iter()
            .map(|(name, marked)| {
                // all the impls are stored in sig_impls
                let (sig, _) = sig_impls.remove(&name).unwrap();
                let stmts = marked.item.block.stmts;
                (name, (sig, stmts))
            })
            .collect();

        // extract the func sig and body from the specs
        let unpacked_specs: BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)> = specs
            .into_iter()
            .map(|(name, marked)| {
                // all the specs are stored in sig_specs
                let (sig, _) = sig_specs.remove(&name).unwrap();
                let stmts = marked.item.block.stmts;
                (name, (sig, stmts))
            })
            .collect();

        let ctxt = ContextWithSig {
            types,
            impls: unpacked_impls,
            specs: unpacked_specs,
            axioms: unpacked_axioms,
            vc_db,
            fn_db,
        };

        // done
        Ok(ctxt)
    }
}

#[derive(Debug)]
/// Context manager after type and function signature analysis is done
pub struct ContextWithSig {
    types: BTreeMap<UsrTypeName, TypeDef>,
    impls: BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)>,
    specs: BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)>,
    axioms: BTreeMap<AxiomName, (FuncSig, Vec<Stmt>)>,
    /// a database for verification conditions (i.e., impl and spec mapping)
    vc_db: BTreeSet<Refinement>,
    /// a database for all the marked functions (impl, spec). These can be standalone user-defined functions, system functions, and methods.
    pub fn_db: ApplyDatabase,
}

impl ContextWithSig {
    /// Get the generics declaration for a user type if that type has already been defined otherwise a None is returned
    pub fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.types.get(name).map(|def| &def.head)
    }

    /// Retrieve the type definition for a given type name
    pub fn get_type_def(&self, name: &UsrTypeName) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Parse function body
    ///
    /// This function will convert a ContextWithSig struct into a ContextWithFunc struct if successful.
    /// The `types` and `vc_db` fields are passed as is.
    /// The `fn_db` field is ignored as it is not needed anymore. It is only used in the expr module to look up the function names.
    /// The `impls`, `specs`, and `axioms` fields are converted from `(FuncSig, Vec<Stmt>)` to `ImplFuncDef`, `SpecFuncDef`, and `Axiom` respectively.
    /// The keys of the `impls`, `specs`, and `axioms` fields remain the same (UsrFuncName, UsrFuncName, and AxiomName).
    /// The `ImplFuncDef`, `SpecFuncDef`, and `Axiom` are defined in the func.rs file.
    /// The `ImplFuncDef` struct encapsulates the function signature and the function body as an expression tree. Same goes for `SpecFuncDef` and `Axiom`.
    /// The expression tree is built from the statements of the function body => ExprParserRoot::new(&self, Kind::Impl, sig).parse(stmts)?
    pub fn parse_func_body(self) -> Result<ContextWithFunc> {
        // unpack impls
        let mut unpacked_impls = BTreeMap::new();
        for (name, (sig, stmts)) in &self.impls {
            trace!("handling impl body: {}", name);
            // build the expression tree from the statements
            // this is called for each function in `rusmart`
            // The function body can only contain one expression statement (must be at the end) and the rest are Local let-binding statements
            // The function is pure (no side effects e.g. mutable references ... ).
            // `body` can be a block expression or unit expression. It is a block, if let bindings are present in the function body and they are stored accordingly.
            // The `body` expression encapsulates the `instruction` which in turn encapsulates the Op and its type
            // `Op` in `body` is the AST of the sole last expression in the function body and the type is the return type of the function
            let body = ExprParserRoot::new(&self, Kind::Impl, sig).parse(stmts)?;
            unpacked_impls.insert(
                name.clone(),
                ImplFuncDef {
                    head: sig.clone(),
                    body,
                },
            );
            trace!("impl body analyzed: {}", name);
        }

        // unpack specs
        let mut unpacked_specs = BTreeMap::new();
        for (name, (sig, stmts)) in &self.specs {
            trace!("handling spec body: {}", name);
            // check for uninterpreted function
            // only a spec can be uninterpreted
            let uninterpreted = Axiom::is_unimplemented(stmts)?;
            let body = if uninterpreted {
                None
            } else {
                // if the spec is interpreted, then build the expression tree from the statements
                // this is called for each function
                Some(ExprParserRoot::new(&self, Kind::Spec, sig).parse(stmts)?)
            };
            trace!("spec body analyzed: {}", name);
            unpacked_specs.insert(
                name.clone(),
                SpecFuncDef {
                    head: sig.clone(),
                    body,
                },
            );
        }

        // unpack axioms
        let mut unpacked_axioms = BTreeMap::new();
        for (name, (sig, stmts)) in &self.axioms {
            trace!("handling axiom body: {}", name);
            // build the expression tree from the statements
            // note axioms are treated as specs kind
            // this is called for each function
            let body = ExprParserRoot::new(&self, Kind::Spec, sig).parse(stmts)?;
            trace!("axiom body analyzed: {}", name);
            unpacked_axioms.insert(
                name.clone(),
                Axiom {
                    head: sig.clone(),
                    body,
                },
            );
        }

        // repacking
        // - fn_db is ignored as it is not needed anymore. It is only used in the expr module to look up the function names
        // - types and vc_db are passed as is
        // - impls, specs, and axioms are unpacked
        let Self {
            types,
            impls: _,
            specs: _,
            axioms: _,
            fn_db: _,
            vc_db,
        } = self;

        Ok(ContextWithFunc {
            types,
            impls: unpacked_impls,
            specs: unpacked_specs,
            axioms: unpacked_axioms,
            vc_db,
        })
    }
}

/// Context manager after type, signature, and expression conversion is done
pub struct ContextWithFunc {
    types: BTreeMap<UsrTypeName, TypeDef>,
    impls: BTreeMap<UsrFuncName, ImplFuncDef>,
    specs: BTreeMap<UsrFuncName, SpecFuncDef>,
    axioms: BTreeMap<AxiomName, Axiom>,
    vc_db: BTreeSet<Refinement>, // a database for verification conditions
}

impl ContextWithFunc {
    /// Finalize parsing context into AST
    /// This function will convert the ContextWithFunc struct into an ASTContext struct.
    ///
    /// The only difference between the two structs is that the `impls` & `specs` fields are merged into one single `funcs` field in the ASTContext struct.
    pub fn finalize(self) -> ASTContext {
        // unpack the context
        let Self {
            types,
            impls,
            specs,
            axioms,
            vc_db,
        } = self;

        // merge the functions
        let num_funcs = impls.len() + specs.len();
        let mut funcs = BTreeMap::new();

        for (name, def) in impls {
            funcs.insert(name, def.into()); // convert the ImplFuncDef into FuncDef
        }
        for (name, def) in specs {
            funcs.insert(name, def.into()); // convert the SpecFuncDef into FuncDef
        }

        // Theoretically, this is already in the sanity check phase so it should not happen
        if funcs.len() != num_funcs {
            panic!("duplicated function names");
        }

        // done
        ASTContext {
            types,
            funcs,
            axioms,
            vc_db,
        }
    }
}

/// Context after AST construction
pub struct ASTContext {
    types: BTreeMap<UsrTypeName, TypeDef>,
    funcs: BTreeMap<UsrFuncName, FuncDef>,
    axioms: BTreeMap<AxiomName, Axiom>,
    vc_db: BTreeSet<Refinement>,
}

impl ASTContext {
    /// Enumerate over the verification conditions
    ///
    /// Each of the verification conditions is a refinement relation between an impl and a spec.
    /// Each of them along with the final ASTContext are passed to build the Intermediary Representation (IR).
    pub fn refinements(&self) -> impl Iterator<Item = &Refinement> {
        self.vc_db.iter()
    }

    /// Get the type definition for a given type name
    pub fn get_type(&self, name: &UsrTypeName) -> &TypeDef {
        self.types
            .get(name)
            .unwrap_or_else(|| panic!("type {} does not exist", name))
    }

    /// Get the function definition for a given function name
    pub fn get_func(&self, name: &UsrFuncName) -> &FuncDef {
        self.funcs
            .get(name)
            .unwrap_or_else(|| panic!("fn {} does not exist", name))
    }

    /// Get the axiom definition for a given axiom name
    pub fn get_axiom(&self, name: &AxiomName) -> &Axiom {
        self.axioms
            .get(name)
            .unwrap_or_else(|| panic!("axiom {} does not exist", name))
    }

    /// Check whether this axiom is relevant
    /// This function is only used in IRBuilder::build in the ir/ctxt.rs file.
    /// IRBuilder::build takes the ASTContext and the refinement relation between an impl and a spec.
    /// For each refinement relation, IRBuilder::build builds the IRContext.
    /// IRBuilder::build is only used in lib.rs in the pipeline function.
    pub fn probe_related_axioms(
        &self,
        name: &UsrFuncName,
        inst: &[TypeTag],
    ) -> BTreeMap<AxiomName, BTreeSet<Monomorphization>> {
        // instantiate the axioms
        let mut related: BTreeMap<AxiomName, BTreeSet<Monomorphization>> = BTreeMap::new();

        // for each axiom, check whether the axiom is relevant
        for (key, axiom) in &self.axioms {
            let mut inst_candidates = vec![];

            // take the body of the axiom (the expression tree)
            let mut body = axiom.body.clone();

            // traverse the whole expression tree
            // visit with a post-order traversal function
            body.visit(&mut |_| Ok(()), &mut |_| Ok(()), &mut |e| {
                // check whether this expr involves the target procedure call

                // first retrieve the operation of the expression
                let op = match e {
                    Expr::Unit(inst) => inst.op.as_ref(),
                    Expr::Block { lets: _, body } => body.op.as_ref(),
                };
                if let Op::Procedure {
                    name: proc_name,
                    inst: proc_inst,
                    args: _,
                } = op
                {
                    // if the procedure name is the same as the target function name, then add the instantiation to the candidates
                    if proc_name == name {
                        inst_candidates.push(
                            proc_inst
                                .iter()
                                .map(|e| e.reverse().expect("expression type complete"))
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                Ok(())
            })
            .unwrap_or_else(|e| panic!("unexpected expression visitation error: {}", e));

            // check relevance of their instantiations
            let inst_ref: Vec<TypeRef> = inst.iter().map(|t| t.into()).collect();
            for candidate in inst_candidates {
                if candidate.len() != inst_ref.len() {
                    panic!("number of type arguments mismatch: {}", name);
                }

                // prepare type variables
                let mut unifier = TypeUnifier::new();
                let generics = GenericsInstPartial::new_without_args(&axiom.head.generics)
                    .complete(&mut unifier);

                // refresh the candidates by replacing type parameters as type variables
                let mut parametric = vec![];
                for tag in candidate {
                    match generics.instantiate(&tag) {
                        None => panic!("uninstantiated axiom type: {}", tag),
                        Some(t) => parametric.push(t),
                    }
                }

                // check whether the type unifies
                let mut unifies = true;
                for (lhs, rhs) in parametric.iter().zip(inst_ref.iter()) {
                    match unifier.unify(lhs, rhs) {
                        Ok(None) => {
                            unifies = false;
                            break;
                        }
                        Ok(Some(_)) => (),
                        Err(TIError::CyclicUnification) => {
                            panic!("type unification error: cyclic type unification")
                        }
                    }
                }
                if !unifies {
                    continue;
                }

                // save the unification result
                let mut axiom_inst = vec![];
                for ty in generics.vec() {
                    let refreshed = unifier.refresh_type(&ty);
                    let inst_mark = match refreshed.reverse() {
                        None => {
                            let var = match refreshed {
                                TypeRef::Var(v) => v,
                                _ => panic!("type parameter must be either assigned or variadic"),
                            };
                            let tp_name = match generics.reverse(&var) {
                                None => panic!("unable to find the origin of type var {}", var),
                                Some((n, _)) => n.clone(),
                            };
                            PartialInst::Unassigned(tp_name)
                        }
                        Some(tag) => PartialInst::Assigned(tag),
                    };
                    axiom_inst.push(inst_mark);
                }
                related
                    .entry(key.clone())
                    .or_default()
                    .insert(Monomorphization { args: axiom_inst });
            }
        }

        // done
        related
    }
}

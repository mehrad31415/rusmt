//! Context manager for holding marked items in rust code; the main logic to parse is in this file.

use crate::parser::apply::ApplyDatabase;
use crate::parser::attr::Mark;
use crate::parser::expr::ExprParserRoot;
use crate::parser::func::{FuncDef, FuncSig};
use crate::parser::generics::Generics;
use crate::parser::name::{UsrFuncName, UsrTypeName};
use crate::parser::ty::{TypeBody, TypeDef};
use crate::{bail_if_exists, bail_on, bail_on_with_note};
use core::panic;
use log::trace;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use syn::ItemMod;
use syn::{File, Ident, Item, ItemEnum, ItemFn, ItemStruct, Result, Stmt};
use walkdir::WalkDir;

#[derive(Debug)]
/// SMT-marked type
pub enum MarkedType {
    /// An enum type.
    Enum(ItemEnum),
    /// A struct type.
    Struct(ItemStruct),
}

impl MarkedType {
    /// Get the name of the type.
    pub fn name(&self) -> &Ident {
        match self {
            Self::Enum(item) => &item.ident,
            Self::Struct(item) => &item.ident,
        }
    }
}

#[derive(Debug, Clone)]
/// SMT-marked function
pub struct MarkedFunc {
    item: ItemFn, // the function item
}

impl MarkedFunc {
    /// Get the name of the function.
    pub fn name(&self) -> &Ident {
        &self.item.sig.ident
    }
}

/// A helper enum to resolve naming conflicts in sanity check
enum NamedItem {
    Type,
    Func,
}

impl Display for NamedItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Func => write!(f, "function"),
        }
    }
}

#[derive(Debug)]
/// Context manager for holding marked items in rust code
pub struct Context {
    types: BTreeMap<UsrTypeName, MarkedType>,
    funcs: BTreeMap<UsrFuncName, MarkedFunc>,
}

impl Context {
    /// Build a context for crate
    // `P` can be any type that has an internal reference to a Path. for example: &str, String, PathBuf, &Path ...
    pub fn new<P: AsRef<Path>>(path_input: P) -> Result<Self> {
        // create fresh context
        let mut ctxt = Self {
            types: BTreeMap::new(),
            funcs: BTreeMap::new(),
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
        if path.extension().is_some_and(|ext| ext == "rs") {
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
            frontmatter: _,
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
                // a function can be marked with #[smt_fn]
                Item::Fn(syntax) => match Mark::parse_attrs(&syntax.attrs)? {
                    None => continue, // the function is not marked with any smt-related attributes
                    Some(Mark::Func) => self.add_func(MarkedFunc { item: syntax })?,
                    _ => bail_on!(
                        syntax.clone(),
                        "invalid annotation\n#only #[smt_fn] is allowed for fn {}",
                        &syntax.sig.ident
                    ),
                },
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
                    bail_if_exists!(unsafety);
                    match content {
                        None => (),
                        Some((brace, _)) => panic!("unsupported module content: {:?}", brace),
                    }
                }
                // a use item is ignored
                Item::Use(_) => (),
                // throw an error for any other item
                _other => panic!("unsupported item in smt derive: {:?}", _other),
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
        trace!("type found: {name}");
        self.types.insert(name, item);
        Ok(())
    }

    /// Add a function to the context
    fn add_func(&mut self, item: MarkedFunc) -> Result<()> {
        // check if the function name is a reserved keyword or underscore, return an error if so
        let name = item.name().try_into()?;

        // check for duplicated func name, return an error if so
        if let Some(prev) = self.funcs.get(&name) {
            bail_on_with_note!(
                prev.name(),
                "previously defined here",
                item.name(),
                "duplicated func name"
            );
        }
        trace!("func found: {name}");
        self.funcs.insert(name, item.clone());
        Ok(())
    }

    /// Check whether the marks declared are consistent or not
    pub fn sanity_check(&self) -> Result<()> {
        let mut names = BTreeMap::new();

        for (key, (kind, ident)) in self
            .types
            .iter()
            .map(|(k, v)| (k.as_ref(), (NamedItem::Type, v.name())))
            .chain(
                self.funcs
                    .iter()
                    .map(|(k, v)| (k.as_ref(), (NamedItem::Func, v.name()))),
            )
        {
            if let Some((_prev_kind, prev_ident)) = names.get(key) {
                bail_on_with_note!(
                    prev_ident,
                    "previously defined here",
                    ident,
                    "naming conflict",
                );
            }
            names.insert(key, (kind, ident));
        }

        Ok(())
    }

    /// Parse the generics declarations of types
    pub fn parse_generics(self) -> Result<ContextWithGenerics> {
        let mut types = BTreeMap::new();
        for (name, marked) in self.types {
            // from_marked_type will check that the generics are of the form <T: SMT, U: SMT...> and return the parsed generics. No duplicate generics, extra trait bounds, lifetime bounds, or const bounds, etc. are allowed.
            let parsed = Generics::from_marked_type(&marked)?;
            types.insert(name, (parsed, marked));
        }

        Ok(ContextWithGenerics {
            types,
            funcs: self.funcs,
        })
    }
}

#[derive(Debug)]
/// Context manager after analyzing generics
pub struct ContextWithGenerics {
    types: BTreeMap<UsrTypeName, (Generics, MarkedType)>,
    funcs: BTreeMap<UsrFuncName, MarkedFunc>,
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
            trace!("handling type: {name}");
            // from_marked tries to convert the MarkedType into TypeBody
            let body = TypeBody::from_marked(&self, generics, marked)?;
            let def = TypeDef {
                head: generics.clone(),
                body,
            };
            trace!("type analyzed: {name}");
            new_types.insert(name.clone(), def);
        }

        // re-packing
        let Self { types: _, funcs } = self;

        Ok(ContextWithType {
            types: new_types,
            funcs,
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
    funcs: BTreeMap<UsrFuncName, MarkedFunc>,
}

impl ContextWithType {
    /// Get the generics declaration for a type - used for finding if a user defined type exists in the context
    /// If return value is None, then the type does not exist in the context
    pub fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.types.get(name).map(|def: &TypeDef| &def.head)
    }

    /// Parse function signatures
    pub fn parse_func_sigs(self) -> Result<ContextWithSig> {
        // extracting the function signature
        let mut sig_funcs = BTreeMap::new();
        for (name, marked) in &self.funcs {
            let ItemFn {
                attrs: _,
                modifiers: _,
                vis: _,
                sig,
                block: _, // handled later
            } = &marked.item; // function item

            trace!("handling func sig: {name}");
            // convert the function signature into a FuncSig struct (abstract syntax tree for function signatures)
            let parsed = FuncSig::from_sig(&self, sig)?;
            trace!("func sig analyzed: {name}");
            let stmts = marked.item.block.stmts.clone();
            sig_funcs.insert(name.clone(), (parsed, stmts));
        }

        let mut fn_db = ApplyDatabase::with_intrinsics(); // a database intialized with the system functions

        for (name, (sig, _)) in sig_funcs.iter() {
            // register_user_func is a function that registers a user-defined function to `fn_db`, which is a database for functions
            // the function takes the name of the function and the signature of the function
            match fn_db.register_user_func(name, sig) {
                Ok(()) => (),
                Err(e) => bail_on!(name.to_string(), "{}", e),
            }
        }

        trace!("databases constructed");

        // re-packing
        let Self { types, funcs: _ } = self;

        let ctxt = ContextWithSig {
            types,
            funcs: sig_funcs,
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
    funcs: BTreeMap<UsrFuncName, (FuncSig, Vec<Stmt>)>, // all functions with their signatures and bodies
    /// The database of functions.
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
    /// The `types` are passed as is.
    /// The `fn_db` field is ignored as it is not needed anymore. It is only used in the expr module to look up the function names.
    /// The `FuncDef` struct encapsulates the function signature and the function body as an expression tree.
    pub fn parse_func_body(self) -> Result<ContextWithFunc> {
        // unpack function bodies
        let mut unpacked_funcs = BTreeMap::new();
        for (name, (sig, stmts)) in &self.funcs {
            trace!("handling function body: {name}");
            // build the expression tree from the statements
            // this is called for each function in `rusmt`
            // The function body can only contain one expression statement (must be at the end) and the rest are Local let-binding statements
            // The function is pure (no side effects e.g. mutable references ... ).
            // `body` can be a block expression or unit expression. It is a block, if let bindings are present in the function body and they are stored accordingly.
            // The `body` expression encapsulates the `instruction` which in turn encapsulates the Op and its type
            // `Op` in `body` is the AST of the sole last expression in the function body and the type is the return type of the function
            let body = ExprParserRoot::new(&self, sig).parse(stmts)?;
            unpacked_funcs.insert(
                name.clone(),
                FuncDef {
                    head: sig.clone(),
                    body,
                },
            );
            trace!("function body analyzed: {name}");
        }

        // repacking
        // fn_db is ignored as it is not needed anymore. It is only used in the expr module to look up the function names
        let Self {
            types,
            funcs: _,
            fn_db: _,
        } = self;

        Ok(ContextWithFunc {
            types,
            funcs: unpacked_funcs,
        })
    }
}

#[derive(Debug)]
/// Context manager after type, signature, and expression conversion is done
pub struct ContextWithFunc {
    /// The registered types in the context (types marked with #[smt_type])
    pub types: BTreeMap<UsrTypeName, TypeDef>,
    /// The registered functions in the context (functions marked with #[smt_fn])
    pub funcs: BTreeMap<UsrFuncName, FuncDef>,
}

impl ContextWithFunc {
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
}

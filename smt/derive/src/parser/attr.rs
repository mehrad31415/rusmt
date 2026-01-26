//! The modules provides `Mark` enum which represents the SMT-related marking attributes.

use crate::bail_on;
use syn::{AttrStyle, Attribute, Meta, MetaList, MetaNameValue, Path, Result};
/// SMT-related marking
pub enum Mark {
    Type,
    Func,
}

impl Mark {
    /// Convert a path to a mark
    pub fn parse_path(path: &Path) -> Option<Self> {
        match path
            .get_ident()
            .expect("path is not an identifier")
            .to_string()
            .as_str()
        {
            "smt_type" => Some(Self::Type),
            "smt_fn" => Some(Self::Func),
            _ => None,
        }
    }

    /// Test whether this attribute represents a mark
    fn parse_attr(attr: &Attribute) -> Result<Option<Self>> {
        let Attribute {
            pound_token: _, // The # token before the attribute like #[my_attr]
            style, // The style of the attribute: outer or inner #[my_attr] is outer or #![my_attr] for inner.
            // The outer style is used for attributes that apply to the item they are attached to. Affects only the specific item (like a function or struct) to which it is attached. It will not apply globally when used at the top of a module or crate.
            // The inner style is used for attributes that apply to items within the item they are attached to. Affects the item and all items within it. It will apply globally when used at the top of a module or crate.
            bracket_token: _, // The brackets around the attribute like #[my_attr]
            meta,             // The content of the attribute
        } = attr;

        // early filtering (only outer attributes are considered)
        if !matches!(style, AttrStyle::Outer) {
            return Ok(None);
        }

        let mark = match meta {
            // rust does not allow multiple attributes in a single attribute like #[my_attr1, my_attr2]. Instead, it should be #[my_attr1] #[my_attr2]...
            // Path like `test` in #[test]
            // If it is a path, we parse it for Annotations.
            // Basically the parse_path checks if the path is an identifier (not a path with leading colons, only one segment, and no arguments).
            // If it is an identifier, the acceptable values are "smt_type", "smt_fn".
            Meta::Path(path) => match Mark::parse_path(path) {
                None => return Ok(None),
                Some(Mark::Type) => Self::Type,
                Some(Mark::Func) => Self::Func,
            },

            // A meta list is like the `derive(Copy)` in `#[derive(Copy)]`
            Meta::List(MetaList {
                path,         // path in the above example is `derive`
                delimiter: _, // delimiter in the above example is Parenthesis
                tokens: _,    // tokens in the above example are `Copy`
            }) => match Mark::parse_path(path) {
                None => return Ok(None),
                Some(_) => {
                    bail_on!(attr, "unexpected list")
                }
            },
            // A name-value meta is like the `path = "..."` in `#[path = "sys/windows.rs"]`.
            Meta::NameValue(MetaNameValue {
                path,        // path in the above example is `path`
                eq_token: _, // equal sign in the above example is `=`
                value: _,    // value in the above example is `"sys/windows.rs"`
            }) => match Mark::parse_path(path) {
                None => return Ok(None),
                Some(_) => bail_on!(attr, "unexpected dict"), // Name-value pairs are not expected for smt_type, smt_fn
            },
        };

        Ok(Some(mark))
    }

    /// This function is used to see whether the item is marked with any smt-related attributes (smt_type, smt_fn).
    pub fn parse_attrs(attrs: &[Attribute]) -> Result<Option<Self>> {
        let mut mark = None;
        for attr in attrs {
            match Self::parse_attr(attr)? {
                None => continue,
                Some(parsed) => {
                    if mark.is_some() {
                        bail_on!(attr, "multiple marks specified"); // if in one of the attributes there exists smt_type, smt_fn, no other attributes containing any of these should exist.
                    }
                    mark = Some(parsed);
                }
            }
        }
        Ok(mark)
    }
}

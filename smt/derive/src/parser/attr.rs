//! The modules provides `Mark` enum which represents the SMT-related marking attributes.

use crate::parser::name::UsrFuncName;
use crate::{bail_if_missing, bail_on};
use proc_macro2::{Ident, TokenStream, TokenTree};
use std::collections::BTreeMap;
use syn::{AttrStyle, Attribute, MacroDelimiter, Meta, MetaList, MetaNameValue, Path, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Allowed annotation
enum Annotation {
    Type,
    Func,
}

// the get_ident(&Path) analyzes the path; if the path is not an ident, it will return None.
// a path is an identifier if it does not have leading colons, only has one segment, and has no arguments.
// if the path is an identifier, the acceptable values are "smt_type", "smt_fn".
impl Annotation {
    /// Convert a path to an annotation
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
}

/// Annotation value
#[derive(Debug, Clone, PartialEq, Eq)]
enum MetaValue {
    One(Ident),
}

#[derive(Debug, Clone)]
/// A mark for an annotated function
pub struct FuncMark {
    /// whether to derive a receiver-style method for this function
    pub method: Option<UsrFuncName>,
}

/// SMT-related marking
pub enum Mark {
    Type,
    Func(FuncMark),
}

impl Mark {
    /// Parse a key-value mapping from a token stream
    ///
    /// This function takes a `TokenStream` and attempts to parse it into a
    /// `BTreeMap<String, MetaValue>`. The expected format is a series of
    /// key-value pairs, where keys are identifiers and values are a single identifier.
    /// Key-value pairs are separated by commas.
    ///
    /// # Arguments
    ///
    /// * `stream` - A reference to a `TokenStream` containing the key-value pairs.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<String, MetaValue>>` - A map of keys to values, or an error if parsing fails.
    fn parse_dict(stream: &TokenStream) -> Result<BTreeMap<String, MetaValue>> {
        let mut store = BTreeMap::new(); // Stores the parsed key-value pairs
        let mut iter = stream.clone().into_iter(); // Creates an ownership iterator over the token stream

        // this will be a None value if the stream is empty
        let mut cursor = iter.next(); // Current token

        while cursor.is_some() {
            // extract key
            let token = bail_if_missing!(cursor.as_ref(), stream, "key"); // this will never lead to a compile error because cursor is checked to be Some at the beginning of the loop.

            // Extract the key as an identifier
            // A key must be an identifier for example in #[my_attr(key = value)]
            // #[my_attr(1 = value)] leads to an error
            let key = match token {
                TokenTree::Ident(ident) => ident.to_string(),
                _ => bail_on!(token, "key not an identifier"), // return an error if the token is not an identifier
            };
            // Check for duplicated keys and return an error if found
            // for example #[my_attr(key1 = value1, key1 = value2)] leads to an error
            if store.contains_key(&key) {
                bail_on!(token, "duplicated key");
            }

            // Extract equal sign (the format is key = value)
            // if the next token is empty (None), it will be caught in the next iteration
            // the error message will be "expect =" for the stream.
            let token = bail_if_missing!(iter.next(), stream, "=");
            // Check if the token is an equal sign
            match &token {
                TokenTree::Punct(punct) if punct.as_char() == '=' => (),
                _ => bail_on!(token, "expect ="), // return an error if the token is not an equal sign
            }

            // Extract value (the format is key = value)
            // if the next token is empty (None), it will be caught in the next iteration
            let token = bail_if_missing!(iter.next(), stream, "val");
            // Extract the value as an identifier
            let val = match token {
                // Single identifier value
                TokenTree::Ident(ident) => MetaValue::One(ident),
                _ => bail_on!(token, "expect identifier as value"),
            };

            // add to the key-value store
            store.insert(key, val);

            // check for more tokens
            cursor = iter.next();
            // Skip commas between key-value pairs
            if matches!(cursor.as_ref(), Some(TokenTree::Punct(punct)) if punct.as_char() == ',') {
                cursor = iter.next();
            } else if cursor.is_some() {
                // Return an error if a comma is missing between key-value pairs
                bail_on!(cursor, "expect comma between key-value pairs");
            }
        }

        Ok(store)
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
            Meta::Path(path) => match Annotation::parse_path(path) {
                None => return Ok(None),
                Some(Annotation::Type) => Self::Type,
                Some(Annotation::Func) => Self::Func(FuncMark { method: None }),
            },
            // A meta list is like the `derive(Copy)` in `#[derive(Copy)]`
            Meta::List(MetaList {
                path,      // path in the above example is `derive`
                delimiter, // delimiter in the above example is Parenthesis
                tokens,    // tokens in the above example are `Copy`
            }) => {
                match Annotation::parse_path(path) {
                    None => return Ok(None),
                    Some(Annotation::Type) => {
                        bail_on!(
                            attr,
                            "unexpected list\nfor smt_type, no attributes are expected"
                        )
                    }
                    Some(Annotation::Func) => {
                        // MacroDelimiter is a grouping token that surrounds a macro body: `m!(...)` or `m!{...}` or `m![...]`
                        // If the delimiter is not Parenthesis (it is a brace or bracket), it will return an error.
                        if !matches!(delimiter, MacroDelimiter::Paren(_)) {
                            bail_on!(attr, "not a parenthesis-enclosed list");
                        }

                        let mut store = Self::parse_dict(tokens)?;
                        // #[smt_fn(method = my_method)] is valid
                        let method = match store.remove("method") {
                            None => None,
                            // if the ident is not a reserved keyword, it will be parsed to a UsrFuncName
                            // reserved keywords are: (SMT), (Boolean, Integer, Rational, Text, Cloak, Seq, Set, Map, Error), (eq, ne), (clone, default), (from, into), (exists, forall, choose), _.
                            Some(MetaValue::One(item)) => Some((&item).try_into()?),
                        };

                        if !store.is_empty() {
                            bail_on!(tokens, "unrecognized entries"); // no other entries are expected except `method`
                        }
                        Self::Func(FuncMark { method })
                    }
                }
            }
            // A name-value meta is like the `path = "..."` in `#[path = "sys/windows.rs"]`.
            Meta::NameValue(MetaNameValue {
                path,        // path in the above example is `path`
                eq_token: _, // equal sign in the above example is `=`
                value: _,    // value in the above example is `"sys/windows.rs"`
            }) => match Annotation::parse_path(path) {
                None => return Ok(None),
                Some(_) => bail_on!(attr, "unexpected dict"), // Name-value pairs are not expected for smt_type, smt_fn
            },
        };

        Ok(Some(mark))
    }

    /// Extract a mark, if any, from the list of attributes
    /// The whole purpose of this function is to return an error if multiple marks are specified.
    ///
    /// # Arguments
    ///
    /// * `attrs` - A reference to a list of attributes.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Self>>`
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

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use quote::quote;
    use syn::{Path, parse_quote};

    #[test]
    #[should_panic(expected = "path is not an identifier")]
    // an identifier path, does not have leading colons, only has one segment, and contains no arguments.
    fn test_parse_path_one() {
        let input: Path = parse_quote! { ::smt_type };
        let parsed = Annotation::parse_path(&input);

        // error if the path is not an identifier
        assert!(parsed.is_none());
    }

    /// Helper function to create an identifier with a given name.
    fn ident(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    #[test]
    fn test_parse_single_key_value() {
        // Test parsing a single key-value pair: key = value
        let tokens = quote! { key = value };
        let result = Mark::parse_dict(&tokens).expect("Failed to parse key-value pair");

        assert_eq!(result.len(), 1);
        match result.get("key") {
            Some(MetaValue::One(id)) => assert_eq!(id, &ident("value")),
            _ => panic!("Expected MetaValue::One with identifier 'value'"),
        }
    }

    #[test]
    fn test_parse_multiple_key_values() {
        // Test parsing multiple key-value pairs: key1 = value1, key2 = value2
        let tokens = quote! { key1 = value1,key2 = value2 };
        let result = Mark::parse_dict(&tokens).unwrap();

        assert_eq!(result.len(), 2);
        match result.get("key1") {
            Some(MetaValue::One(id)) => assert_eq!(id, &ident("value1")),
            _ => panic!("Expected MetaValue::One with identifier 'value1'"),
        }
        match result.get("key2") {
            Some(MetaValue::One(id)) => assert_eq!(id, &ident("value2")),
            _ => panic!("Expected MetaValue::One with identifier 'value2'"),
        }
    }

    #[test]
    fn test_parse_mixed_values() {
        let tokens = quote! { key1 = value1 };
        let result = Mark::parse_dict(&tokens).unwrap();

        assert_eq!(result.len(), 2);
        // Check key1
        match result.get("key1") {
            Some(MetaValue::One(id)) => assert_eq!(id, &ident("value1")),
            _ => panic!("Expected MetaValue::One with identifier 'value1'"),
        }
    }

    #[test]
    fn test_parse_empty_input() {
        // Test parsing an empty input
        let tokens = quote! {};
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_ok_and(|f| f.is_empty())); // store is empty
    }

    #[test]
    fn test_not_a_key() {
        // Test parsing when the key is not an identifier: 1 = value
        // this gets invoked here: let key = match token { TokenTree::Ident(ident) => ident.to_string(), _ => bail_on!(token, "key not an identifier") };
        let tokens = quote! { 1 = value };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert!(result.contains("key not an identifier"));
    }

    #[test]
    fn test_parse_duplicate_keys() {
        // Test parsing with duplicate keys: key = value1, key = value2
        // this gets invoked here: if store.contains_key(&key) { bail_on!(token, "duplicated key"); }
        let tokens = quote! { key = value1, key = value2 };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert!(result.contains("duplicated key"));
    }

    #[test]
    fn test_no_tokens_after_key() {
        // Test parsing when there are no tokens after the key: key
        // this gets invoked here: let token = bail_if_missing!(iter.next(), stream, "=");
        let tokens = quote! { key };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(result, "expect =\nkey");
    }

    #[test]
    fn test_parse_invalid_syntax() {
        // Test parsing invalid syntax: key value1 value2
        // this gets invoked here: bail_on!(token, "expect =")
        let tokens = quote! { key value1 value2 };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert!(result.contains("expect ="));
    }

    #[test]
    fn test_parse_missing_equal_sign() {
        // Test parsing when the equal sign is missing: key value
        // this gets invoked here: bail_on!(token, "expect =")
        let tokens = quote! { key value };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert!(result.contains("expect ="));
    }

    #[test]
    fn test_parse_missing_value() {
        // Test parsing when the value is missing: key =
        // this gets invoked here: let token = bail_if_missing!(iter.next(), stream, "val");
        let tokens = quote! { key = };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert!(result.contains("expect val"));
    }

    #[test]
    fn test_parse_invalid_value() {
        // Test parsing when the value is not an identifier or a set: key = 1
        // This gets invoked here: _ => bail_on!(token, "expect identifier or set of identifiers as value")
        let tokens = quote! { key = 1 };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(
            result,
            "expect identifier or set of identifiers as value\n1"
        );
    }

    #[test]
    fn test_group_not_identifier() {
        // Test parsing when the group item is not an identifier: key = [1, 2]
        // This gets invoked here: _ => bail_on!(token, "item not an identifier")
        let tokens = quote! { key = [1, 2] };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(result, "item not an identifier\n1");
    }

    #[test]
    fn test_parse_duplicate_items_in_set() {
        // Test parsing with duplicate items in set: key = [value1, value1]
        // This gets invoked here: if !set.insert(item.clone()) { bail_on!(group, "duplicated item"); }
        let tokens = quote! { key = [value1, value1] };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(result, "duplicated item\n[value1 , value1]");
    }

    #[test]
    fn test_expect_comma_between_items() {
        // Test parsing when a comma is missing between items in a set: key = [value1 value2]
        // This gets invoked here: if matches!(sub_cursor.as_ref(), Some(TokenTree::Punct(punct)) if punct.as_char() != ',')
        let tokens = quote! { key = [value1 value2] };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(result, "expect comma between items\nvalue2");
    }

    #[test]
    fn test_expect_comma_between_key_values() {
        // Test parsing when a comma is missing between key-value pairs: key1 = value1 key2 = value2
        // This gets invoked here: if matches!(cursor.as_ref(), Some(TokenTree::Punct(punct)) if punct.as_char() != ',')
        let tokens = quote! { key1 = value1 key2 = value2 };
        let result = Mark::parse_dict(&tokens);

        assert!(result.is_err());

        let result = result.unwrap_err().to_string();
        assert_eq!(result, "expect comma between key-value pairs\nkey2");
    }

    #[test]
    // the attribute past to the parse_attr method should be an outer attribute
    fn test_parse_attr_inner_attribute() {
        let attr: Attribute = parse_quote! { #![inner_attribute]};

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_none()));
    }

    #[test]
    // attribute except for smt_type, smt_fn, smt_spec, and smt_axiom should be ignored
    fn test_parse_attr_meta_path_none() {
        let attr: Attribute = parse_quote! { #[test] };

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_none()));
    }

    #[test]
    // smt_type is a valid annotation and the attribute is parsed to a Mark::Type
    fn test_parse_attr_meta_path_type() {
        let attr: Attribute = parse_quote! { #[smt_type] };

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_some_and(|e| matches!(e, Mark::Type))));
    }

    #[test]
    // smt_fn is a valid annotation and the attribute is parsed to a Mark::Impl with an empty ImplMark
    fn test_parse_attr_meta_path_impl() {
        let attr: Attribute = parse_quote! { #[smt_fn] };

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_some_and(|e| matches!(
            e,
            Mark::Func(
                FuncMark {
                    method: None,
                }
            )
        ))));
    }

    #[test]
    // if the attribute is a list, the path should be smt_type, smt_fn, smt_spec, or smt_axiom
    // otherwise it should be ignored
    fn test_parse_attr_meta_list_none() {
        let attr: Attribute = parse_quote! {#[derive(copy)]};

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_none()));
    }

    #[test]
    // The attribute cannot be a smt_type with a list of tokens
    fn test_parse_attr_meta_list_type() {
        let attr: Attribute = parse_quote! { #[smt_type(copy)]};

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "unexpected list\nfor smt_type, no attributes are expected\n# [smt_type (copy)]"
        );
    }

    #[test]
    // The attribute cannot be a smt_axiom with a list of tokens
    fn test_parse_attr_meta_list_axiom() {
        let attr: Attribute = parse_quote! { #[smt_axiom(copy)]};

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(res.err().unwrap().to_string().as_str(), "expect =\ncopy");
    }

    #[test]
    // if !matches!(delimiter, MacroDelimiter::Paren(_)) { bail_on!(attr, "not a parenthesis-enclosed list"); } is invoked.
    fn test_parse_attr_meta_list_impl_no_paren() {
        let attr: Attribute = parse_quote! { #[smt_fn{xxx}] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "not a parenthesis-enclosed list\n# [smt_fn { xxx }]"
        );
    }

    #[test]
    // let mut store = Self::parse_dict(tokens)?; is invoked with an error.
    // all the keys must be an identifier
    fn test_parse_attr_meta_list_impl_parse_dict_err() {
        let attr: Attribute = parse_quote! { #[smt_fn(1 = val)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "key not an identifier\n1"
        );
    }

    // multiple methods are not allowed for impl
    // Some(_) => bail_on!(tokens, "invalid method"), // at most one method is expected
    #[test]
    fn test_parse_attr_meta_list_impl_multiple_methods() {
        let attr: Attribute = parse_quote! { #[smt_fn(method = [m1,m2])] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "invalid method\nmethod = [m1 , m2]"
        );
    }

    // Some((&item).try_into()?) is invoked with an error. So the item needs to be a reserved keyword.
    #[test]
    fn test_parse_attr_meta_list_impl_method_reserved_keyword() {
        let attr: Attribute = parse_quote! { #[smt_fn(method = eq)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "reserved method name (eq, ne)\neq"
        );
    }

    // method None and specs is empty for smt_fn
    #[test]
    fn test_parse_attr_meta_list_impl_no_token() {
        let attr: Attribute = parse_quote! { #[smt_fn()] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_ok_and(|f| f.is_some_and(|e| matches!(
            e,
            Mark::Func(
                FuncMark {
                    method: None,
                }
            )
        ))));
    }

    // bail_on!(tokens, "unrecognized entries"); // no other entries are expected except `method` and `specs`
    #[test]
    fn test_parse_attr_meta_list_impl_unrecognized_entries() {
        let attr: Attribute =
            parse_quote! { #[smt_fn(method = m, specs = [s1, s2], unrecognized = entry)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "unrecognized entries\nmethod = m , specs = [s1 , s2] , unrecognized = entry"
        );
    }

    #[test]
    // if !matches!(delimiter, MacroDelimiter::Paren(_)) { bail_on!(attr, "not a parenthesis-enclosed list"); } is invoked.
    fn test_parse_attr_meta_list_spec_no_paren() {
        let attr: Attribute = parse_quote! { #[smt_spec{xxx}] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "not a parenthesis-enclosed list\n# [smt_spec { xxx }]"
        );
    }

    #[test]
    // let mut store = Self::parse_dict(tokens)?; is invoked with an error.
    // all the keys must be an identifier
    fn test_parse_attr_meta_list_spec_parse_dict_err() {
        let attr: Attribute = parse_quote! { #[smt_spec(1 = val)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "key not an identifier\n1"
        );
    }

    // multiple methods are not allowed for spec
    // Some(_) => bail_on!(tokens, "invalid method"), // at most one method is expected
    #[test]
    fn test_parse_attr_meta_list_spec_multiple_methods() {
        let attr: Attribute = parse_quote! { #[smt_spec(method = [m1,m2])] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "invalid method\nmethod = [m1 , m2]"
        );
    }

    // Some((&item).try_into()?) is invoked with an error. So the item needs to be a reserved keyword.
    #[test]
    fn test_parse_attr_meta_list_spec_method_reserved_keyword() {
        let attr: Attribute = parse_quote! { #[smt_spec(method = eq)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "reserved method name (eq, ne)\neq"
        );
    }

    // bail_on!(tokens, "unrecognized entries"); // no other entries are expected except `method` and `impls`
    #[test]
    fn test_parse_attr_meta_list_spec_unrecognized_entries() {
        let attr: Attribute =
            parse_quote! { #[smt_spec(method = m, impls = [s1, s2], unrecognized = entry)] };

        let res = Mark::parse_attr(&attr);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "unrecognized entries\nmethod = m , impls = [s1 , s2] , unrecognized = entry"
        );
    }

    #[test]
    // name-value pairs are not expected for smt_type, smt_fn, smt_spec, and smt_axiom
    fn test_parse_attr_meta_name_value() {
        let attr: Attribute = parse_quote! { #[smt_type = "type"] };

        let res = Mark::parse_attr(&attr);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "unexpected dict\n# [smt_type = \"type\"]"
        );
    }

    #[test]
    fn test_parse_attr_meta_name_value_two() {
        let attr: Attribute = parse_quote! { #[derive = "type"] };

        let res = Mark::parse_attr(&attr);
        assert!(res.is_ok_and(|f| f.is_none()));
    }

    #[test]
    // testing parse_attrs where multiple marks are specified
    fn test_parse_attrs_multiple_marks() {
        let attr1: Attribute = parse_quote! { #[derive(Debug)] };
        let attr2: Attribute = parse_quote! { #[smt_type] };
        let attr3: Attribute = parse_quote! { #[smt_fn] };

        let res = Mark::parse_attrs(&[attr1, attr2, attr3]);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "multiple marks specified\n# [smt_fn]"
        );
    }

    #[test]
    fn test_parse_attrs_single_mark() {
        let attr1: Attribute = parse_quote! { #[derive(Debug)] };
        let attr2: Attribute = parse_quote! { #[smt_type] };

        let res = Mark::parse_attrs(&[attr1, attr2]);

        assert!(res.is_ok_and(|f| f.is_some_and(|e| matches!(e, Mark::Type))));
    }

    // match Self::parse_attr(attr)? is invoked with an error
    #[test]
    fn test_parse_attrs_parse_attr_err() {
        let attr1: Attribute = parse_quote! { #[derive(Debug)] };
        let attr2: Attribute = parse_quote! { #[smt_type = "type"] };

        let res = Mark::parse_attrs(&[attr1, attr2]);

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string().as_str(),
            "unexpected dict\n# [smt_type = \"type\"]"
        );
    }
}

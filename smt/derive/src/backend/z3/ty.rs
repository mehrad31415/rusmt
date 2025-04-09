//! This module contains the conversion of user defined types to SMT-LIB
//! It has the following functions:
//! - `tydef_in_smt`: Converts defining a user defined type into the corresponding SMT-LIB as a `String`.
//! - `tyuse_in_smt`: Converts using a user defined type in a function signature into the corresponding SMT-LIB as a `String`.

use crate::backend::z3::sort::sort_to_smt;
use crate::ir::index::UsrSortId;
use crate::ir::sort::{DataType, Variant};
use crate::IRContext;

/// Translates a user defined type to SMT-LIB.
/// This is where the definition of the data type is generated.
pub fn tydef_in_smt(sid: UsrSortId, ir: &IRContext) -> String {
    // get the data type
    let (type_name, gen_or_elem, dt) = {
        let dt = ir.ty_registry.retrieve(sid);
        let (type_name, gen_or_elem) = ir.ty_registry.reverse_lookup(sid);
        (type_name, gen_or_elem, dt)
    };

    // first analyze tuples
    if type_name.is_none() {
        // then it is a tuple like (a, b, c)
        if let DataType::Tuple(_) = dt {
            // unique name for tuple
            let tuple_name = format!(
                "Tuple_{}",
                gen_or_elem // for tuples it is the elements list type
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );
            let constructor_name = format!("mk-{}", tuple_name);

            // Generate field names: field1_, field2_, ...
            let field_names: Vec<String> = (0..gen_or_elem.len())
                .map(|i| format!("field_{}_{}_", tuple_name, i + 1))
                .collect();

            // Combine fields with their respective sorts
            let field_defs: Vec<String> = gen_or_elem
                .iter()
                .zip(field_names.iter())
                .map(|(sort, field_name)| {
                    format!("({} {})", field_name, sort_to_smt(sort, ir))
                })
                .collect();

            // for tuple (Integer, Bool):
            // (declare-datatypes () ((Tuple_Integer_Bool (mk-Tuple_Integer_Bool (field1_ Int) (field2_ Bool)))))
            return format!(
                "(declare-datatypes () (({} ({} {}))))",
                tuple_name,
                constructor_name,
                field_defs.join(" ")
            );
        } else {
            panic!("A data tupe without a name must be a tuple {}", sid);
        }
    }

    // now it has a name
    let type_name = type_name.expect("should have a name");
    let constructor_name = format!("mk-{}", type_name);
    match dt {
        // a struct tuple
        DataType::Tuple(elems) => {
            // Generate field names
            let field_names: Vec<String> = (0..elems.len())
                .map(|i| format!("field_{}_{}_", type_name, i + 1))
                .collect();

            // Combine field names with their respective sorts (types)
            let field_defs: Vec<String> = elems
                .iter()
                .zip(field_names.iter())
                .map(|(sort, field_name)| {
                    format!("({} {})", field_name, sort_to_smt(sort, ir))
                })
                .collect();

            if gen_or_elem.is_empty() {
                return format!(
                    "(declare-datatypes () (({} ({} {}))))",
                    type_name,
                    constructor_name,
                    field_defs.join(" ")
                );
            } else {
                return format!(
                    "(declare-datatypes (({})) (({} ({} {}))))",
                    gen_or_elem
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    type_name,
                    constructor_name,
                    field_defs.join(" ")
                );
            }
        }
        DataType::Record(recs) => {
            let field_defs: Vec<String> = recs
                .iter()
                .map(|(field_name, sort)| {
                    format!("({} {})", field_name, sort_to_smt(sort, ir))
                })
                .collect();

            if gen_or_elem.is_empty() {
                return format!(
                    "(declare-datatypes () (({} ({} {}))))",
                    type_name,
                    constructor_name,
                    field_defs.join(" ")
                );
            } else {
                return format!(
                    "(declare-datatypes (({})) (({} ({} {}))))",
                    gen_or_elem
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    type_name,
                    constructor_name,
                    field_defs.join(" ")
                );
            }
        }
        DataType::Enum(vars) => {
            let mut variants = Vec::new();
            for (variant_name, variant_df) in vars {
                match variant_df {
                    Variant::Unit => {
                        variants.push(format!("({})", variant_name.clone()));
                    }
                    Variant::Tuple(t) => {
                        if t.is_empty() {
                            panic!("slots in tuple is empty");
                        }

                        let field_names: Vec<String> =
                            (0..t.len()).map(|i| format!("field_{}_{}_", variant_name, i + 1)).collect();

                        // Combine field names with their respective sorts (types)
                        let field_defs: Vec<String> = t
                            .iter()
                            .zip(field_names.iter())
                            .map(|(sort, field_name)| {
                                format!("({} {})", field_name, sort_to_smt(sort, ir))
                            })
                            .collect();

                        variants.push(format!("({} {})", variant_name, field_defs.join(" ")));
                    }
                    Variant::Record(r) => {
                        if r.is_empty() {
                            panic!("slots in record is empty");
                        }

                        let field_defs: Vec<String> = r
                            .iter()
                            .map(|(field_name, sort)| {
                                format!("(record_{}_ {})", field_name, sort_to_smt(sort, ir))
                            })
                            .collect();

                        variants.push(format!("({} {})", variant_name, field_defs.join(" ")));
                    }
                }
            }

            if gen_or_elem.is_empty() {
                return format!(
                    "(declare-datatypes () (({} {})))",
                    type_name,
                    variants.join(" ")
                );
            } else {
                return format!(
                    "(declare-datatypes (({})) (({} {})))",
                    gen_or_elem
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    type_name,
                    variants.join(" ")
                );
            }
        }
    }
}

/// Converts using a user defined type in a function signature into the corresponding SMT-LIB as a `String`.
/// This is where the data type is used in a function signature. For example:
/// MyStruct in fn foo(x: MyStruct) -> MyStruct
pub fn tyuse_in_smt(sid: UsrSortId, ir: &IRContext) -> String {
    // get the data type
    let (type_name, gen_or_elem, dt) = {
        let dt = ir.ty_registry.retrieve(sid);
        let (type_name, gen_or_elem) = ir.ty_registry.reverse_lookup(sid);
        (type_name, gen_or_elem, dt)
    };

    // first analyze tuples
    if type_name.is_none() {
        // then it is a tuple like (a, b, c)
        if let DataType::Tuple(_) = dt {
            // the name should correspond to the generated name in the definition
            let tuple_name = format!(
                "Tuple_{}",
                gen_or_elem // for tuples it is the elements list type
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            );

            // for tuple (Integer, Bool):
            // Tuple_Integer_Bool, basically gives the sort name
            return format!("{}", tuple_name);
        } else {
            panic!("A data tupe without a name must be a tuple {}", sid);
        }
    }

    // now it has a name
    let type_name = type_name.expect("should have a name");
    return format!("{}", type_name);
}

//! Converts a type definition to a Z3 datatype.

use crate::ir::{
    ctxt::IRContext,
    index::UsrSortId,
    sort::{DataType, Sort, Variant},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

/// Helper to resolve the SMT name of a Sort ID
pub fn resolve_type_name(ir: &IRContext, sid: UsrSortId) -> String {
    let dt = ir.ty_registry.retrieve(sid);
    let (ty_name, type_params) = ir.ty_registry.reverse_lookup(sid);
    match dt {
        // z3 accepts <, >, $ in the type names (other theorem provers might not)
        DataType::Tuple(_) if ty_name.is_none() => {
            // Unnamed tuple naming logic
            format!(
                "Tuple_{}",
                type_params
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            )
        }
        _ => ty_name.expect("type name").to_string(),
    }
}

/// get the number of generic parameters for a type
pub fn get_generic_param_count(ir: &IRContext, sid: UsrSortId) -> (usize, Vec<Sort>) {
    // if unnamed tuple, no generics
    let dt = ir.ty_registry.retrieve(sid);
    match dt {
        DataType::Tuple(_) if ir.ty_registry.reverse_lookup(sid).0.is_none() => (0, vec![]),
        _ => {
            let (_, generics) = ir.ty_registry.reverse_lookup(sid);
            (generics.len(), generics.to_vec())
        }
    }
}

/// Helper to format a Sort into its SMT string representation.
pub(crate) fn format_sort(sort: &Sort, ir: &IRContext) -> String {
    match sort {
        Sort::User(sid) => {
            let (ty_name_opt, user_type_params) = ir.ty_registry.reverse_lookup(*sid);
            let type_name = resolve_type_name(ir, *sid);

            match ty_name_opt {
                None => type_name, // unnamed tuple — name encodes the params
                Some(_) if user_type_params.is_empty() => type_name, // named type without generics
                Some(_) => {
                    let formatted_params: Vec<String> = user_type_params
                        .iter()
                        .map(|param| format_sort(param, ir))
                        .collect();
                    format!("({} {})", type_name, formatted_params.join(" "))
                }
            }
        }
        Sort::Cloak(inner) => format!("(Cloak {})", format_sort(inner, ir)),
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Real => "Real".to_string(),
        Sort::String => "String".to_string(),
        Sort::Seq(inner) => format!("(Seq {})", format_sort(inner, ir)),
        Sort::Set(inner) => format!("(RusmartSet {})", format_sort(inner, ir)),
        Sort::Array(key, value) => format!(
            "(RusmartArray {} {})",
            format_sort(key, ir),
            format_sort(value, ir)
        ),
        Sort::F32 => "(_ FloatingPoint 8 24)".to_string(),
        Sort::F64 => "(_ FloatingPoint 11 53)".to_string(),
        Sort::I32 => "(_ BitVec 32)".to_string(),
        Sort::I64 => "(_ BitVec 64)".to_string(),
        Sort::U32 => "(_ BitVec 32)".to_string(),
        Sort::U64 => "(_ BitVec 64)".to_string(),
        Sort::Uninterpreted(x) => x.to_string(),
        Sort::Error => "(Array Int Bool)".to_string(), // Error is a set of integer IDs
    }
}

/// Converts an unnamed tuple to SMT-LIB string body.
/// Output format: for `(Integer, String)` it returns `((mk-Tuple-Integer-String (field_Tuple-Integer-String_1 Integer) (field_Tuple-Integer-String_2 String)))`
pub fn mk_unnamed_tuple_str(type_name: String, elems: &[Sort], ir: &IRContext) -> String {
    let constructor_name = format!("mk-{type_name}");

    let fields: Vec<String> = elems
        .iter()
        .enumerate()
        .map(|(i, sort)| {
            let field_name = format!("field_{}_{}_", type_name, i + 1);
            format!("({} {})", field_name, format_sort(sort, ir))
        })
        .collect();

    format!("(({} {}))", constructor_name, fields.join(" "))
}

/// Converts a named tuple to SMT-LIB string body.
/// For polymorphic types, uses `par` syntax: (par (T E) ((mk-Name ...)))
/// For `MyStruct(Integer, String)` it returns `((mk-MyStruct (field_MyStruct_1 Int) (field_MyStruct_2 String)))`
pub fn mk_named_tuple_str(
    type_name: String,
    elems: &[Sort],
    ir: &IRContext,
    type_params: &[Sort],
) -> String {
    let constructor_name = format!("mk-{type_name}");

    let fields: Vec<String> = elems
        .iter()
        .enumerate()
        .map(|(i, sort)| {
            let field_name = format!("field_{}_{}_", type_name, i + 1);
            format!("({} {})", field_name, format_sort(sort, ir))
        })
        .collect();

    let body = format!("(({} {}))", constructor_name, fields.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        // for `MyStruct(T, Integer)` it returns `(par (T) ((mk-MyStruct (field_MyStruct_1 T) (field_MyStruct_2 Int))))`
        format!(
            "(par ({}) {})",
            type_params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            body
        )
    } else {
        body
    }
}

/// Converts a record to SMT-LIB string body.
/// For polymorphic types, uses `par` syntax: (par (T E) ((mk-Name ...)))
/// For `MyRecord{x:Integer,y:String}` it returns `((mk-MyRecord (record_MyRecord_x_ Int) (record_MyRecord_y_ String)))`
pub fn mk_record_str(
    type_name: String,
    fields: &BTreeMap<String, Sort>,
    ir: &IRContext,
    type_params: &[Sort],
) -> String {
    let constructor_name = format!("mk-{type_name}");

    let field_strs: Vec<String> = fields
        .iter()
        .map(|(field_name, sort)| {
            let smt_field_name = format!("record_{type_name}_{field_name}_");
            format!("({} {})", smt_field_name, format_sort(sort, ir))
        })
        .collect();

    let body = format!("(({} {}))", constructor_name, field_strs.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!(
            "(par ({}) {})",
            type_params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            body
        )
    } else {
        body
    }
}

/// Converts an enum to SMT-LIB string body.
/// Output format for `MyEnum { A, B(Integer, String), C{x:Integer,y:String} }` it returns `((MyEnum_A) (MyEnum_B (field_MyEnum_B_1 Int) (field_MyEnum_B_2 String)) (MyEnum_C (record_MyEnum_C_x_ Int) (record_MyEnum_C_y_ String)))`
/// For polymorphic types, uses `par` syntax: (par (T E) (...))
pub fn mk_enum_str(
    type_name: String,
    variants: &BTreeMap<String, Variant>,
    ir: &IRContext,
    type_params: &[Sort],
) -> String {
    let mut variant_strs = Vec::new();

    for (vname, vdef) in variants {
        match vdef {
            Variant::Unit => {
                variant_strs.push(format!("({}_{})", type_name, vname));
            }
            Variant::Tuple(slots) => {
                let fields: Vec<String> = slots
                    .iter()
                    .enumerate()
                    .map(|(i, sort)| {
                        let field_name = format!(
                            "field_{ty_name}_{vname}_{idx}_",
                            ty_name = type_name,
                            vname = vname,
                            idx = i + 1
                        );
                        format!("({} {})", field_name, format_sort(sort, ir))
                    })
                    .collect();

                variant_strs.push(format!("({}_{} {})", type_name, vname, fields.join(" ")));
            }
            Variant::Record(rec) => {
                let fields: Vec<String> = rec
                    .iter()
                    .map(|(field_key, sort)| {
                        let field_name = format!(
                            "record_{ty_name}_{vname}_{field}_",
                            ty_name = type_name,
                            vname = vname,
                            field = field_key
                        );
                        format!("({} {})", field_name, format_sort(sort, ir))
                    })
                    .collect();

                variant_strs.push(format!("({}_{} {})", type_name, vname, fields.join(" ")));
            }
        }
    }

    let body = format!("({})", variant_strs.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!(
            "(par ({}) {})",
            type_params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            body
        )
    } else {
        body
    }
}

/// Collects type edges from the definitions to build a graph of dependencies.
pub fn collect_type_edges(defs: &BTreeMap<UsrSortId, DataType>) -> Vec<(UsrSortId, UsrSortId)> {
    let mut edges = Vec::new();

    for (src_id, datatype) in defs.iter() {
        match datatype {
            DataType::Tuple(elems) => {
                for s in elems {
                    visit_sort(s, *src_id, &mut edges)
                }
            }
            DataType::Record(fields) => {
                for s in fields.values() {
                    visit_sort(s, *src_id, &mut edges)
                }
            }
            DataType::Enum(variants) => {
                for fields in variants.values() {
                    match fields {
                        Variant::Unit => {}
                        Variant::Tuple(slots) => {
                            for s in slots {
                                visit_sort(s, *src_id, &mut edges)
                            }
                        }
                        Variant::Record(rec) => {
                            for s in rec.values() {
                                visit_sort(s, *src_id, &mut edges)
                            }
                        }
                    }
                }
            }
        }
    }
    edges
}

/// Visits a sort and collects edges to its user-defined types.
fn visit_sort(sort: &Sort, src: UsrSortId, edges: &mut Vec<(UsrSortId, UsrSortId)>) {
    match sort {
        Sort::User(target) => edges.push((src, *target)),
        Sort::Seq(elem) | Sort::Set(elem) => visit_sort(elem, src, edges),
        Sort::Array(k, v) => {
            visit_sort(k, src, edges);
            visit_sort(v, src, edges);
        }
        _ => {}
    }
}

/// Kosaraju's algorithm for finding strongly connected components in a directed graph.
pub fn scc_from_edges<T: Copy + Eq + Hash + Ord>(edges: &[(T, T)]) -> Vec<BTreeSet<T>> {
    let mut adj: HashMap<T, Vec<T>> = HashMap::new(); // outgoing edges
    let mut radj: HashMap<T, Vec<T>> = HashMap::new(); // incoming edges

    for &(u, v) in edges {
        adj.entry(u).or_default().push(v);
        radj.entry(v).or_default().push(u);
        adj.entry(v).or_default();
        radj.entry(u).or_default();
    }

    let mut seen = HashSet::new();
    let mut order = Vec::new();
    for &u in adj.keys() {
        if !seen.contains(&u) {
            dfs(u, &adj, &mut seen, &mut order);
        }
    }

    let mut comps = Vec::new();
    let mut seen2 = HashSet::new();
    while let Some(u) = order.pop() {
        if !seen2.contains(&u) {
            let mut cur = BTreeSet::new();
            dfs_rev(u, &radj, &mut seen2, &mut cur);
            comps.push(cur);
        }
    }
    comps
}

/// DFS
fn dfs<T: Copy + Eq + Hash + Ord>(
    u: T,
    g: &HashMap<T, Vec<T>>,
    seen: &mut HashSet<T>,
    order: &mut Vec<T>,
) {
    seen.insert(u);
    for &v in &g[&u] {
        if !seen.contains(&v) {
            dfs(v, g, seen, order);
        }
    }
    order.push(u);
}

/// DFS reverse
fn dfs_rev<T: Copy + Eq + Hash + Ord>(
    u: T,
    g: &HashMap<T, Vec<T>>,
    seen: &mut HashSet<T>,
    acc: &mut BTreeSet<T>,
) {
    seen.insert(u);
    acc.insert(u);
    for &v in &g[&u] {
        if !seen.contains(&v) {
            dfs_rev(v, g, seen, acc);
        }
    }
}

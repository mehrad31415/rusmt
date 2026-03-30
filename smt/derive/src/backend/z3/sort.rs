//! Converts a type definition to a Z3 datatype.

use crate::ir::{
    ctxt::IRContext,
    index::UsrSortId,
    sort::{DataType, Sort, Variant},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Helper to resolve the SMT name of a Sort ID
pub fn resolve_type_name(ir: &IRContext, sid: UsrSortId) -> String {
    let dt = ir.ty_registry.retrieve(sid);
    match dt {
        DataType::Tuple(_) if ir.ty_registry.reverse_lookup(sid).0.is_none() => {
            // Unnamed tuple naming logic
            format!(
                "Tuple_{}",
                ir.ty_registry
                    .reverse_lookup(sid)
                    .1
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join("_")
            )
        }
        _ => {
            let (ty_name, _) = ir.ty_registry.reverse_lookup(sid);
            ty_name.as_ref().expect("type name").to_string()
        }
    }
}

/// get the number of generic parameters for a type
pub fn get_generic_param_count(ir: &IRContext, sid: UsrSortId) -> usize {
    // if unnamed tuple, no generics
    let dt = ir.ty_registry.retrieve(sid);
    match dt {
        DataType::Tuple(_) if ir.ty_registry.reverse_lookup(sid).0.is_none() => 0,
        _ => {
            let (_, generics) = ir.ty_registry.reverse_lookup(sid);
            generics.len()
        }
    }
}

/// Helper to format a Sort into its SMT string representation.
///
/// When formatting inside a polymorphic datatype declaration with `par` syntax:
/// - `type_params` contains the type parameters (Sort::Uninterpreted)
/// - `type_param_names` contains the corresponding parameter names (e.g., ["T", "E"])
/// - `scc_sids` contains the set of sort IDs being declared together in the same SCC
///   (for mutually recursive types, we need to reference types by name directly)
/// - Any uninterpreted sorts that match the type parameters will be formatted using their actual names
fn format_sort(
    sort: &Sort,
    ir: &IRContext,
    type_params: &[Sort],
    type_param_names: &[String],
    scc_sids: &BTreeSet<UsrSortId>,
) -> String {
    match sort {
        Sort::User(sid) => {
            let type_name = resolve_type_name(ir, *sid);
            let (_, user_type_params) = ir.ty_registry.reverse_lookup(*sid);

            // If this type has type parameters, we need to format them
            if !user_type_params.is_empty() {
                let formatted_params: Vec<String> = user_type_params
                    .iter()
                    .map(|param| format_sort(param, ir, type_params, type_param_names, scc_sids))
                    .collect();
                format!("({} {})", type_name, formatted_params.join(" "))
            } else {
                type_name
            }
        }
        Sort::Boolean => "Bool".to_string(),
        Sort::Integer => "Int".to_string(),
        Sort::Real => "Real".to_string(),
        Sort::String => "String".to_string(),
        Sort::Seq(inner) => format!(
            "(Seq {})",
            format_sort(inner, ir, type_params, type_param_names, scc_sids)
        ),
        Sort::Set(inner) => format!(
            "(Set {})",
            format_sort(inner, ir, type_params, type_param_names, scc_sids)
        ),
        Sort::Array(key, value) => format!(
            "(Array {} {})",
            format_sort(key, ir, type_params, type_param_names, scc_sids),
            format_sort(value, ir, type_params, type_param_names, scc_sids)
        ),
        Sort::F32 => "(_ FloatingPoint 8 24)".to_string(),
        Sort::F64 => "(_ FloatingPoint 11 53)".to_string(),
        Sort::I32 => "(_ BitVec 32)".to_string(),
        Sort::I64 => "(_ BitVec 64)".to_string(),
        Sort::U32 => "(_ BitVec 32)".to_string(),
        Sort::U64 => "(_ BitVec 64)".to_string(),
        Sort::Uninterpreted(x) => {
            // Check if this uninterpreted sort is a type parameter of the current type being declared
            // If so, use its actual parameter name (from type_param_names)
            if let Some(index) = type_params.iter().position(|p| p == sort) {
                type_param_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| format!("Type_{}", index))
            } else {
                x.to_string()
            }
        }
        Sort::Error => "(Array Int Bool)".to_string(), // Error is a set of integer IDs
    }
}

/// Converts an unnamed tuple to SMT-LIB string body.
/// Output format: ((mk-Name (field_1 Type) (field_2 Type)))
/// For polymorphic types, uses `par` syntax: (par (T E) ((mk-Name ...)))
pub fn mk_unnamed_tuple_str(
    type_name: String,
    elems: &[Sort],
    ir: &IRContext,
    type_params: &[Sort],
    type_param_names: &[String],
    scc_sids: &BTreeSet<UsrSortId>,
) -> String {
    let constructor_name = format!("mk-{type_name}");

    let fields: Vec<String> = elems
        .iter()
        .enumerate()
        .map(|(i, sort)| {
            let field_name = format!("field_{}_{}_", type_name, i + 1);
            format!(
                "({} {})",
                field_name,
                format_sort(sort, ir, type_params, type_param_names, scc_sids)
            )
        })
        .collect();

    let body = format!("(({} {}))", constructor_name, fields.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!("(par ({}) {})", type_param_names.join(" "), body)
    } else {
        body
    }
}

/// Converts a named tuple to SMT-LIB string body.
/// For polymorphic types, uses `par` syntax: (par (T E) ((mk-Name ...)))
pub fn mk_named_tuple_str(
    type_name: String,
    elems: &[Sort],
    ir: &IRContext,
    type_params: &[Sort],
    type_param_names: &[String],
    scc_sids: &BTreeSet<UsrSortId>,
) -> String {
    let constructor_name = format!("mk-{type_name}");

    let fields: Vec<String> = elems
        .iter()
        .enumerate()
        .map(|(i, sort)| {
            let field_name = format!("field_{}_{}_", type_name, i + 1);
            format!(
                "({} {})",
                field_name,
                format_sort(sort, ir, type_params, type_param_names, scc_sids)
            )
        })
        .collect();

    let body = format!("(({} {}))", constructor_name, fields.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!("(par ({}) {})", type_param_names.join(" "), body)
    } else {
        body
    }
}

/// Converts a record to SMT-LIB string body.
/// For polymorphic types, uses `par` syntax: (par (T E) ((mk-Name ...)))
///
/// **Well-foundedness fix**: When we have mutually recursive types (scc_sids.len() > 1),
/// we need to ensure there's a base case. If ANY field in this record references another
/// type in the same SCC (creating a cycle), we convert the entire record to an enum with
/// a unit variant to provide a base case that breaks the cycle.
pub fn mk_record_str(
    type_name: String,
    fields: &BTreeMap<String, Sort>,
    ir: &IRContext,
    type_params: &[Sort],
    type_param_names: &[String],
    scc_sids: &BTreeSet<UsrSortId>,
) -> String {
    let constructor_name = format!("mk-{type_name}");

    // Check if we have multiple types in the SCC (mutually recursive)
    let is_mutually_recursive = scc_sids.len() > 1;

    // Check if ANY field references another type in the same SCC (creates a cycle)
    // Even if there are other fields that don't reference the SCC, the cycle still exists
    let has_scc_cycle = is_mutually_recursive
        && fields.values().any(|sort| {
            if let Sort::User(target_sid) = sort {
                scc_sids.contains(target_sid)
            } else {
                false
            }
        });

    let field_strs: Vec<String> = fields
        .iter()
        .map(|(field_name, sort)| {
            let smt_field_name = format!("record_{type_name}_{field_name}_");
            format!(
                "({} {})",
                smt_field_name,
                format_sort(sort, ir, type_params, type_param_names, scc_sids)
            )
        })
        .collect();

    // If there's a cycle through SCC types, convert to enum with unit variant as base case
    let body = if has_scc_cycle {
        // Convert record to enum with unit variant + record variant
        // This breaks the cycle by providing a base case (None) that doesn't require other types
        let unit_variant = format!("({}_None)", type_name);
        let record_variant = format!("({}_Some {})", constructor_name, field_strs.join(" "));
        format!("({} {})", unit_variant, record_variant)
    } else {
        format!("(({} {}))", constructor_name, field_strs.join(" "))
    };

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!("(par ({}) {})", type_param_names.join(" "), body)
    } else {
        body
    }
}

/// Converts an enum to SMT-LIB string body.
/// Output format: ((Unit) (TupleVariant (f1 Int)) (RecordVariant (f1 Int)))
/// For polymorphic types, uses `par` syntax: (par (T E) (...))
pub fn mk_enum_str(
    type_name: String,
    variants: &BTreeMap<String, Variant>,
    ir: &IRContext,
    type_params: &[Sort],
    type_param_names: &[String],
    scc_sids: &BTreeSet<UsrSortId>,
) -> String {
    let mut variant_strs = Vec::new();

    for (vname, vdef) in variants {
        match vdef {
            Variant::Unit => {
                variant_strs.push(format!("({})", vname));
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
                        format!(
                            "({} {})",
                            field_name,
                            format_sort(sort, ir, type_params, type_param_names, scc_sids)
                        )
                    })
                    .collect();

                variant_strs.push(format!("({} {})", vname, fields.join(" ")));
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
                        format!(
                            "({} {})",
                            field_name,
                            format_sort(sort, ir, type_params, type_param_names, scc_sids)
                        )
                    })
                    .collect();

                variant_strs.push(format!("({} {})", vname, fields.join(" ")));
            }
        }
    }

    let body = format!("({})", variant_strs.join(" "));

    // Wrap in `par` if polymorphic
    if !type_params.is_empty() {
        format!("(par ({}) {})", type_param_names.join(" "), body)
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

/// Computes strongly connected components from the type edges to get mutually recursive types.
pub fn scc_from_edges(edges: &[(UsrSortId, UsrSortId)]) -> Vec<BTreeSet<UsrSortId>> {
    let mut adj: HashMap<UsrSortId, Vec<UsrSortId>> = HashMap::new(); // outgoing edges
    let mut radj: HashMap<UsrSortId, Vec<UsrSortId>> = HashMap::new(); // incoming edges

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
fn dfs(
    u: UsrSortId,
    g: &HashMap<UsrSortId, Vec<UsrSortId>>,
    seen: &mut HashSet<UsrSortId>,
    order: &mut Vec<UsrSortId>,
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
fn dfs_rev(
    u: UsrSortId,
    g: &HashMap<UsrSortId, Vec<UsrSortId>>,
    seen: &mut HashSet<UsrSortId>,
    acc: &mut BTreeSet<UsrSortId>,
) {
    seen.insert(u);
    acc.insert(u);
    for &v in &g[&u] {
        if !seen.contains(&v) {
            dfs_rev(v, g, seen, acc);
        }
    }
}

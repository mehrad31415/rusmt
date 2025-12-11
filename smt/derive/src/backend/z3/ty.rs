//! Converts a type definition to a Z3 datatype.

use crate::ir::{
    ctxt::IRContext,
    index::UsrSortId,
    sort::{DataType, Sort, Variant},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Helper to resolve the SMT name of a Sort ID
fn resolve_type_name(ir: &IRContext, sid: UsrSortId) -> String {
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

/// Helper to format a Sort into its SMT string representation.
fn format_sort(sort: &Sort, ir: &IRContext) -> String {
    match sort {
        Sort::User(sid) => resolve_type_name(ir, *sid),
        // Assuming other sorts (Int, Bool, etc) implement Display correctly for SMT-LIB
        _ => sort.to_string(),
    }
}

/// Converts an unnamed tuple to SMT-LIB string body.
/// Output format: ((mk-Name (field_1 Type) (field_2 Type)))
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
pub fn mk_named_tuple_str(type_name: String, elems: &[Sort], ir: &IRContext) -> String {
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

/// Converts a record to SMT-LIB string body.
pub fn mk_record_str(type_name: String, fields: &BTreeMap<String, Sort>, ir: &IRContext) -> String {
    let constructor_name = format!("mk-{type_name}");

    let field_strs: Vec<String> = fields
        .iter()
        .map(|(field_name, sort)| {
            let smt_field_name = format!("record_{type_name}_{field_name}_");
            format!("({} {})", smt_field_name, format_sort(sort, ir))
        })
        .collect();

    format!("(({} {}))", constructor_name, field_strs.join(" "))
}

/// Converts an enum to SMT-LIB string body.
/// Output format: ((Unit) (TupleVariant (f1 Int)) (RecordVariant (f1 Int)))
pub fn mk_enum_str(
    type_name: String,
    variants: &BTreeMap<String, Variant>,
    ir: &IRContext,
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
                        format!("({} {})", field_name, format_sort(sort, ir))
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
                        format!("({} {})", field_name, format_sort(sort, ir))
                    })
                    .collect();

                variant_strs.push(format!("({} {})", vname, fields.join(" ")));
            }
        }
    }

    format!("({})", variant_strs.join(" "))
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

fn get_name(ir: &IRContext, sid: &UsrSortId) -> z3::Symbol {
    let dt = ir.ty_registry.retrieve(*sid);
    match dt {
        DataType::Tuple(_) if ir.ty_registry.reverse_lookup(*sid).0.is_none() => format!(
            "Tuple_{}",
            ir.ty_registry
                .reverse_lookup(*sid)
                .1
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("_")
        )
        .into(),
        _ => {
            let (ty_name, _) = ir.ty_registry.reverse_lookup(*sid);
            let ty_name = ty_name.as_ref().expect("type name for named tuple");
            ty_name.to_string().into()
        }
    }
}

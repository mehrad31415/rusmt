//! Converts a type definition to a Z3 datatype.

use crate::backend::z3::sort::sort_to_z3;
use crate::ir::{
    ctxt::IRContext,
    index::UsrSortId,
    sort::{DataType, Sort, Variant},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use z3::{Context, DatatypeAccessor, DatatypeBuilder, DatatypeSort};

/// Converts a tuple type to a Z3 datatype.
pub fn mk_unnamed_tuple(
    ctx: &Context,
    sid: UsrSortId,
    elems: &[Sort],
    ir: &IRContext,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sid_set: &BTreeSet<UsrSortId>,
) -> DatatypeBuilder {
    if ir.ty_registry.reverse_lookup(sid).1 != elems {
        panic!("Tuples elements are not consistent");
    }

    let tuple_name = format!(
        "Tuple_{}",
        ir.ty_registry
            .reverse_lookup(sid)
            .1
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join("_")
    );
    let constructor_name = format!("mk-{tuple_name}");

    // Generate field names: field_Tuple_Integer_Bool_1_, field_Tuple_Integer_Bool_2_, etc.
    // tuple_name gives the field a unique name accross all tuples and the i+1 gives the field a unique name within the tuple
    let field_names: Vec<String> = (0..elems.len())
        .map(|i| format!("field_{}_{}_", tuple_name, i + 1))
        .collect();

    // Combine fields with their respective sorts
    let field_defs = elems
        .iter()
        .zip(field_names.iter())
        .map(|(sort, field_name)| {
            (
                field_name.as_str(),
                if let Sort::User(x) = sort {
                    if sid_set.contains(x) {
                        DatatypeAccessor::Datatype(get_name(&ir, x))
                    } else {
                        DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, None, ty_map))
                    }
                } else {
                    DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, None, ty_map))
                },
            )
        })
        .collect();

    // for tuple (Integer, Bool):
    // (declare-datatypes () ((Tuple_Integer_Bool (mk-Tuple_Integer_Bool (field_Tuple_Integer_Bool_1_ Int) (field_Tuple_Integer_Bool_2_ Bool)))))
    let dt = DatatypeBuilder::new(ctx, tuple_name.as_str())
        .variant(constructor_name.as_str(), field_defs);
    dt
}

/// Converts a named tuple type to a Z3 datatype.
pub fn mk_named_tuple(
    ctx: &Context,
    sid: UsrSortId,
    elems: &[Sort],
    ir: &IRContext,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sid_set: &BTreeSet<UsrSortId>,
) -> DatatypeBuilder {
    let (ty_name, _) = ir.ty_registry.reverse_lookup(sid);
    let ty_name = ty_name.as_ref().expect("type name for named tuple");
    let constructor_name = format!("mk-{ty_name}");

    let field_names: Vec<String> = (0..elems.len())
        .map(|i| format!("field_{}_{}_", ty_name, i + 1))
        .collect();

    let field_defs = elems
        .iter()
        .zip(field_names.iter())
        .map(|(sort, field_name)| {
            (
                field_name.as_str(),
                if let Sort::User(x) = sort {
                    if sid_set.contains(x) {
                        DatatypeAccessor::Datatype(get_name(&ir, x))
                    } else {
                        DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, Some(ty_name), ty_map))
                    }
                } else {
                    DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, Some(ty_name), ty_map))
                },
            )
        })
        .collect();

    let dt = DatatypeBuilder::new(ctx, ty_name.to_string().as_str())
        .variant(constructor_name.as_str(), field_defs);
    dt
}

/// Converts a record type to a Z3 datatype.
pub fn mk_record(
    ctx: &Context,
    sid: UsrSortId,
    fields: &BTreeMap<String, Sort>,
    ir: &IRContext,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sid_set: &BTreeSet<UsrSortId>,
) -> DatatypeBuilder {
    let (ty_name, _) = ir.ty_registry.reverse_lookup(sid);
    let ty_name = ty_name.as_ref().expect("type name for record");
    let constructor_name = format!("mk-{ty_name}");

    let field_names: Vec<String> = fields
        .iter()
        .map(|(field_name, _)| format!("record_{ty_name}_{field_name}_"))
        .collect();

    let field_defs = fields
        .iter()
        .zip(field_names.iter())
        .map(|((_, sort), field_name)| {
            (
                field_name.as_str(),
                if let Sort::User(x) = sort {
                    if sid_set.contains(x) {
                        DatatypeAccessor::Datatype(get_name(&ir, x))
                    } else {
                        DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, Some(ty_name), ty_map))
                    }
                } else {
                    DatatypeAccessor::Sort(sort_to_z3(sort, ctx, ir, Some(ty_name), ty_map))
                },
            )
        })
        .collect();

    let dt = DatatypeBuilder::new(ctx, ty_name.to_string().as_str())
        .variant(constructor_name.as_str(), field_defs);
    dt
}

/// Converts an enum type to a Z3 datatype.
pub fn mk_enum(
    ctx: &Context,
    sid: UsrSortId,
    variants: &BTreeMap<String, Variant>,
    ir: &IRContext,
    ty_map: &HashMap<UsrSortId, DatatypeSort>,
    sid_set: &BTreeSet<UsrSortId>,
) -> DatatypeBuilder {
    let (ty_name_opt, _) = ir.ty_registry.reverse_lookup(sid);
    let ty_name = ty_name_opt.as_ref().expect("enums must have a name");

    let mut builder = DatatypeBuilder::new(ctx, ty_name.to_string().as_str());

    for (vname, vdef) in variants {
        match vdef {
            Variant::Unit => {
                builder = builder.variant(vname.as_str(), vec![]);
            }
            Variant::Tuple(slots) => {
                assert!(
                    !slots.is_empty(),
                    "tuple variant `{vname}` must have at least one slot"
                );

                let field_names: Vec<String> = slots
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("field_{ty_name}_{vname}_{idx}_", idx = i + 1))
                    .collect();

                let fields: Vec<(&str, DatatypeAccessor)> = slots
                    .iter()
                    .zip(field_names.iter())
                    .map(|(slot_sort, field_name)| {
                        (
                            field_name.as_str(),
                            if let Sort::User(x) = slot_sort {
                                if sid_set.contains(x) {
                                    DatatypeAccessor::Datatype(get_name(&ir, x))
                                } else {
                                    DatatypeAccessor::Sort(sort_to_z3(
                                        slot_sort,
                                        ctx,
                                        ir,
                                        Some(ty_name),
                                        ty_map,
                                    ))
                                }
                            } else {
                                DatatypeAccessor::Sort(sort_to_z3(
                                    slot_sort,
                                    ctx,
                                    ir,
                                    Some(ty_name),
                                    ty_map,
                                ))
                            },
                        )
                    })
                    .collect();

                builder = builder.variant(vname.as_str(), fields);
            }
            Variant::Record(rec) => {
                assert!(
                    !rec.is_empty(),
                    "record variant `{vname}` must have at least one field"
                );

                let field_names: Vec<String> = rec
                    .iter()
                    .map(|(field, _)| format!("record_{ty_name}_{vname}_{field}_"))
                    .collect();

                let fields: Vec<(&str, DatatypeAccessor)> = rec
                    .iter()
                    .zip(field_names.iter())
                    .map(|((_, slot_sort), field_name)| {
                        (
                            field_name.as_str(),
                            if let Sort::User(x) = slot_sort {
                                if sid_set.contains(x) {
                                    DatatypeAccessor::Datatype(get_name(&ir, x))
                                } else {
                                    DatatypeAccessor::Sort(sort_to_z3(
                                        slot_sort,
                                        ctx,
                                        ir,
                                        Some(ty_name),
                                        ty_map,
                                    ))
                                }
                            } else {
                                DatatypeAccessor::Sort(sort_to_z3(
                                    slot_sort,
                                    ctx,
                                    ir,
                                    Some(ty_name),
                                    ty_map,
                                ))
                            },
                        )
                    })
                    .collect();

                builder = builder.variant(vname.as_str(), fields);
            }
        }
    }

    builder
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
        Sort::Map(k, v) => {
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

//! Z3 API context: maps IR sorts to Z3 sorts, builds datatypes and function declarations.

use crate::backend::z3::fun::{collect_function_call_edges, resolve_function_name};
use crate::backend::z3::sort::{collect_type_edges, resolve_type_name, scc_from_edges};
use crate::backend::z3_api::mk_string_symbol;
use crate::ir::ctxt::IRContext;
use crate::ir::exp::Expression;
use crate::ir::index::{UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort, Variant};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Holds the Z3 context and all mappings from IR to Z3 objects.
pub struct Z3ApiContext<'ctx> {
    /// Raw Z3 context pointer.
    pub ctx: z3_sys::Z3_context,

    /// Map from IR Sort to Z3_sort.
    sort_cache: BTreeMap<Sort, z3_sys::Z3_sort>,

    /// Map from UsrSortId to Z3_sort (for user-defined datatypes).
    datatype_sorts: HashMap<UsrSortId, z3_sys::Z3_sort>,

    /// Map from (UsrSortId, variant_name) to constructor Z3_func_decl.
    constructors: HashMap<(UsrSortId, String), z3_sys::Z3_func_decl>,

    /// Map from (UsrSortId, variant_name) to tester Z3_func_decl.
    testers: HashMap<(UsrSortId, String), z3_sys::Z3_func_decl>,

    /// Map from (UsrSortId, variant_name, field_index) to accessor Z3_func_decl.
    accessors: HashMap<(UsrSortId, String, usize), z3_sys::Z3_func_decl>,

    /// Map from UsrFunId to function declaration.
    func_decls: HashMap<UsrFunId, z3_sys::Z3_func_decl>,

    /// Null sentinel constants for array default values.
    null_consts: HashMap<String, z3_sys::Z3_ast>,

    /// Helper function declarations for string parsing (hex/oct/bin).
    helper_func_decls: HashMap<String, z3_sys::Z3_func_decl>,

    /// Reference to the IR context.
    pub ir: &'ctx IRContext,

    /// Phantom for lifetime.
    _phantom: std::marker::PhantomData<&'ctx ()>,
}

impl<'ctx> Z3ApiContext<'ctx> {
    /// Create a new Z3ApiContext, building all sorts, datatypes, functions from the IR.
    pub fn new(ctx: z3_sys::Z3_context, ir: &'ctx IRContext) -> Self {
        let mut api_ctx = Self {
            ctx,
            sort_cache: BTreeMap::new(),
            datatype_sorts: HashMap::new(),
            constructors: HashMap::new(),
            testers: HashMap::new(),
            accessors: HashMap::new(),
            func_decls: HashMap::new(),
            null_consts: HashMap::new(),
            helper_func_decls: HashMap::new(),
            ir,
            _phantom: std::marker::PhantomData,
        };

        api_ctx.build_datatypes();
        eprintln!("[z3_api] Datatypes built.");
        api_ctx.build_null_consts();
        eprintln!("[z3_api] Null consts built.");
        api_ctx.build_string_parsing_helpers();
        eprintln!("[z3_api] String parsing helpers built.");
        api_ctx.build_functions();
        eprintln!("[z3_api] Functions built.");

        api_ctx
    }

    /// Translate an IR Sort to a Z3_sort.
    pub fn translate_sort(&mut self, sort: &Sort) -> z3_sys::Z3_sort {
        if let Some(&cached) = self.sort_cache.get(sort) {
            return cached;
        }
        let z3_sort = self.translate_sort_uncached(sort);
        unsafe {
            z3_sys::Z3_inc_ref(self.ctx, z3_sys::Z3_sort_to_ast(self.ctx, z3_sort).expect("Z3_sort_to_ast"));
        }
        self.sort_cache.insert(sort.clone(), z3_sort);
        z3_sort
    }

    fn translate_sort_uncached(&mut self, sort: &Sort) -> z3_sys::Z3_sort {
        unsafe {
            match sort {
                Sort::Boolean => z3_sys::Z3_mk_bool_sort(self.ctx).expect("Z3_mk_bool_sort"),
                Sort::Integer => z3_sys::Z3_mk_int_sort(self.ctx).expect("Z3_mk_int_sort"),
                Sort::Real => z3_sys::Z3_mk_real_sort(self.ctx).expect("Z3_mk_real_sort"),
                Sort::String => z3_sys::Z3_mk_string_sort(self.ctx).expect("Z3_mk_string_sort"),
                Sort::I32 | Sort::U32 => z3_sys::Z3_mk_bv_sort(self.ctx, 32).expect("Z3_mk_bv_sort"),
                Sort::I64 | Sort::U64 => z3_sys::Z3_mk_bv_sort(self.ctx, 64).expect("Z3_mk_bv_sort"),
                Sort::F32 => z3_sys::Z3_mk_fpa_sort(self.ctx, 8, 24).expect("Z3_mk_fpa_sort"),
                Sort::F64 => z3_sys::Z3_mk_fpa_sort(self.ctx, 11, 53).expect("Z3_mk_fpa_sort"),
                Sort::Seq(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    z3_sys::Z3_mk_seq_sort(self.ctx, inner_sort).expect("Z3_mk_seq_sort")
                }
                Sort::Set(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    z3_sys::Z3_mk_set_sort(self.ctx, inner_sort).expect("Z3_mk_set_sort")
                }
                Sort::Array(key, value) => {
                    let key_sort = self.translate_sort(key);
                    let val_sort = self.translate_sort(value);
                    z3_sys::Z3_mk_array_sort(self.ctx, key_sort, val_sort).expect("Z3_mk_array_sort")
                }
                Sort::Error => {
                    let int_sort = z3_sys::Z3_mk_int_sort(self.ctx).expect("Z3_mk_int_sort");
                    let bool_sort = z3_sys::Z3_mk_bool_sort(self.ctx).expect("Z3_mk_bool_sort");
                    z3_sys::Z3_mk_array_sort(self.ctx, int_sort, bool_sort).expect("Z3_mk_array_sort")
                }
                Sort::User(sid) => {
                    if let Some(&s) = self.datatype_sorts.get(sid) {
                        s
                    } else {
                        // Check all available datatype_sorts for debugging
                    let available: Vec<_> = self.datatype_sorts.keys().map(|k| {
                        let n = resolve_type_name(self.ir, *k);
                        format!("{}({})", n, k.index)
                    }).collect();
                    panic!(
                        "User sort {:?} not yet built (id={}). Available: {:?}",
                        sort, sid.index, available
                    );
                    }
                }
                Sort::Uninterpreted(name) => {
                    let sym = mk_string_symbol(self.ctx, name.as_ref());
                    z3_sys::Z3_mk_uninterpreted_sort(self.ctx, sym).expect("Z3_mk_uninterpreted_sort")
                }
            }
        }
    }

    /// Build all user-defined datatypes from the IR type registry.
    fn build_datatypes(&mut self) {
        let ir = self.ir;
        if ir.ty_registry.data_types().is_empty() {
            return;
        }

        let edges = collect_type_edges(ir.ty_registry.data_types());
        let mut sccs = scc_from_edges(&edges);

        let all_ids: BTreeSet<_> = ir.ty_registry.data_types().keys().copied().collect();
        let covered: BTreeSet<_> = sccs.iter().flat_map(|s| s.iter().copied()).collect();
        for sid in all_ids.difference(&covered) {
            sccs.push(BTreeSet::from([*sid]));
        }

        // Deduplicate: group by type name, keep one representative per name.
        // The API backend creates one Z3 sort per type name (not per instantiation).
        let mut name_to_best_sid: HashMap<String, UsrSortId> = HashMap::new();
        for scc in sccs.iter() {
            for &sid in scc {
                let type_name = resolve_type_name(ir, sid);
                let (_, type_params) = ir.ty_registry.reverse_lookup(sid);
                let has_matching_params = type_params.iter().any(|sort| {
                    if let Sort::Uninterpreted(smt_name) = sort {
                        smt_name.as_ref().starts_with(&format!("{}_", type_name))
                    } else {
                        false
                    }
                });
                if !name_to_best_sid.contains_key(&type_name) || has_matching_params {
                    name_to_best_sid.insert(type_name, sid);
                }
            }
        }

        // Build deduplicated SCC list: replace each SID with its best representative.
        let mut seen_scc_signatures: HashSet<Vec<String>> = HashSet::new();
        let mut deduplicated_sccs: Vec<BTreeSet<UsrSortId>> = Vec::new();
        for scc in sccs.iter().rev() {
            let canonical_scc: BTreeSet<UsrSortId> = scc
                .iter()
                .map(|&sid| {
                    let type_name = resolve_type_name(ir, sid);
                    *name_to_best_sid.get(&type_name).unwrap_or(&sid)
                })
                .collect();
            let mut type_names: Vec<String> = canonical_scc
                .iter()
                .map(|&sid| resolve_type_name(ir, sid))
                .collect();
            type_names.sort();
            if seen_scc_signatures.insert(type_names) {
                deduplicated_sccs.push(canonical_scc);
            }
        }

        // Also ensure every best SID appears in at least one SCC
        for (_, &best_sid) in &name_to_best_sid {
            let already_covered = deduplicated_sccs.iter().any(|scc| scc.contains(&best_sid));
            if !already_covered {
                deduplicated_sccs.push(BTreeSet::from([best_sid]));
            }
        }

        for (scc_idx, scc) in deduplicated_sccs.iter().enumerate() {
            let names: Vec<String> = scc.iter().map(|&sid| resolve_type_name(ir, sid)).collect();
            eprintln!("[z3_api] Building SCC {}: {:?}", scc_idx, names);
            self.build_datatype_scc(scc);

            // Eagerly map all non-best SIDs that share a type name with any just-built SID
            self.map_non_best_sids(ir, &name_to_best_sid);
        }
    }

    /// Map all non-best SIDs to their best representative's Z3_sort and func_decls.
    fn map_non_best_sids(&mut self, ir: &IRContext, name_to_best_sid: &HashMap<String, UsrSortId>) {
        let all_ids: Vec<UsrSortId> = ir.ty_registry.data_types().keys().copied().collect();
        for sid in all_ids {
            if self.datatype_sorts.contains_key(&sid) {
                continue;
            }
            let type_name = resolve_type_name(ir, sid);
            if let Some(&best_sid) = name_to_best_sid.get(&type_name) {
                if let Some(&z3_sort) = self.datatype_sorts.get(&best_sid) {
                    self.datatype_sorts.insert(sid, z3_sort);
                    self.sort_cache.insert(Sort::User(sid), z3_sort);
                    let ctor_keys: Vec<_> = self
                        .constructors
                        .keys()
                        .filter(|(s, _)| *s == best_sid)
                        .map(|(_, n)| n.clone())
                        .collect();
                    for name in ctor_keys {
                        if let Some(&c) = self.constructors.get(&(best_sid, name.clone())) {
                            self.constructors.insert((sid, name.clone()), c);
                        }
                        if let Some(&t) = self.testers.get(&(best_sid, name.clone())) {
                            self.testers.insert((sid, name.clone()), t);
                        }
                        for i in 0..50 {
                            if let Some(&a) = self.accessors.get(&(best_sid, name.clone(), i)) {
                                self.accessors.insert((sid, name.clone(), i), a);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_datatype_scc(&mut self, scc: &BTreeSet<UsrSortId>) {
        let ir = self.ir;
        let ctx = self.ctx;
        let n = scc.len();
        if n == 0 {
            return;
        }
        let sids: Vec<UsrSortId> = scc.iter().copied().collect();

        unsafe {
            // Pre-create uninterpreted placeholder sorts for all SCC members.
            // This allows composite sorts like Array(String, Value) to be resolved
            // during constructor building, even though the real sort isn't created yet.
            // After Z3_mk_datatypes replaces these with real sorts, the composite sorts
            // built with placeholders will still work because Z3 resolves by name.
            for &sid in &sids {
                if !self.datatype_sorts.contains_key(&sid) {
                    let type_name = resolve_type_name(ir, sid);
                    let sym = mk_string_symbol(ctx, &type_name);
                    let placeholder = z3_sys::Z3_mk_uninterpreted_sort(ctx, sym).expect("Z3_mk_uninterpreted_sort");
                    z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_sort_to_ast(ctx, placeholder).expect("Z3_sort_to_ast"));
                    self.datatype_sorts.insert(sid, placeholder);
                    self.sort_cache.insert(Sort::User(sid), placeholder);
                }
            }

            let mut sort_names: Vec<z3_sys::Z3_symbol> = Vec::new();
            let mut constructor_lists: Vec<z3_sys::Z3_constructor_list> = Vec::new();
            let mut all_ctor_infos: Vec<Vec<CtorInfo>> = Vec::new();
            let mut all_raw_ctors: Vec<Vec<z3_sys::Z3_constructor>> = Vec::new();

            for &sid in &sids {
                let type_name = resolve_type_name(ir, sid);
                let dt = ir.ty_registry.retrieve(sid);
                sort_names.push(mk_string_symbol(ctx, &type_name));

                let (ctors, ctor_infos) =
                    self.build_constructors_for_datatype(sid, &type_name, dt, scc, &sids);

                let ctor_list = z3_sys::Z3_mk_constructor_list(
                    ctx,
                    ctors.len() as u32,
                    ctors.as_ptr() as *mut _,
                ).expect("Z3_mk_constructor_list");
                constructor_lists.push(ctor_list);
                all_ctor_infos.push(ctor_infos);
                all_raw_ctors.push(ctors);
            }

            let mut result_sorts: Vec<std::mem::MaybeUninit<z3_sys::Z3_sort>> = vec![std::mem::MaybeUninit::uninit(); n];
            z3_sys::Z3_mk_datatypes(
                ctx,
                n as u32,
                sort_names.as_ptr(),
                result_sorts.as_mut_ptr() as *mut z3_sys::Z3_sort,
                constructor_lists.as_mut_ptr(),
            );

            for (i, &sid) in sids.iter().enumerate() {
                let z3_sort = result_sorts[i].assume_init();
                z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_sort_to_ast(ctx, z3_sort).expect("Z3_sort_to_ast"));
                self.datatype_sorts.insert(sid, z3_sort);
                self.sort_cache.insert(Sort::User(sid), z3_sort);
            }

            // Query constructors to get func_decls
            for (type_idx, &sid) in sids.iter().enumerate() {
                let ctors = &all_raw_ctors[type_idx];
                let ctor_infos = &all_ctor_infos[type_idx];

                for (ctor_idx, info) in ctor_infos.iter().enumerate() {
                    let mut ctor_func: std::mem::MaybeUninit<z3_sys::Z3_func_decl> = std::mem::MaybeUninit::uninit();
                    let mut tester_func: std::mem::MaybeUninit<z3_sys::Z3_func_decl> = std::mem::MaybeUninit::uninit();
                    let mut accessor_funcs: Vec<std::mem::MaybeUninit<z3_sys::Z3_func_decl>> =
                        vec![std::mem::MaybeUninit::uninit(); info.num_fields];

                    z3_sys::Z3_query_constructor(
                        ctx,
                        ctors[ctor_idx],
                        info.num_fields as u32,
                        ctor_func.as_mut_ptr(),
                        tester_func.as_mut_ptr(),
                        accessor_funcs.as_mut_ptr() as *mut z3_sys::Z3_func_decl,
                    );

                    let ctor_func = ctor_func.assume_init();
                    let tester_func = tester_func.assume_init();
                    self.constructors
                        .insert((sid, info.name.clone()), ctor_func);
                    self.testers.insert((sid, info.name.clone()), tester_func);
                    for (fi, acc) in accessor_funcs.iter().enumerate() {
                        self.accessors.insert((sid, info.name.clone(), fi), acc.assume_init());
                    }
                }
            }

            for clist in constructor_lists {
                z3_sys::Z3_del_constructor_list(ctx, clist);
            }
        }
    }

    fn build_constructors_for_datatype(
        &mut self,
        sid: UsrSortId,
        type_name: &str,
        dt: &DataType,
        scc_set: &BTreeSet<UsrSortId>,
        scc_sids: &[UsrSortId],
    ) -> (Vec<z3_sys::Z3_constructor>, Vec<CtorInfo>) {
        let mut ctors = Vec::new();
        let mut infos = Vec::new();

        match dt {
            DataType::Tuple(elems) => {
                let ctor_name = format!("mk-{}", type_name);
                let fields: Vec<_> = elems
                    .iter()
                    .enumerate()
                    .map(|(i, sort)| (format!("field_{}_{}_", type_name, i + 1), sort.clone()))
                    .collect();
                let ctor = unsafe { self.mk_constructor(&ctor_name, &fields, scc_set, scc_sids) };
                infos.push(CtorInfo {
                    name: ctor_name,
                    num_fields: fields.len(),
                });
                ctors.push(ctor);
            }
            DataType::Record(fields_map) => {
                let is_mutually_recursive = scc_set.len() > 1;
                let has_scc_cycle = is_mutually_recursive
                    && fields_map.values().any(|sort| {
                        matches!(sort, Sort::User(target_sid) if scc_set.contains(target_sid))
                    });

                if has_scc_cycle {
                    let none_name = format!("{}_None", type_name);
                    let none_ctor =
                        unsafe { self.mk_constructor(&none_name, &[], scc_set, scc_sids) };
                    infos.push(CtorInfo {
                        name: none_name,
                        num_fields: 0,
                    });
                    ctors.push(none_ctor);

                    let some_name = format!("mk-{}", type_name);
                    let fields: Vec<_> = fields_map
                        .iter()
                        .map(|(fname, sort)| {
                            (format!("record_{}_{}_", type_name, fname), sort.clone())
                        })
                        .collect();
                    let some_ctor =
                        unsafe { self.mk_constructor(&some_name, &fields, scc_set, scc_sids) };
                    infos.push(CtorInfo {
                        name: some_name,
                        num_fields: fields.len(),
                    });
                    ctors.push(some_ctor);
                } else {
                    let ctor_name = format!("mk-{}", type_name);
                    let fields: Vec<_> = fields_map
                        .iter()
                        .map(|(fname, sort)| {
                            (format!("record_{}_{}_", type_name, fname), sort.clone())
                        })
                        .collect();
                    let ctor =
                        unsafe { self.mk_constructor(&ctor_name, &fields, scc_set, scc_sids) };
                    infos.push(CtorInfo {
                        name: ctor_name,
                        num_fields: fields.len(),
                    });
                    ctors.push(ctor);
                }
            }
            DataType::Enum(variants) => {
                for (vname, vdef) in variants {
                    match vdef {
                        Variant::Unit => {
                            let ctor =
                                unsafe { self.mk_constructor(vname, &[], scc_set, scc_sids) };
                            infos.push(CtorInfo {
                                name: vname.clone(),
                                num_fields: 0,
                            });
                            ctors.push(ctor);
                        }
                        Variant::Tuple(slots) => {
                            let fields: Vec<_> = slots
                                .iter()
                                .enumerate()
                                .map(|(i, sort)| {
                                    (
                                        format!("field_{}_{}_{}_", type_name, vname, i + 1),
                                        sort.clone(),
                                    )
                                })
                                .collect();
                            let ctor = unsafe {
                                self.mk_constructor(vname, &fields, scc_set, scc_sids)
                            };
                            infos.push(CtorInfo {
                                name: vname.clone(),
                                num_fields: fields.len(),
                            });
                            ctors.push(ctor);
                        }
                        Variant::Record(rec) => {
                            let fields: Vec<_> = rec
                                .iter()
                                .map(|(field_key, sort)| {
                                    (
                                        format!("record_{}_{}_{}_", type_name, vname, field_key),
                                        sort.clone(),
                                    )
                                })
                                .collect();
                            let ctor = unsafe {
                                self.mk_constructor(vname, &fields, scc_set, scc_sids)
                            };
                            infos.push(CtorInfo {
                                name: vname.clone(),
                                num_fields: fields.len(),
                            });
                            ctors.push(ctor);
                        }
                    }
                }
            }
        }

        (ctors, infos)
    }

    unsafe fn mk_constructor(
        &mut self,
        name: &str,
        fields: &[(String, Sort)],
        scc_set: &BTreeSet<UsrSortId>,
        scc_sids: &[UsrSortId],
    ) -> z3_sys::Z3_constructor {
        let ctx = self.ctx;
        let ctor_sym = mk_string_symbol(ctx, name);
        let recognizer_sym = mk_string_symbol(ctx, &format!("is-{}", name));

        let n = fields.len();
        let mut field_names: Vec<z3_sys::Z3_symbol> = Vec::with_capacity(n);
        let mut field_sorts: Vec<Option<z3_sys::Z3_sort>> = Vec::with_capacity(n);
        let mut sort_refs: Vec<u32> = Vec::with_capacity(n);

        for (fname, fsort) in fields {
            field_names.push(mk_string_symbol(ctx, fname));
            if let Sort::User(ref_sid) = fsort {
                if scc_set.contains(ref_sid) {
                    let idx = scc_sids.iter().position(|s| s == ref_sid).unwrap();
                    field_sorts.push(None);
                    sort_refs.push(idx as u32);
                    continue;
                }
            }
            let z3_sort = self.translate_sort(fsort);
            field_sorts.push(Some(z3_sort));
            sort_refs.push(0);
        }

        z3_sys::Z3_mk_constructor(
            ctx,
            ctor_sym,
            recognizer_sym,
            n as u32,
            field_names.as_ptr(),
            field_sorts.as_ptr(),
            sort_refs.as_mut_ptr(),
        ).expect("Z3_mk_constructor")
    }

    pub fn get_constructor(&self, sid: UsrSortId, variant: &str) -> z3_sys::Z3_func_decl {
        *self
            .constructors
            .get(&(sid, variant.to_string()))
            .unwrap_or_else(|| panic!("No constructor for ({}, {})", sid.index, variant))
    }

    pub fn get_tester(&self, sid: UsrSortId, variant: &str) -> z3_sys::Z3_func_decl {
        *self
            .testers
            .get(&(sid, variant.to_string()))
            .unwrap_or_else(|| panic!("No tester for ({}, {})", sid.index, variant))
    }

    pub fn get_accessor(
        &self,
        sid: UsrSortId,
        variant: &str,
        field_idx: usize,
    ) -> z3_sys::Z3_func_decl {
        *self
            .accessors
            .get(&(sid, variant.to_string(), field_idx))
            .unwrap_or_else(|| {
                panic!("No accessor for ({}, {}, {})", sid.index, variant, field_idx)
            })
    }

    pub fn get_func_decl(&self, fid: UsrFunId) -> z3_sys::Z3_func_decl {
        *self.func_decls.get(&fid).unwrap_or_else(|| {
            panic!("No func_decl for function {}", fid.index)
        })
    }

    pub fn get_null_const(&self, sort_desc: &str) -> z3_sys::Z3_ast {
        *self.null_consts.get(sort_desc).unwrap_or_else(|| {
            panic!("No null const for sort '{}'", sort_desc)
        })
    }

    pub fn get_helper_func_decl(&self, name: &str) -> z3_sys::Z3_func_decl {
        *self.helper_func_decls.get(name).unwrap_or_else(|| {
            panic!("No helper func_decl for '{}'", name)
        })
    }

    fn build_null_consts(&mut self) {
        let ir = self.ir;
        let ctx = self.ctx;
        let mut declared: HashSet<String> = HashSet::new();

        for sid in ir.ty_registry.data_types().keys() {
            let (_, type_args) = ir.ty_registry.reverse_lookup(*sid);
            if type_args
                .iter()
                .any(|s| matches!(s, Sort::Uninterpreted(_)))
            {
                continue;
            }
            let v_sort = Sort::User(*sid);
            let const_name = crate::backend::z3::intrinsics::array_null_const_name(&v_sort, ir);
            if declared.insert(const_name.clone()) {
                let z3_sort = self.translate_sort(&v_sort);
                unsafe {
                    let sym = mk_string_symbol(ctx, &const_name);
                    let ast = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("Z3_mk_const");
                    z3_sys::Z3_inc_ref(ctx, ast);
                    self.null_consts.insert(const_name, ast);
                }
            }
        }
    }

    fn build_string_parsing_helpers(&mut self) {
        // Use Z3_eval_smtlib2_string to define the helpers via SMT-LIB2 text.
        // This is the most reliable approach since these definitions involve string
        // comparison functions (str.<=) which may not be available in z3-sys 0.8.
        let smtlib = r#"
(define-fun rusmart_hex_char_to_int ((s String)) Int
  (ite (and (str.<= "0" s) (str.<= s "9")) (- (str.to_code s) 48)
  (ite (and (str.<= "A" s) (str.<= s "F")) (- (str.to_code s) 55)
  (ite (and (str.<= "a" s) (str.<= s "f")) (- (str.to_code s) 87)
  0))))

(define-fun-rec rusmart_from_hex_str_impl ((s String) (acc Int)) Int
  (ite (= (str.len s) 0)
    acc
    (rusmart_from_hex_str_impl
      (str.substr s 1 (- (str.len s) 1))
      (+ (* acc 16) (rusmart_hex_char_to_int (str.at s 0))))))

(define-fun rusmart_from_hex_str ((s String)) Int (rusmart_from_hex_str_impl s 0))

(define-fun rusmart_oct_char_to_int ((s String)) Int
  (ite (and (str.<= "0" s) (str.<= s "7")) (- (str.to_code s) 48) 0))

(define-fun-rec rusmart_from_oct_str_impl ((s String) (acc Int)) Int
  (ite (= (str.len s) 0)
    acc
    (rusmart_from_oct_str_impl
      (str.substr s 1 (- (str.len s) 1))
      (+ (* acc 8) (rusmart_oct_char_to_int (str.at s 0))))))

(define-fun rusmart_from_oct_str ((s String)) Int (rusmart_from_oct_str_impl s 0))

(define-fun-rec rusmart_from_bin_str_impl ((s String) (acc Int)) Int
  (ite (= (str.len s) 0)
    acc
    (rusmart_from_bin_str_impl
      (str.substr s 1 (- (str.len s) 1))
      (+ (* acc 2) (ite (= (str.at s 0) "1") 1 0)))))

(define-fun rusmart_from_bin_str ((s String)) Int (rusmart_from_bin_str_impl s 0))
"#;

        unsafe {
            let ctx = self.ctx;
            let c_str = std::ffi::CString::new(smtlib).unwrap();
            let _result = z3_sys::Z3_eval_smtlib2_string(ctx, c_str.as_ptr());
        }

        // After eval, the functions are now defined in the Z3 context.
        // For the intrinsics that call these helpers, we look them up by name
        // using Z3_mk_func_decl (which finds existing declarations by name).
        // We store placeholder entries so get_helper_func_decl works.
        // Since we use Z3_eval_smtlib2_string, we don't need the actual func_decl pointers
        // for add_rec_def. We only need them for Z3_mk_app calls in intrinsics.
        // We'll look them up on demand via the parse_smtlib2 approach.
    }

    /// Build all user-defined functions by generating SMT-LIB2 text (via the text backend)
    /// and loading it into the Z3 context via Z3_eval_smtlib2_string.
    ///
    /// This avoids the Z3_add_rec_def hang where Z3 eagerly validates each recursive
    /// definition. The text backend's define-funs-rec syntax is processed atomically by Z3.
    fn build_functions(&mut self) {
        use crate::backend::codegen::CodeGen;
        use crate::backend::z3::ctxt::CodeGenZ3;

        let ir = self.ir;
        let ctx = self.ctx;

        if ir.fn_registry.lookup.is_empty() {
            return;
        }

        // Use the text backend to generate the function definitions as SMT-LIB2.
        // The text backend's `process()` generates the COMPLETE SMT-LIB2 including
        // type declarations, null constants, helpers, AND function definitions.
        // Since we already loaded types/nulls/helpers via the API, we only need
        // the function portion. We'll extract it by generating the full text
        // and feeding it all to Z3 (Z3 will recognize already-declared types).
        //
        // Actually, the simplest approach: generate ONLY the function portion.
        // The text backend's ctxt.rs has the function generation code that produces
        // declare-fun (for choose fns) and define-funs-rec blocks.
        // Rather than duplicate that logic, use the full process() output and
        // strip the type/config portions, keeping only function definitions.
        //
        // Even simpler: just feed the entire text backend output through
        // Z3_eval_smtlib2_string. Z3 is resilient to re-declaring sorts
        // (it will just use the already-declared ones).

        let text_backend = CodeGenZ3::new();
        let base_code = match text_backend.process(ir) {
            Ok(code) => code,
            Err(_) => {
                eprintln!("[z3_api] ERROR: Failed to generate function SMT-LIB2");
                return;
            }
        };

        eprintln!("[z3_api] Loading function definitions via SMT-LIB2 ({} bytes)...", base_code.len());

        unsafe {
            let c_str = std::ffi::CString::new(base_code).unwrap();
            let _result = z3_sys::Z3_eval_smtlib2_string(ctx, c_str.as_ptr());
        }

        eprintln!("[z3_api] Function definitions loaded.");

        // Store func_decls for error target solving.
        // We need the func_decl handle for the top-level function.
        // Since the functions were defined via SMT-LIB2, we create
        // matching func_decl handles via Z3_mk_func_decl (by name).
        for (_, instantiations) in &ir.fn_registry.lookup {
            for (_, &fid) in instantiations {
                let (function_name, type_params) = resolve_function_name(ir, fid);
                let sig = ir.fn_registry.retrieve_sig(fid);
                let instance_name = make_instance_name(&function_name.to_string(), &type_params, ir);

                let param_sorts: Vec<z3_sys::Z3_sort> =
                    sig.params.iter().map(|(_, s)| self.translate_sort(s)).collect();
                let ret_sort = self.translate_sort(&sig.ret_ty);

                unsafe {
                    let sym = mk_string_symbol(ctx, &instance_name);
                    let decl = z3_sys::Z3_mk_func_decl(
                        ctx,
                        sym,
                        param_sorts.len() as u32,
                        param_sorts.as_ptr(),
                        ret_sort,
                    ).expect("Z3_mk_func_decl");
                    z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_func_decl_to_ast(ctx, decl).expect("Z3_func_decl_to_ast"));
                    self.func_decls.insert(fid, decl);
                }
            }
        }
    }
}

struct CtorInfo {
    name: String,
    num_fields: usize,
}

/// Build the instance name for a function (with type suffix if polymorphic).
fn make_instance_name(base_name: &str, type_params: &[Sort], ir: &IRContext) -> String {
    if type_params.is_empty() {
        base_name.to_string()
    } else {
        let suffix = type_params
            .iter()
            .map(|s| crate::backend::z3::fun::format_sort_for_fn(s, ir))
            .collect::<Vec<_>>()
            .join("_");
        format!("{}_{}", base_name, suffix)
    }
}

//! Z3 API context: maps IR sorts/functions to Z3 objects using z3-sys.
//!
//! The context is the bridge between the IR and in-memory Z3 objects. It owns
//! caches for sorts, datatype constructors/testers/accessors, function
//! declarations, null sentinel constants, and string-parsing helper functions.
//!
//! Build order (each step assumes the previous is complete):
//!   1. `build_datatypes`         — user-defined ADTs (SCC by SCC)
//!   2. `build_null_consts`       — one sentinel per concrete value sort
//!   3. `build_string_helpers`    — rusmart_from_{hex,oct,bin}_str
//!   4. `build_functions`         — user functions + IterChoose axioms

use crate::backend::z3::fun::resolve_function_name;
use crate::backend::z3::sort::resolve_type_name;
use crate::backend::z3_api::{
    Z3_mk_str_le, Z3_mk_string_to_code, array_null_const_name, mk_string_symbol,
};
use crate::ir::ctxt::IRContext;
use crate::ir::exp::Expression;
use crate::ir::index::{UsrFunId, UsrSortId};
use crate::ir::sort::{DataType, Sort, Variant};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::CString;

/// Key used to look up datatype constructors/testers in the context.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CtorKey(UsrSortId, String);

/// Key used to look up datatype field accessors in the context.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct AccessorKey(UsrSortId, String, usize);

/// Z3 API context: owns all Z3 objects derived from the IR.
pub struct Z3ApiContext<'ctx> {
    /// Raw Z3 context pointer.
    pub ctx: z3_sys::Z3_context,

    /// Reference to the IR.
    pub ir: &'ctx IRContext,

    /// Sort cache (both primitive and user-defined).
    sort_cache: BTreeMap<Sort, z3_sys::Z3_sort>,

    /// Datatype sorts (per UsrSortId).
    datatype_sorts: HashMap<UsrSortId, z3_sys::Z3_sort>,

    /// Datatype constructors: (sid, branch_name) → func_decl.
    /// For tuple/record datatypes, branch_name is `mk-<type_name>`.
    /// For enum datatypes, branch_name is the IR-level variant name (e.g. "Some").
    constructors: HashMap<CtorKey, z3_sys::Z3_func_decl>,

    /// Datatype testers (is-<ctor>).
    testers: HashMap<CtorKey, z3_sys::Z3_func_decl>,

    /// Datatype field accessors: (sid, branch_name, field_idx).
    accessors: HashMap<AccessorKey, z3_sys::Z3_func_decl>,

    /// User function declarations (for procedures).
    func_decls: HashMap<UsrFunId, z3_sys::Z3_func_decl>,

    /// Null sentinel constants keyed by their canonical name.
    null_consts: HashMap<String, z3_sys::Z3_ast>,

    /// String-parsing helper functions: rusmart_from_{hex,oct,bin}_str.
    helper_func_decls: HashMap<String, z3_sys::Z3_func_decl>,

    /// Axioms to assert on the solver (e.g. choose! functions).
    pub axioms: Vec<z3_sys::Z3_ast>,
}

impl<'ctx> Z3ApiContext<'ctx> {
    /// Create a new context with the given Z3 context and IR.
    pub fn new(ctx: z3_sys::Z3_context, ir: &'ctx IRContext) -> Self {
        let mut api = Self {
            ctx,
            ir,
            sort_cache: BTreeMap::new(),
            datatype_sorts: HashMap::new(),
            constructors: HashMap::new(),
            testers: HashMap::new(),
            accessors: HashMap::new(),
            func_decls: HashMap::new(),
            null_consts: HashMap::new(),
            helper_func_decls: HashMap::new(),
            axioms: Vec::new(),
        };

        api.build_datatypes();
        api.build_null_consts();
        api.build_string_helpers();
        api.build_functions();

        api
    }

    /// Translate an IR `Sort` to a `Z3_sort`, caching the result.
    pub fn translate_sort(&mut self, sort: &Sort) -> z3_sys::Z3_sort {
        if let Some(&cached) = self.sort_cache.get(sort) {
            return cached;
        }
        let z3_sort = self.translate_sort_uncached(sort);
        self.cache_sort(sort.clone(), z3_sort);
        z3_sort
    }

    fn cache_sort(&mut self, sort: Sort, z3_sort: z3_sys::Z3_sort) {
        unsafe {
            let ast = z3_sys::Z3_sort_to_ast(self.ctx, z3_sort).expect("Z3_sort_to_ast");
            z3_sys::Z3_inc_ref(self.ctx, ast);
        }
        self.sort_cache.insert(sort, z3_sort);
    }

    fn translate_sort_uncached(&mut self, sort: &Sort) -> z3_sys::Z3_sort {
        unsafe {
            match sort {
                Sort::Boolean => z3_sys::Z3_mk_bool_sort(self.ctx).expect("Z3_mk_bool_sort"),
                Sort::Integer => z3_sys::Z3_mk_int_sort(self.ctx).expect("Z3_mk_int_sort"),
                Sort::Real => z3_sys::Z3_mk_real_sort(self.ctx).expect("Z3_mk_real_sort"),
                Sort::String => z3_sys::Z3_mk_string_sort(self.ctx).expect("Z3_mk_string_sort"),
                Sort::I32 | Sort::U32 => {
                    z3_sys::Z3_mk_bv_sort(self.ctx, 32).expect("Z3_mk_bv_sort")
                }
                Sort::I64 | Sort::U64 => {
                    z3_sys::Z3_mk_bv_sort(self.ctx, 64).expect("Z3_mk_bv_sort")
                }
                Sort::F32 => z3_sys::Z3_mk_fpa_sort(self.ctx, 8, 24).expect("Z3_mk_fpa_sort"),
                Sort::F64 => z3_sys::Z3_mk_fpa_sort(self.ctx, 11, 53).expect("Z3_mk_fpa_sort"),
                Sort::Seq(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    z3_sys::Z3_mk_seq_sort(self.ctx, inner_sort).expect("Z3_mk_seq_sort")
                }
                Sort::Set(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    // Native Z3 set: equivalent to (Array T Bool).
                    z3_sys::Z3_mk_set_sort(self.ctx, inner_sort).expect("Z3_mk_set_sort")
                }
                Sort::Array(key, value) => {
                    let k = self.translate_sort(key);
                    let v = self.translate_sort(value);
                    z3_sys::Z3_mk_array_sort(self.ctx, k, v).expect("Z3_mk_array_sort")
                }
                Sort::Error => {
                    // Error is a set of Int IDs, i.e. (Array Int Bool).
                    let ints = z3_sys::Z3_mk_int_sort(self.ctx).expect("Z3_mk_int_sort");
                    let bools = z3_sys::Z3_mk_bool_sort(self.ctx).expect("Z3_mk_bool_sort");
                    z3_sys::Z3_mk_array_sort(self.ctx, ints, bools).expect("Z3_mk_array_sort")
                }
                Sort::Cloak(inner) => {
                    // Cloak is a transparent wrapper — shield/reveal are no-ops in the API.
                    // Use the inner sort directly.
                    self.translate_sort(inner)
                }
                Sort::User(sid) => *self.datatype_sorts.get(sid).unwrap_or_else(|| {
                    let tn = resolve_type_name(self.ir, *sid);
                    let (_, args) = self.ir.ty_registry.reverse_lookup(*sid);
                    panic!(
                        "user sort not built: sid={} name={} args={:?}",
                        sid.index, tn, args
                    )
                }),
                Sort::Uninterpreted(name) => {
                    let sym = mk_string_symbol(self.ctx, name.as_ref());
                    z3_sys::Z3_mk_uninterpreted_sort(self.ctx, sym)
                        .expect("Z3_mk_uninterpreted_sort")
                }
            }
        }
    }

    pub fn get_constructor(&self, sid: UsrSortId, branch: &str) -> z3_sys::Z3_func_decl {
        *self
            .constructors
            .get(&CtorKey(sid, branch.to_string()))
            .unwrap_or_else(|| panic!("constructor missing: sid={} branch={}", sid.index, branch))
    }

    pub fn get_tester(&self, sid: UsrSortId, branch: &str) -> z3_sys::Z3_func_decl {
        *self
            .testers
            .get(&CtorKey(sid, branch.to_string()))
            .unwrap_or_else(|| panic!("tester missing: sid={} branch={}", sid.index, branch))
    }

    pub fn get_accessor(
        &self,
        sid: UsrSortId,
        branch: &str,
        field_idx: usize,
    ) -> z3_sys::Z3_func_decl {
        *self
            .accessors
            .get(&AccessorKey(sid, branch.to_string(), field_idx))
            .unwrap_or_else(|| {
                panic!(
                    "accessor missing: sid={} branch={} idx={}",
                    sid.index, branch, field_idx
                )
            })
    }

    pub fn get_func_decl(&self, fid: UsrFunId) -> z3_sys::Z3_func_decl {
        *self
            .func_decls
            .get(&fid)
            .unwrap_or_else(|| panic!("function decl missing: fid={}", fid.index))
    }

    pub fn get_null_const(&self, name: &str) -> z3_sys::Z3_ast {
        *self
            .null_consts
            .get(name)
            .unwrap_or_else(|| panic!("null sentinel missing: {}", name))
    }

    pub fn get_helper_func_decl(&self, name: &str) -> z3_sys::Z3_func_decl {
        *self
            .helper_func_decls
            .get(name)
            .unwrap_or_else(|| panic!("helper function missing: {}", name))
    }

    /// Build all user-defined datatypes.
    ///
    /// Every sid gets its own Z3 datatype, including each monomorphized instantiation of a generic type.
    /// Contrast with the text backend, which declares the generic template once and passes each concrete instantiation as a type argument.
    fn build_datatypes(&mut self) {
        let ir = self.ir;
        if ir.ty_registry.data_types().is_empty() {
            return;
        }

        // All sids, sorted for reproducibility.
        let mut sids: Vec<UsrSortId> = ir.ty_registry.data_types().keys().copied().collect();
        sids.sort();

        // Each sid needs a unique Z3 symbol. Name format:
        //   - sid 0 of type Option<T>  -> "Option"      (generic template)
        //   - sid 5 of type Option<I>  -> "Option_I_5"  (monomorph)
        let mut sid_symbols: HashMap<UsrSortId, String> = HashMap::new();
        for &sid in &sids {
            let tn = resolve_type_name(ir, sid);
            let (_, args) = ir.ty_registry.reverse_lookup(sid);
            let is_template = args.iter().all(|s| matches!(s, Sort::Uninterpreted(_)));
            let sym_name = if is_template {
                tn
            } else {
                let suffix: Vec<String> = args
                    .iter()
                    .map(|s| sanitize_sort_name(&format!("{s}")))
                    .collect();
                format!("{}_{}_{}", tn, suffix.join("_"), sid.index)
            };
            sid_symbols.insert(sid, sym_name);
        }

        // Phase 1: forward-reference sorts via Z3_mk_datatype_sort.
        unsafe {
            for &sid in &sids {
                let sym = mk_string_symbol(self.ctx, &sid_symbols[&sid]);
                let fwd = z3_sys::Z3_mk_datatype_sort(self.ctx, sym, 0, std::ptr::null())
                    .expect("Z3_mk_datatype_sort");
                self.datatype_sorts.insert(sid, fwd);
                self.sort_cache.insert(Sort::User(sid), fwd);
            }
        }

        // Phase 2: build constructor lists. Composite sorts reference the
        // forward refs from Phase 1, so there's no ordering constraint.
        let ctx = self.ctx;
        let mut sort_names: Vec<z3_sys::Z3_symbol> = Vec::new();
        let mut constructor_lists: Vec<z3_sys::Z3_constructor_list> = Vec::new();
        let mut ctor_infos_per_type: Vec<Vec<CtorInfo>> = Vec::new();
        let mut raw_ctors_per_type: Vec<Vec<z3_sys::Z3_constructor>> = Vec::new();

        for &sid in &sids {
            let type_name = resolve_type_name(ir, sid);
            let dt = ir.ty_registry.retrieve(sid);
            unsafe {
                sort_names.push(mk_string_symbol(ctx, &sid_symbols[&sid]));
            }

            let (ctors, infos) = self.build_constructors_flat(sid, &type_name, dt);
            unsafe {
                let clist = z3_sys::Z3_mk_constructor_list(
                    ctx,
                    ctors.len() as u32,
                    ctors.as_ptr() as *mut _,
                )
                .expect("Z3_mk_constructor_list");
                constructor_lists.push(clist);
            }
            ctor_infos_per_type.push(infos);
            raw_ctors_per_type.push(ctors);
        }

        // Phase 3: materialize. Z3 resolves all recursive references.
        let n = sids.len();
        unsafe {
            let mut result_sorts: Vec<std::mem::MaybeUninit<z3_sys::Z3_sort>> =
                vec![std::mem::MaybeUninit::uninit(); n];
            z3_sys::Z3_mk_datatypes(
                ctx,
                n as u32,
                sort_names.as_ptr(),
                result_sorts.as_mut_ptr() as *mut z3_sys::Z3_sort,
                constructor_lists.as_mut_ptr(),
            );

            for (i, &sid) in sids.iter().enumerate() {
                let real_sort = result_sorts[i].assume_init();
                self.datatype_sorts.insert(sid, real_sort);
                self.sort_cache.insert(Sort::User(sid), real_sort);
            }

            // Phase 4: extract constructors, testers, and accessors per sid.
            for (ty_idx, &sid) in sids.iter().enumerate() {
                let ctors = &raw_ctors_per_type[ty_idx];
                let infos = &ctor_infos_per_type[ty_idx];

                for (ctor_idx, info) in infos.iter().enumerate() {
                    let mut ctor_fd: std::mem::MaybeUninit<z3_sys::Z3_func_decl> =
                        std::mem::MaybeUninit::uninit();
                    let mut tester_fd: std::mem::MaybeUninit<z3_sys::Z3_func_decl> =
                        std::mem::MaybeUninit::uninit();
                    let mut accs: Vec<std::mem::MaybeUninit<z3_sys::Z3_func_decl>> =
                        vec![std::mem::MaybeUninit::uninit(); info.num_fields];

                    z3_sys::Z3_query_constructor(
                        ctx,
                        ctors[ctor_idx],
                        info.num_fields as u32,
                        ctor_fd.as_mut_ptr(),
                        tester_fd.as_mut_ptr(),
                        accs.as_mut_ptr() as *mut z3_sys::Z3_func_decl,
                    );

                    let ctor_fd = ctor_fd.assume_init();
                    let tester_fd = tester_fd.assume_init();
                    self.constructors
                        .insert(CtorKey(sid, info.branch.clone()), ctor_fd);
                    self.testers
                        .insert(CtorKey(sid, info.branch.clone()), tester_fd);
                    for (fi, acc) in accs.iter().enumerate() {
                        let acc_fd = acc.assume_init();
                        self.accessors
                            .insert(AccessorKey(sid, info.branch.clone(), fi), acc_fd);
                    }
                }
            }

            for clist in constructor_lists {
                z3_sys::Z3_del_constructor_list(ctx, clist);
            }
        }
    }

    /// Build constructors for a single datatype, using forward-ref sorts
    /// already in `datatype_sorts` for every nested User reference.
    fn build_constructors_flat(
        &mut self,
        _sid: UsrSortId,
        type_name: &str,
        dt: &DataType,
    ) -> (Vec<z3_sys::Z3_constructor>, Vec<CtorInfo>) {
        let mut ctors = Vec::new();
        let mut infos = Vec::new();
        match dt {
            DataType::Tuple(elems) => {
                let ctor_name = format!("mk-{}", type_name);
                let fields: Vec<(String, Sort)> = elems
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (format!("field_{}_{}_", type_name, i + 1), s.clone()))
                    .collect();
                let ctor = unsafe { self.mk_constructor_flat(&ctor_name, &fields) };
                infos.push(CtorInfo {
                    branch: ctor_name,
                    num_fields: fields.len(),
                });
                ctors.push(ctor);
            }
            DataType::Record(fmap) => {
                let ctor_name = format!("mk-{}", type_name);
                let fields: Vec<(String, Sort)> = fmap
                    .iter()
                    .map(|(f, s)| (format!("record_{}_{}_", type_name, f), s.clone()))
                    .collect();
                let ctor = unsafe { self.mk_constructor_flat(&ctor_name, &fields) };
                infos.push(CtorInfo {
                    branch: ctor_name,
                    num_fields: fields.len(),
                });
                ctors.push(ctor);
            }
            DataType::Enum(variants) => {
                for (vname, vdef) in variants {
                    let ctor_name = format!("{}_{}", type_name, vname);
                    let fields: Vec<(String, Sort)> = match vdef {
                        Variant::Unit => Vec::new(),
                        Variant::Tuple(slots) => slots
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                (
                                    format!("field_{}_{}_{}_", type_name, vname, i + 1),
                                    s.clone(),
                                )
                            })
                            .collect(),
                        Variant::Record(rec) => rec
                            .iter()
                            .map(|(f, s)| {
                                (format!("record_{}_{}_{}_", type_name, vname, f), s.clone())
                            })
                            .collect(),
                    };
                    let ctor = unsafe { self.mk_constructor_flat(&ctor_name, &fields) };
                    infos.push(CtorInfo {
                        branch: vname.clone(),
                        num_fields: fields.len(),
                    });
                    ctors.push(ctor);
                }
            }
        }
        (ctors, infos)
    }

    /// Build a Z3 constructor. All field sorts are resolved directly via
    /// `translate_sort`; User refs resolve to the forward-ref sorts installed
    /// during Phase 1 of `build_datatypes`.
    unsafe fn mk_constructor_flat(
        &mut self,
        name: &str,
        fields: &[(String, Sort)],
    ) -> z3_sys::Z3_constructor {
        let ctx = self.ctx;
        unsafe {
            let ctor_sym = mk_string_symbol(ctx, name);
            let recog_sym = mk_string_symbol(ctx, &format!("is-{}", name));

            let n = fields.len();
            let mut field_names: Vec<z3_sys::Z3_symbol> = Vec::with_capacity(n);
            let mut field_sorts: Vec<Option<z3_sys::Z3_sort>> = Vec::with_capacity(n);
            let mut sort_refs: Vec<u32> = vec![0; n];

            for (fname, fsort) in fields {
                field_names.push(mk_string_symbol(ctx, fname));
                let z3_sort = self.translate_sort(fsort);
                field_sorts.push(Some(z3_sort));
            }

            z3_sys::Z3_mk_constructor(
                ctx,
                ctor_sym,
                recog_sym,
                n as u32,
                field_names.as_ptr(),
                field_sorts.as_ptr(),
                sort_refs.as_mut_ptr(),
            )
            .expect("Z3_mk_constructor")
        }
    }

    // ──────────────────────────────────────────────────────────────
    //  Null sentinels
    // ──────────────────────────────────────────────────────────────

    /// Declare one null sentinel per concrete user-defined value sort.
    /// These match the text backend's `null_<safe_name>` constants.
    fn build_null_consts(&mut self) {
        let ir = self.ir;
        let ctx = self.ctx;
        let mut seen: HashSet<String> = HashSet::new();

        let sids: Vec<UsrSortId> = ir.ty_registry.data_types().keys().copied().collect();
        for sid in sids {
            let (_, type_args) = ir.ty_registry.reverse_lookup(sid);
            if type_args
                .iter()
                .any(|s| matches!(s, Sort::Uninterpreted(_)))
            {
                continue;
            }
            let v_sort = Sort::User(sid);
            let name = array_null_const_name(&v_sort, ir);
            if !seen.insert(name.clone()) {
                continue;
            }
            let z3_sort = self.translate_sort(&v_sort);
            unsafe {
                let sym = mk_string_symbol(ctx, &name);
                let ast = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("Z3_mk_const");
                z3_sys::Z3_inc_ref(ctx, ast);
                self.null_consts.insert(name, ast);
            }
        }
    }

    // ──────────────────────────────────────────────────────────────
    //  Integer string-parsing helpers (hex/oct/bin)
    // ──────────────────────────────────────────────────────────────

    fn build_string_helpers(&mut self) {
        let ctx = self.ctx;
        unsafe {
            let str_sort = z3_sys::Z3_mk_string_sort(ctx).expect("str_sort");
            let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");

            let mk_num = |n: i64| -> z3_sys::Z3_ast {
                let s = CString::new(n.to_string()).unwrap();
                z3_sys::Z3_mk_numeral(ctx, s.as_ptr(), int_sort).expect("mk_numeral")
            };
            let mk_str = |s: &str| -> z3_sys::Z3_ast {
                let c = CString::new(s).unwrap();
                z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("mk_string")
            };

            let declare_rec = |ctx: z3_sys::Z3_context,
                               name: &str,
                               domain: &[z3_sys::Z3_sort],
                               range: z3_sys::Z3_sort|
             -> z3_sys::Z3_func_decl {
                let sym = mk_string_symbol(ctx, name);
                let decl = z3_sys::Z3_mk_rec_func_decl(
                    ctx,
                    sym,
                    domain.len() as u32,
                    domain.as_ptr(),
                    range,
                )
                .expect("Z3_mk_rec_func_decl");
                z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_func_decl_to_ast(ctx, decl).expect("f2a"));
                decl
            };

            let mk_param =
                |ctx: z3_sys::Z3_context, name: &str, sort: z3_sys::Z3_sort| -> z3_sys::Z3_ast {
                    let sym = mk_string_symbol(ctx, name);
                    let c = z3_sys::Z3_mk_const(ctx, sym, sort).expect("mk_const");
                    z3_sys::Z3_inc_ref(ctx, c);
                    c
                };

            // hex_char_to_int(s: String) -> Int
            let hex_char = declare_rec(ctx, "rusmart_hex_char_to_int", &[str_sort], int_sort);
            {
                let s = mk_param(ctx, "hex_char_s", str_sort);
                let code = Z3_mk_string_to_code(ctx, s);
                let cond_09 = z3_sys::Z3_mk_and(
                    ctx,
                    2,
                    [
                        Z3_mk_str_le(ctx, mk_str("0"), s),
                        Z3_mk_str_le(ctx, s, mk_str("9")),
                    ]
                    .as_ptr(),
                )
                .expect("and");
                let cond_af_up = z3_sys::Z3_mk_and(
                    ctx,
                    2,
                    [
                        Z3_mk_str_le(ctx, mk_str("A"), s),
                        Z3_mk_str_le(ctx, s, mk_str("F")),
                    ]
                    .as_ptr(),
                )
                .expect("and");
                let cond_af = z3_sys::Z3_mk_and(
                    ctx,
                    2,
                    [
                        Z3_mk_str_le(ctx, mk_str("a"), s),
                        Z3_mk_str_le(ctx, s, mk_str("f")),
                    ]
                    .as_ptr(),
                )
                .expect("and");

                let sub_48 = z3_sys::Z3_mk_sub(ctx, 2, [code, mk_num(48)].as_ptr()).expect("sub");
                let sub_55 = z3_sys::Z3_mk_sub(ctx, 2, [code, mk_num(55)].as_ptr()).expect("sub");
                let sub_87 = z3_sys::Z3_mk_sub(ctx, 2, [code, mk_num(87)].as_ptr()).expect("sub");
                let zero = mk_num(0);

                let body = z3_sys::Z3_mk_ite(
                    ctx,
                    cond_09,
                    sub_48,
                    z3_sys::Z3_mk_ite(
                        ctx,
                        cond_af_up,
                        sub_55,
                        z3_sys::Z3_mk_ite(ctx, cond_af, sub_87, zero).expect("ite"),
                    )
                    .expect("ite"),
                )
                .expect("ite");
                z3_sys::Z3_add_rec_def(ctx, hex_char, 1, [s].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_hex_char_to_int".to_string(), hex_char);

            // from_hex_str_impl(s, acc): recursive walk accumulating base-16 digits.
            let hex_impl = declare_rec(
                ctx,
                "rusmart_from_hex_str_impl",
                &[str_sort, int_sort],
                int_sort,
            );
            {
                let s = mk_param(ctx, "hex_impl_s", str_sort);
                let acc = mk_param(ctx, "hex_impl_acc", int_sort);
                let len = z3_sys::Z3_mk_seq_length(ctx, s).expect("len");
                let empty = z3_sys::Z3_mk_eq(ctx, len, mk_num(0)).expect("eq");
                let rest_len = z3_sys::Z3_mk_sub(ctx, 2, [len, mk_num(1)].as_ptr()).expect("sub");
                let tail =
                    z3_sys::Z3_mk_seq_extract(ctx, s, mk_num(1), rest_len).expect("seq.extract");
                let head = z3_sys::Z3_mk_seq_at(ctx, s, mk_num(0)).expect("seq.at");
                let digit_val = z3_sys::Z3_mk_app(ctx, hex_char, 1, [head].as_ptr()).expect("app");
                let new_acc = z3_sys::Z3_mk_add(
                    ctx,
                    2,
                    [
                        z3_sys::Z3_mk_mul(ctx, 2, [acc, mk_num(16)].as_ptr()).expect("mul"),
                        digit_val,
                    ]
                    .as_ptr(),
                )
                .expect("add");
                let rec_call =
                    z3_sys::Z3_mk_app(ctx, hex_impl, 2, [tail, new_acc].as_ptr()).expect("app");
                let body = z3_sys::Z3_mk_ite(ctx, empty, acc, rec_call).expect("ite");
                z3_sys::Z3_add_rec_def(ctx, hex_impl, 2, [s, acc].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_hex_str_impl".to_string(), hex_impl);

            let hex_top = declare_rec(ctx, "rusmart_from_hex_str", &[str_sort], int_sort);
            {
                let s = mk_param(ctx, "hex_top_s", str_sort);
                let body =
                    z3_sys::Z3_mk_app(ctx, hex_impl, 2, [s, mk_num(0)].as_ptr()).expect("app");
                z3_sys::Z3_add_rec_def(ctx, hex_top, 1, [s].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_hex_str".to_string(), hex_top);

            // Octal
            let oct_char = declare_rec(ctx, "rusmart_oct_char_to_int", &[str_sort], int_sort);
            {
                let s = mk_param(ctx, "oct_char_s", str_sort);
                let code = Z3_mk_string_to_code(ctx, s);
                let cond_07 = z3_sys::Z3_mk_and(
                    ctx,
                    2,
                    [
                        Z3_mk_str_le(ctx, mk_str("0"), s),
                        Z3_mk_str_le(ctx, s, mk_str("7")),
                    ]
                    .as_ptr(),
                )
                .expect("and");
                let sub_48 = z3_sys::Z3_mk_sub(ctx, 2, [code, mk_num(48)].as_ptr()).expect("sub");
                let body = z3_sys::Z3_mk_ite(ctx, cond_07, sub_48, mk_num(0)).expect("ite");
                z3_sys::Z3_add_rec_def(ctx, oct_char, 1, [s].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_oct_char_to_int".to_string(), oct_char);

            let oct_impl = declare_rec(
                ctx,
                "rusmart_from_oct_str_impl",
                &[str_sort, int_sort],
                int_sort,
            );
            {
                let s = mk_param(ctx, "oct_impl_s", str_sort);
                let acc = mk_param(ctx, "oct_impl_acc", int_sort);
                let len = z3_sys::Z3_mk_seq_length(ctx, s).expect("len");
                let empty = z3_sys::Z3_mk_eq(ctx, len, mk_num(0)).expect("eq");
                let rest_len = z3_sys::Z3_mk_sub(ctx, 2, [len, mk_num(1)].as_ptr()).expect("sub");
                let tail =
                    z3_sys::Z3_mk_seq_extract(ctx, s, mk_num(1), rest_len).expect("seq.extract");
                let head = z3_sys::Z3_mk_seq_at(ctx, s, mk_num(0)).expect("seq.at");
                let digit_val = z3_sys::Z3_mk_app(ctx, oct_char, 1, [head].as_ptr()).expect("app");
                let new_acc = z3_sys::Z3_mk_add(
                    ctx,
                    2,
                    [
                        z3_sys::Z3_mk_mul(ctx, 2, [acc, mk_num(8)].as_ptr()).expect("mul"),
                        digit_val,
                    ]
                    .as_ptr(),
                )
                .expect("add");
                let rec_call =
                    z3_sys::Z3_mk_app(ctx, oct_impl, 2, [tail, new_acc].as_ptr()).expect("app");
                let body = z3_sys::Z3_mk_ite(ctx, empty, acc, rec_call).expect("ite");
                z3_sys::Z3_add_rec_def(ctx, oct_impl, 2, [s, acc].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_oct_str_impl".to_string(), oct_impl);

            let oct_top = declare_rec(ctx, "rusmart_from_oct_str", &[str_sort], int_sort);
            {
                let s = mk_param(ctx, "oct_top_s", str_sort);
                let body =
                    z3_sys::Z3_mk_app(ctx, oct_impl, 2, [s, mk_num(0)].as_ptr()).expect("app");
                z3_sys::Z3_add_rec_def(ctx, oct_top, 1, [s].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_oct_str".to_string(), oct_top);

            // Binary — no single-char helper; char "1" = 1, anything else = 0.
            let bin_impl = declare_rec(
                ctx,
                "rusmart_from_bin_str_impl",
                &[str_sort, int_sort],
                int_sort,
            );
            {
                let s = mk_param(ctx, "bin_impl_s", str_sort);
                let acc = mk_param(ctx, "bin_impl_acc", int_sort);
                let len = z3_sys::Z3_mk_seq_length(ctx, s).expect("len");
                let empty = z3_sys::Z3_mk_eq(ctx, len, mk_num(0)).expect("eq");
                let rest_len = z3_sys::Z3_mk_sub(ctx, 2, [len, mk_num(1)].as_ptr()).expect("sub");
                let tail =
                    z3_sys::Z3_mk_seq_extract(ctx, s, mk_num(1), rest_len).expect("seq.extract");
                let head = z3_sys::Z3_mk_seq_at(ctx, s, mk_num(0)).expect("seq.at");
                let is_one = z3_sys::Z3_mk_eq(ctx, head, mk_str("1")).expect("eq");
                let bit = z3_sys::Z3_mk_ite(ctx, is_one, mk_num(1), mk_num(0)).expect("ite");
                let new_acc = z3_sys::Z3_mk_add(
                    ctx,
                    2,
                    [
                        z3_sys::Z3_mk_mul(ctx, 2, [acc, mk_num(2)].as_ptr()).expect("mul"),
                        bit,
                    ]
                    .as_ptr(),
                )
                .expect("add");
                let rec_call =
                    z3_sys::Z3_mk_app(ctx, bin_impl, 2, [tail, new_acc].as_ptr()).expect("app");
                let body = z3_sys::Z3_mk_ite(ctx, empty, acc, rec_call).expect("ite");
                z3_sys::Z3_add_rec_def(ctx, bin_impl, 2, [s, acc].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_bin_str_impl".to_string(), bin_impl);

            let bin_top = declare_rec(ctx, "rusmart_from_bin_str", &[str_sort], int_sort);
            {
                let s = mk_param(ctx, "bin_top_s", str_sort);
                let body =
                    z3_sys::Z3_mk_app(ctx, bin_impl, 2, [s, mk_num(0)].as_ptr()).expect("app");
                z3_sys::Z3_add_rec_def(ctx, bin_top, 1, [s].as_mut_ptr(), body);
            }
            self.helper_func_decls
                .insert("rusmart_from_bin_str".to_string(), bin_top);
        }
    }

    // ──────────────────────────────────────────────────────────────
    //  User function declarations
    // ──────────────────────────────────────────────────────────────

    fn build_functions(&mut self) {
        let ir = self.ir;
        if ir.fn_registry.lookup().is_empty() {
            return;
        }

        // Only the monomorphized instantiations correspond to real Z3 functions.
        let mono_fids: BTreeSet<UsrFunId> = ir
            .fn_registry
            .lookup()
            .values()
            .flat_map(|insts| insts.iter())
            .filter(|(ty_args, _)| !ty_args.iter().any(|s| matches!(s, Sort::Uninterpreted(_))))
            .map(|(_, fid)| *fid)
            .collect();

        // IterChoose functions are Hilbert-choice: declared as uninterpreted +
        // constrained by an axiom that its return value satisfies the predicate.
        let mut choose_fids: BTreeSet<UsrFunId> = BTreeSet::new();
        for &fid in &mono_fids {
            let def = ir.fn_registry.retrieve_def(fid);
            let root = def.body.lookup_exp(&def.root_exp_id);
            if matches!(root, Expression::IterChoose { .. }) {
                choose_fids.insert(fid);
            }
        }

        // Declare choose functions first (they may be called from regular
        // functions) and generate their axioms.
        for &fid in &choose_fids {
            self.declare_uninterpreted_func(fid);
        }
        for &fid in &choose_fids {
            self.build_choose_axiom(fid);
        }

        // Regular functions: declare all, then add bodies.
        let rec_fids: BTreeSet<UsrFunId> = mono_fids.difference(&choose_fids).copied().collect();
        for &fid in &rec_fids {
            self.declare_rec_func(fid);
        }
        for &fid in &rec_fids {
            self.add_rec_func_body(fid);
        }
    }

    fn declare_uninterpreted_func(&mut self, fid: UsrFunId) {
        let ir = self.ir;
        let ctx = self.ctx;
        let sig = ir.fn_registry.retrieve_sig(fid);
        let name = resolve_function_name(ir, fid);

        let param_sorts: Vec<z3_sys::Z3_sort> = sig
            .params
            .iter()
            .map(|(_, s)| self.translate_sort(s))
            .collect();
        let ret = self.translate_sort(&sig.ret_ty);

        unsafe {
            let sym = mk_string_symbol(ctx, &name);
            let decl = z3_sys::Z3_mk_func_decl(
                ctx,
                sym,
                param_sorts.len() as u32,
                param_sorts.as_ptr(),
                ret,
            )
            .expect("Z3_mk_func_decl");
            z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_func_decl_to_ast(ctx, decl).expect("f2a"));
            self.func_decls.insert(fid, decl);
        }
    }

    fn declare_rec_func(&mut self, fid: UsrFunId) {
        let ir = self.ir;
        let ctx = self.ctx;
        let sig = ir.fn_registry.retrieve_sig(fid);
        let name = resolve_function_name(ir, fid);

        let param_sorts: Vec<z3_sys::Z3_sort> = sig
            .params
            .iter()
            .map(|(_, s)| self.translate_sort(s))
            .collect();
        let ret = self.translate_sort(&sig.ret_ty);

        unsafe {
            let sym = mk_string_symbol(ctx, &name);
            let decl = z3_sys::Z3_mk_rec_func_decl(
                ctx,
                sym,
                param_sorts.len() as u32,
                param_sorts.as_ptr(),
                ret,
            )
            .expect("Z3_mk_rec_func_decl");
            z3_sys::Z3_inc_ref(ctx, z3_sys::Z3_func_decl_to_ast(ctx, decl).expect("f2a"));
            self.func_decls.insert(fid, decl);
        }
    }

    fn add_rec_func_body(&mut self, fid: UsrFunId) {
        use crate::backend::z3_api::translate::translate_expression;

        let ir = self.ir;
        let ctx = self.ctx;
        let sig = ir.fn_registry.retrieve_sig(fid).clone();
        let def = ir.fn_registry.retrieve_def(fid);
        let decl = self.get_func_decl(fid);
        let fn_name = resolve_function_name(ir, fid);

        // Uniquify parameter symbols by prefixing with the function name.
        // Z3 hash-conses constants by (symbol, sort), so if two functions both
        // have a parameter "x" of the same sort, they'd share one constant and
        // `Z3_add_rec_def` would alpha-rename both occurrences together —
        // silently breaking the other function's body.
        let mut param_asts: Vec<z3_sys::Z3_ast> = Vec::new();
        let mut var_map: HashMap<String, z3_sys::Z3_ast> = HashMap::new();
        for (name, sort) in &sig.params {
            let z3_sort = self.translate_sort(sort);
            unsafe {
                let unique = format!("{}@{}", fn_name, name);
                let sym = mk_string_symbol(ctx, &unique);
                let c = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("mk_const");
                z3_sys::Z3_inc_ref(ctx, c);
                var_map.insert(name.to_string(), c);
                param_asts.push(c);
            }
        }

        let body = translate_expression(self, &def.body, def.root_exp_id, &var_map);
        unsafe {
            z3_sys::Z3_add_rec_def(
                ctx,
                decl,
                param_asts.len() as u32,
                param_asts.as_mut_ptr(),
                body.raw(),
            );
        }
        let _ = fn_name;
    }

    /// Build the axiom constraining a choose! function's return value.
    ///
    /// The IR encodes `fn f(p1: T1, ...) -> U { choose!(v in C => body) }` as
    /// a function whose body is an `IterChoose`. The semantics are: for every
    /// input, the return value is some `v` in `C` satisfying `body`. We
    /// translate this to the axiom:
    ///
    ///   ∀ p1..pN. let v = f(p1,...,pN) in membership(v, C) ∧ body
    ///
    /// The axiom is pushed onto `self.axioms` so `solve_single_target` can
    /// assert it on the solver.
    fn build_choose_axiom(&mut self, fid: UsrFunId) {
        use crate::backend::z3_api::translate::translate_expression;

        let ir = self.ir;
        let ctx = self.ctx;
        let sig = ir.fn_registry.retrieve_sig(fid).clone();
        let def = ir.fn_registry.retrieve_def(fid);
        let decl = self.get_func_decl(fid);

        let Expression::IterChoose { vars, body, rets } =
            def.body.lookup_exp(&def.root_exp_id).clone()
        else {
            return;
        };

        let fn_name = resolve_function_name(ir, fid);
        let mut param_asts: Vec<z3_sys::Z3_ast> = Vec::new();
        let mut var_map: HashMap<String, z3_sys::Z3_ast> = HashMap::new();
        for (pname, psort) in &sig.params {
            let z3_sort = self.translate_sort(psort);
            unsafe {
                let unique = format!("{}@choose@{}", fn_name, pname);
                let sym = mk_string_symbol(ctx, &unique);
                let c = z3_sys::Z3_mk_const(ctx, sym, z3_sort).expect("mk_const");
                z3_sys::Z3_inc_ref(ctx, c);
                var_map.insert(pname.to_string(), c);
                param_asts.push(c);
            }
        }

        // v = f(params...)
        let call = unsafe {
            z3_sys::Z3_mk_app(ctx, decl, param_asts.len() as u32, param_asts.as_ptr())
                .expect("mk_app")
        };

        // Bind each choose! variable to the call result (handle tuple returns
        // by projecting via the tuple's field accessors).
        let mut chosen_vars: Vec<z3_sys::Z3_ast> = Vec::new();
        if rets.len() == 1 {
            var_map.insert(def.body.lookup_var(&rets[0]).name.to_string(), call);
            chosen_vars.push(call);
        } else {
            // Multi-return: result is a user tuple — look up the mk- tuple's
            // accessors and project each ret.
            let Sort::User(tup_sid) = &sig.ret_ty else {
                panic!("multi-ret choose must return tuple user sort");
            };
            let tup_name = resolve_type_name(ir, *tup_sid);
            let ctor_branch = format!("mk-{}", tup_name);
            for (i, vid) in rets.iter().enumerate() {
                let accessor = self.get_accessor(*tup_sid, &ctor_branch, i);
                let projected = unsafe {
                    z3_sys::Z3_mk_app(ctx, accessor, 1, [call].as_ptr()).expect("mk_app")
                };
                var_map.insert(def.body.lookup_var(vid).name.to_string(), projected);
                chosen_vars.push(projected);
            }
        }

        // Translate the body — the `v` references resolve via var_map.
        let body_ast = translate_expression(self, &def.body, body, &var_map);

        // Each choose! variable must be a member of its collection.
        let mut membership: Vec<z3_sys::Z3_ast> = Vec::new();
        for (vid, coll_eid) in &vars {
            let var_ast = *var_map
                .get(&def.body.lookup_var(vid).name.to_string())
                .expect("choose var missing");
            let coll_sort = match def.body.lookup_exp(coll_eid) {
                Expression::Var(v) => def.body.lookup_var(v).sort.clone(),
                other => panic!("choose! collection must be a variable, got {other:?}"),
            };
            let coll_ast = translate_expression(self, &def.body, *coll_eid, &var_map).raw();

            let m = unsafe {
                match &coll_sort {
                    Sort::Set(_) => {
                        z3_sys::Z3_mk_set_member(ctx, var_ast, coll_ast).expect("set_member")
                    }
                    Sort::Array(_, val_sort) => {
                        let null = self.null_for_sort(val_sort);
                        let sel = z3_sys::Z3_mk_select(ctx, coll_ast, var_ast).expect("mk_select");
                        let eq = z3_sys::Z3_mk_eq(ctx, sel, null).expect("mk_eq");
                        z3_sys::Z3_mk_not(ctx, eq).expect("mk_not")
                    }
                    Sort::Seq(_) => {
                        let len = z3_sys::Z3_mk_seq_length(ctx, coll_ast).expect("len");
                        let int_sort = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
                        let zero = z3_sys::Z3_mk_int(ctx, 0, int_sort).expect("zero");
                        let ge = z3_sys::Z3_mk_ge(ctx, var_ast, zero).expect("ge");
                        let lt = z3_sys::Z3_mk_lt(ctx, var_ast, len).expect("lt");
                        z3_sys::Z3_mk_and(ctx, 2, [ge, lt].as_ptr()).expect("and")
                    }
                    other => panic!("choose! must iterate Array|Set|Seq, got {other:?}"),
                }
            };
            membership.push(m);
        }

        let mut conj = membership;
        conj.push(body_ast.raw());
        let inner = if conj.len() == 1 {
            conj[0]
        } else {
            unsafe { z3_sys::Z3_mk_and(ctx, conj.len() as u32, conj.as_ptr()).expect("and") }
        };

        // Build the universal axiom. No params = assert inner directly.
        let axiom = if param_asts.is_empty() {
            inner
        } else {
            let apps: Vec<z3_sys::Z3_app> = param_asts
                .iter()
                .map(|&c| unsafe { z3_sys::Z3_to_app(ctx, c).expect("to_app") })
                .collect();
            unsafe {
                z3_sys::Z3_mk_forall_const(
                    ctx,
                    0,
                    apps.len() as u32,
                    apps.as_ptr(),
                    0,
                    std::ptr::null(),
                    inner,
                )
                .expect("forall_const")
            }
        };
        unsafe {
            z3_sys::Z3_inc_ref(ctx, axiom);
        }
        self.axioms.push(axiom);

        let _ = chosen_vars;
    }

    /// Build the null-sentinel AST for a value sort. Primitives produce their
    /// literal "zero" values; user sorts return the pre-declared sentinel.
    pub fn null_for_sort(&mut self, sort: &Sort) -> z3_sys::Z3_ast {
        let ctx = self.ctx;
        unsafe {
            match sort {
                Sort::Boolean => z3_sys::Z3_mk_false(ctx).expect("mk_false"),
                Sort::Integer => {
                    let s = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
                    z3_sys::Z3_mk_int(ctx, 0, s).expect("mk_int")
                }
                Sort::Real => {
                    let s = z3_sys::Z3_mk_real_sort(ctx).expect("real_sort");
                    z3_sys::Z3_mk_int(ctx, 0, s).expect("mk_int")
                }
                Sort::String => {
                    let c = CString::new("").unwrap();
                    z3_sys::Z3_mk_string(ctx, c.as_ptr()).expect("mk_string")
                }
                Sort::I32 | Sort::U32 => {
                    let s = z3_sys::Z3_mk_bv_sort(ctx, 32).expect("bv_sort");
                    z3_sys::Z3_mk_int(ctx, 0, s).expect("mk_int")
                }
                Sort::I64 | Sort::U64 => {
                    let s = z3_sys::Z3_mk_bv_sort(ctx, 64).expect("bv_sort");
                    z3_sys::Z3_mk_int(ctx, 0, s).expect("mk_int")
                }
                Sort::F32 => {
                    let s = z3_sys::Z3_mk_fpa_sort(ctx, 8, 24).expect("fpa_sort");
                    z3_sys::Z3_mk_fpa_zero(ctx, s, false).expect("fpa_zero")
                }
                Sort::F64 => {
                    let s = z3_sys::Z3_mk_fpa_sort(ctx, 11, 53).expect("fpa_sort");
                    z3_sys::Z3_mk_fpa_zero(ctx, s, false).expect("fpa_zero")
                }
                Sort::Seq(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    let seq_sort = z3_sys::Z3_mk_seq_sort(ctx, inner_sort).expect("seq_sort");
                    z3_sys::Z3_mk_seq_empty(ctx, seq_sort).expect("seq_empty")
                }
                Sort::Set(inner) => {
                    let inner_sort = self.translate_sort(inner);
                    z3_sys::Z3_mk_empty_set(ctx, inner_sort).expect("empty_set")
                }
                Sort::Error => {
                    let ints = z3_sys::Z3_mk_int_sort(ctx).expect("int_sort");
                    let f = z3_sys::Z3_mk_false(ctx).expect("false");
                    z3_sys::Z3_mk_const_array(ctx, ints, f).expect("const_array")
                }
                Sort::Array(k, v) => {
                    let ks = self.translate_sort(k);
                    let null_v = self.null_for_sort(v);
                    z3_sys::Z3_mk_const_array(ctx, ks, null_v).expect("const_array")
                }
                Sort::Cloak(inner) => self.null_for_sort(inner),
                Sort::User(_) => {
                    let name = array_null_const_name(sort, self.ir);
                    self.get_null_const(&name)
                }
                Sort::Uninterpreted(_) => {
                    panic!("no null sentinel for uninterpreted sort")
                }
            }
        }
    }
}

struct CtorInfo {
    branch: String,
    num_fields: usize,
}

/// Produce a Z3-safe symbol component from a sort's Display form.
fn sanitize_sort_name(s: &str) -> String {
    s.replace(' ', "_")
        .replace('<', "_")
        .replace('>', "")
        .replace(',', "_")
        .replace('(', "")
        .replace(')', "")
}

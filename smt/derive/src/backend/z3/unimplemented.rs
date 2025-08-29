//! This module contains the unimplemented functions from the high level z3 Rust API.

use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
};
use z3::{
    Context, FuncDecl, Solver,
    ast::{self, Ast, Dynamic, Int, String as Z3String},
};
use z3_sys::*;

/// Convert Dynamic AST to String AST with error handling
pub fn dynamic_to_string(ast: &Dynamic) -> Result<Z3String, &'static str> {
    ast.as_string()
        .ok_or("Expected Z3 String AST, got different sort")
}

/// Generic string comparison function for Dynamic AST nodes
pub fn compare_string_asts(
    lhs: &Dynamic,
    rhs: &Dynamic,
    op: StringCompareOp,
) -> Result<ast::Bool, &'static str> {
    let lhs_str = dynamic_to_string(lhs)?;
    let rhs_str = dynamic_to_string(rhs)?;

    let result = match op {
        StringCompareOp::Lt => lhs_str.str_lt(&rhs_str),
        StringCompareOp::Le => lhs_str.str_le(&rhs_str),
        StringCompareOp::Gt => lhs_str.str_gt(&rhs_str),
        StringCompareOp::Ge => lhs_str.str_ge(&rhs_str),
    };

    Ok(result)
}

#[derive(Debug, Clone, Copy)]
pub enum StringCompareOp {
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

/// CloakManager manages the creation and retrieval of cloaked sorts and their associated shield/reveal functions.
pub struct CloakManager<'a> {
    ctx: &'a Context,
    cloak_sorts: HashMap<String, z3::Sort>,
    shield_functions: HashMap<String, FuncDecl>,
    reveal_functions: HashMap<String, FuncDecl>,
    initialized_types: HashSet<String>,
}

impl<'a> CloakManager<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        Self {
            ctx,
            cloak_sorts: HashMap::new(),
            shield_functions: HashMap::new(),
            reveal_functions: HashMap::new(),
            initialized_types: HashSet::new(),
        }
    }

    pub fn get_or_create_cloak_for_type(
        &mut self,
        solver: &Solver,
        base_sort: &z3::Sort,
    ) -> (&FuncDecl, &FuncDecl) {
        let type_name = base_sort.to_string().replace(" ", "_");
        if !self.initialized_types.contains(&type_name) {
            // Create sorts and functions
            let cloak_sort_name = format!("Cloak_{type_name}");
            let cloak_sort = z3::Sort::uninterpreted(self.ctx, cloak_sort_name.clone().into());

            let shield_name = format!("shield_{type_name}");
            let reveal_name = format!("reveal_{type_name}");

            let shield_decl =
                FuncDecl::new(self.ctx, shield_name.clone(), &[base_sort], &cloak_sort);
            let reveal_decl =
                FuncDecl::new(self.ctx, reveal_name.clone(), &[&cloak_sort], base_sort);

            // Add axioms
            create_shield_reveal_axioms(
                self.ctx,
                solver,
                &shield_decl,
                &reveal_decl,
                base_sort,
                &cloak_sort,
            );

            // Store everything
            self.cloak_sorts.insert(type_name.to_string(), cloak_sort);
            self.shield_functions.insert(shield_name, shield_decl);
            self.reveal_functions.insert(reveal_name, reveal_decl);
            self.initialized_types.insert(type_name.to_string());
        }

        let shield_name = format!("shield_{type_name}");
        let reveal_name = format!("reveal_{type_name}");

        (
            self.shield_functions.get(&shield_name).unwrap(),
            self.reveal_functions.get(&reveal_name).unwrap(),
        )
    }
}

/// Creates axioms for the shield and reveal functions to ensure they behave as expected.
fn create_shield_reveal_axioms(
    ctx: &Context,
    solver: &Solver,
    shield_decl: &FuncDecl,
    reveal_decl: &FuncDecl,
    base_sort: &z3::Sort,
    cloak_sort: &z3::Sort,
) {
    // Axiom 1: forall x: Cloak<T>. shield(reveal(x)) = x
    {
        let x_cloak = ast::Dynamic::fresh_const(ctx, "x", cloak_sort);
        let reveal_x = reveal_decl.apply(&[&x_cloak]);
        let shield_reveal_x = shield_decl.apply(&[&reveal_x]);
        let axiom1 = x_cloak._eq(&shield_reveal_x);

        let forall_axiom1 = ast::forall_const(ctx, &[&x_cloak], &[], &axiom1);

        solver.assert(&forall_axiom1);
    }

    // Axiom 2: forall x: T. reveal(shield(x)) = x
    {
        let x_base = ast::Dynamic::fresh_const(ctx, "x", base_sort);
        let shield_x = shield_decl.apply(&[&x_base]);
        let reveal_shield_x = reveal_decl.apply(&[&shield_x]);
        let axiom2 = x_base._eq(&reveal_shield_x);

        let forall_axiom2 = ast::forall_const(ctx, &[&x_base], &[], &axiom2);

        solver.assert(&forall_axiom2);
    }
}

/// Return the literal `seq.empty` of sort `Seq<T>`.
pub fn mk_seq_empty(ctx: &Context, elem_sort: &z3::Sort) -> ast::Dynamic {
    let seq_sort = z3::Sort::seq(ctx, elem_sort);
    let empty_seq_ast = unsafe { Z3_mk_seq_empty(ctx.get_z3_context(), seq_sort.get_z3_sort()) };

    unsafe { ast::Dynamic::wrap(ctx, empty_seq_ast) }
}

/// Build `(seq.contains container containee)`
pub fn seq_contains(ctx: &Context, container: &ast::Seq, containee: &ast::Seq) -> ast::Bool {
    unsafe {
        let raw_ctx = ctx.get_z3_context();
        let c_ast = container.get_z3_ast();
        let s_ast = containee.get_z3_ast();
        let res_ast = Z3_mk_seq_contains(raw_ctx, c_ast, s_ast);
        ast::Bool::wrap(ctx, res_ast)
    }
}

// Return the cardinality of a set `s`.
// pub fn set_card(ctx: &Context, s: &ast::Set) -> ast::Int {
//     unsafe { ast::Int::wrap(ctx, Z3_mk_set_card(ctx.get_z3_context(), s.get_z3_ast())) }
// }

/// Create a unique "not_present" constant for any Z3 sort.
/// The returned value will be of the same sort given.
pub fn not_present(ctx: &Context, sort: &z3::Sort) -> Dynamic {
    Dynamic::new_const(ctx, "not_present_", sort)
}

/// Generates an "empty” map.
pub fn empty_map(ctx: &Context, key_sort: &z3::Sort, val_sort: &z3::Sort) -> Dynamic {
    let sentinel = not_present(ctx, val_sort);
    let arr = z3::ast::Array::const_array(ctx, key_sort, &sentinel);
    arr.into()
}

/// Manages the creation and retrieval of uninterpreted functions for map lengths and membership checks.
pub struct MapLengthManager {
    length_functions: HashMap<String, FuncDecl>,
    membership_functions: HashMap<String, FuncDecl>,
}

impl MapLengthManager {
    /// Create a new MapLengthManager.
    pub fn new() -> Self {
        Self {
            length_functions: HashMap::new(),
            membership_functions: HashMap::new(),
        }
    }

    /// Install all axioms for correct length semantics
    fn install_length_axioms(
        &self,
        solver: &Solver,
        length_fn: &FuncDecl,
        membership_fn: &FuncDecl,
        key_sort: &z3::Sort,
        val_sort: &z3::Sort,
    ) {
        let array_sort = z3::Sort::array(solver.get_context(), key_sort, val_sort);
        let sentinel = not_present(solver.get_context(), val_sort);

        // === AXIOM 1: Define membership function ===
        // in_map(m, k) ≡ (select m k) ≠ not_present
        {
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let k = Dynamic::fresh_const(solver.get_context(), "k", key_sort);

            let selected_value = m.as_array().unwrap().select(&k);
            let is_not_sentinel = selected_value._eq(&sentinel).not();
            let membership_def = membership_fn
                .apply(&[&m, &k])
                .as_bool()
                .unwrap()
                ._eq(&is_not_sentinel);

            let axiom = ast::forall_const(solver.get_context(), &[&m, &k], &[], &membership_def);
            solver.assert(&axiom);
        }

        // === AXIOM 2: Base case - empty map has length 0 ===
        // len_map(const_array(not_present)) = 0
        {
            let empty_map = ast::Array::const_array(solver.get_context(), key_sort, &sentinel);
            let zero = ast::Int::from_i64(solver.get_context(), 0);
            let empty_length = length_fn.apply(&[&empty_map]).as_int().unwrap();

            let axiom = empty_length._eq(&zero);
            solver.assert(&axiom);
        }

        // === AXIOM 3: Adding non-sentinel to sentinel key increases length by 1 ===
        // ∀m,k,v. (¬in_map(m,k) ∧ v≠not_present) ⟹ len_map(store(m,k,v)) = len_map(m) + 1
        {
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let k = Dynamic::fresh_const(solver.get_context(), "k", key_sort);
            let v = Dynamic::fresh_const(solver.get_context(), "v", val_sort);

            let not_in_map = membership_fn.apply(&[&m, &k]).as_bool().unwrap().not();
            let value_not_sentinel = v._eq(&sentinel).not();
            let condition =
                ast::Bool::and(solver.get_context(), &[&not_in_map, &value_not_sentinel]);

            let new_map = m.as_array().unwrap().store(&k, &v);
            let old_length = length_fn.apply(&[&m]).as_int().unwrap();
            let new_length = length_fn.apply(&[&new_map]).as_int().unwrap();
            let incremented = old_length + ast::Int::from_i64(solver.get_context(), 1);

            let consequence = new_length._eq(&incremented);
            let implication = ast::Bool::implies(&condition, &consequence);

            let axiom = ast::forall_const(solver.get_context(), &[&m, &k, &v], &[], &implication);
            solver.assert(&axiom);
        }

        // === AXIOM 4: Adding non-sentinel to existing non-sentinel key keeps length unchanged ===
        // ∀m,k,v. (in_map(m,k) ∧ v≠not_present) ⟹ len_map(store(m,k,v)) = len_map(m)
        {
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let k = Dynamic::fresh_const(solver.get_context(), "k", key_sort);
            let v = Dynamic::fresh_const(solver.get_context(), "v", val_sort);

            let in_map = membership_fn.apply(&[&m, &k]).as_bool().unwrap();
            let value_not_sentinel = v._eq(&sentinel).not();
            let condition = ast::Bool::and(solver.get_context(), &[&in_map, &value_not_sentinel]);

            let new_map = m.as_array().unwrap().store(&k, &v);
            let old_length = length_fn.apply(&[&m]).as_int().unwrap();
            let new_length = length_fn.apply(&[&new_map]).as_int().unwrap();

            let consequence = new_length._eq(&old_length);
            let implication = ast::Bool::implies(&condition, &consequence);

            let axiom = ast::forall_const(solver.get_context(), &[&m, &k, &v], &[], &implication);
            solver.assert(&axiom);
        }

        // === AXIOM 5: Removing existing non-sentinel key decreases length by 1 ===
        // ∀m,k. in_map(m,k) ⟹ len_map(store(m,k,not_present)) = len_map(m) - 1
        {
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let k = Dynamic::fresh_const(solver.get_context(), "k", key_sort);

            let in_map = membership_fn.apply(&[&m, &k]).as_bool().unwrap();
            let new_map = m.as_array().unwrap().store(&k, &sentinel);
            let old_length = length_fn.apply(&[&m]).as_int().unwrap();
            let new_length = length_fn.apply(&[&new_map]).as_int().unwrap();
            let decremented = old_length - ast::Int::from_i64(solver.get_context(), 1);

            let consequence = new_length._eq(&decremented);
            let implication = ast::Bool::implies(&in_map, &consequence);

            let axiom = ast::forall_const(solver.get_context(), &[&m, &k], &[], &implication);
            solver.assert(&axiom);
        }

        // === AXIOM 6: "Removing" already absent key keeps length unchanged ===
        // ∀m,k. ¬in_map(m,k) ⟹ len_map(store(m,k,not_present)) = len_map(m)
        {
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let k = Dynamic::fresh_const(solver.get_context(), "k", key_sort);

            let not_in_map = membership_fn.apply(&[&m, &k]).as_bool().unwrap().not();
            let new_map = m.as_array().unwrap().store(&k, &sentinel);
            let old_length = length_fn.apply(&[&m]).as_int().unwrap();
            let new_length = length_fn.apply(&[&new_map]).as_int().unwrap();

            let consequence = new_length._eq(&old_length);
            let implication = ast::Bool::implies(&not_in_map, &consequence);

            let axiom = ast::forall_const(solver.get_context(), &[&m, &k], &[], &implication);
            solver.assert(&axiom);
        }
        {
            // Non-negativity
            let m = Dynamic::fresh_const(solver.get_context(), "m", &array_sort);
            let zero = ast::Int::from_i64(solver.get_context(), 0);
            let len_m = length_fn.apply(&[&m]).as_int().unwrap();
            solver.assert(&ast::forall_const(
                solver.get_context(),
                &[&m],
                &[],
                &len_m.ge(&zero),
            ));
        }
    }

    /// Populate the manager.
    pub fn populate(&mut self, solver: &Solver, key_sort: &z3::Sort, val_sort: &z3::Sort) {
        let sort_key = format!("{key_sort}_{val_sort}");
        let contains = self.length_functions.contains_key(&sort_key);

        if !contains {
            let array_sort = z3::Sort::array(solver.get_context(), key_sort, val_sort);
            // Create the length and membership functions
            let length_fn = FuncDecl::new(
                solver.get_context(),
                format!("len_map_{sort_key}"),
                &[&array_sort],
                &z3::Sort::int(solver.get_context()),
            );
            let membership_fn = FuncDecl::new(
                solver.get_context(),
                format!("in_map_{sort_key}"),
                &[&array_sort, key_sort],
                &z3::Sort::bool(solver.get_context()),
            );

            // Store the functions
            self.length_functions.insert(sort_key.clone(), length_fn);
            self.membership_functions
                .insert(sort_key.clone(), membership_fn);

            let length_fn = self
                .get_length_function(key_sort, val_sort)
                .expect("Length function not found");
            let membership_fn = self
                .get_membership_function(key_sort, val_sort)
                .expect("Membership function not found");
            // install length axioms
            self.install_length_axioms(solver, length_fn, membership_fn, key_sort, val_sort);
        }
    }

    /// Get the length helper function for a map
    fn get_length_function(&self, key_sort: &z3::Sort, val_sort: &z3::Sort) -> Option<&FuncDecl> {
        let sort_key = format!("{key_sort}_{val_sort}");
        self.length_functions.get(&sort_key)
    }

    /// Get the membership helper function for a map
    fn get_membership_function(
        &self,
        key_sort: &z3::Sort,
        val_sort: &z3::Sort,
    ) -> Option<&FuncDecl> {
        let sort_key = format!("{key_sort}_{val_sort}");
        self.membership_functions.get(&sort_key)
    }

    /// Apply the length function to a map
    pub fn get_map_length(
        &mut self,
        map: &ast::Array,
        key_sort: &z3::Sort,
        val_sort: &z3::Sort,
    ) -> ast::Int {
        let length_fn = self
            .get_length_function(key_sort, val_sort)
            .expect("Length function not found");
        length_fn
            .apply(&[map])
            .as_int()
            .expect("Expected Int result")
    }
}

pub fn from_i128(ctx: &Context, v: i128) -> Int {
    let int_sort = z3::Sort::int(ctx);
    let s = v.to_string();
    let c_str = CString::new(s).unwrap();
    unsafe {
        Int::wrap(
            ctx,
            Z3_mk_numeral(ctx.get_z3_context(), c_str.as_ptr(), int_sort.get_z3_sort()),
        )
    }
}

pub fn from_u128(ctx: &Context, v: u128) -> Int {
    let int_sort = z3::Sort::int(ctx);
    let s = v.to_string();
    let c_str = CString::new(s).unwrap();
    unsafe {
        Int::wrap(
            ctx,
            Z3_mk_numeral(ctx.get_z3_context(), c_str.as_ptr(), int_sort.get_z3_sort()),
        )
    }
}

pub fn as_i128(i: &Int) -> Option<i128> {
    if !unsafe { Z3_is_numeral_ast(i.get_ctx().get_z3_context(), i.get_z3_ast()) } {
        return None;
    }

    let c_str = unsafe {
        std::ffi::CStr::from_ptr(Z3_get_numeral_string(
            i.get_ctx().get_z3_context(),
            i.get_z3_ast(),
        ))
    };

    let s = c_str.to_string_lossy();
    s.parse::<i128>().ok()
}

pub fn as_u128(i: &Int) -> Option<u128> {
    if !unsafe { Z3_is_numeral_ast(i.get_ctx().get_z3_context(), i.get_z3_ast()) } {
        return None;
    }

    let c_str = unsafe {
        std::ffi::CStr::from_ptr(Z3_get_numeral_string(
            i.get_ctx().get_z3_context(),
            i.get_z3_ast(),
        ))
    };

    let s = c_str.to_string_lossy();
    s.parse::<u128>().ok()
}

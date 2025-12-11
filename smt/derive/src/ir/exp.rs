use crate::ir::ctxt::IRBuilder;
use crate::ir::fun::FunSig;
use crate::ir::index::{ExpId, UsrFunId, UsrSortId, VarId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::name::Symbol;
use crate::ir::sort::{DataType, Sort, Variant};
use crate::parser::expr::{Expr, LetBinding, Op, Unpack, VarDecl};
use crate::parser::intrinsics::Intrinsic as Native;
use crate::parser::name::VarName;
use crate::parser::path::ADTBranch;
use crate::parser::ty::TypeTag;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, Clone)]
/// The origin of a variable
pub enum VarKind {
    /// function parameter (x in fn f(x: i32) -> i32 { x + 1 })
    Param,
    /// bounded variable used in a quantifier (forall, exists)
    /// x in  ∀x. P(x)
    Quant,
    /// axiomatized (through a list of predicates - choose)
    Axiom,
    /// let-binding to an expression
    /// let x = e where x is assigned to e
    Bound { bind: ExpId },
    /// match-introduced
    /// match (e1,e2....) { (a1,a2,...) => e1 } where head is (e1,e2,...) and sort defines the type of the match
    /// and branch is the name of the enum variant
    /// and selector is how to destruct the enum variant
    Match {
        head: ExpId,
        sort: UsrSortId,
        branch: String,
        selector: EnumSelector,
    },
}

#[derive(Debug, Clone)]
/// Information about a variable
pub struct Variable {
    pub name: Symbol, // the name of the variable (in the Intermediate Representation a variable is represented by a Symbol)
    pub kind: VarKind, // the kind of the variable
    pub sort: Sort, // the type of the variable (in the Intermediate Representation a type is represented by a Sort)
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// Denotes how a variable gets match-bounded
pub enum EnumSelector {
    Tuple(usize),
    Record(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Denotes how to construct an enum variant
pub enum VariantCtor {
    Unit,
    Tuple(Vec<ExpId>),
    Record(BTreeMap<String, ExpId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Denotes how to destruct an enum variant and bind variables
pub enum VariantDtor {
    Unit,
    Tuple(Vec<Option<VarId>>),
    Record(BTreeMap<String, Option<VarId>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One atom in the match case to unpack
pub struct MatchAtom {
    pub head: ExpId,
    pub sort: UsrSortId,
    pub branch: String,
    pub variant: VariantDtor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One match case
pub struct MatchCase {
    pub atoms: Vec<MatchAtom>,
    pub body: ExpId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One phi case (i.e., conditional branch)
pub struct PhiCase {
    pub cond: ExpId,
    pub body: ExpId,
}

#[derive(Debug, Clone, PartialEq)]
/// An expression (which is the intermediate representation of the Op in the parser)
pub enum Expression {
    /// `<var>` - Var(VarName) in the parser
    Var(VarId), // VarId is a unique identifier for a variable
    /// `(v1, v2, ...)` - Pack { elems: Vec<Expr> } in the parser
    // UsrSortId is a unique identifier for a user-defined type (a tuple is represented as a user-defined type without a name in the IR)
    // ExpId is a unique identifier for an expression
    Pack { sort: UsrSortId, elems: Vec<ExpId> },
    /// `<tuple-name>(<inst>?)(v1, v2. ...)` -     Tuple { name: UsrTypeName, inst: Vec<TypeRef>, slots: Vec<Expr>} in the parser
    // UsrSortId is a unique identifier for a user-defined type (a struct tuple is represented as a user-defined type without a name in the IR)
    // the name and the inst are stored in the TypeRegistry where using the UsrSortId we can retrieve the name and the inst
    Tuple { sort: UsrSortId, slots: Vec<ExpId> },
    /// `<record-name>(<inst>?){ f1: v1, f2: v2, ... }`  - Record { name: UsrTypeName, inst: Vec<TypeRef>, fields: BTreeMap<String, Expr>} in the parser
    // UsrSortId is a unique identifier for a user-defined type (a struct record is represented as a user-defined type without a name in the IR)
    // the name and the inst are stored in the TypeRegistry where using the UsrSortId we can retrieve the name and the inst
    Record {
        sort: UsrSortId,
        fields: BTreeMap<String, ExpId>,
    },
    /// `<adt-name>(<inst>?)::<branch>(<ctor>)` - Op has three different variants EnumUnit, EnumTuple, EnumRecord where they are all represented as Enum in the IR
    /// An Enum is a user-defined type (UserSortId)
    Enum {
        sort: UsrSortId, // the name and the inst are stored in the TypeRegistry where using the UsrSortId we can retrieve the name and the inst
        branch: String,  // the name of the enum variant
        variant: VariantCtor, // the call to the constructor of the enum variant (can be unit, tuple or record)
    },
    /// `<base>.<index>` - AccessSlot { base: Expr, slot: usize } in the parser
    AccessSlot { base: ExpId, slot: usize },
    /// `<base>.<field>` - AccessField { base: Expr, field: String } in the parser
    AccessField { base: ExpId, field: String },
    /// `match (v1, v2, ...) { (a1, a2, ...) => <body1> } ...` - Match { heads: Vec<Expr>, combo: Vec<MatchCombo> } in the parser
    Match { cases: Vec<MatchCase> },
    /// `if (<c1>) { <v1> } else if (<c2>) { <v2> } ... else { <default> }` - Phi { nodes: Vec<PhiNode>, default: Expr } in the parser
    // basically the name just the Expr is replaced by the ExpId
    Phi { cases: Vec<PhiCase>, default: ExpId },
    /// `forall!(|<v>: <t>| {<expr>})` - Forall { vars: Vec<(VarName, TypeTag)>, body: Expr } in the parser
    Forall {
        vars: BTreeMap<VarId, Sort>,
        body: ExpId,
    },
    /// `exists!(|<v>: <t>| {<expr>})` - Exists { vars: Vec<(VarName, TypeTag)>, body: Expr } in the parser
    Exists {
        vars: BTreeMap<VarId, Sort>,
        body: ExpId,
    },
    /// `choose!(|<v>: <t>| {<expr>})` - Choose { vars: Vec<(VarName, TypeTag)>, body: Expr } in the parser
    Choose {
        vars: BTreeMap<VarId, Sort>,
        body: ExpId,
        rets: Vec<VarId>,
    },
    /// `forall!(<v> in <c> ... => <expr>)` - IterForall { vars: Vec<(VarName, Expr)>, body: Expr } in the parser
    IterForall {
        vars: BTreeMap<VarId, ExpId>,
        body: ExpId,
    },
    /// `exists!(<v> in <c> ... => <expr>)` - IterExists { vars: Vec<(VarName, Expr)>, body: Expr } in the parser
    IterExists {
        vars: BTreeMap<VarId, ExpId>,
        body: ExpId,
    },
    /// `choose!(<v> in <c> ... => <expr>)` - IterChoose { vars: Vec<(VarName, Expr)>, body: Expr } in the parser
    IterChoose {
        vars: BTreeMap<VarId, ExpId>,
        body: ExpId,
        rets: Vec<VarId>,
    },
    /// `<class>::<method>(<a1>, <a2>, ...)` - Intrinsic(Intrinsic) in the parser (the definition of the intrinsic is in the IR)
    Intrinsic(Box<Intrinsic>),
    /// `<function>(<a1>, <a2>, ...)` - Procedure { name: UsrFuncName, inst: Vec<TypeRef>, args: Vec<Expr>} in the parser
    // in the FunRegistry, the name, inst, signature and the body are stored where using the UsrFunId they can be retrieved
    Procedure { callee: UsrFunId, args: Vec<ExpId> },
}

/// A registry of expressions (organized around a function body)
#[derive(Default, Debug, Clone)]
pub struct ExpRegistry {
    /// a map from variable id to variables
    pub vars: BTreeMap<VarId, Variable>,
    /// a map from expression id to expressions
    pub exps: BTreeMap<ExpId, Expression>,
}

impl ExpRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
            exps: BTreeMap::new(),
        }
    }

    /// Add a new parameter to the registry
    fn add_param(&mut self, name: Symbol, sort: Sort) -> VarId {
        let id = VarId {
            index: self.vars.len(), // create a unique id for the variable
        };
        self.vars.insert(
            id,
            Variable {
                name,
                kind: VarKind::Param,
                sort,
            },
        );
        id
    }

    /// Add a new quantified variable (free variable) to the registry
    fn add_quant_var(&mut self, name: Symbol, sort: Sort) -> VarId {
        let id = VarId {
            index: self.vars.len(),
        };
        self.vars.insert(
            id,
            Variable {
                name,
                kind: VarKind::Quant,
                sort,
            },
        );
        id
    }

    /// Add a new axiomatized variable to the registry
    fn add_axiom_var(&mut self, name: Symbol, sort: Sort) -> VarId {
        let id = VarId {
            index: self.vars.len(),
        };
        self.vars.insert(
            id,
            Variable {
                name,
                kind: VarKind::Axiom,
                sort,
            },
        );
        id
    }

    /// Add a new let-binding to the registry
    fn add_bound(&mut self, name: Symbol, sort: Sort, bind: ExpId) -> VarId {
        let id = VarId {
            index: self.vars.len(),
        };
        self.vars.insert(
            id,
            Variable {
                name,
                kind: VarKind::Bound { bind },
                sort,
            },
        );
        id
    }

    /// Add a new match-binding to the registry
    fn add_match(
        &mut self,
        name: Symbol,
        sort: Sort,
        head: ExpId,
        sid: UsrSortId,
        branch: String,
        selector: EnumSelector,
    ) -> VarId {
        let id = VarId {
            index: self.vars.len(),
        };
        self.vars.insert(
            id,
            Variable {
                name,
                kind: VarKind::Match {
                    head,
                    sort: sid,
                    branch,
                    selector,
                },
                sort,
            },
        );
        id
    }

    /// Register an expression
    fn register(&mut self, exp: Expression) -> ExpId {
        let id = ExpId {
            index: self.exps.len(), // create a unique id for the expression
        };
        self.exps.insert(id, exp);
        id
    }

    /// Retrieve the variable
    pub fn lookup_var(&self, idx: &VarId) -> &Variable {
        self.vars.get(idx).expect("no such var id")
    }

    /// Retrieve the expression
    pub fn lookup_exp(&self, idx: &ExpId) -> &Expression {
        self.exps.get(idx).expect("no such exp id")
    }
}

/// A context builder originated from a refinement relation
pub struct ExpBuilder<'b, 'ir: 'b, 'a: 'ir, 'ctx: 'a> {
    /// the parent IR builder
    pub parent: &'ir mut IRBuilder<'a, 'ctx>,
    /// a map from variable id to variables
    pub registry: &'b mut ExpRegistry,
    /// a set of valid variable ids in the current expression
    pub namespace: BTreeMap<Symbol, VarId>,
}

impl<'b, 'ir: 'b, 'a: 'ir, 'ctx: 'a> ExpBuilder<'b, 'ir, 'a, 'ctx> {
    /// Create a new expression builder
    fn new(
        parent: &'ir mut IRBuilder<'a, 'ctx>,
        registry: &'b mut ExpRegistry,
        params: &[(Symbol, Sort)],
    ) -> Self {
        let mut namespace = BTreeMap::new();
        for (name, sort) in params {
            let id = registry.add_param(name.clone(), sort.clone());
            match namespace.insert(name.clone(), id) {
                None => (),
                Some(_) => panic!("symbol conflict: {name}"),
            }
        }
        Self {
            parent,
            registry,
            namespace,
        }
    }

    /// Utility: expect type match
    fn check_sort(expect: &Sort, actual: &Sort) {
        if expect != actual {
            panic!("type mismatch: expect {expect} | actual {actual}");
        }
    }

    /// Utility: resolve user sort id from the sort
    fn expect_sort_user(sort: &Sort) -> UsrSortId {
        match sort {
            Sort::User(sid) => *sid,
            _ => panic!("type mismatch: expect $? | actual {sort}"),
        }
    }

    /// Utility: retrieve a tuple data type from a sort id
    fn expect_type_tuple(&self, sort_id: UsrSortId) -> Vec<Sort> {
        match self.parent.ir.ty_registry.retrieve(sort_id) {
            DataType::Tuple(tuple) => tuple.clone(),
            DataType::Enum(adt) => {
                let mut tuple = vec![];
                for (_, variant) in adt.iter() {
                    if let Variant::Tuple(t) = variant {
                        tuple.extend(t.clone());
                    }
                }
                tuple
            }
            dt => panic!("type mismatch: expect <tuple> | actual {dt}"),
        }
    }

    /// Utility: retrieve a record data type from a sort id
    fn expect_type_record(&self, sort_id: UsrSortId) -> BTreeMap<String, Sort> {
        match self.parent.ir.ty_registry.retrieve(sort_id) {
            DataType::Record(record) => record.clone(),
            DataType::Enum(adt) => {
                let mut record = BTreeMap::new();
                for (_, variant) in adt.iter() {
                    if let Variant::Record(r) = variant {
                        record.extend(r.clone());
                    }
                }
                record
            }
            dt => panic!("type mismatch: expect <record> | actual {dt}"),
        }
    }

    /// Utility: retrieve an enum-unit data type from a sort id and branch name
    fn expect_type_enum_unit(&self, sort_id: UsrSortId, branch: &str) {
        match self.parent.ir.ty_registry.retrieve(sort_id) {
            DataType::Enum(adt) => match adt.get(branch) {
                None => panic!("no such branch: {branch}"),
                Some(Variant::Unit) => (),
                Some(variant) => panic!("type mismatch: expect <enum::unit> | actual {variant}"),
            },
            dt => panic!("type mismatch: expect <enum::unit> | actual {dt}"),
        }
    }

    /// Utility: retrieve an enum-tuple data type from a sort id and branch name
    fn expect_type_enum_tuple(&self, sort_id: UsrSortId, branch: &str) -> Vec<Sort> {
        match self.parent.ir.ty_registry.retrieve(sort_id) {
            DataType::Enum(adt) => match adt.get(branch) {
                None => panic!("no such branch: {branch}"),
                Some(Variant::Tuple(tuple)) => tuple.clone(),
                Some(variant) => panic!("type mismatch: expect <enum::tuple> | actual {variant}"),
            },
            dt => panic!("type mismatch: expect <enum::tuple> | actual {dt}"),
        }
    }

    /// Utility: retrieve an enum-record data type from a sort id and branch name
    fn expect_type_enum_record(
        &self,
        sort_id: UsrSortId,
        branch: &String,
    ) -> BTreeMap<String, Sort> {
        match self.parent.ir.ty_registry.retrieve(sort_id) {
            DataType::Enum(adt) => match adt.get(branch) {
                None => panic!("no such branch: {branch}"),
                Some(Variant::Record(record)) => record.clone(),
                Some(variant) => {
                    panic!("type mismatch: expect <enum::record> | actual {variant}")
                }
            },
            dt => panic!("type mismatch: expect <enum::record> | actual {dt}"),
        }
    }

    /// Utility: resolve a tuple of expressions
    fn resolve_expr_tuple(&mut self, tuple: &[Sort], slots: &[Expr]) -> Vec<ExpId> {
        if tuple.len() != slots.len() {
            panic!(
                "tuple slot number mismatch: expect {} | actual {}",
                tuple.len(),
                slots.len()
            );
        }

        let mut converted = vec![];
        for (expr, sort) in slots.iter().zip(tuple) {
            let eid = self.resolve(expr, Some(sort));
            converted.push(eid);
        }
        converted
    }

    /// Utility: resolve a record of expressions
    fn resolve_expr_record(
        &mut self,
        record: &BTreeMap<String, Sort>,
        fields: &BTreeMap<String, Expr>,
    ) -> BTreeMap<String, ExpId> {
        if record.len() != fields.len() {
            panic!(
                "record field number mismatch: expect {} | actual {}",
                record.len(),
                fields.len()
            );
        }

        let mut converted = BTreeMap::new();
        for ((name_ref, expr), (name, sort)) in fields.iter().zip(record) {
            if name_ref != name {
                panic!("record field name mismatch: expect {name_ref} | actual {name}");
            }
            let field_eid = self.resolve(expr, Some(sort));
            converted.insert(name.clone(), field_eid);
        }
        converted
    }

    /// Bind a variable declaration to an expression
    // sort is the rhs type and exp is the rhs expression
    fn bind_decl(&mut self, decl: &VarDecl, ety: Sort, exp: ExpId) {
        match decl {
            VarDecl::One(name, ty) => {
                let sort = self.parent.resolve_type(ty);
                Self::check_sort(&ety, &sort);

                let sym = Symbol::from(name);
                let vid = self.registry.add_bound(sym.clone(), sort, exp);
                match self.namespace.insert(sym, vid) {
                    None => (),
                    Some(_) => panic!("naming conflict: {name}"),
                }
            }
            VarDecl::Pack(elems) => {
                let sort_id = Self::expect_sort_user(&ety);
                let tuple = self.expect_type_tuple(sort_id);
                if elems.len() != tuple.len() {
                    panic!(
                        "tuple slot number mismatch: expect {} | actual {}",
                        tuple.len(),
                        elems.len(),
                    );
                }
                let e = self.registry.lookup_exp(&exp).clone();
                if let Expression::Pack {
                    sort: _,
                    elems: elems_pack,
                } = e
                {
                    if elems_pack.len() != elems.len() {
                        panic!(
                            "pack slot number mismatch: expect {} | actual {}",
                            elems_pack.len(),
                            elems.len()
                        );
                    }
                    for ((elem_decl, elem_sort), ex) in elems.iter().zip(tuple).zip(elems_pack) {
                        self.bind_decl(elem_decl, elem_sort, ex);
                    }
                } else {
                    panic!("expect a pack expression");
                }
            }
        }
    }

    /// Bind a variable match-destruction
    fn bind_dtor(
        &mut self,
        name: &VarName,
        sort: Sort,
        head: ExpId,
        sid: UsrSortId,
        branch: String,
        selector: EnumSelector,
    ) -> VarId {
        let sym = Symbol::from(name);
        let vid = self
            .registry
            .add_match(sym.clone(), sort, head, sid, branch, selector);
        match self.namespace.insert(sym, vid) {
            None => (),
            Some(_) => panic!("naming conflict: {name}"),
        }
        vid
    }

    /// Add a quantified variable (i.e., free variable) induced by iteration
    fn iter_quant_var(&mut self, name: &VarName, expr: ExpId) -> VarId {
        let sym = Symbol::from(name);
        let sort = match self.derive_type(expr) {
            Sort::Seq(_) => Sort::Integer,
            Sort::Set(sub) => *sub,
            Sort::Array(key, _) => *key,
            s => panic!("iterating over a non-iterable type: {s}"),
        };
        let vid = self.registry.add_quant_var(sym.clone(), sort);
        match self.namespace.insert(sym, vid) {
            None => (),
            Some(_) => panic!("naming conflict: {name}"),
        }
        vid
    }

    /// Add a quantified variable (i.e., free variable)
    fn free_quant_var(&mut self, name: &VarName, tag: &TypeTag) -> (VarId, Sort) {
        let sort = self.parent.resolve_type(&tag.into());
        let sym = Symbol::from(name);
        let vid = self.registry.add_quant_var(sym.clone(), sort.clone());
        match self.namespace.insert(sym, vid) {
            None => (),
            Some(_) => panic!("naming conflict: {name}"),
        }
        (vid, sort)
    }

    /// Add an axiomatized variable induced by iteration
    fn iter_axiom_var(&mut self, name: &VarName, expr: ExpId) -> VarId {
        let sym = Symbol::from(name);
        let sort = match self.derive_type(expr) {
            Sort::Seq(_) => Sort::Integer,
            Sort::Set(sub) => *sub,
            Sort::Array(key, _) => *key,
            s => panic!("iterating over a non-iterable type: {s}"),
        };
        let vid = self.registry.add_axiom_var(sym.clone(), sort);
        match self.namespace.insert(sym, vid) {
            None => (),
            Some(_) => panic!("naming conflict: {name}"),
        }
        vid
    }

    /// Add an axiomatized variable
    fn free_axiom_var(&mut self, name: &VarName, tag: &TypeTag) -> (VarId, Sort) {
        let sort = self.parent.resolve_type(&tag.into());
        let sym = Symbol::from(name);
        let vid = self.registry.add_axiom_var(sym.clone(), sort.clone());
        match self.namespace.insert(sym, vid) {
            None => (),
            Some(_) => panic!("naming conflict: {name}"),
        }
        (vid, sort)
    }

    /// Process an expression
    fn resolve(&mut self, expr: &Expr, exp_ty: Option<&Sort>) -> ExpId {
        // save the namespace
        let old_namespace = self.namespace.clone();

        // handle let bindings
        let inst = match expr {
            Expr::Unit(inst) => inst,
            Expr::Block { lets, body } => {
                for LetBinding { decl, bind } in lets {
                    let bind_ty = self.parent.resolve_type(&decl.ty());
                    let bind_exp = self.resolve(bind, Some(&bind_ty));
                    self.bind_decl(decl, bind_ty, bind_exp);
                }
                body
            }
        };

        // resolve type and check consistency
        let sort = self.parent.resolve_type(&inst.ty);
        match exp_ty {
            None => (),
            Some(ety) => Self::check_sort(&sort, ety),
        }

        // parse the expression
        let expression = match inst.op.as_ref() {
            Op::Var(name) => {
                let vid = match self.namespace.get(&name.into()) {
                    None => panic!("no such variable: {name}"),
                    Some(id) => *id,
                };
                Expression::Var(vid)
            }
            Op::Pack { elems } => {
                let sort_id = Self::expect_sort_user(&sort);
                let tuple = self.expect_type_tuple(sort_id);
                let resolved = self.resolve_expr_tuple(&tuple, elems);
                Expression::Pack {
                    sort: sort_id,
                    elems: resolved,
                }
            }
            Op::Tuple { name, inst, slots } => {
                let sort_id = self.parent.register_type(Some(name), inst);
                let tuple = self.expect_type_tuple(sort_id);
                let resolved = self.resolve_expr_tuple(&tuple, slots);
                Expression::Tuple {
                    sort: sort_id,
                    slots: resolved,
                }
            }
            Op::Record { name, inst, fields } => {
                let sort_id = self.parent.register_type(Some(name), inst);
                let record = self.expect_type_record(sort_id);
                let resolved = self.resolve_expr_record(&record, fields);
                Expression::Record {
                    sort: sort_id,
                    fields: resolved,
                }
            }
            Op::EnumUnit {
                branch: ADTBranch { ty_name, variant },
                inst,
            } => {
                let sort_id = self.parent.register_type(Some(ty_name), inst);
                self.expect_type_enum_unit(sort_id, variant);
                Expression::Enum {
                    sort: sort_id,
                    branch: variant.clone(),
                    variant: VariantCtor::Unit,
                }
            }
            Op::EnumTuple {
                branch: ADTBranch { ty_name, variant },
                inst,
                slots,
            } => {
                let sort_id = self.parent.register_type(Some(ty_name), inst);
                let tuple = self.expect_type_enum_tuple(sort_id, variant);
                let resolved = self.resolve_expr_tuple(&tuple, slots);
                Expression::Enum {
                    sort: sort_id,
                    branch: variant.clone(),
                    variant: VariantCtor::Tuple(resolved),
                }
            }
            Op::EnumRecord {
                branch: ADTBranch { ty_name, variant },
                inst,
                fields,
            } => {
                let sort_id = self.parent.register_type(Some(ty_name), inst);
                let record = self.expect_type_record(sort_id);
                let resolved = self.resolve_expr_record(&record, fields);
                Expression::Enum {
                    sort: sort_id,
                    branch: variant.clone(),
                    variant: VariantCtor::Record(resolved),
                }
            }
            Op::AccessSlot { base, slot } => {
                let resolved = self.resolve(base, None);
                Expression::AccessSlot {
                    base: resolved,
                    slot: *slot,
                }
            }
            Op::AccessField { base, field } => {
                let resolved = self.resolve(base, None);
                Expression::AccessField {
                    base: resolved,
                    field: field.clone(),
                }
            }
            Op::Match { heads, combo } => {
                // resolve heads
                let resolved_heads: Vec<_> = heads.iter().map(|e| self.resolve(e, None)).collect();

                // resolve match arms
                let mut cases = vec![];
                for arm in combo {
                    // sanity check
                    if arm.variants.len() != resolved_heads.len() {
                        panic!(
                            "match atom number mismatch: expect {} | actual {}",
                            resolved_heads.len(),
                            arm.variants.len()
                        );
                    }

                    // snapshot arm-specific namespace
                    let arm_namespace = self.namespace.clone();

                    // process atoms
                    let mut atoms = vec![];
                    for (variant, &head) in arm.variants.iter().zip(resolved_heads.iter()) {
                        let head_type = self.derive_type(head);
                        let head_sid = Self::expect_sort_user(&head_type);
                        let branch = variant.branch.variant.clone();
                        let dtor = match &variant.unpack {
                            Unpack::Unit => {
                                self.expect_type_enum_unit(head_sid, &branch);
                                VariantDtor::Unit
                            }
                            Unpack::Tuple(bind_slots) => {
                                let sort_tuple = self.expect_type_enum_tuple(head_sid, &branch);
                                for &k in bind_slots.keys() {
                                    if k >= sort_tuple.len() {
                                        panic!(
                                            "type {head_type} at branch {branch} does not have slot {k}"
                                        );
                                    }
                                }
                                let mut binds = vec![];
                                for (i, s) in sort_tuple.into_iter().enumerate() {
                                    let item = match bind_slots.get(&i) {
                                        None => None,
                                        Some(var_name) => {
                                            let vid = self.bind_dtor(
                                                var_name,
                                                s,
                                                head,
                                                head_sid,
                                                branch.clone(),
                                                EnumSelector::Tuple(i),
                                            );
                                            Some(vid)
                                        }
                                    };
                                    binds.push(item);
                                }
                                VariantDtor::Tuple(binds)
                            }
                            Unpack::Record(bind_fields) => {
                                let sort_record = self.expect_type_enum_record(head_sid, &branch);
                                for k in bind_fields.keys() {
                                    if !sort_record.contains_key(k) {
                                        panic!(
                                            "type {head_type} at branch {branch} does not have field {k}"
                                        );
                                    }
                                }
                                let mut binds = BTreeMap::new();
                                for (i, s) in sort_record.into_iter() {
                                    let item = match bind_fields.get(&i) {
                                        None => None,
                                        Some(var_name) => {
                                            let vid = self.bind_dtor(
                                                var_name,
                                                s,
                                                head,
                                                head_sid,
                                                branch.clone(),
                                                EnumSelector::Record(i.clone()),
                                            );
                                            Some(vid)
                                        }
                                    };
                                    binds.insert(i, item);
                                }
                                VariantDtor::Record(binds)
                            }
                        };

                        let atom = MatchAtom {
                            head,
                            sort: head_sid,
                            branch,
                            variant: dtor,
                        };
                        atoms.push(atom);
                    }

                    // handle body
                    let body = self.resolve(&arm.body, Some(&sort));

                    // construct and register the case
                    cases.push(MatchCase { atoms, body });

                    // restore the namespace after body is parsed
                    self.namespace = arm_namespace;
                }
                Expression::Match { cases }
            }
            Op::Phi { nodes, default } => {
                let converted_default = self.resolve(default, Some(&sort));
                let mut cases = vec![];
                for node in nodes {
                    let cond = self.resolve(&node.cond, Some(&Sort::Boolean));
                    let body = self.resolve(&node.body, Some(&sort));
                    cases.push(PhiCase { cond, body });
                }
                Expression::Phi {
                    cases,
                    default: converted_default,
                }
            }
            Op::Forall { vars, body } => {
                let mut free_vars = BTreeMap::new();
                for (var_name, var_tag) in vars {
                    let (vid, var_sort) = self.free_quant_var(var_name, var_tag);
                    free_vars.insert(vid, var_sort);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::Forall {
                    vars: free_vars,
                    body: converted_body,
                }
            }
            Op::Exists { vars, body } => {
                let mut free_vars = BTreeMap::new();
                for (var_name, var_tag) in vars {
                    let (vid, var_sort) = self.free_quant_var(var_name, var_tag);
                    free_vars.insert(vid, var_sort);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::Exists {
                    vars: free_vars,
                    body: converted_body,
                }
            }
            Op::Choose { vars, body } => {
                let mut axiom_vars = BTreeMap::new();
                let mut axiom_rets = vec![];
                for (var_name, var_tag) in vars {
                    let (vid, var_sort) = self.free_axiom_var(var_name, var_tag);
                    axiom_vars.insert(vid, var_sort);
                    axiom_rets.push(vid);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::Choose {
                    vars: axiom_vars,
                    body: converted_body,
                    rets: axiom_rets,
                }
            }
            Op::IterForall { vars, body } => {
                let mut free_vars = BTreeMap::new();
                for (var_name, var_host) in vars {
                    let eid = self.resolve(var_host, None);
                    let vid = self.iter_quant_var(var_name, eid);
                    free_vars.insert(vid, eid);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::IterForall {
                    vars: free_vars,
                    body: converted_body,
                }
            }
            Op::IterExists { vars, body } => {
                let mut free_vars = BTreeMap::new();
                for (var_name, var_host) in vars {
                    let eid = self.resolve(var_host, None);
                    let vid = self.iter_quant_var(var_name, eid);
                    free_vars.insert(vid, eid);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::IterExists {
                    vars: free_vars,
                    body: converted_body,
                }
            }
            Op::IterChoose { vars, body } => {
                let mut axiom_vars = BTreeMap::new();
                let mut axiom_rets = vec![];
                for (var_name, var_host) in vars {
                    let eid = self.resolve(var_host, None);
                    let vid = self.iter_axiom_var(var_name, eid);
                    axiom_vars.insert(vid, eid);
                    axiom_rets.push(vid);
                }
                let converted_body = self.resolve(body, Some(&Sort::Boolean));
                Expression::IterChoose {
                    vars: axiom_vars,
                    body: converted_body,
                    rets: axiom_rets,
                }
            }
            Op::Intrinsic(native) => {
                let intrinsic = match native.as_ref() {
                    // ---------------------------------------------------------
                    // Boolean
                    // ---------------------------------------------------------
                    Native::BoolVal(v) => Intrinsic::BoolVal(*v),
                    Native::BoolNot { val } => Intrinsic::BoolNot {
                        val: self.resolve(val, Some(&Sort::Boolean)),
                    },
                    Native::BoolAnd { lhs, rhs } => Intrinsic::BoolAnd {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolOr { lhs, rhs } => Intrinsic::BoolOr {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolXor { lhs, rhs } => Intrinsic::BoolXor {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolImplies { lhs, rhs } => Intrinsic::BoolImplies {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolIff { lhs, rhs } => Intrinsic::BoolIff {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolNand { lhs, rhs } => Intrinsic::BoolNand {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolNor { lhs, rhs } => Intrinsic::BoolNor {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },
                    Native::BoolXnor { lhs, rhs } => Intrinsic::BoolXnor {
                        lhs: self.resolve(lhs, Some(&Sort::Boolean)),
                        rhs: self.resolve(rhs, Some(&Sort::Boolean)),
                    },

                    // ---------------------------------------------------------
                    // Integer
                    // ---------------------------------------------------------
                    Native::IntVal(v) => Intrinsic::IntVal(v.clone()),
                    Native::IntNeg { val } => Intrinsic::IntNeg {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntLt { lhs, rhs } => Intrinsic::IntLt {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntLe { lhs, rhs } => Intrinsic::IntLe {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntGe { lhs, rhs } => Intrinsic::IntGe {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntGt { lhs, rhs } => Intrinsic::IntGt {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntAdd { lhs, rhs } => Intrinsic::IntAdd {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntSub { lhs, rhs } => Intrinsic::IntSub {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntMul { lhs, rhs } => Intrinsic::IntMul {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntDiv { lhs, rhs } => Intrinsic::IntDiv {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntMod { lhs, rhs } => Intrinsic::IntMod {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntRem { lhs, rhs } => Intrinsic::IntRem {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    Native::IntAbs { val } => Intrinsic::IntAbs {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntPow { base, exp } => Intrinsic::IntPow {
                        base: self.resolve(base, Some(&Sort::Integer)),
                        exp: self.resolve(exp, Some(&Sort::Integer)),
                    },
                    Native::IntDivides { lhs, rhs } => Intrinsic::IntDivides {
                        lhs: self.resolve(lhs, Some(&Sort::Integer)),
                        rhs: self.resolve(rhs, Some(&Sort::Integer)),
                    },
                    // Integer Conversions
                    Native::IntToReal { val } => Intrinsic::IntoToReal {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToI32 { val } => Intrinsic::IntToI32 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToI64 { val } => Intrinsic::IntToI64 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToU32 { val } => Intrinsic::IntToU32 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToU64 { val } => Intrinsic::IntToU64 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToF32 { val } => Intrinsic::IntToF32 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntToF64 { val } => Intrinsic::IntToF64 {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    // Integer Parsing
                    Native::IntFromHex { val } => Intrinsic::IntFromHex {
                        val: self.resolve(val, Some(&Sort::String)),
                    },
                    Native::IntFromOct { val } => Intrinsic::IntFromOct {
                        val: self.resolve(val, Some(&Sort::String)),
                    },
                    Native::IntFromBin { val } => Intrinsic::IntFromBin {
                        val: self.resolve(val, Some(&Sort::String)),
                    },
                    // Integer Range Checks
                    Native::IntIsGtI64Max { val } => Intrinsic::IntIsGtI64Max {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsLtI64Min { val } => Intrinsic::IntIsLtI64Min {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsGtU64Max { val } => Intrinsic::IntIsGtU64Max {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsLtU64Min { val } => Intrinsic::IntIsLtU64Min {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsLtI32Min { val } => Intrinsic::IntIsLtI32Min {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsGtI32Max { val } => Intrinsic::IntIsGtI32Max {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsLtU32Min { val } => Intrinsic::IntIsLtU32Min {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::IntIsGtU32Max { val } => Intrinsic::IntIsGtU32Max {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },

                    // ---------------------------------------------------------
                    // Rational / Real
                    // ---------------------------------------------------------
                    Native::RealVal(v) => Intrinsic::RealVal(v.clone()),
                    Native::RealNeg { val } => Intrinsic::RealNeg {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealLt { lhs, rhs } => Intrinsic::RealLt {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealLe { lhs, rhs } => Intrinsic::RealLe {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealGe { lhs, rhs } => Intrinsic::RealGe {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealGt { lhs, rhs } => Intrinsic::RealGt {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealAdd { lhs, rhs } => Intrinsic::RealAdd {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealSub { lhs, rhs } => Intrinsic::RealSub {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealMul { lhs, rhs } => Intrinsic::RealMul {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealDiv { lhs, rhs } => Intrinsic::RealDiv {
                        lhs: self.resolve(lhs, Some(&Sort::Real)),
                        rhs: self.resolve(rhs, Some(&Sort::Real)),
                    },
                    Native::RealPow { base, exp } => Intrinsic::RealPow {
                        base: self.resolve(base, Some(&Sort::Real)),
                        exp: self.resolve(exp, Some(&Sort::Real)),
                    },
                    Native::RealAbs { val } => Intrinsic::RealAbs {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealRound { val } => Intrinsic::RealRound {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealFloor { val } => Intrinsic::RealFloor {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealCeil { val } => Intrinsic::RealCeil {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealIsInt { val } => Intrinsic::RealIsInt {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealToInt { val } => Intrinsic::RealToInt {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealToF32 { val } => Intrinsic::RealToF32 {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealToF64 { val } => Intrinsic::RealToF64 {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealNumer { val } => Intrinsic::RealRealer {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },
                    Native::RealDenom { val } => Intrinsic::RealDenom {
                        val: self.resolve(val, Some(&Sort::Real)),
                    },

                    // ---------------------------------------------------------
                    // String (Text)
                    // ---------------------------------------------------------
                    Native::StrVal(v) => Intrinsic::StrVal(v.clone()),
                    Native::StrLen { seq } => Intrinsic::StrLength {
                        seq: self.resolve(seq, Some(&Sort::String)),
                    },
                    Native::StrLt { lhs, rhs } => Intrinsic::StrLt {
                        lhs: self.resolve(lhs, Some(&Sort::String)),
                        rhs: self.resolve(rhs, Some(&Sort::String)),
                    },
                    Native::StrLe { lhs, rhs } => Intrinsic::StrLe {
                        lhs: self.resolve(lhs, Some(&Sort::String)),
                        rhs: self.resolve(rhs, Some(&Sort::String)),
                    },
                    Native::StrGt { lhs, rhs } => Intrinsic::StrGt {
                        lhs: self.resolve(lhs, Some(&Sort::String)),
                        rhs: self.resolve(rhs, Some(&Sort::String)),
                    },
                    Native::StrGe { lhs, rhs } => Intrinsic::StrGe {
                        lhs: self.resolve(lhs, Some(&Sort::String)),
                        rhs: self.resolve(rhs, Some(&Sort::String)),
                    },
                    Native::StrConcat { lhs, rhs } => Intrinsic::StrConcat {
                        lhs: self.resolve(lhs, Some(&Sort::String)),
                        rhs: self.resolve(rhs, Some(&Sort::String)),
                    },
                    Native::StrAt { seq, idx } => Intrinsic::StrAt {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        idx: self.resolve(idx, Some(&Sort::Integer)),
                    },
                    Native::StrContains { seq, item } => Intrinsic::StrIncludes {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        item: self.resolve(item, Some(&Sort::String)),
                    },
                    Native::StrStartsWith { seq, item } => Intrinsic::StrStartsWith {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        item: self.resolve(item, Some(&Sort::String)),
                    },
                    Native::StrEndsWith { seq, item } => Intrinsic::StrEndsWith {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        item: self.resolve(item, Some(&Sort::String)),
                    },
                    Native::StrIsEmpty { seq } => Intrinsic::StrIsEmpty {
                        seq: self.resolve(seq, Some(&Sort::String)),
                    },
                    Native::StrIsDigit { seq } => Intrinsic::StrIsDigit {
                        seq: self.resolve(seq, Some(&Sort::String)),
                    },
                    Native::StrIndexOf { seq, sub, offset } => Intrinsic::StrIndexOf {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        sub: self.resolve(sub, Some(&Sort::String)),
                        offset: self.resolve(offset, Some(&Sort::Integer)),
                    },
                    Native::StrReplace { seq, src, dst } => Intrinsic::StrReplace {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        src: self.resolve(src, Some(&Sort::String)),
                        dst: self.resolve(dst, Some(&Sort::String)),
                    },
                    Native::StrReplaceAll { seq, src, dst } => Intrinsic::StrReplaceAll {
                        seq: self.resolve(seq, Some(&Sort::String)),
                        src: self.resolve(src, Some(&Sort::String)),
                        dst: self.resolve(dst, Some(&Sort::String)),
                    },
                    Native::StrToInt { val } => Intrinsic::StrToInt {
                        val: self.resolve(val, Some(&Sort::String)),
                    },
                    Native::StrFromInt { val } => Intrinsic::StrFromInt {
                        val: self.resolve(val, Some(&Sort::Integer)),
                    },
                    Native::StrFromCode { val } => Intrinsic::StrFromCode {
                        val: self.resolve(val, Some(&Sort::U32)),
                    },
                    Native::StrToCode { val } => Intrinsic::StrToCode {
                        val: self.resolve(val, Some(&Sort::String)),
                    },

                    // ---------------------------------------------------------
                    // Cloak
                    // ---------------------------------------------------------
                    Native::BoxShield { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BoxShield { t: sort, val }
                    }
                    Native::BoxReveal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, None);
                        Intrinsic::BoxReveal { t: sort, val }
                    }

                    // ---------------------------------------------------------
                    // Sequence
                    // ---------------------------------------------------------
                    Native::SeqEmpty { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::SeqEmpty { t: sort }
                    }
                    Native::SeqLen { t, seq } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        Intrinsic::SeqLength { t: sort, seq }
                    }
                    Native::SeqUnit { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::SeqUnit { t: sort, val }
                    }
                    Native::SeqPush { t, seq, item } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        let item = self.resolve(item, Some(&sort));
                        Intrinsic::SeqAppend { t: sort, seq, item }
                    }
                    Native::SeqNth { t, seq, idx } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        let idx = self.resolve(idx, Some(&Sort::Integer));
                        Intrinsic::SeqNth { t: sort, seq, idx }
                    }
                    Native::SeqExtract {
                        t,
                        seq,
                        offset,
                        len,
                    } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        let offset = self.resolve(offset, Some(&Sort::Integer));
                        let len = self.resolve(len, Some(&Sort::Integer));
                        Intrinsic::SeqExtract {
                            t: sort,
                            seq,
                            offset,
                            len,
                        }
                    }
                    Native::SeqConcat { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SeqConcat { t: sort, lhs, rhs }
                    }
                    Native::SeqContains { t, seq, item } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        let item = self.resolve(item, Some(&sort));
                        Intrinsic::SeqIncludes { t: sort, seq, item }
                    }
                    Native::SeqPrefixOf { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SeqPrefixOf { t: sort, lhs, rhs }
                    }
                    Native::SeqSuffixOf { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SeqSuffixOf { t: sort, lhs, rhs }
                    }
                    Native::SeqReplace { t, seq, src, dst } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        let src = self.resolve(src, Some(&sort));
                        let dst = self.resolve(dst, Some(&sort));
                        Intrinsic::SeqReplace {
                            t: sort,
                            seq,
                            src,
                            dst,
                        }
                    }
                    Native::SeqIsEmpty { t, seq } => {
                        let sort = self.parent.resolve_type(t);
                        let seq = self.resolve(seq, None);
                        Intrinsic::SeqIsEmpty { t: sort, seq }
                    }

                    // ---------------------------------------------------------
                    // Set
                    // ---------------------------------------------------------
                    Native::SetEmpty { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::SetEmpty { t: sort }
                    }
                    Native::SetLen { t, set } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        Intrinsic::SetLength { t: sort, set }
                    }
                    Native::SetInsert { t, set, item } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        let item = self.resolve(item, Some(&sort));
                        Intrinsic::SetInsert { t: sort, set, item }
                    }
                    Native::SetRemove { t, set, item } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        let item = self.resolve(item, Some(&sort));
                        Intrinsic::SetRemove { t: sort, set, item }
                    }
                    Native::SetContains { t, set, item } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        let item = self.resolve(item, Some(&sort));
                        Intrinsic::SetContains { t: sort, set, item }
                    }
                    Native::SetIntersect { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetIntersection { t: sort, lhs, rhs }
                    }
                    Native::SetUnion { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetUnion { t: sort, lhs, rhs }
                    }
                    Native::SetDiff { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetDifference { t: sort, lhs, rhs }
                    }
                    Native::SetSymDiff { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetSymDiff { t: sort, lhs, rhs }
                    }
                    Native::SetIsSubset { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetIsSubset { t: sort, lhs, rhs }
                    }
                    Native::SetIsProperSubset { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetIsProperSubset { t: sort, lhs, rhs }
                    }
                    Native::SetIsSuperset { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetIsSuperset { t: sort, lhs, rhs }
                    }
                    Native::SetIsDisjoint { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, None);
                        let rhs = self.resolve(rhs, None);
                        Intrinsic::SetIsDisjoint { t: sort, lhs, rhs }
                    }
                    Native::SetHasSize { t, set, size } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        let size = self.resolve(size, Some(&Sort::Integer));
                        Intrinsic::SetHasSize { t: sort, set, size }
                    }
                    Native::SetIsEmpty { t, set } => {
                        let sort = self.parent.resolve_type(t);
                        let set = self.resolve(set, None);
                        Intrinsic::SetIsEmpty { t: sort, set }
                    }

                    // ---------------------------------------------------------
                    // Map / Array
                    // ---------------------------------------------------------
                    Native::ArrayEmpty { k, v } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        Intrinsic::MapEmpty {
                            k: k_sort,
                            v: v_sort,
                        }
                    }
                    Native::ArrayLen { k, v, arr } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        Intrinsic::MapLength {
                            k: k_sort,
                            v: v_sort,
                            map,
                        }
                    }
                    Native::ArrayStore {
                        k,
                        v,
                        arr,
                        key,
                        val,
                    } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        let key = self.resolve(key, Some(&k_sort));
                        let val = self.resolve(val, Some(&v_sort));
                        Intrinsic::MapPut {
                            k: k_sort,
                            v: v_sort,
                            map,
                            key,
                            val,
                        }
                    }
                    Native::ArraySelect { k, v, arr, key } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        let key = self.resolve(key, Some(&k_sort));
                        Intrinsic::MapGet {
                            k: k_sort,
                            v: v_sort,
                            map,
                            key,
                        }
                    }
                    Native::ArrayRemove { k, v, arr, key } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        let key = self.resolve(key, Some(&k_sort));
                        Intrinsic::MapDel {
                            k: k_sort,
                            v: v_sort,
                            map,
                            key,
                        }
                    }
                    Native::ArrayContainsKey { k, v, arr, key } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        let key = self.resolve(key, Some(&k_sort));
                        Intrinsic::MapContainsKey {
                            k: k_sort,
                            v: v_sort,
                            map,
                            key,
                        }
                    }
                    Native::ArrayIsEmpty { k, v, arr } => {
                        let k_sort = self.parent.resolve_type(k);
                        let v_sort = self.parent.resolve_type(v);
                        let map = self.resolve(arr, None);
                        Intrinsic::MapIsEmpty {
                            k: k_sort,
                            v: v_sort,
                            map,
                        }
                    }

                    // ---------------------------------------------------------
                    // Bitvector (Bv)
                    // ---------------------------------------------------------
                    Native::BvVal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let v = val.to_u64_digits().1.first().copied().unwrap_or(0);
                        Intrinsic::BvVal { t: sort, val: v }
                    }
                    Native::BvNot { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvNot { t: sort, val }
                    }
                    Native::BvNeg { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvNeg { t: sort, val }
                    }
                    Native::BvAnd { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvAnd { t: sort, lhs, rhs }
                    }
                    Native::BvOr { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvOr { t: sort, lhs, rhs }
                    }
                    Native::BvXor { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvXor { t: sort, lhs, rhs }
                    }
                    Native::BvNand { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvNand { t: sort, lhs, rhs }
                    }
                    Native::BvNor { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvNor { t: sort, lhs, rhs }
                    }
                    Native::BvXnor { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvXnor { t: sort, lhs, rhs }
                    }
                    Native::BvAdd { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvAdd { t: sort, lhs, rhs }
                    }
                    Native::BvSub { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvSub { t: sort, lhs, rhs }
                    }
                    Native::BvMul { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvMul { t: sort, lhs, rhs }
                    }
                    Native::BvDiv { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvDiv { t: sort, lhs, rhs }
                    }
                    Native::BvRem { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvRem { t: sort, lhs, rhs }
                    }
                    Native::BvMod { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvMod { t: sort, lhs, rhs }
                    }
                    Native::BvShl { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvShl { t: sort, lhs, rhs }
                    }
                    Native::BvLshr { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvLshr { t: sort, lhs, rhs }
                    }
                    Native::BvAshr { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvAshr { t: sort, lhs, rhs }
                    }
                    Native::BvRotLeft { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvRotLeft { t: sort, lhs, rhs }
                    }
                    Native::BvRotRight { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvRotRight { t: sort, lhs, rhs }
                    }
                    Native::BvLt { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvLt { t: sort, lhs, rhs }
                    }
                    Native::BvLe { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvLe { t: sort, lhs, rhs }
                    }
                    Native::BvGt { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvGt { t: sort, lhs, rhs }
                    }
                    Native::BvGe { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvGe { t: sort, lhs, rhs }
                    }
                    Native::BvRedAnd { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvRedAnd { t: sort, val }
                    }
                    Native::BvRedOr { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvRedOr { t: sort, val }
                    }
                    Native::BvToInt { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvToInt { t: sort, val }
                    }
                    Native::BvAddNoOverflow { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvAddNoOverflow { t: sort, lhs, rhs }
                    }
                    Native::BvSubNoOverflow { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvSubNoOverflow { t: sort, lhs, rhs }
                    }
                    Native::BvNegNoOverflow { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::BvNegNoOverflow { t: sort, val }
                    }
                    Native::BvMulNoOverflow { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvMulNoOverflow { t: sort, lhs, rhs }
                    }
                    Native::BvDivNoOverflow { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::BvDivNoOverflow { t: sort, lhs, rhs }
                    }

                    // ---------------------------------------------------------
                    // Float (F32, F64)
                    // ---------------------------------------------------------
                    Native::FloatVal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatVal {
                            t: sort,
                            val: val.clone(),
                        }
                    }
                    Native::FloatNaN { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatNaN { t: sort }
                    }
                    Native::FloatPosInf { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatPosInf { t: sort }
                    }
                    Native::FloatNegInf { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatNegInf { t: sort }
                    }
                    Native::FloatPosZero { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatPosZero { t: sort }
                    }
                    Native::FloatNegZero { t } => {
                        let sort = self.parent.resolve_type(t);
                        Intrinsic::FloatNegZero { t: sort }
                    }
                    Native::FloatNeg { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatNeg { t: sort, val }
                    }
                    Native::FloatAbs { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatAbs { t: sort, val }
                    }
                    Native::FloatSqrt { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatSqrt { t: sort, val }
                    }
                    Native::FloatAdd { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatAdd { t: sort, lhs, rhs }
                    }
                    Native::FloatSub { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatSub { t: sort, lhs, rhs }
                    }
                    Native::FloatMul { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatMul { t: sort, lhs, rhs }
                    }
                    Native::FloatDiv { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatDiv { t: sort, lhs, rhs }
                    }
                    Native::FloatRem { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatRem { t: sort, lhs, rhs }
                    }
                    Native::FloatMin { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatMin { t: sort, lhs, rhs }
                    }
                    Native::FloatMax { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatMax { t: sort, lhs, rhs }
                    }
                    Native::FloatIsNaN { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsNaN { t: sort, val }
                    }
                    Native::FloatIsInf { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsInf { t: sort, val }
                    }
                    Native::FloatIsZero { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsZero { t: sort, val }
                    }
                    Native::FloatIsNormal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsNormal { t: sort, val }
                    }
                    Native::FloatIsSubnormal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsSubnormal { t: sort, val }
                    }
                    Native::FloatIsNeg { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsNeg { t: sort, val }
                    }
                    Native::FloatIsPos { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatIsPos { t: sort, val }
                    }
                    Native::FloatLt { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatLt { t: sort, lhs, rhs }
                    }
                    Native::FloatLe { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatLe { t: sort, lhs, rhs }
                    }
                    Native::FloatGt { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatGt { t: sort, lhs, rhs }
                    }
                    Native::FloatGe { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::FloatGe { t: sort, lhs, rhs }
                    }
                    Native::FloatToInt { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatToInt { t: sort, val }
                    }
                    Native::FloatToReal { t, val } => {
                        let sort = self.parent.resolve_type(t);
                        let val = self.resolve(val, Some(&sort));
                        Intrinsic::FloatToReal { t: sort, val }
                    }

                    // ---------------------------------------------------------
                    // Error / Generic
                    // ---------------------------------------------------------
                    Native::ErrFresh => Intrinsic::ErrFresh,
                    Native::ErrMerge { lhs, rhs } => Intrinsic::ErrMerge {
                        lhs: self.resolve(lhs, None),
                        rhs: self.resolve(rhs, None),
                    },
                    Native::SmtEq { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::SmtEq { t: sort, lhs, rhs }
                    }
                    Native::SmtNe { t, lhs, rhs } => {
                        let sort = self.parent.resolve_type(t);
                        let lhs = self.resolve(lhs, Some(&sort));
                        let rhs = self.resolve(rhs, Some(&sort));
                        Intrinsic::SmtNe { t: sort, lhs, rhs }
                    }
                };
                Expression::Intrinsic(Box::new(intrinsic))
            }
            Op::Procedure { name, inst, args } => {
                let callee = self.parent.register_func(name, inst);
                let params = self
                    .parent
                    .ir
                    .fn_registry
                    .retrieve_sig(callee)
                    .params
                    .clone();
                if params.len() != args.len() {
                    panic!(
                        "callee argument number mismatch: expect {} | actual {}",
                        params.len(),
                        args.len()
                    );
                }

                let mut converted_args = vec![];
                for ((_, arg_sort), arg_expr) in params.into_iter().zip(args) {
                    let eid = self.resolve(arg_expr, Some(&arg_sort));
                    converted_args.push(eid);
                }
                Expression::Procedure {
                    callee,
                    args: converted_args,
                }
            }
        };

        // register the expression
        let eid = self.registry.register(expression);

        // cross-check type consistency again (this is a paranoid check)
        let derived_type = self.derive_type(eid);
        Self::check_sort(&derived_type, &sort);

        // restore the namespace
        self.namespace = old_namespace;

        // done
        eid
    }

    /// Derive type of an expression
    fn derive_type(&self, eid: ExpId) -> Sort {
        let sort = match self.registry.lookup_exp(&eid) {
            Expression::Var(vid) => self.registry.lookup_var(vid).sort.clone(),
            Expression::Pack { sort, elems: _ }
            | Expression::Tuple { sort, slots: _ }
            | Expression::Record { sort, fields: _ }
            | Expression::Enum {
                sort,
                branch: _,
                variant: _,
            } => Sort::User(*sort),
            Expression::AccessSlot { base, slot } => {
                let base_sort = self.derive_type(*base);
                let base_tuple = self.expect_type_tuple(Self::expect_sort_user(&base_sort));
                base_tuple
                    .into_iter()
                    .nth(*slot)
                    .unwrap_or_else(|| panic!("type mismatch: no slot {slot} in tuple {base_sort}"))
            }
            Expression::AccessField { base, field } => {
                let base_sort = self.derive_type(*base);
                let mut base_record = self.expect_type_record(Self::expect_sort_user(&base_sort));
                base_record
                    .remove(field)
                    .unwrap_or_else(|| {
                        panic!("type mismatch: no field {field} in record {base_sort}")
                    })
                    .clone()
            }
            Expression::Match { cases } => {
                let mut case_sort = None;
                for case in cases {
                    let sort = self.derive_type(case.body);
                    match &case_sort {
                        None => {
                            case_sort = Some(sort);
                        }
                        Some(s) => Self::check_sort(s, &sort),
                    }
                }
                match case_sort {
                    None => panic!("expect at least one match arm"),
                    Some(sort) => sort,
                }
            }
            Expression::Phi { cases, default } => {
                if cases.is_empty() {
                    panic!("expect at least one phi case");
                }
                let case_sort = self.derive_type(*default);
                for case in cases {
                    let sort = self.derive_type(case.body);
                    Self::check_sort(&case_sort, &sort);
                }
                case_sort
            }
            Expression::Forall { .. }
            | Expression::Exists { .. }
            | Expression::IterForall { .. }
            | Expression::IterExists { .. } => Sort::Boolean,
            Expression::Choose {
                vars,
                body: _,
                rets,
            } => {
                let mut inst = vec![];
                for vid in rets {
                    match vars.get(vid) {
                        None => panic!("invalid axiom variable to return"),
                        Some(sort) => {
                            inst.push(sort.clone());
                        }
                    }
                }
                // unwrap the single-element tuple for choose
                if inst.len() == 1 {
                    inst.into_iter().next().unwrap()
                } else {
                    Sort::User(self.parent.lookup_type(None, &inst))
                }
            }
            Expression::IterChoose {
                vars,
                body: _,
                rets,
            } => {
                let mut inst = vec![];
                for vid in rets {
                    match vars.get(vid) {
                        None => panic!("invalid iterator variable to return"),
                        Some(eid) => {
                            let vty = match self.derive_type(*eid) {
                                Sort::Seq(_) => Sort::Integer,
                                Sort::Set(e) => *e,
                                Sort::Array(k, _) => *k,
                                _ => panic!("not a collection sort"),
                            };
                            inst.push(vty);
                        }
                    }
                }
                // unwrap the single-element tuple for choose
                if inst.len() == 1 {
                    inst.into_iter().next().unwrap()
                } else {
                    Sort::User(self.parent.lookup_type(None, &inst))
                }
            }
            Expression::Intrinsic(intrinsic) => match intrinsic.as_ref() {
                // -------------------------------------------------------------
                // Boolean
                // -------------------------------------------------------------
                Intrinsic::BoolVal(_)
                | Intrinsic::BoolNot { .. }
                | Intrinsic::BoolAnd { .. }
                | Intrinsic::BoolOr { .. }
                | Intrinsic::BoolXor { .. }
                | Intrinsic::BoolImplies { .. }
                | Intrinsic::BoolIff { .. }
                | Intrinsic::BoolNand { .. }
                | Intrinsic::BoolNor { .. }
                | Intrinsic::BoolXnor { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Integer
                // -------------------------------------------------------------
                Intrinsic::IntVal(_)
                | Intrinsic::IntNeg { .. }
                | Intrinsic::IntAdd { .. }
                | Intrinsic::IntSub { .. }
                | Intrinsic::IntMul { .. }
                | Intrinsic::IntDiv { .. }
                | Intrinsic::IntMod { .. }
                | Intrinsic::IntRem { .. }
                | Intrinsic::IntPow { .. }
                | Intrinsic::IntAbs { .. } => Sort::Integer,

                Intrinsic::IntLt { .. }
                | Intrinsic::IntLe { .. }
                | Intrinsic::IntGe { .. }
                | Intrinsic::IntGt { .. }
                | Intrinsic::IntDivides { .. } => Sort::Boolean,

                // Integer Conversions
                Intrinsic::IntoToReal { .. } => Sort::Real,
                Intrinsic::IntToI32 { .. } => Sort::I32,
                Intrinsic::IntToI64 { .. } => Sort::I64,
                Intrinsic::IntToU32 { .. } => Sort::U32,
                Intrinsic::IntToU64 { .. } => Sort::U64,
                Intrinsic::IntToF32 { .. } => Sort::F32,
                Intrinsic::IntToF64 { .. } => Sort::F64,

                // Integer Parsing
                Intrinsic::IntFromHex { .. }
                | Intrinsic::IntFromOct { .. }
                | Intrinsic::IntFromBin { .. } => Sort::Integer,

                // Integer Range Checks
                Intrinsic::IntIsGtI64Max { .. }
                | Intrinsic::IntIsLtI64Min { .. }
                | Intrinsic::IntIsGtU64Max { .. }
                | Intrinsic::IntIsLtU64Min { .. }
                | Intrinsic::IntIsLtI32Min { .. }
                | Intrinsic::IntIsGtI32Max { .. }
                | Intrinsic::IntIsLtU32Min { .. }
                | Intrinsic::IntIsGtU32Max { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Real (Rational)
                // -------------------------------------------------------------
                Intrinsic::RealVal(_)
                | Intrinsic::RealNeg { .. }
                | Intrinsic::RealAdd { .. }
                | Intrinsic::RealSub { .. }
                | Intrinsic::RealMul { .. }
                | Intrinsic::RealDiv { .. }
                | Intrinsic::RealPow { .. }
                | Intrinsic::RealAbs { .. } => Sort::Real,

                Intrinsic::RealRound { .. }
                | Intrinsic::RealFloor { .. }
                | Intrinsic::RealCeil { .. }
                | Intrinsic::RealToInt { .. }
                | Intrinsic::RealRealer { .. } // Numerator
                | Intrinsic::RealDenom { .. } => Sort::Integer,

                Intrinsic::RealLt { .. }
                | Intrinsic::RealLe { .. }
                | Intrinsic::RealGe { .. }
                | Intrinsic::RealGt { .. }
                | Intrinsic::RealIsInt { .. } => Sort::Boolean,

                Intrinsic::RealToF32 { .. } => Sort::F32,
                Intrinsic::RealToF64 { .. } => Sort::F64,

                // -------------------------------------------------------------
                // String
                // -------------------------------------------------------------
                Intrinsic::StrVal(_)
                | Intrinsic::StrConcat { .. }
                | Intrinsic::StrAt { .. }
                | Intrinsic::StrReplace { .. }
                | Intrinsic::StrReplaceAll { .. }
                | Intrinsic::StrFromInt { .. }
                | Intrinsic::StrFromCode { .. } => Sort::String,

                Intrinsic::StrLength { .. }
                | Intrinsic::StrIndexOf { .. }
                | Intrinsic::StrToInt { .. } => Sort::Integer,

                Intrinsic::StrToCode { .. } => Sort::U32,

                Intrinsic::StrLt { .. }
                | Intrinsic::StrLe { .. }
                | Intrinsic::StrGt { .. }
                | Intrinsic::StrGe { .. }
                | Intrinsic::StrIsEmpty { .. }
                | Intrinsic::StrIncludes { .. }
                | Intrinsic::StrStartsWith { .. }
                | Intrinsic::StrEndsWith { .. }
                | Intrinsic::StrIsDigit { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Cloak
                // -------------------------------------------------------------
                Intrinsic::BoxShield { t, .. } => t.clone(),
                Intrinsic::BoxReveal { t, .. } => t.clone(),

                // -------------------------------------------------------------
                // Sequence
                // -------------------------------------------------------------
                Intrinsic::SeqEmpty { t }
                | Intrinsic::SeqUnit { t, .. }
                | Intrinsic::SeqAppend { t, .. }
                | Intrinsic::SeqExtract { t, .. }
                | Intrinsic::SeqConcat { t, .. }
                | Intrinsic::SeqReplace { t, .. } => Sort::Seq(Box::new(t.clone())),

                Intrinsic::SeqAt { t, .. }
                | Intrinsic::SeqNth { t, .. } => t.clone(),

                Intrinsic::SeqLength { .. } => Sort::Integer,

                Intrinsic::SeqIncludes { .. }
                | Intrinsic::SeqPrefixOf { .. }
                | Intrinsic::SeqSuffixOf { .. }
                | Intrinsic::SeqIsEmpty { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Set
                // -------------------------------------------------------------
                Intrinsic::SetEmpty { t }
                | Intrinsic::SetInsert { t, .. }
                | Intrinsic::SetRemove { t, .. }
                | Intrinsic::SetIntersection { t, .. }
                | Intrinsic::SetUnion { t, .. }
                | Intrinsic::SetDifference { t, .. }
                | Intrinsic::SetSymDiff { t, .. } => Sort::Set(Box::new(t.clone())),

                Intrinsic::SetLength { .. } => Sort::Integer,

                Intrinsic::SetContains { .. }
                | Intrinsic::SetIsEmpty { .. }
                | Intrinsic::SetIsSubset { .. }
                | Intrinsic::SetIsProperSubset { .. }
                | Intrinsic::SetIsSuperset { .. }
                | Intrinsic::SetIsDisjoint { .. }
                | Intrinsic::SetHasSize { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Map / Array
                // -------------------------------------------------------------
                Intrinsic::MapEmpty { k, v }
                | Intrinsic::MapPut { k, v, .. }
                | Intrinsic::MapDel { k, v, .. } => {
                    Sort::Array(Box::new(k.clone()), Box::new(v.clone()))
                }

                Intrinsic::MapGet { v, .. } => v.clone(),
                Intrinsic::MapLength { .. } => Sort::Integer,
                Intrinsic::MapContainsKey { .. }
                | Intrinsic::MapIsEmpty { .. } => Sort::Boolean,

                // -------------------------------------------------------------
                // Bitvector
                // -------------------------------------------------------------
                Intrinsic::BvVal { t, .. }
                | Intrinsic::BvNot { t, .. }
                | Intrinsic::BvNeg { t, .. }
                | Intrinsic::BvAnd { t, .. }
                | Intrinsic::BvOr { t, .. }
                | Intrinsic::BvXor { t, .. }
                | Intrinsic::BvNand { t, .. }
                | Intrinsic::BvNor { t, .. }
                | Intrinsic::BvXnor { t, .. }
                | Intrinsic::BvAdd { t, .. }
                | Intrinsic::BvSub { t, .. }
                | Intrinsic::BvMul { t, .. }
                | Intrinsic::BvDiv { t, .. }
                | Intrinsic::BvRem { t, .. }
                | Intrinsic::BvMod { t, .. }
                | Intrinsic::BvShl { t, .. }
                | Intrinsic::BvLshr { t, .. }
                | Intrinsic::BvAshr { t, .. }
                | Intrinsic::BvRotLeft { t, .. }
                | Intrinsic::BvRotRight { t, .. } => t.clone(),

                Intrinsic::BvLt { .. }
                | Intrinsic::BvLe { .. }
                | Intrinsic::BvGt { .. }
                | Intrinsic::BvGe { .. }
                | Intrinsic::BvRedAnd { .. }
                | Intrinsic::BvRedOr { .. }
                | Intrinsic::BvAddNoOverflow { .. }
                | Intrinsic::BvSubNoOverflow { .. }
                | Intrinsic::BvNegNoOverflow { .. }
                | Intrinsic::BvMulNoOverflow { .. }
                | Intrinsic::BvDivNoOverflow { .. } => Sort::Boolean,

                Intrinsic::BvToInt { .. } => Sort::Integer,

                // -------------------------------------------------------------
                // Float
                // -------------------------------------------------------------
                Intrinsic::FloatVal { t, .. }
                | Intrinsic::FloatNaN { t }
                | Intrinsic::FloatPosInf { t }
                | Intrinsic::FloatNegInf { t }
                | Intrinsic::FloatPosZero { t }
                | Intrinsic::FloatNegZero { t }
                | Intrinsic::FloatNeg { t, .. }
                | Intrinsic::FloatAbs { t, .. }
                | Intrinsic::FloatSqrt { t, .. }
                | Intrinsic::FloatAdd { t, .. }
                | Intrinsic::FloatSub { t, .. }
                | Intrinsic::FloatMul { t, .. }
                | Intrinsic::FloatDiv { t, .. }
                | Intrinsic::FloatRem { t, .. }
                | Intrinsic::FloatMin { t, .. }
                | Intrinsic::FloatMax { t, .. } => t.clone(),

                Intrinsic::FloatIsNaN { .. }
                | Intrinsic::FloatIsInf { .. }
                | Intrinsic::FloatIsZero { .. }
                | Intrinsic::FloatIsNormal { .. }
                | Intrinsic::FloatIsSubnormal { .. }
                | Intrinsic::FloatIsNeg { .. }
                | Intrinsic::FloatIsPos { .. }
                | Intrinsic::FloatLt { .. }
                | Intrinsic::FloatLe { .. }
                | Intrinsic::FloatGt { .. }
                | Intrinsic::FloatGe { .. } => Sort::Boolean,

                Intrinsic::FloatToInt { .. } => Sort::Integer,
                Intrinsic::FloatToReal { .. } => Sort::Real,

                // -------------------------------------------------------------
                // Error / Generic
                // -------------------------------------------------------------
                Intrinsic::ErrFresh
                | Intrinsic::ErrMerge { .. } => Sort::Error,

                Intrinsic::SmtEq { .. }
                | Intrinsic::SmtNe { .. } => Sort::Boolean,
            },
            Expression::Procedure { callee, args: _ } => self
                .parent
                .ir
                .fn_registry
                .retrieve_sig(*callee)
                .ret_ty
                .clone(),
        };
        sort
    }

    /// Materialize the entire function (signature + body, if any)
    pub fn materialize(
        mut parent: IRBuilder<'a, 'ctx>,
        sig: &FunSig,
        body: &Expr,
    ) -> (ExpRegistry, ExpId) {
        // initialize the registry and builder
        let mut registry = ExpRegistry::new(); // the registry is empty at the beginning
        let mut builder = ExpBuilder::new(&mut parent, &mut registry, &sig.params); // the namespace is the signature parameters at the beginning

        // build the expression
        // resolve takes an expression and the expected return type and returns the expression id which corresponds to the expression
        let id = builder.resolve(body, Some(&sig.ret_ty));

        // done
        (registry, id)
    }
}

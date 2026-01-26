//! Module for parsing expressions

use crate::parser::{
    adt::{MatchAnalyzer, MatchOrganizer},
    apply::TypeFn,
    ctxt::ContextWithSig,
    dsl::{Quantifier, SysMacroName},
    func::{CastFuncName, FuncName, FuncSig, SysFuncName},
    generics::Generics,
    infer::{TypeRef, TypeUnifier, ti_unify},
    intrinsics::Intrinsic,
    name::{UsrFuncName, UsrTypeName, VarName},
    path::{ADTBranch, ExprPathAsCallee, ExprPathAsRecord, ExprPathAsTarget, QualifiedPath},
    ty::{CtxtForType, EnumVariant, SysTypeName, TypeBody, TypeDef, TypeTag},
};
use crate::{bail_if_exists, bail_if_missing, bail_if_non_empty, bail_on};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use syn::{
    Arm, Block, Expr as Exp, ExprBlock, ExprCall, ExprField, ExprIf, ExprMatch, ExprMethodCall,
    ExprParen, ExprStruct, ExprTuple, ExprUnary, FieldValue, Local, LocalInit, Member, Pat,
    PatTuple, PatType, Result, Stmt, UnOp,
};

/// Marks a declaration of variable(s)
#[derive(Clone, Debug)]
pub enum VarDecl {
    One(VarName, TypeRef), // for identifiers with types for example `let x: i32 = 42;` stores the (x, i32) pair
    Pack(Vec<VarDecl>), // for tuples with types for example `let (x, y): (i32, i32) = (1, 2);` stores the [(x, i32), (y, i32)] pair
}

impl VarDecl {
    /// Derive the type of this decl
    pub fn ty(&self) -> TypeRef {
        match self {
            Self::One(_, t) => t.clone(),
            Self::Pack(elems) => TypeRef::Pack(elems.iter().map(|d| d.ty()).collect()),
        }
    }

    /// Visitor with ty traversal function
    /// Only used in the Expr::visit function
    /// Basically traverses all the VarDecls in the VarDecl and applies the ty function to the TypeRef in the VarDecl
    pub fn visit<TY>(&mut self, ty: &mut TY) -> Result<()>
    where
        TY: FnMut(&mut TypeRef) -> Result<()>,
    {
        match self {
            Self::One(_, t) => ty(t)?, // early return the error
            Self::Pack(elems) => {
                for decl in elems {
                    decl.visit(ty)?;
                }
            }
        }
        Ok(())
    }
}

impl Display for VarDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::One(name, ty) => write!(f, "{name}:{ty}"),
            Self::Pack(decls) => {
                write!(f, "({})", decls.iter().format(","))
            }
        }
    }
}

/// Variable Declaration
/// For example, `let x: i32 = 42;`, this only encapsulates the left-hand side of the let binding that is `x: i32`
pub struct LetDecl {
    // it is a mapping to TypeRef (instead of TypeTag) because if there is no type, we have a TypeRef::Var where the type is subsequently inferred
    pub vars: BTreeMap<VarName, TypeRef>, // vars is only to prevent duplicate declarations of the same variable inside one let statement. Also the vars is added to the overall vars to prevent duplicate declarations of the same variable in the whole function
    pub decl: VarDecl,                    // marks the current declaration (name and type)
}

impl LetDecl {
    /// Parse a pattern for declaration only (left-hand side of the let binding)
    ///
    /// # Arguments
    ///
    /// * vars: the variables in scope, this is a mutable reference because we add new variables to it
    /// * unifier: the type unifier used to unify the types
    /// * pat: the pattern without any potential type
    /// * ety: the expected type, if any
    ///
    /// This function is used in the `LetDecl::parse` function to parse the pattern for the declaration and create the VarDecl
    fn parse_decl(
        vars: &mut BTreeMap<VarName, TypeRef>,
        unifier: &mut TypeUnifier,
        pat: &Pat,
        ety: Option<&TypeTag>,
    ) -> Result<VarDecl> {
        // If the original pattern was not Pat::Type, then it can only be Pat::Ident or Pat::Tuple.
        // If the original pattern was Pat::Type like `let x: i32 = 42;` in the LetDecl::parse function, we abstracted the type and the inner pattern, i.e., `x` can only be Pat::Ident or Pat::Tuple.
        let decl = match pat {
            // let x: i32 = 42; or let x = 42; both are processed here
            Pat::Ident(ident) => {
                let name: VarName = pat.try_into()?; // Returns an error if the pattern is not an identifier or includes references, mutability, or sub-patterns. Otherwise, an identifier is extracted. If the identifier is reserved or an underscore, an error is returned. Otherwise, the VarName is returned.
                let ty = match ety {
                    // when we use TypeUnifier::new() at the start of the parsing process, it creates a new TypeUnifier like: TypeUnifier { typing: Typing::new() }
                    // Typing::new() creates a new Typing ( Self { params: BTreeMap::new(), groups: vec![] } )
                    // unifier.mk_var() creates a new TypeVar(usize) by calling typing.mk_var()
                    // It creates a new TypeVar(usize) by incrementing the counter in Typing and adding the new TypeVar(usize) to the params BTreeMap
                    None => TypeRef::Var(unifier.mk_var()), // if there was no type, we create a new variable with a new type (for later type inference)
                    Some(t) => {
                        if matches!(t, TypeTag::Pack(_)) {
                            bail_on!(ident, "expect a non-pack type"); // if there was a type, and the type was a pack, we return an error (situation like `let x: (i32, i32) = (1, 2);` is not allowed) or even let (x,y):((i32,i32), i32) = ((1,2), 3); is not allowed ...
                        }
                        t.into() // converts the TypeTag to TypeRef which is trivial because TypeTag is a subset of TypeRef (TypeRef only has an extra Var variant)
                    }
                };
                // if the variable name is already declared in the let statement, we return an error
                // this will only happen in case of tuple patterns like `let (x, x) = (1, 2);` where the same variable is declared twice
                if vars.insert(name.clone(), ty.clone()).is_some() {
                    bail_on!(ident, "duplicated name");
                }
                VarDecl::One(name, ty)
            }
            // let (x, y) = (1, 2); or let (x, y): (i32, i32) = (1, 2); both are processed here
            Pat::Tuple(PatTuple {
                attrs: _,
                paren_token: _,
                elems,
            }) => {
                let mut decls = vec![];
                match ety {
                    None => {
                        // if there was no type, for each element in the tuple, we create a new variable with a new type by unifying
                        for item in elems {
                            let sub = Self::parse_decl(vars, unifier, item, None)?;
                            decls.push(sub); // VarDecl::Pack(decls) encapsulates a list of VarDecls
                        }
                    }
                    Some(TypeTag::Pack(pack)) => {
                        // if there was a type, we expect it to be a pack type and the number of elements in the pack type should match the number of elements in the tuple for example `let (x, y, z): (i32, i32) = (1, 2, 3);` is not allowed
                        if pack.len() != elems.len() {
                            bail_on!(elems, "pack size mismatch");
                        }
                        // for each element in the tuple and the corresponding type in the pack type, we create a new variable with the given type
                        for (e, t) in elems.iter().zip(pack) {
                            let sub = Self::parse_decl(vars, unifier, e, Some(t))?;
                            decls.push(sub);
                        }
                    }
                    // if there was a type, but it was not a pack type for a tuple, we return an error
                    _ => bail_on!(elems, "expect a pack type"),
                }
                VarDecl::Pack(decls)
            }
            // if the pattern without any potential type was not Pat::Ident or Pat::Tuple, we return an error
            _ => bail_on!(
                pat,
                "unrecognized pattern for declaration - expect an identifier or a tuple"
            ),
        };
        Ok(decl)
    }

    /// Creates the AST for a let binding statement by parsing the pattern and potentially the type
    /// This only encapsulates the left-hand side of the let binding for example `let x: i32 = 42;` where `x: i32` is the pattern
    /// Used for each Local statement in the convert_stmts function to convert the statement to an expression
    /// ctxt: ExprParserRoot implements the CtxtForType trait, providing the generics (retrieves the generics of the current function under analysis) and get_type_generics (retrieves the generics of the given user defined type from the whole context) functions
    /// The ctxt is used in the conversion of the Type to TypeTag
    /// unifier: the type unifier used to unify the types
    /// pat: the pattern to be parsed for the declaration for example `let x: i32 = 42;` where `x: i32` is the pattern
    pub fn parse<T: CtxtForExpr>(
        ctxt: &T,
        unifier: &mut TypeUnifier,
        pat_one: &Pat,
    ) -> Result<Self> {
        // separate types and names
        let (pat_res, ty_opt) = match pat_one {
            // A type ascription pattern: `foo: f64`.
            Pat::Type(PatType {
                attrs: _,
                pat,
                colon_token: _,
                ty,
            }) => (pat.as_ref(), Some(TypeTag::from_type(ctxt, ty.as_ref())?)), // we take the inners pattern; the type is converted to TypeTag
            _ => (pat_one, None), // for example if it is Pat::Ident(PatIdent) like `foo` there are no types
        };

        // parse them recursively
        let mut vars = BTreeMap::new(); // vars is only to prevent duplicate declarations of the same variable inside ONE let statement. Also the vars is added to the overall vars to prevent duplicate declarations of the same variable in the whole function. At the start of parsing the sole let statement, vars is empty.
        let decl = Self::parse_decl(&mut vars, unifier, pat_res, ty_opt.as_ref())?; // parse_decl creates the VarDecl for the pattern
        Ok(Self { vars, decl })
    }
}

/// Let bindings
/// AST for a let binding statement
/// The decl is an AST for the left-hand side of the let binding for example `let x: i32 = 42;` where `x: i32` is the decl
/// The bind is an AST for the right-hand side of the let binding for example `let x: i32 = 42;` where `42` is the bind
/// LetBinding is a struct encapsulating the declaration (VarDecl) and the binding of a variable (Expr)
/// VarDecl is a struct encapsulating the variable name and its type (VarName, TypeRef)
/// Expr encapsulates the Instruction (Inst) which encapsulates the operation (Op) and the type of the operation (TypeRef). This type is the same as the type of the variable in the VarDecl. The Op encapsulates the kind of expression.
#[derive(Clone, Debug)]
pub struct LetBinding {
    pub decl: VarDecl,
    pub bind: Expr,
}

impl Display for LetBinding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "let {} := {}", self.decl, self.bind)
    }
}

/// A context suitable for expr analysis
/// This trait is only implemented for the ExprParserRoot struct through the whole codebase
/// ExprParserRoot is the parser for the function under analysis
/// CtxtForType is a supertrait of CtxtForExpr meaning that any type that implements CtxtForExpr must also implement CtxtForType
pub trait CtxtForExpr: CtxtForType {
    /// Retrieve the definition of a user-defined type from the whole context
    fn get_type_def(&self, name: &UsrTypeName) -> Option<&TypeDef>;

    /// Retrieve the definition of an enum variant
    /// Converts ADTBranch to EnumVariant
    fn get_adt_variant_details(&self, branch: &ADTBranch) -> Option<&EnumVariant> {
        // retrieve the type definition of the type name of the branch
        let def = self.get_type_def(&branch.ty_name)?; // if the type is not found, we return None
        let variant = match &def.body {
            // def.body has the TypeBody enum which has the Enum variants
            TypeBody::Enum(adt) => adt.variants.get(&branch.variant)?, // adt.variants is a BTreeMap<String, EnumVariant> where the key is the variant name and the value is the EnumVariant. If the variant is not found, we return None
            _ => return None, // if the type is not an enum, we return None
        };
        Some(variant)
    }

    /// Retrieve the signature of a user-defined function
    fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn>;

    /// Retrieve the signature for a user function on a system type (a.k.a., an intrinsic)
    fn lookup_usr_func_on_sys_type(
        &self,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn>;
}

/// Bindings through unpacking during match
#[derive(Clone, Debug)]
pub enum Unpack {
    Unit,
    Tuple(BTreeMap<usize, VarName>),
    Record(BTreeMap<String, VarName>),
}

impl Display for Unpack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "()"),
            Self::Tuple(binds) => {
                let content = binds
                    .iter()
                    .format_with(",", |(k, v), f| f(&format_args!("{}:{}", *k, v)));
                write!(f, "[{content}]")
            }
            Self::Record(binds) => {
                let content = binds
                    .iter()
                    .format_with(",", |(k, v), f| f(&format_args!("{k}:{v}")));
                write!(f, "{{{content}}}")
            }
        }
    }
}

/// Marks how a variable of an ADT type is matched
#[derive(Clone, Debug)]
pub struct MatchVariant {
    pub branch: ADTBranch,
    pub unpack: Unpack,
}

impl Display for MatchVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.branch, self.unpack)
    }
}

/// Marks how all variables in the match head are matched
#[derive(Clone, Debug)]
pub struct MatchCombo {
    pub variants: Vec<MatchVariant>,
    pub body: Expr,
}

impl Display for MatchCombo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) => {}", self.variants.iter().format(","), self.body)
    }
}

/// Phi node, guarded by condition
#[derive(Clone, Debug)]
pub struct PhiNode {
    pub cond: Expr, // the condition of the if statement
    pub body: Expr, // the then part of the if statement
}

impl Display for PhiNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if {} => {}", self.cond, self.body)
    }
}

/// Operations
#[derive(Clone, Debug)]
pub enum Op {
    /// `<var>`
    Var(VarName),
    /// `(a1, a2, ...)`
    Pack { elems: Vec<Expr> },
    /// `<tuple>(a1, a2, ...)`
    Tuple {
        name: UsrTypeName,
        inst: Vec<TypeRef>,
        slots: Vec<Expr>,
    },
    /// `<record>{ f1: a1, f2: a2, ... }`
    Record {
        name: UsrTypeName,
        inst: Vec<TypeRef>,
        fields: BTreeMap<String, Expr>,
    },
    /// `<adt>::<branch>`
    EnumUnit {
        branch: ADTBranch,
        inst: Vec<TypeRef>,
    },
    /// `<adt>::<branch>(a1, a2, ...)`
    EnumTuple {
        branch: ADTBranch,
        inst: Vec<TypeRef>,
        slots: Vec<Expr>,
    },
    /// `<adt>::<branch>{ f1: a1, f2: a2, ... }`
    EnumRecord {
        branch: ADTBranch,
        inst: Vec<TypeRef>,
        fields: BTreeMap<String, Expr>,
    },
    /// `<base>.<index>`
    AccessSlot { base: Expr, slot: usize },
    /// `<base>.<field>`
    AccessField { base: Expr, field: String },
    /// `match (v1, v2, ...) { (a1, a2, ...) => <body1> } ...`
    Match {
        heads: Vec<Expr>,
        combo: Vec<MatchCombo>,
    },
    /// `if (<c1>) { <v1> } else if (<c2>) { <v2> } ... else { <default> }`
    Phi { nodes: Vec<PhiNode>, default: Expr },
    /// `forall!(|<v>: <t>| {<expr>})`
    Forall {
        vars: Vec<(VarName, TypeTag)>,
        body: Expr,
    },
    /// `exists!(|<v>: <t>| {<expr>})`
    Exists {
        vars: Vec<(VarName, TypeTag)>,
        body: Expr,
    },
    /// `choose!(|<v>: <t>| {<expr>})`
    Choose {
        vars: Vec<(VarName, TypeTag)>,
        body: Expr,
    },
    /// `forall!(<v> in <c> ... => <expr>)`
    IterForall {
        vars: Vec<(VarName, Expr)>,
        body: Expr,
    },
    /// `exists!(<v> in <c> ... => <expr>)`
    IterExists {
        vars: Vec<(VarName, Expr)>,
        body: Expr,
    },
    /// `choose!(<v> in <c> ... => <expr>)`
    IterChoose {
        vars: Vec<(VarName, Expr)>,
        body: Expr,
    },
    /// `<class>::<method>(<a1>, <a2>, ...)`
    Intrinsic(Box<Intrinsic>),
    /// `<function>(<a1>, <a2>, ...)`
    Procedure {
        name: UsrFuncName,
        inst: Vec<TypeRef>,
        args: Vec<Expr>,
    },
}

impl Display for Op {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(name) => name.fmt(f),
            Self::Pack { elems } => {
                write!(f, "({})", elems.iter().format(","))
            }
            Self::Tuple { name, inst, slots } => {
                write!(
                    f,
                    "{}<{}>({})",
                    name,
                    inst.iter().format(","),
                    slots.iter().format(",")
                )
            }
            Self::Record { name, inst, fields } => {
                write!(
                    f,
                    "{}<{}>({})",
                    name,
                    inst.iter().format(","),
                    fields
                        .iter()
                        .format_with(",", |(k, v), p| { p(&format_args!("{k}:{v}")) })
                )
            }
            Self::EnumUnit { branch, inst } => {
                write!(f, "{}<{}>", branch, inst.iter().format(","))
            }
            Self::EnumTuple {
                branch,
                inst,
                slots,
            } => {
                write!(
                    f,
                    "{}<{}>({})",
                    branch,
                    inst.iter().format(","),
                    slots.iter().format(",")
                )
            }
            Self::EnumRecord {
                branch,
                inst,
                fields,
            } => {
                write!(
                    f,
                    "{}<{}>({})",
                    branch,
                    inst.iter().format(","),
                    fields
                        .iter()
                        .format_with(",", |(k, v), p| { p(&format_args!("{k}:{v}")) })
                )
            }
            Self::AccessSlot { base, slot } => {
                write!(f, "{base}.{slot}")
            }
            Self::AccessField { base, field } => {
                write!(f, "{base}.{field}")
            }
            Self::Match { heads, combo } => {
                writeln!(f, "match ({}) {{", heads.iter().format(","))?;
                for case in combo {
                    writeln!(f, "  case {case}")?;
                }
                write!(f, "}}")
            }
            Self::Phi { nodes, default } => {
                writeln!(f, "phi {{")?;
                for node in nodes {
                    writeln!(f, "  {node}")?;
                }
                writeln!(f, "  default => {default}")?;
                write!(f, "}}")
            }
            Self::Forall { vars, body } => {
                write!(
                    f,
                    "forall [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{n}:{t}"))),
                    body
                )
            }
            Self::Exists { vars, body } => {
                write!(
                    f,
                    "exists [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{n}:{t}"))),
                    body
                )
            }
            Self::Choose { vars, body } => {
                write!(
                    f,
                    "choose [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{n}:{t}"))),
                    body
                )
            }
            Self::IterForall { vars, body } => {
                write!(
                    f,
                    "forall [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{n} in {h}"))),
                    body
                )
            }
            Self::IterExists { vars, body } => {
                write!(
                    f,
                    "exists [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{n} in {h}"))),
                    body
                )
            }
            Self::IterChoose { vars, body } => {
                write!(
                    f,
                    "choose [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{n} in {h}"))),
                    body
                )
            }
            Self::Intrinsic(intrinsic) => intrinsic.fmt(f),
            Self::Procedure { name, inst, args } => {
                write!(
                    f,
                    "{}<{}>({})",
                    name,
                    inst.iter().format(","),
                    args.iter().format(",")
                )
            }
        }
    }
}

/// An instruction is an operation with a type
/// The Op encapsulates the kind of expression and the TypeRef encapsulates the type of the expression
#[derive(Clone, Debug)]
pub struct Inst {
    pub op: Box<Op>,
    pub ty: TypeRef,
}

impl Display for Inst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.op, self.ty)
    }
}

/// Expressions
/// An expression is either a single instruction or a block of instructions
#[derive(Clone, Debug)]
pub enum Expr {
    /// a single instruction
    Unit(Inst),
    /// `{ let <v1> = ...; let <v2> = ...; ...; <op>(<v1>, <v2>, ...) }`
    Block { lets: Vec<LetBinding>, body: Inst },
}

impl Expr {
    /// Retrieve the type of an expression
    pub fn ty(&self) -> &TypeRef {
        let i = match self {
            Self::Unit(inst) => inst,
            Self::Block { lets: _, body } => body,
        };
        &i.ty
    }

    /// Visitor with ty/pre/post traversal function
    /// ty: function to apply on each TypeRef in the expression
    /// pre: function to apply on the current expression before visiting the expression
    /// post: function to apply on the current expression after visiting the expression
    /// This function is used in ExprParserRoot::parse after converting the statements to expressions (convert_stmts function)
    pub fn visit<TY, PRE, POST>(
        &mut self,
        ty: &mut TY,
        pre: &mut PRE,
        post: &mut POST,
    ) -> Result<()>
    where
        TY: FnMut(&mut TypeRef) -> Result<()>,
        PRE: FnMut(&mut Self) -> Result<()>,
        POST: FnMut(&mut Self) -> Result<()>,
    {
        // pre-order invocation (applies the function on the current expression)
        pre(self)?;

        // get the instruction from the expression
        let inst = match self {
            Expr::Unit(inst) => inst,
            Expr::Block { lets, body } => {
                for binding in lets {
                    binding.decl.visit(ty)?; // the left-hand side of the let binding which is a declaration is visited (the ty function is applied on each TypeRef in the declaration)
                    binding.bind.visit(ty, pre, post)?; // the right-hand side of the let binding which is an expression is visited
                }
                body
            }
        };

        // run on type
        ty(&mut inst.ty)?;

        // run on opcode
        match inst.op.as_mut() {
            Op::Var(_) => (),
            Op::Pack { elems } => {
                for elem in elems {
                    elem.visit(ty, pre, post)?;
                }
            }
            Op::Tuple {
                name: _,
                inst,
                slots,
            } => {
                for targ in inst {
                    ty(targ)?;
                }
                for slot in slots {
                    slot.visit(ty, pre, post)?;
                }
            }
            Op::Record {
                name: _,
                inst,
                fields,
            } => {
                for targ in inst {
                    ty(targ)?;
                }
                for field in fields.values_mut() {
                    field.visit(ty, pre, post)?;
                }
            }
            Op::EnumUnit { branch: _, inst } => {
                for targ in inst {
                    ty(targ)?;
                }
            }
            Op::EnumTuple {
                branch: _,
                inst,
                slots,
            } => {
                for targ in inst {
                    ty(targ)?;
                }
                for slot in slots {
                    slot.visit(ty, pre, post)?;
                }
            }
            Op::EnumRecord {
                branch: _,
                inst,
                fields,
            } => {
                for targ in inst {
                    ty(targ)?;
                }
                for field in fields.values_mut() {
                    field.visit(ty, pre, post)?;
                }
            }
            Op::AccessSlot { base, slot: _ } | Op::AccessField { base, field: _ } => {
                base.visit(ty, pre, post)?;
            }
            Op::Match { heads, combo } => {
                for head in heads {
                    head.visit(ty, pre, post)?;
                }
                for item in combo {
                    item.body.visit(ty, pre, post)?;
                }
            }
            Op::Phi { nodes, default } => {
                for node in nodes {
                    node.cond.visit(ty, pre, post)?;
                    node.body.visit(ty, pre, post)?;
                }
                default.visit(ty, pre, post)?;
            }
            Op::Forall { vars: _, body }
            | Op::Exists { vars: _, body }
            | Op::Choose { vars: _, body } => {
                body.visit(ty, pre, post)?;
            }
            Op::IterForall { vars, body }
            | Op::IterExists { vars, body }
            | Op::IterChoose { vars, body } => {
                for (_, bind) in vars {
                    bind.visit(ty, pre, post)?;
                }
                body.visit(ty, pre, post)?;
            }
            Op::Intrinsic(intrinsic) => match intrinsic.as_mut() {
                // -------------------------------------------------------------------------
                // Boolean
                // -------------------------------------------------------------------------
                Intrinsic::BoolVal(_) => (),
                Intrinsic::BoolNot { val } => val.visit(ty, pre, post)?,
                Intrinsic::BoolAnd { lhs, rhs }
                | Intrinsic::BoolOr { lhs, rhs }
                | Intrinsic::BoolXor { lhs, rhs }
                | Intrinsic::BoolImplies { lhs, rhs }
                | Intrinsic::BoolIff { lhs, rhs }
                | Intrinsic::BoolNand { lhs, rhs }
                | Intrinsic::BoolNor { lhs, rhs }
                | Intrinsic::BoolXnor { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Integer
                // -------------------------------------------------------------------------
                Intrinsic::IntVal(_) => (),
                // Unary
                Intrinsic::IntNeg { val }
                | Intrinsic::IntAbs { val }
                | Intrinsic::IntToReal { val }
                | Intrinsic::IntToI32 { val }
                | Intrinsic::IntToI64 { val }
                | Intrinsic::IntToU32 { val }
                | Intrinsic::IntToU64 { val }
                | Intrinsic::IntToF32 { val }
                | Intrinsic::IntToF64 { val }
                | Intrinsic::IntFromHex { val }
                | Intrinsic::IntFromOct { val }
                | Intrinsic::IntFromBin { val }
                | Intrinsic::IntIsGtI64Max { val }
                | Intrinsic::IntIsLtI64Min { val }
                | Intrinsic::IntIsGtU64Max { val }
                | Intrinsic::IntIsLtU64Min { val }
                | Intrinsic::IntIsLtI32Min { val }
                | Intrinsic::IntIsGtI32Max { val }
                | Intrinsic::IntIsLtU32Min { val }
                | Intrinsic::IntIsGtU32Max { val } => {
                    val.visit(ty, pre, post)?;
                }
                // Binary
                Intrinsic::IntAdd { lhs, rhs }
                | Intrinsic::IntSub { lhs, rhs }
                | Intrinsic::IntMul { lhs, rhs }
                | Intrinsic::IntDiv { lhs, rhs }
                | Intrinsic::IntMod { lhs, rhs }
                | Intrinsic::IntRem { lhs, rhs }
                | Intrinsic::IntPow {
                    base: lhs,
                    exp: rhs,
                }
                | Intrinsic::IntDivides { lhs, rhs }
                | Intrinsic::IntLt { lhs, rhs }
                | Intrinsic::IntLe { lhs, rhs }
                | Intrinsic::IntGt { lhs, rhs }
                | Intrinsic::IntGe { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Real
                // -------------------------------------------------------------------------
                Intrinsic::RealVal(_) => (),
                // Unary
                Intrinsic::RealNeg { val }
                | Intrinsic::RealAbs { val }
                | Intrinsic::RealRound { val }
                | Intrinsic::RealFloor { val }
                | Intrinsic::RealCeil { val }
                | Intrinsic::RealIsInt { val }
                | Intrinsic::RealToInt { val }
                | Intrinsic::RealToF32 { val }
                | Intrinsic::RealToF64 { val }
                | Intrinsic::RealNumer { val }
                | Intrinsic::RealDenom { val } => {
                    val.visit(ty, pre, post)?;
                }
                // Binary
                Intrinsic::RealAdd { lhs, rhs }
                | Intrinsic::RealSub { lhs, rhs }
                | Intrinsic::RealMul { lhs, rhs }
                | Intrinsic::RealDiv { lhs, rhs }
                | Intrinsic::RealPow {
                    base: lhs,
                    exp: rhs,
                }
                | Intrinsic::RealLt { lhs, rhs }
                | Intrinsic::RealLe { lhs, rhs }
                | Intrinsic::RealGt { lhs, rhs }
                | Intrinsic::RealGe { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // String
                // -------------------------------------------------------------------------
                Intrinsic::StrVal(_) => (),
                // Unary (on seq or val)
                Intrinsic::StrLen { seq: val }
                | Intrinsic::StrIsEmpty { seq: val }
                | Intrinsic::StrIsDigit { seq: val }
                | Intrinsic::StrToInt { val }
                | Intrinsic::StrFromInt { val }
                | Intrinsic::StrFromCode { val }
                | Intrinsic::StrToCode { val } => {
                    val.visit(ty, pre, post)?;
                }
                // Binary
                Intrinsic::StrConcat { lhs, rhs }
                | Intrinsic::StrAt { seq: lhs, idx: rhs }
                | Intrinsic::StrContains {
                    seq: lhs,
                    item: rhs,
                }
                | Intrinsic::StrStartsWith {
                    seq: lhs,
                    item: rhs,
                }
                | Intrinsic::StrEndsWith {
                    seq: lhs,
                    item: rhs,
                }
                | Intrinsic::StrLe { lhs, rhs }
                | Intrinsic::StrLt { lhs, rhs }
                | Intrinsic::StrGe { lhs, rhs }
                | Intrinsic::StrGt { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // Ternary
                Intrinsic::StrIndexOf { seq, sub, offset } => {
                    seq.visit(ty, pre, post)?;
                    sub.visit(ty, pre, post)?;
                    offset.visit(ty, pre, post)?;
                }
                Intrinsic::StrReplace { seq, src, dst }
                | Intrinsic::StrReplaceAll { seq, src, dst } => {
                    seq.visit(ty, pre, post)?;
                    src.visit(ty, pre, post)?;
                    dst.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Cloak
                // -------------------------------------------------------------------------
                Intrinsic::BoxShield { t, val } | Intrinsic::BoxReveal { t, val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Sequence
                // -------------------------------------------------------------------------
                Intrinsic::SeqEmpty { t } => {
                    ty(t)?;
                }
                Intrinsic::SeqUnit { t, val }
                | Intrinsic::SeqLen { t, seq: val }
                | Intrinsic::SeqIsEmpty { t, seq: val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }
                Intrinsic::SeqNth { t, seq, idx }
                | Intrinsic::SeqPush { t, seq, item: idx }
                | Intrinsic::SeqContains { t, seq, item: idx } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                    idx.visit(ty, pre, post)?;
                }
                Intrinsic::SeqConcat { t, lhs, rhs }
                | Intrinsic::SeqPrefixOf { t, lhs, rhs }
                | Intrinsic::SeqSuffixOf { t, lhs, rhs } => {
                    ty(t)?;
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                Intrinsic::SeqExtract {
                    t,
                    seq,
                    offset,
                    len,
                } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                    offset.visit(ty, pre, post)?;
                    len.visit(ty, pre, post)?;
                }
                Intrinsic::SeqReplace { t, seq, src, dst } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                    src.visit(ty, pre, post)?;
                    dst.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Set
                // -------------------------------------------------------------------------
                Intrinsic::SetEmpty { t } => {
                    ty(t)?;
                }
                Intrinsic::SetLen { t, set: val } | Intrinsic::SetIsEmpty { t, set: val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }
                Intrinsic::SetInsert { t, set, item }
                | Intrinsic::SetRemove { t, set, item }
                | Intrinsic::SetContains { t, set, item } => {
                    ty(t)?;
                    set.visit(ty, pre, post)?;
                    item.visit(ty, pre, post)?;
                }
                Intrinsic::SetHasSize { t, set, size } => {
                    ty(t)?;
                    set.visit(ty, pre, post)?;
                    size.visit(ty, pre, post)?;
                }
                Intrinsic::SetIntersect { t, lhs, rhs }
                | Intrinsic::SetUnion { t, lhs, rhs }
                | Intrinsic::SetDiff { t, lhs, rhs }
                | Intrinsic::SetSymDiff { t, lhs, rhs }
                | Intrinsic::SetIsSubset { t, lhs, rhs }
                | Intrinsic::SetIsProperSubset { t, lhs, rhs }
                | Intrinsic::SetIsSuperset { t, lhs, rhs }
                | Intrinsic::SetIsDisjoint { t, lhs, rhs } => {
                    ty(t)?;
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Array / Map
                // -------------------------------------------------------------------------
                Intrinsic::ArrayEmpty { k, v } => {
                    ty(k)?;
                    ty(v)?;
                }
                Intrinsic::ArrayLen { k, v, arr } | Intrinsic::ArrayIsEmpty { k, v, arr } => {
                    ty(k)?;
                    ty(v)?;
                    arr.visit(ty, pre, post)?;
                }
                Intrinsic::ArraySelect { k, v, arr, key }
                | Intrinsic::ArrayRemove { k, v, arr, key }
                | Intrinsic::ArrayContainsKey { k, v, arr, key } => {
                    ty(k)?;
                    ty(v)?;
                    arr.visit(ty, pre, post)?;
                    key.visit(ty, pre, post)?;
                }
                Intrinsic::ArrayStore {
                    k,
                    v,
                    arr,
                    key,
                    val,
                } => {
                    ty(k)?;
                    ty(v)?;
                    arr.visit(ty, pre, post)?;
                    key.visit(ty, pre, post)?;
                    val.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Bitvector
                // -------------------------------------------------------------------------
                Intrinsic::BvVal { t, val: _ } => {
                    ty(t)?;
                }
                Intrinsic::BvNot { t, val }
                | Intrinsic::BvRedAnd { t, val }
                | Intrinsic::BvRedOr { t, val }
                | Intrinsic::BvNeg { t, val }
                | Intrinsic::BvNegNoOverflow { t, val }
                | Intrinsic::BvToInt { t, val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }
                Intrinsic::BvAnd { t, lhs, rhs }
                | Intrinsic::BvOr { t, lhs, rhs }
                | Intrinsic::BvXor { t, lhs, rhs }
                | Intrinsic::BvNand { t, lhs, rhs }
                | Intrinsic::BvNor { t, lhs, rhs }
                | Intrinsic::BvXnor { t, lhs, rhs }
                | Intrinsic::BvAdd { t, lhs, rhs }
                | Intrinsic::BvSub { t, lhs, rhs }
                | Intrinsic::BvMul { t, lhs, rhs }
                | Intrinsic::BvDiv { t, lhs, rhs }
                | Intrinsic::BvRem { t, lhs, rhs }
                | Intrinsic::BvMod { t, lhs, rhs }
                | Intrinsic::BvAddNoOverflow { t, lhs, rhs }
                | Intrinsic::BvSubNoOverflow { t, lhs, rhs }
                | Intrinsic::BvMulNoOverflow { t, lhs, rhs }
                | Intrinsic::BvDivNoOverflow { t, lhs, rhs }
                | Intrinsic::BvShl { t, lhs, rhs }
                | Intrinsic::BvLshr { t, lhs, rhs }
                | Intrinsic::BvAshr { t, lhs, rhs }
                | Intrinsic::BvRotLeft { t, lhs, rhs }
                | Intrinsic::BvRotRight { t, lhs, rhs }
                | Intrinsic::BvLt { t, lhs, rhs }
                | Intrinsic::BvLe { t, lhs, rhs }
                | Intrinsic::BvGt { t, lhs, rhs }
                | Intrinsic::BvGe { t, lhs, rhs } => {
                    ty(t)?;
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Float
                // -------------------------------------------------------------------------
                Intrinsic::FloatNaN { t }
                | Intrinsic::FloatPosInf { t }
                | Intrinsic::FloatNegInf { t }
                | Intrinsic::FloatPosZero { t }
                | Intrinsic::FloatNegZero { t }
                | Intrinsic::FloatVal { t, val: _ } => {
                    ty(t)?;
                }
                Intrinsic::FloatNeg { t, val }
                | Intrinsic::FloatAbs { t, val }
                | Intrinsic::FloatSqrt { t, val }
                | Intrinsic::FloatIsNaN { t, val }
                | Intrinsic::FloatIsInf { t, val }
                | Intrinsic::FloatIsZero { t, val }
                | Intrinsic::FloatIsNormal { t, val }
                | Intrinsic::FloatIsSubnormal { t, val }
                | Intrinsic::FloatIsNeg { t, val }
                | Intrinsic::FloatIsPos { t, val }
                | Intrinsic::FloatToInt { t, val }
                | Intrinsic::FloatToReal { t, val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }
                Intrinsic::FloatAdd { t, lhs, rhs }
                | Intrinsic::FloatSub { t, lhs, rhs }
                | Intrinsic::FloatMul { t, lhs, rhs }
                | Intrinsic::FloatDiv { t, lhs, rhs }
                | Intrinsic::FloatRem { t, lhs, rhs }
                | Intrinsic::FloatMin { t, lhs, rhs }
                | Intrinsic::FloatMax { t, lhs, rhs }
                | Intrinsic::FloatLt { t, lhs, rhs }
                | Intrinsic::FloatLe { t, lhs, rhs }
                | Intrinsic::FloatGt { t, lhs, rhs }
                | Intrinsic::FloatGe { t, lhs, rhs } => {
                    ty(t)?;
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }

                // -------------------------------------------------------------------------
                // Error / Generic
                // -------------------------------------------------------------------------
                Intrinsic::ErrFresh { .. } => (),
                Intrinsic::ErrMerge { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                Intrinsic::SmtEq { t, lhs, rhs } | Intrinsic::SmtNe { t, lhs, rhs } => {
                    ty(t)?;
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
            },
            Op::Procedure {
                name: _,
                inst,
                args,
            } => {
                for targ in inst {
                    ty(targ)?;
                }
                for arg in args {
                    arg.visit(ty, pre, post)?;
                }
            }
        }

        // post-order invocation (applies the function on the current expression)
        post(self)?;

        // done
        Ok(())
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit(inst) => inst.fmt(f),
            Self::Block { lets, body } => {
                for binding in lets {
                    writeln!(f, "{binding};")?;
                }
                write!(f, "{body}")
            }
        }
    }
}

#[derive(Clone, Debug)]
/// Root expression parser for each function as a whole
pub struct ExprParserRoot<'ctx> {
    /// context provider
    ctxt: &'ctx ContextWithSig,
    /// function generics
    generics: Generics,
    /// function parameters
    params: BTreeMap<VarName, TypeRef>,
    /// function return type
    ret_ty: TypeRef,
}

#[derive(Clone, Debug)]
/// Derived expression parser (for one and only one expression)
struct ExprParserCursor<'r, 'ctx: 'r> {
    /// parent of the parser (this will always remain the same because we are parsing only inside one function - so the context and the signature will remain the same)
    root: &'r ExprParserRoot<'ctx>,
    /// expected type (this changes depending on whether the expression is a let-binding, method call, etc.)
    exp_ty: TypeRef,
    /// variables in scope and their types (at the start, the variables in scope are the function parameters. As new variables are declared, they are added to this BTreeMap)
    vars: BTreeMap<VarName, TypeRef>,
    /// new let-bindings created, if any (for example `let x = 42;` creates a new let-binding. At the start, there are no let-bindings)
    bindings: Vec<LetBinding>, // LetBinding is a struct with decl: VarDecl and bind: Expr
}

impl<'ctx> ExprParserRoot<'ctx> {
    /// Creating a new context for parsing a single function body
    /// For each separate function, a new ExprParserRoot is created
    pub fn new(ctxt: &'ctx ContextWithSig, sig: &FuncSig) -> Self {
        Self {
            ctxt, // the whole context of the rust file containing all the types and funcs
            generics: sig.generics.clone(), // generics, params, ret_ty are derived from the function signature
            params: sig
                .param_map() // param_map() converts the Vec<(VarName, TypeTag)> to BTreeMap<VarName, TypeTag>
                .into_iter()
                .map(|(k, v)| (k, (&v).into())) // converting the BTreeMap<VarName, TypeTag> to BTreeMap<VarName, TypeRef>
                .collect(), // the (&v).into() converts the TypeTag to TypeRef
            ret_ty: (&sig.ret_ty).into(), // conversion from TypeTag to TypeRef. This is the return type of the function
        }
    }

    /// Parse the body of a single function
    ///
    /// This function is called when ContextWithSig::parse_func_body() is called in ctxt.rs
    /// The self contains the whole context of the rust file, signature of the current function
    /// The function also has a body which is a Vec<Stmt> (stmts) and is passed to this function
    /// This function basically converts the body statements to an expression
    pub fn parse(self, stmts: &[Stmt]) -> Result<Expr> {
        // create an empty unifier at the start of the parsing
        // pub struct TypeUnifier { typing: Typing }
        // `typing` is the internal typing worker that performs unification.
        // pub struct Typing {
        //     params: BTreeMap<usize, usize>,
        //     groups: Vec<TypeEquivGroup>,
        // }
        // When TypeUnifier::new() is called, it creates a new Typing with empty params and groups
        let mut unifier = TypeUnifier::new(); // for each separate function body, a new unifier is created (so unification process cannot happen across different functions)

        // derive a parser
        // ExprParserCursor is used to parse the current expression
        // ExprParserRoot is the root parser for the whole function under consideration
        let exp_ty = self.ret_ty.clone(); // the return type of the function is the expected type of ExprParserCursor at the start (the default expected type). This is because ExprParserCursor is used to parse the current expression. Every function body has some optional local let-binding statements and a mandatory sole expression (which is the return value of the function). Note that every function in `rusmart` must have a return value. Therefore, ExprParserCursor `definitely` parses the sole last expression of the body and the expected type of this expression is the function return type. For other expressions (for example right-hand side of a let-binding), the expected type needs to be altered. The is done by `forking` the parser to the defined type.
        let parser = ExprParserCursor {
            root: &self, // the root parser which encapsulates the following details: ContextWithSig (the whole context of the rust file), Generics (generics), BTreeMap<VarName, TypeRef> (Params), and TypeRef (return type)
            exp_ty, // at the start, the expected type is the return type of the function (the default expected type)
            vars: self.params.clone(), // at the start, the variables in scope are the function parameters
            bindings: vec![],          // no new let-bindings created at the start
        };

        // the unifier is created at the start of the parsing and passed on to the convert_stmts function for two reasons:
        // 1. when we have a variable declaration without an explicit type, the type of the variable is a freshly made TypeRef::Var and it is added to the unifier
        // 2. when we want to convert an expression to an ADT self-defined expression, the unifier is passed on the the convert_expr function to unify the types
        // with the given parser, convert the statements to an expression
        let mut parsed = parser.convert_stmts(&mut unifier, stmts)?;

        // check type completeness of the expression
        // basically this function traverses the whole expression and replaces any TypeRef::Var with the actual type (if the type is known) and if not, returns an error
        let result = parsed.visit(
            &mut |ty| {
                // unnessecary to refresh the type if we had just used the type unification in forming the OP in the convert_expr function.
                let refreshed = unifier.refresh_type(ty); // replaces any TypeRef::Var with the actual type (if the type is known). because this is inside visit, it traverses the whole `parsed` expression and replaces all the TypeRef::Var with the inferred types

                // if there are still any TypeRef::Var left, the type is incomplete and we return an error
                // this will happen when the type variable is not unified to a concrete type
                if !refreshed.validate() {
                    // Return an error - it will be caught and re-thrown with proper span info below
                    return Err(syn::Error::new(
                        syn::spanned::Spanned::span(&stmts.last().expect("at least one statement")),
                        format!(
                            "incomplete type: {} (some type variables could not be inferred)",
                            refreshed
                        ),
                    ));
                }
                *ty = refreshed; // replace the type with the inferred type in `parsed` expression
                Ok(())
            },
            &mut |_| Ok(()), // pre-order traversal function - does nothing in this case
            &mut |_| Ok(()), // post-order traversal function - does nothing in this case
        );

        match result {
            Ok(()) => Ok(parsed),
            Err(e) => bail_on!(stmts.last().expect("at least one statement"), "{}", e), // this will be invoked when the type is incomplete
        }
    }
}

/// Implementing the CtxtForType trait for the ExprParserRoot struct
impl CtxtForType for ExprParserRoot<'_> {
    /// Retrieve the generics of the current function in the signature
    fn generics(&self) -> &Generics {
        &self.generics
    }

    /// Retrieve the type parameters (generics) of a user-defined type from the whole context
    fn get_type_generics(&self, name: &UsrTypeName) -> Option<&Generics> {
        self.ctxt.get_type_generics(name)
    }
}

/// Implementing the CtxtForExpr trait for the ExprParserRoot struct
impl CtxtForExpr for ExprParserRoot<'_> {
    /// Retrieve the definition of a user-defined type from the whole context
    fn get_type_def(&self, name: &UsrTypeName) -> Option<&TypeDef> {
        self.ctxt.get_type_def(name)
    }

    /// Retrieve the signature of a user-defined function given the function name.
    fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn> {
        self.ctxt.fn_db.lookup_unqualified(name)
    }

    /// Retrieve the function type for a user function on a system type (a.k.a., an intrinsic) and inherenting the kind from the current function
    fn lookup_usr_func_on_sys_type(
        &self,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.ctxt
            .fn_db
            .lookup_usr_func_on_sys_type(ty_name, fn_name)
    }
}

impl<'r, 'ctx: 'r> ExprParserCursor<'r, 'ctx> {
    /// Fork a child parser (this basically redefines the expected type)
    /// The root containing the whole context of the rust file, and the signature of the function, is retained because we are still processing the same function body
    /// The expected type can be the return value of a function, the type of a let-binding, etc. When it has to be redefined, we fork the parser.
    /// The variables in scope are retained.
    /// The new let-bindings created are empty. This is because we only fork inside type definitions to redefine the new expected type with the type annotation (if present). If we do pass on the let bindings, it will for example render all the rh side expressions of (let x = expr) to be a Expr::Block. The main purpose of these bindings is to just add the local declarations to the final exp to create the expression tree of the whole body when the convert_expr function is called on the sole expression statement of the function body.
    fn fork(&self, exp_ty: TypeRef) -> Self {
        Self {
            root: self.root,
            exp_ty,                  // the expected type is redefined
            vars: self.vars.clone(), // retain the variables in the current scope
            bindings: vec![],        // no new let-bindings created
        }
    }

    /// Consume a stream of statements
    ///
    /// After the cursor parser is created, the statements are converted to an expression
    ///
    /// Arguments:
    ///
    /// * self: the parser cursor containing the root parser, the expected type, the variables in scope, and the new let-bindings created
    /// * unifier: the type unifier
    /// * stmts: the statements in the function body
    ///
    /// The root parser contains the following details: ContextWithSig (the whole context of the rust file), and Function signature (Generics, Params, and Return type)
    /// The expected type is the return type of the function at the start
    /// The variables in scope are the function parameters at the start and will contain all the local variables in the current scope
    /// The new let-bindings created are empty at the start and will contain all the new let-bindings created inside function body
    /// The type unifier is used for type unification
    /// The stmts are the statements in the function body
    ///
    /// Returns:
    ///
    /// * Result<Expr>: the returning expression of the function - every function should have a return value
    ///
    /// The Expr is the self-defined expression type which contains the following enum variants: Unit(Inst) and Block{lets: Vec<LetBinding>, body: Inst})
    ///
    /// The function can only contain one `expression statement` and the rest should be let-bindings (Local statements). The let-bindings are added to the bindings Vec<LetBinding>. The vars keeps track of conflicting variable names in the current scope. The Expr result, is the conversion of the sole expression statement to the self-defined expression type.
    fn convert_stmts(mut self, unifier: &mut TypeUnifier, stmts: &[Stmt]) -> Result<Expr> {
        // at the start no expression is found
        let mut expr_found = None;
        let mut iter = stmts.iter();
        // iterate over the statements
        for stmt in iter.by_ref() {
            // parse the statement
            match stmt {
                // update the variables in scope and the new let-bindings created
                Stmt::Local(binding) => {
                    // this is a let-binding
                    let Local {
                        attrs: _,
                        let_token: _,
                        pat,
                        init,
                        semi_token: _,
                    } = binding;

                    // if the init is missing, bail (the init is the right-hand side of the let-binding)
                    // for example, in let x = 5, = 5 is the init
                    // all let declarations must have an initializer
                    let body = bail_if_missing!(
                        init,
                        binding,
                        "initializer for let-binding statement missing"
                    );
                    let LocalInit {
                        eq_token: _,
                        expr,
                        diverge,
                    } = body;

                    // diverge should not exist
                    // for example, this is not allowed: let Ok(x) = 5 else { panic!() }; because it has a diverge
                    // Note that let x = if false { 5 } else { panic!() } does not have a diverge...
                    bail_if_exists!(diverge.as_ref().map(|(_, div)| div));

                    // extract names and the type (if present) from the pattern
                    // vars is a list of variables and their types in the current sole let-binding statement
                    // if the type is not specified, it is TypeRef::Var(TypeVar(usize))) with a unique usize). for example, let x = 5; will have x with TypeRef::Var(TypeVar(0))
                    // decl encapsulates the current variable and its type in one let statement
                    // the sole purpose of vars is to check if there are conflicting variable names in one let statement. Also, the vars are added to the overall vars in the current scope.
                    // potential conflicts in one let statement only happens in tuples like let (x, x) = (1, 2);
                    let LetDecl { vars, decl } = LetDecl::parse(self.root, unifier, pat)?;

                    // here we are adding the variables defined in the current let statement to the current scope
                    for (name, ty) in vars {
                        // self.vars contains all the variables in the current scope
                        if self.vars.insert(name, ty).is_some() {
                            bail_on!(pat, "conflicting variable names"); // if the variable already exists in the current scope, bail
                        }
                    }

                    // parse the body with a new parser
                    // the new parser has the same root, the expected type is the type of the let-binding, and the variables are retained, but the new let-binding is initialized as empty
                    // `fork` is used to redefine the expected type of the parser. The let-binding is empty because the right-hand side of the let-binding is a unit and not a block (in our design choice)
                    let body_expr = self.fork(decl.ty()).convert_expr(unifier, expr)?;

                    // add the let-binding to the list of let-bindings
                    // the let-binding contains the declaration and the binding of a variable
                    // done, continue to next statement
                    self.bindings.push(LetBinding {
                        decl,
                        bind: body_expr,
                    });
                }
                Stmt::Expr(expr, semi_token) => {
                    // For return statements, semicolon is allowed and expected
                    // For other expressions (last expression), semicolon is not allowed
                    if !matches!(expr, Exp::Return(_)) {
                        bail_if_exists!(semi_token);
                    }

                    // mark that we have found the main expression in the statements
                    expr_found = Some(expr);

                    // end of loop
                    // any statement after this will not be parsed because it is not executable
                    break;
                }
                // inside the function you can't have items (like structs, enums, etc.) or macros
                Stmt::Item(_) | Stmt::Macro(_) => bail_on!(stmt, "unexpected item"),
            }
        }

        // bail if there are more statements
        // for example, if there are statements after the return value of the function
        bail_if_exists!(iter.next());

        // check that we have an expression and convert it if so
        match expr_found {
            None => bail_on!(
                stmts.last().expect("at least one statement"), // there should be at least one statement in the function body (because every function should have a return value so at least one expression should be there)
                "unable to find the main expression" // because every function should have a return value so every function needs to have an expression
            ),
            Some(expr) => self.convert_expr(unifier, expr), // convert the expression to a self-defined expression type
        }
    }

    /// Parse an expression
    ///
    /// Arguments:
    ///
    /// * Exp is the expression to be parsed
    /// * TypeUnifier is for type unification
    ///
    /// Returns:
    ///
    /// * Expr is the self-defined ADT expression type which contains the following enum variants: Unit(Inst) and Block{lets: Vec<LetBinding>, body: Inst}
    ///
    /// In a standard function body, `convert_expr` is called for the right hand side of each `let-binding` statement and the sole expression statement which is placed at the end of the function body.
    fn convert_expr(self, unifier: &mut TypeUnifier, target: &Exp) -> Result<Expr> {
        // case on expression type
        let op = match target {
            // A path like `std::mem::replace` possibly containing generic
            // parameters and a qualified self-type.
            //
            // A plain identifier like `x` is a path of length 1.
            Exp::Path(expr_path) => {
                // converts a ExprPath to ExprPathAsTarget
                // the root contains the whole context of the rust file & the signature of the function
                match ExprPathAsTarget::parse(self.root, expr_path)? {
                    // if the path only has one segment
                    ExprPathAsTarget::Var(name) => {
                        // self.vars contains all the local variables in the current scope
                        let ty = match self.vars.get(&name) {
                            None => bail_on!(expr_path, "unknown local variable"), // if the variable is not found in the current scope, bail
                            Some(t) => t.clone(), // if the variable is found in the current scope, get its type
                        };

                        // unify the type and then build the opcode
                        // ti_unify! is a macro that unifies the two types using the unifier (in let a = 5; and let b = a; the type of a and b should be unifiable)
                        // ty is the return type of the variable and self.exp_ty is the expected type
                        // if the return type is not compatible with the expected type, bail (the last argument is the target expression so that if an error occurs defines the span of the error)
                        ti_unify!(unifier, &ty, &self.exp_ty, target);
                        Op::Var(name)
                    }
                    // if the path has two segments. For example:
                    // enum MyEnum<T> {
                    //     Variant1,
                    //     Variant2(T),
                    // }
                    // let x: syn::Expr = syn::parse_quote! { MyEnum::<i32>::Variant2(1)}; // i32 is optional here
                    // The above is Expr::Call (...).
                    // This is a path with two segments: let x: syn::Expr = syn::parse_quote! { MyEnum::<i32>::Variant1}; // here if <i32> is removed, an error will occur
                    // i32 is not a segment of the path. It is a type argument to the enum.
                    ExprPathAsTarget::EnumUnit(adt) => {
                        // ADTPath encapsulates the enum type name, the variant name, plus the type arguments (if any). The type arguments of the enum are matched with the generics of the enum definition to form the GenericsInstPartial.
                        // The GenericsInstPartial originally contains the matching TypeTag taken from the argument to the Type Parameter taken from the definition
                        // ADTBranch is a struct encapsulating the enum type name and the variant name
                        // Through calling the complete function on the ADTPath, the GenericsInstPartial is converted to GenericsInstFull and the ADTBranch is derived
                        // The difference is that each TypeTag is converted to TypeRef, if there is no TypeTag (None), it is converted to TypeRef::Var
                        let (branch, inst) = adt.complete(unifier);
                        // self.root is ExprParserRoot which contains the whole context of the rust file and signature of the function
                        // get_adt_variant_details converts the ADTBranch to EnumVariant
                        // It first looks up the type name in the context, if not found, it bails
                        // If type found, but not an enum, it bails
                        // Then it looks up the variant name in the definition of the type, if not found, it bails
                        // Otherwise, it returns the EnumVariant for the variant name
                        let variant = match self.root.get_adt_variant_details(&branch) {
                            None => bail_on!(expr_path, "[invariant] no such type or variant"), // if the type or variant is not found in the context, bail. The NONE will never happen because it is checked in line let ty_name: UsrTypeName = ident.try_into()?; and later in `if !variants.contains_key(&variant)` in the ADTPath::from_path function
                            Some(details) => details,
                        };
                        // This will never happen! because if not unit enum variant, it will be Expr::Call (if tuple) or Expr::Struct (if record) not Expr::Path
                        if !matches!(variant, EnumVariant::Unit) {
                            bail_on!(expr_path, "not a unit variant");
                        }

                        // make_ty creates a TypeRef::User with the given type name and the given type arguments
                        let ret_ty = inst.make_ty(branch.ty_name.clone()); // branch.ty_name gets the enum type name

                        // the last argument is the target expression so that if an error occurs defines the span of the error
                        // ti_unify! is a macro that unifies the two types using the unifier
                        // let a : MyEnum<i32> = MyEnum::<i32>::Variant1; // unifies MyEnum<i32> on the left with MyEnum on the right (in this case, they are compatible)
                        ti_unify!(unifier, &ret_ty, &self.exp_ty, target);

                        // build the opcode
                        Op::EnumUnit {
                            branch,           // the enum type name and the variant name
                            inst: inst.vec(), // the corresponding typerefs for each type parameter in the GenericsInstFull (this is a list of TypeRef arguments in the call). This may still contain TypeRef::Var. So what happens with the result of the unification? (ti_unify!). basically the ti_unify! updates the unifier. In the ExprParserRoot::parse function after converting the statements to an expression, the unifier is used to check if there are any TypeRef::Var left. If there are, it receives the unified type and if still incomplete, it bails.
                        }
                    }
                }
            }
            // A tuple expression: `(a, b, c, d)`.
            Exp::Tuple(expr_tuple) => {
                let ExprTuple {
                    attrs: _,
                    paren_token: _,
                    elems,
                } = expr_tuple;

                // parse the elements by tentatively marking their types as variables
                let mut parsed_elems = vec![];
                for elem in elems {
                    let parsed = self
                        .fork(TypeRef::Var(unifier.mk_var()))
                        .convert_expr(unifier, elem)?; // the expected type for each element is a fresh type variable
                    parsed_elems.push(parsed);
                }

                // construct the tuple type
                // an expression is either Unit(Inst) or Block{lets: Vec<LetBinding>, body: Inst}
                // e.ty() retrieves the type field of the instruction (Inst has two fields: op and ty)
                // ty is unifier.refresh_type(&self.exp_ty) where self.exp_ty is TypeRef::Var(unifier.mk_var()) - it may get unified with another type
                let ty_pack = parsed_elems.iter().map(|e| e.ty().clone()).collect();

                // unify the return type
                ti_unify!(unifier, &TypeRef::Pack(ty_pack), &self.exp_ty, target);

                // done
                Op::Pack {
                    elems: parsed_elems, // again the unified type is not used here. So in the visit function call after the conversion of the statements to an expression, the unifier is used to convert the TypeRef::Var to the actual type (refresh_type). If the type is still incomplete, it bails.
                }
            }
            // if the expression is a struct expression
            // A struct literal expression: `Point { x: 1, y: 1 }`.
            //
            // The `rest` provides the value of the remaining fields as in `S { a:
            // 1, b: 1, ..rest }`.
            Exp::Struct(expr_struct) => {
                // unpacking the struct expression (the fields are ignored)
                let ExprStruct {
                    attrs: _,
                    qself, // anything between <...> in the path at the start of the struct expression
                    path,
                    brace_token: _, // {}
                    fields: _,
                    dot2_token,
                    rest,
                } = expr_struct;
                // qself, dot2_token, and rest should not exist
                bail_if_exists!(qself.as_ref().map(|q| q.ty.as_ref()));
                // let p1 = Point { x: 5, y: 10 };
                // let p2 = Point { y: 20, ..p1 }; // this is not allowed (has dot2_token .. - has rest p1)
                bail_if_exists!(dot2_token);
                bail_if_exists!(rest);

                // ExprPathAsRecord::parse converts a Path to an ExprPathAsRecord
                // analyze the path
                match ExprPathAsRecord::parse(self.root, path)? {
                    // if the path has one segment (these are standalone structs)
                    // the record is RecordPath which contains the user type name and the GenericsInstPartial and is built from the path (RecordPath::from_path(ctxt, path)?)
                    ExprPathAsRecord::CtorRecord(record) => {
                        // when calling complete on the RecordPath, it returns the user type name as is and the GenericsInstFull
                        // so basically the GenericsInstPartial is converted to GenericsInstFull (meaning that the TypeTag is converted to TypeRef and if None, it is converted to TypeRef::Var). So this means that for the type parameters (generics) in the enum/struct definition, if there are no corresponding explicit type arguments, they are converted to TypeRef::Var. Otherwise, they are converted to the corresponding TypeRef.
                        let (ty_name, inst) = record.complete(unifier);

                        // fields is a map for each field name in the struct definition to the corresponding type ref. If in the definition of the struct, the field type is a type parameter, the corresponding type ref used in the arguments is added to the map. Otherwise, the type tag is simply converted to type ref and added to the map.
                        let fields = match self.root.get_type_def(&ty_name) {
                            None => bail_on!(expr_struct, "[invariant] no such type"), // if the type is not found in the context, bail (this will never happen because it is checked in match ctxt.get_type_def(&ty_name) in the RecordPath::from_path function)
                            Some(def) => match &def.body {
                                // def.head has the generics and the def.body has the type body
                                // a struct is a record type (tuple structs fall under Expr::Call and unit structs cannot exist and even if they did they will be Expr::Path)
                                // record_def is a TypeRecord type that encapsulates the fields of the record (map of field name to type tag)
                                TypeBody::Record(record_def) => {
                                    let mut fields_instantiated = BTreeMap::new();
                                    // for each field and respective type tag in the record definition
                                    for (k, t) in &record_def.fields {
                                        // inst is the GenericsInstFull
                                        // what inst.instantiate(t) does is that 1) if t is a type tag other than a type parameter, it returns the type ref (the conversion is straightforward) 2) if t is a type parameter, it returns the type ref corresponding to the type parameter in the GenericsInstFull. The resulting type ref is the one used in the arguments when calling the struct. For example, if the struct definition is struct MyStruct<T> { x: T }, then MyStruct::<i32> { x: 5 } will have x as TypeRef::Integer.
                                        match inst.instantiate(t) {
                                            // if the type of a field has a type parameter than does not exist in the GenericsInstFull (definition of the type), bail (this will never happen as in making the GenericsInstPartial, the generics are made by taking the type parameters from the definition of the type)
                                            None => bail_on!(
                                                expr_struct,
                                                "[invariant] no such type parameter in struct field {}",
                                                k
                                            ),
                                            Some(instantiated) => {
                                                fields_instantiated.insert(k.clone(), instantiated); // otherwise convert the TypeTag to TypeRef and add it to the fields_instantiated with its field name (for type parameters, the corresponding type ref used in the arguments is added)
                                            }
                                        }
                                    }
                                    fields_instantiated // fields_instantiated is a map of field name to TypeRef
                                }
                                _ => bail_on!(expr_struct, "[invariant] not a record type"), // this will never happen either because it is checked in the RecordPath::from_path function
                            },
                        };

                        // Make a TypeRef::User with the ty_name as the user type name
                        // The arguments are the type arguments of inst (GenericsInstFull) which are the using in the struct call. So ret_ty is for the struct call.
                        let ret_ty = inst.make_ty(ty_name.clone());

                        // parse the arguments
                        // fields is a map of field name to TypeRef in the struct definition
                        // expr_struct is the struct expression in the call (this is passed on to access the field in the call)
                        // parse_record_fields checks if the fields in the call and definition are compatible
                        let field_vals = self.parse_record_fields(unifier, &fields, expr_struct)?;

                        // unity the return type on the right hand side (ret_ty) with the expected type annotation on the left hand side (self.exp_ty)
                        // expr_struct is the target expression so that if an error occurs defines the span of the error.
                        ti_unify!(unifier, &ret_ty, &self.exp_ty, expr_struct);

                        // build the opcode
                        Op::Record {
                            name: ty_name,      // the user type name
                            inst: inst.vec(), // the type arguments of the user type name (struct call)
                            fields: field_vals, // the fields of the struct call
                        }
                    }

                    // if the path has two segments
                    ExprPathAsRecord::CtorEnum(adt) => {
                        let (branch, inst) = adt.complete(unifier);
                        let variant = match self.root.get_adt_variant_details(&branch) {
                            None => bail_on!(expr_struct, "[invariant] no such type or variant"),
                            Some(details) => details,
                        };

                        // definition
                        let fields = match variant {
                            // because the enum variant is a record struct
                            EnumVariant::Record(record_def) => {
                                let mut fields_instantiated = BTreeMap::new();
                                // for each field and respective type tag in the record definition
                                for (k, t) in &record_def.fields {
                                    match inst.instantiate(t) {
                                        // this will throw an error for:
                                        // enum MyEnum {
                                        //     Variant1,
                                        //     Variant2 { x: T },
                                        // }
                                        // correction: enum MyEnum<T> {...}
                                        None => bail_on!(
                                            expr_struct,
                                            "[invariant] no such type parameter in enum variant {} and field {}",
                                            branch.variant,
                                            k
                                        ),
                                        Some(instantiated) => {
                                            fields_instantiated.insert(k.clone(), instantiated);
                                        }
                                    }
                                }
                                fields_instantiated
                            }
                            // this will never happen because it is already checked in the ADTPath::from_path function
                            _ => bail_on!(expr_struct, "not a record variant"),
                        };

                        // make_ty creates a TypeRef::User with the given type name and the given type arguments
                        let ret_ty = inst.make_ty(branch.ty_name.clone());

                        // parse the arguments
                        let field_vals = self.parse_record_fields(unifier, &fields, expr_struct)?;

                        // unity the return type on the right hand side (ret_ty) with the expected type annotation on the left hand side (self.exp_ty)
                        ti_unify!(unifier, &ret_ty, &self.exp_ty, expr_struct);

                        // build the opcode
                        Op::EnumRecord {
                            branch,             // the type name and the variant name
                            inst: inst.vec(), // the type arguments of the user type name (enum variant call)
                            fields: field_vals, // the fields of the enum variant (struct) call
                        }
                    }
                }
            }
            // Access of a named struct field (`obj.k`) or unnamed tuple struct field (`obj.0`).
            Exp::Field(expr_field) => {
                let ExprField {
                    attrs: _,
                    base, // the base expression (struct)
                    dot_token: _,
                    member, // the field name or index (field name for record and index for tuple struct)
                } = expr_field;

                // tentatively parse the receiver without assuming its type
                // convert the base expression to Expr ADT
                // the base may fall under the Exp::Path for example
                let parsed_base = self
                    .fork(TypeRef::Var(unifier.mk_var()))
                    .convert_expr(unifier, base)?;
                // parsed_base.ty() receives the refreshed type of the self.exp_ty (the expected type) - this is the type of the base expression which has been initialized to TypeRef::Var(unifier.mk_var()).
                let base_ty = match unifier.refresh_type(parsed_base.ty()) {
                    TypeRef::Var(_) => bail_on!(base, "type annotation needed"), // it cannot still be a type variable because the type of the base expression is needed to access the field
                    TypeRef::User(ty_name, ty_args) => match self.root.get_type_def(&ty_name) {
                        // then type needs to be a struct user type and marked with #[smt_type]. If not, bail
                        None => bail_on!(base, "no such type"),
                        Some(def) => {
                            // the number of generics in the type definition should match the number of type arguments in the base expression
                            if def.head.params.len() != ty_args.len() {
                                bail_on!(base, "type parameter number mismatch");
                            }
                            &def.body
                        }
                    },
                    // TypeRef::Pack(s) => &TypeBody::Tuple(TypeTuple {
                    //     slots: s
                    //         .iter()
                    //         .map(|t| unifier.refresh_type(t).reverse().expect("expected a type"))
                    //         .collect::<Vec<TypeTag>>(),
                    // }),
                    // any other type is invalid (because it does not have fields to access)
                    // the inner of Integer, Boolean, etc. cannot be accessed as it is private in dt.rs of stdlib
                    _ => bail_on!(base, "invalid type"),
                };

                // derive the return type
                // base_ty is the type definition of the base expression
                // member is the field name or index used in the call
                let (op, tag) = match (base_ty, member) {
                    // for a tuple struct, the member must be unnamed (0, 1, 2, ...)
                    (TypeBody::Tuple(def_tuple), Member::Unnamed(index)) => {
                        let slot = index.index as usize;
                        match def_tuple.slots.get(slot) {
                            None => bail_on!(index, "no such slot"), // the index should be from 0 - the number of slots in the tuple struct (exclusive)
                            Some(t) => (
                                Op::AccessSlot {
                                    base: parsed_base, // the base expression as Expr ADT
                                    slot,              // the index of the slot
                                },
                                t, // the type of the slot in the definition of the tuple struct
                            ),
                        }
                    }
                    // for a record struct, the member must be named
                    (TypeBody::Record(def_record), Member::Named(ident)) => {
                        let field = ident.to_string();
                        match def_record.fields.get(&field) {
                            None => bail_on!(ident, "no such field"), // the field should be in the definition of the record struct
                            Some(t) => (
                                Op::AccessField {
                                    base: parsed_base, // the base expression as Expr ADT
                                    field,             // the field name
                                },
                                t, // the type of the field in the definition of the record struct
                            ),
                        }
                    }
                    // the base expression should be a tuple struct with an unnamed member or a record struct with a named member
                    _ => bail_on!(member, "slot/field mismatch"),
                };

                // unify the return type
                // the type of the slot or field in the definition of the tuple struct or record struct is unified with the expected type annotation (on the left side).
                ti_unify!(unifier, &tag.into(), &self.exp_ty, expr_field);

                // return the constructed opcode (contains the type name and the field name or index)
                op
            }
            // A parenthesized expression: `(a + b)`.
            Exp::Paren(expr_paren) => {
                let ExprParen {
                    attrs: _,
                    paren_token: _,
                    expr,
                } = expr_paren;
                return self.convert_expr(unifier, expr);
            }
            // A blocked scope: `{ ... }`.
            Exp::Block(expr_block) => {
                let ExprBlock {
                    attrs: _,
                    label,
                    block:
                        Block {
                            brace_token: _,
                            stmts,
                        },
                } = expr_block;
                // if the block has a label, bail (labels are not allowed)
                // for example { 'a: { let x = 5; } } is not allowed
                bail_if_exists!(label);
                return self.convert_stmts(unifier, stmts);
            }
            // An `if` expression with an optional `else` block: `if expr { ... }
            // else { ... }`.
            //
            // The `else` branch expression may only be an `If` or `Block`
            // expression, not any of the other types of expression.
            Exp::If(expr_if) => {
                // contains all the PhiNodes (conditions and then blocks).
                let mut phi_nodes = vec![];
                // the current if expression (it is updated in the loop)
                let mut cursor = expr_if;
                // default_expr is the expression in the (last) else branch (it is a block expression)
                let default_expr = loop {
                    // unpack the current if expression
                    let ExprIf {
                        attrs: _,
                        if_token: _,
                        cond,
                        then_branch,
                        else_branch,
                    } = cursor;

                    // convert the condition (only unary expressions are allowed)
                    // so this is not allowed: if x && y { do_then } else { do_else }
                    // instead it should be written as: if x { if y { do_then } else { do_else } } else { do_else }
                    // the new_cond is the conversion of the condition Exp to Expr ADT
                    let new_cond = match cond.as_ref() {
                        // A unary operation: `!x`, `*x`, `-x`.
                        Exp::Unary(expr_unary) => {
                            // unpack the unary expression
                            let ExprUnary { attrs: _, op, expr } = expr_unary;
                            // A unary operator: `*`, `!`, `-`.
                            // The `*` operator for dereferencing
                            // The `!` operator for logical inversion
                            // The `-` operator for negation
                            // only the `*` operator is allowed because if you look at dt.rs in stdlib:
                            // impl Deref for Boolean { type Target = bool; ... }
                            // so the *x operation is allowed and x must be a Boolean which is dereferenced to a bool (rust type). ! and - do not make sense for a Boolean type (if we wanted to use them we had to define the operations in the stdlib)
                            if !matches!(op, UnOp::Deref(_)) {
                                bail_on!(cond, "not a Boolean deref");
                            }
                            // in if *x { do_then } else { do_else }, x must be a Boolean so the expected type is a Boolean
                            self.fork(TypeRef::Boolean).convert_expr(unifier, expr)?
                        }
                        // if the condition is not a unary expression, bail
                        // for example if 1+2==3 { do_then } else { do_else } should be written as:
                        // x = 1+2==3; if *x { do_then } else { do_else }
                        // because our operations are not like x+y (they are not binary) but are like x.add(y) … so they are always unary
                        _ => bail_on!(cond, "not a Unary expression"),
                    };

                    // convert the then block
                    // the result of the then block should match the expected type on the left side or the function return type
                    // the nested if conditions in the then block are analyzed in here
                    let new_then = self
                        .fork(self.exp_ty.clone())
                        .convert_stmts(unifier, &then_branch.stmts)?;

                    // now we have a node (condition + then block)
                    phi_nodes.push(PhiNode {
                        cond: new_cond,
                        body: new_then,
                    });

                    // convert the else block
                    // in functional programming it is mandatory to have an else block (so is in rusmart)
                    let else_expr = bail_if_missing!(
                        else_branch.as_ref().map(|(_, e)| e.as_ref()),
                        expr_if,
                        "expect else branch"
                    );

                    // the else branch should be an if expression or a block expression
                    match else_expr {
                        // analyze the nested if expression in the else branch by updating the cursor and continuing the loop
                        Exp::If(elseif_expr) => {
                            cursor = elseif_expr;
                        }
                        Exp::Block(_) => {
                            break self
                                .fork(self.exp_ty.clone())
                                .convert_expr(unifier, else_expr)?; // the result of the else block should match the expected type on the left side or the function return type
                        }
                        // the else branch should be an if expression or a block expression
                        _ => bail_on!(else_expr, "expect if or block expressions in else branch"),
                    }
                };

                // construct the operator
                Op::Phi {
                    nodes: phi_nodes, // contains all the PhiNodes (conditions and then blocks). An if expression in the else block creates a new PhiNode
                    default: default_expr, // the expression in the (last) else branch (a block expression)
                }
            }
            // A `match` expression: `match n { Some(n) => {}, None => {} }`.
            Exp::Match(expr_match) => {
                // unpack the match expression (ExprMatch)
                let ExprMatch {
                    attrs: _,
                    match_token: _,
                    expr: base,
                    brace_token: _,
                    arms,
                } = expr_match;

                // collect heads: match head { case1 => body1, case2 => body2, ... }
                // depending on whether the head expression is a tuple or not, the head_exprs are collected
                let (is_tuple, head_exprs) = match base.as_ref() {
                    // match (n1, n2) has a tuple expression as the base expression
                    // A tuple expression: `(a, b, c, d)`.
                    Exp::Tuple(expr_tuple) => {
                        let ExprTuple {
                            attrs: _,
                            paren_token: _,
                            elems,
                        } = expr_tuple;
                        // if the base expression is a tuple, head_exprs is a list of the elements in the tuple (head_exprs = [1, 2] for match (1, 2) {...})
                        (true, elems.iter().collect::<Vec<_>>())
                    }
                    // if the base expression is not a tuple, it is a single expression
                    // for example: match x {...}. In this case, head_exprs is a list containing the single expression (head_exprs = [x])
                    _ => (false, vec![base.as_ref()]),
                };

                // parse the headers
                // heads is a list of tuples containing the converted expressions (as Expr ADT) and the original expressions (as Exp in rust syntax)
                // the originals are solely used to define the span of the error if an error occurs
                let mut heads = vec![];
                for elem in head_exprs {
                    // parse the headers by tentatively marking their types as variables
                    let converted = self
                        .fork(TypeRef::Var(unifier.mk_var()))
                        .convert_expr(unifier, elem)?;
                    heads.push((converted, elem));
                }

                // organizer encapsulates a list of all the match arms
                // when going over the arms, the arms are added to the organizer
                // pub struct MatchOrganizer {
                //     arms: Vec<MatchArm>,
                // }
                let mut organizer = MatchOrganizer::new();

                // go over the arms (match head { case1 => body1, case2 => body2, ... })
                // for each case_i => body_i
                for arm in arms {
                    // unpack the arm
                    let Arm {
                        attrs: _,
                        pat: case,
                        guard,
                        fat_arrow_token: _,
                        body,
                        comma: _,
                    } = arm;
                    // if the guard exists, bail (guards are not allowed)
                    // for example: match x { 1 if x > 0 => 1, _ => 0 } is not allowed
                    bail_if_exists!(guard.as_ref().map(|(_, exp)| exp.as_ref())); // bail on the expression of the if guard.

                    // find the bindings (patterns)
                    // elem_pats is a list of patterns in a single case_i (for example match (x, y) { (1, 2) => 1, (1, 3) => 2 }) for the first arm, elem_pats = [(1, 2)] and for the second arm, elem_pats = [(1, 3)]
                    let elem_pats = if is_tuple {
                        match case {
                            // if the match head is a tuple, all patterns should also be a tuple
                            Pat::Tuple(pat_tuple) => {
                                let PatTuple {
                                    attrs: _,
                                    paren_token: _,
                                    elems,
                                } = pat_tuple;
                                elems.iter().collect()
                            }
                            // if the match head is a tuple, the pattern should also be a tuple
                            // for example this is not allowed: match (1, 2) { x => 1, y => 2 }
                            // a single pattern like x cannot be used
                            // just like let x : (i32,i32) = (1,2) is not allowed in rusmart
                            // you cannot have (x , y) | (z , w) in the pattern so the or pattern is not allowed for tuples
                            _ => bail_on!(case, "expect tuple"),
                        }
                    } else {
                        vec![case]
                    };

                    // if the number of patterns does not match the number of heads, bail
                    // for example: match (1, 2) { (1, 2) => 1, (1, 2, 3) => 2 } is not allowed
                    // never happens as it is caught by the rust compiler
                    if elem_pats.len() != heads.len() {
                        bail_on!(case, "pattern number mismatch");
                    }

                    // analyze each atom (pattern) in the arm
                    let mut atoms = vec![];
                    // bindings is empty at the beginning when analyzing each arm. It contains the variables defined in the pattern of the arm for example in match (x, y) { (1, z) => 1 }, the bindings are [(z, i32)] only for that arm
                    let mut bindings = BTreeMap::new();
                    // elem_pats.iter().zip(heads.iter()) matches the patterns with the head expression
                    // for example match (x, y) { (1, 2) => 1 }. We first take each arm one by one. Now here, the zip makes [(x,1), (y,2)] (roughly speaking)
                    for (elem, (head, _)) in elem_pats.iter().zip(heads.iter()) {
                        // partial are the variables defined in the pattern of the arm. if the pattern is an or pattern like MyEnum::Variant1(a) | MyEnum::Variant2(a), then the partial is [(a, i32)]. also all the variables in the pattern should be the same set. This is only needed to convert the body of the arm to an Expr ADT as the variables are used in the body.
                        // MatchAtom encapsulates each atom in the arm (the pattern)
                        let (atom, partial) =
                            MatchAnalyzer::analyze_pat_match_head(self.root, unifier, head, elem)?;
                        for (var_name, var_ty) in partial {
                            if bindings.insert(var_name, var_ty).is_some() {
                                bail_on!(elem, "naming conflict");
                            } // you can't have the same variable name in the same arm for example MyEnum::Variant2(a, a) but this will never happen because partial is a BTreeMap and has unique keys.
                        }
                        atoms.push(atom);
                    }

                    // the new context is created for converting the body in match head { case_i => body_i, ... } from a rust defined expression to an Expr ADT
                    // the fork is used to change the expected type of the context.
                    // also this fork allows us to use the same variable names in different arms without any conflict as for each body of each arm a new context parser is created so lines 1752-1754 do not give an error
                    let mut new_ctxt = self.fork(self.exp_ty.clone());
                    // the bindings are added to the new context for example in match (x, y) { (1, z) => 1 }, the bindings are [(z, i32)] only for that arm
                    // the bindings are the variables defined in the pattern of the arm
                    for (var_name, var_ty) in bindings {
                        // cannot have the same variable name for example let a = 1; match something { MyEnum::Variant1(a) => 1, ... } is wrong as a is defined twice even though rust allows it
                        if new_ctxt.vars.insert(var_name, var_ty).is_some() {
                            bail_on!(case, "naming conflict");
                        }
                    }
                    // the body of the arm is converted to an Expr ADT
                    let body_expr = new_ctxt.convert_expr(unifier, body)?;

                    // save the match arm
                    organizer.add_arm(atoms, body_expr);
                }

                // list options of the head
                // head_exprs is is a list of converted expressions (as Expr ADT) in the match head for example in match (1, 2) { ... }, we first convert 1 and 2 to Expr ADT and then head_exprs = [converted_1, converted_2]
                let mut heads_exprs = vec![];
                // heads_options is a list of tuples containing the type name and the variants of the ADT for each corresponding head expression. so for example in match (MyEnum::Variant1, AnotherEnum::Variant2) { ... }, heads_exprs = [converted_MyEnum::Variant1, converted_AnotherEnum::Variant2] and heads_options = [(MyEnum, variants_MyEnum), (AnotherEnum, variants_AnotherEnum)]
                // variants_MyEnum contains all the variants of MyEnum and variants_AnotherEnum contains all the variants of AnotherEnum
                // however the variants are saved such that the variant name is the key and the value is the default unpack (Unpack::Unit, Unpack::Tuple, Unpack::Record) so the details inside the variant is not saved
                let mut heads_options = vec![];
                for (converted, original) in heads {
                    // retrieve all variants of the ADT
                    let (adt_name, adt_variants) = match unifier.refresh_type(converted.ty()) {
                        // Refreshes a type by replacing any type variables with their inferred types. The heads were tentatively marked as type variables. In analyze_pat_match_head, the unifier is updated with the type of the head. So here, the type of the head is refreshed.
                        TypeRef::User(name, _) => {
                            let adt = match self.root.get_type_def(&name) {
                                None => bail_on!(original, "no such type"), // the type should be defined in the context
                                Some(def) => match &def.body {
                                    TypeBody::Enum(adt) => adt, // the type used in the match should be a user defined enum type
                                    _ => bail_on!(original, "not an enum type"),
                                },
                            };
                            let mut variants = BTreeMap::new();
                            for (variant_key, variant_def) in &adt.variants {
                                let variant_default_unpack = match variant_def {
                                    EnumVariant::Unit => Unpack::Unit,
                                    EnumVariant::Tuple(_) => Unpack::Tuple(BTreeMap::new()),
                                    EnumVariant::Record(_) => Unpack::Record(BTreeMap::new()),
                                };
                                variants.insert(variant_key.clone(), variant_default_unpack);
                            }
                            (name, variants)
                        }
                        // in the head, each element in the tuple or the single expression should be a user type. So matching only happens on user types which are enums and are marked with #[smt_type]
                        _ => bail_on!(original, "not a user-defined type"),
                    };
                    heads_exprs.push(converted);
                    heads_options.push((adt_name, adt_variants));
                }

                // organize the entire match expression with exhaustive expansion
                // organizer is a MatchOrganizer which contains all the match arms (atoms and body)
                // heads_options is a list of tuples containing the type name and the variants of the ADT for each corresponding head expression
                // expr_match is the target expression so that if an error occurs defines the span of the error.
                // the into organized function masically checks whether the match is exhaustive, whether all the variants are covered, whether the types of the head and arm are compatible, whether all the arms are useful, etc. Basically it is a sanity check. In the end, combo is a vector, one MatchCombo for each arm. Each MatchCombo contains the body of the arm and the enum::variant of the arm along with the variables defined in the pattern of the arm.
                let combo = organizer.into_organized(expr_match, &heads_options)?;

                // construct the operator
                Op::Match {
                    heads: heads_exprs, // the head is the converted expression (as Expr ADT) in the match head we got at the start
                    combo,
                }
            }
            // A function call expression: `invoke(a, b)`.
            Exp::Call(expr_call) => {
                let ExprCall {
                    attrs: _,
                    func,
                    paren_token: _,
                    args,
                } = expr_call;

                // extract the callee
                // converts the expression to ExprPathAsCallee
                match ExprPathAsCallee::parse(self.root, func)? {
                    // like let a = my_func() where fn my_func() -> i32 { 1 }
                    // these are standalone functions
                    ExprPathAsCallee::FuncNoPrefix(path) => {
                        let (fn_name, inst) = path.complete(unifier);

                        // substitute types in function parameters
                        let fty = match self.root.lookup_unqualified(&fn_name) {
                            None => bail_on!(func, "[invariant] no such function"),
                            Some(fty) => fty,
                        };
                        let (params, ret_ty) = match fty.instantiate(&inst) {
                            None => bail_on!(func, "no such type parameter"),
                            Some(instantiated) => instantiated,
                        };

                        // parse arguments
                        // expr_call is the target expression so that if an error occurs defines the span of the error.
                        let parsed_args =
                            self.parse_call_arguments(unifier, &params, &ret_ty, expr_call)?;

                        // build the opcode
                        Op::Procedure {
                            name: fn_name,
                            inst: inst.vec(),
                            args: parsed_args,
                        }
                    }
                    // like let a = MyType::my_func() where fn my_func() -> i32 { 1 } in this case my)func is either a function on a system type or a method in the smt of a user type
                    ExprPathAsCallee::FuncWithType(path) => match path {
                        // casts
                        // Boolean::from()
                        QualifiedPath::CastFromBool => {
                            let intrinsic = Intrinsic::BoolVal(Intrinsic::unpack_lit_bool(args)?);
                            ti_unify!(unifier, &TypeRef::Boolean, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // Integer::from()
                        QualifiedPath::CastFromInt => {
                            let intrinsic = Intrinsic::IntVal(Intrinsic::unpack_lit_int(args)?);
                            ti_unify!(unifier, &TypeRef::Integer, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // Real::from()
                        QualifiedPath::CastFromReal => {
                            let intrinsic = Intrinsic::RealVal(Intrinsic::unpack_lit_float(args)?);
                            ti_unify!(unifier, &TypeRef::Real, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // String::from()
                        QualifiedPath::CastFromStr => {
                            let intrinsic = Intrinsic::StrVal(Intrinsic::unpack_lit_str(args)?);
                            ti_unify!(unifier, &TypeRef::String, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // U32::from()
                        QualifiedPath::CastFromU32 => {
                            let intrinsic = Intrinsic::BvVal {
                                t: TypeRef::U32,
                                val: Intrinsic::unpack_lit_int(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::U32, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // U64::from()
                        QualifiedPath::CastFromU64 => {
                            let intrinsic = Intrinsic::BvVal {
                                t: TypeRef::U64,
                                val: Intrinsic::unpack_lit_int(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::U64, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // I32::from()
                        QualifiedPath::CastFromI32 => {
                            let intrinsic = Intrinsic::BvVal {
                                t: TypeRef::I32,
                                val: Intrinsic::unpack_lit_int(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::I32, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // I64::from()
                        QualifiedPath::CastFromI64 => {
                            let intrinsic = Intrinsic::BvVal {
                                t: TypeRef::I64,
                                val: Intrinsic::unpack_lit_int(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::I64, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // f32::from()
                        QualifiedPath::CastFromF32 => {
                            let intrinsic = Intrinsic::FloatVal {
                                t: TypeRef::F32,
                                val: Intrinsic::unpack_lit_float(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::F32, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // f64::from()
                        QualifiedPath::CastFromF64 => {
                            let intrinsic = Intrinsic::FloatVal {
                                t: TypeRef::F64,
                                val: Intrinsic::unpack_lit_float(args)?,
                            };
                            ti_unify!(unifier, &TypeRef::F64, &self.exp_ty, target);
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                        // system function
                        // Integer::eq()
                        QualifiedPath::SysFuncOnSysType(ty_name, ty_inst, fn_name) => {
                            // derive the operand type
                            let ty_inst = ty_inst.complete(unifier);
                            // basically operand_ty is the type at the start with all the type parameters substituted
                            let operand_ty = match ty_inst.instantiate(&ty_name.as_type_tag()) {
                                None => bail_on!(func, "no such type parameter"),
                                Some(t) => t,
                            };

                            // parse arguments
                            self.parse_sys_func_args(unifier, &fn_name, operand_ty, expr_call)?
                        }
                        // MyInteger::eq()
                        QualifiedPath::SysFuncOnUsrType(ty_name, ty_inst, fn_name) => {
                            // derive the operand type
                            let operand_tag = match self.root.get_type_generics(&ty_name) {
                                None => bail_on!(func, "[invariant] no such type"),
                                Some(ty_generics) => TypeTag::User(
                                    ty_name,
                                    ty_generics
                                        .params
                                        .iter()
                                        .map(|n| TypeTag::Parameter(n.clone()))
                                        .collect(),
                                ),
                            };
                            let ty_inst = ty_inst.complete(unifier);
                            let operand_ty = match ty_inst.instantiate(&operand_tag) {
                                None => bail_on!(func, "no such type parameter"),
                                Some(t) => t,
                            };

                            // parse arguments
                            self.parse_sys_func_args(unifier, &fn_name, operand_ty, expr_call)?
                        }
                        // T::eq()
                        QualifiedPath::SysFuncOnParamType(ty_name, fn_name) => self
                            .parse_sys_func_args(
                                unifier,
                                &fn_name,
                                TypeRef::Parameter(ty_name),
                                expr_call,
                            )?,
                        // user-defined function on a system type (i.e., intrinsic function)
                        // Integer::add() or Error::fresh()
                        QualifiedPath::UsrFuncOnSysType(ty_name, ty_inst, fn_name, fn_inst) => {
                            // derive type param substitutions
                            let fty =
                            // user function on a system type (a.k.a., an intrinsic)
                                match self.root.lookup_usr_func_on_sys_type(&ty_name, &fn_name) {
                                    None => bail_on!(func, "[invariant] no such function"),
                                    Some(fty) => fty,
                                };

                            // Merge type instantiations from both type and function generics
                            // The order doesn't matter since instantiation is name-based
                            let inst =
                                match ty_inst.complete(unifier).merge(&fn_inst.complete(unifier)) {
                                    None => bail_on!(func, "conflicting type parameter name"),
                                    Some(inst) => inst,
                                };

                            let (params, ret_ty) = match fty.instantiate(&inst) {
                                None => bail_on!(func, "no such type parameter"),
                                Some(instantiated) => instantiated,
                            };

                            // parse the arguments
                            let parsed_args =
                                self.parse_call_arguments(unifier, &params, &ret_ty, expr_call)?;

                            // Extract type arguments in the order expected by the function's generics
                            // This ensures type arguments are provided in the correct order regardless
                            // of how they were merged
                            let ty_args = match inst.vec_for_generics(&fty.generics) {
                                None => bail_on!(func, "missing type parameter in instantiation"),
                                Some(args) => args,
                            };

                            // build the opcode
                            let intrinsic =
                                match Intrinsic::new(&ty_name, &fn_name, ty_args, parsed_args) {
                                    Ok(parsed) => parsed,
                                    Err(e) => bail_on!(target, "{}", e),
                                };
                            Op::Intrinsic(Box::new(intrinsic))
                        }
                    },
                    // like let a = MyTuple(1) where struct MyTuple(i32);
                    ExprPathAsCallee::CtorTuple(tuple) => {
                        let (ty_name, inst) = tuple.complete(unifier);
                        let params: Vec<_> = match self.root.get_type_def(&ty_name) {
                            None => bail_on!(expr_call, "[invariant] no such type"),
                            Some(def) => match &def.body {
                                TypeBody::Tuple(tuple_def) => {
                                    tuple_def.slots.iter().map(|t| t.into()).collect()
                                }
                                _ => bail_on!(expr_call, "[invariant] not a tuple type"),
                            },
                        };
                        let ret_ty = inst.make_ty(ty_name.clone());

                        // parse the arguments
                        let parsed_slots =
                            self.parse_call_arguments(unifier, &params, &ret_ty, expr_call)?;

                        // build the opcode
                        Op::Tuple {
                            name: ty_name,
                            inst: inst.vec(),
                            slots: parsed_slots,
                        }
                    }
                    // like let a = MyEnum::MyVariant(13) where enum MyEnum { MyVariant(i32) }
                    ExprPathAsCallee::CtorEnum(adt) => {
                        let (branch, inst) = adt.complete(unifier);
                        let variant = match self.root.get_adt_variant_details(&branch) {
                            None => bail_on!(expr_call, "[invariant] no such type or variant"),
                            Some(details) => details,
                        };
                        let params: Vec<_> = match variant {
                            EnumVariant::Tuple(tuple_def) => {
                                tuple_def.slots.iter().map(|t| t.into()).collect()
                            }
                            _ => bail_on!(expr_call, "not a tuple variant"),
                        };
                        let ret_ty = inst.make_ty(branch.ty_name.clone());

                        // parse the arguments
                        let parsed_slots =
                            self.parse_call_arguments(unifier, &params, &ret_ty, expr_call)?;

                        // build the opcode
                        Op::EnumTuple {
                            branch,
                            inst: inst.vec(),
                            slots: parsed_slots,
                        }
                    }
                }
            }
            // A method call expression: `x.foo::<T>(a, b)`.
            Exp::MethodCall(expr_method) => {
                let ExprMethodCall {
                    attrs: _,
                    receiver,
                    dot_token: _,
                    method,
                    turbofish,
                    paren_token: _,
                    args,
                } = expr_method;

                match FuncName::try_from(method)? {
                    // from and into are cast functions
                    FuncName::Cast(name) => {
                        // none of the cast functions take path arguments
                        // a turbosh is like: x.into::<T>() where x is the receiver and T is the type argument
                        bail_if_exists!(turbofish);

                        match name {
                            CastFuncName::Into => {
                                // no arguments are allowed for into. use case for example: x.into()
                                bail_if_non_empty!(args);
                                // The expression receiver must be a literal otherwise bail in Intrinsics::parse_literal_into
                                // the receiver can only be a literal boolean, integer, float, or string
                                let (intrinsic, ty) = Intrinsic::parse_literal_into(receiver)?;
                                ti_unify!(unifier, &ty, &self.exp_ty, target);
                                Op::Intrinsic(Box::new(intrinsic))
                            }
                            // from cannot be a method call
                            CastFuncName::From => bail_on!(expr_method, "unexpected"),
                        }
                    }
                    // eq and ne are system functions
                    FuncName::Sys(name) => {
                        // none of the system functions take path arguments
                        bail_if_exists!(turbofish);

                        // parse the arguments by tentatively marking their types as variables
                        let operand_ty = TypeRef::Var(unifier.mk_var());
                        let parsed_lhs = self
                            .fork(operand_ty.clone())
                            .convert_expr(unifier, receiver)?;

                        let mut iter = args.iter();
                        let rhs = bail_if_missing!(iter.next(), args, "expect 1 argument");
                        bail_if_exists!(iter.next()); // expecting ONLY 1 argument for example x.eq(y,z) is not allowed
                        let parsed_rhs =
                            self.fork(operand_ty.clone()).convert_expr(unifier, rhs)?; // the type of the right hand side should be the same as the left hand side

                        // unity the return type
                        // eq and ne return a Boolean (see the definition of SMT trait in the stdlib)
                        ti_unify!(unifier, &TypeRef::Boolean, &self.exp_ty, target);

                        // build the opcode
                        let intrinsic = match name {
                            SysFuncName::Eq => Intrinsic::SmtEq {
                                t: unifier.refresh_type(&operand_ty),
                                lhs: parsed_lhs,
                                rhs: parsed_rhs,
                            },
                            SysFuncName::Ne => Intrinsic::SmtNe {
                                t: unifier.refresh_type(&operand_ty),
                                lhs: parsed_lhs,
                                rhs: parsed_rhs,
                            },
                        };

                        Op::Intrinsic(Box::new(intrinsic))
                    }
                    // user-defined functions
                    FuncName::Usr(name) => {
                        // collect type arguments, if any
                        let ty_args_opt = match turbofish {
                            None => None,
                            Some(ty_args) => Some(TypeTag::from_generics(self.root, ty_args)?), // throws an error if the args is empty, or there are any liftimes, consts, ..., or if there is no leading ::
                        };

                        // parse the arguments by tentatively marking their types as variables
                        // parsed_args contains the receiver and the arguments converted to Expr ADT
                        let mut parsed_args = vec![];
                        let parsed_receiver = self
                            .fork(TypeRef::Var(unifier.mk_var()))
                            .convert_expr(unifier, receiver)?;
                        parsed_args.push(parsed_receiver);

                        for arg in args {
                            let parsed = self
                                .fork(TypeRef::Var(unifier.mk_var()))
                                .convert_expr(unifier, arg)?;
                            parsed_args.push(parsed);
                        }

                        // choose the function from function database
                        // the unifier is passed to update the type variables
                        // the root is passed as it contains the context (function signature, and a context of all the types and funcs marked as smt)
                        // the name of the user-defined function
                        // the type arguments of the function
                        // the receiver and the arguments converted to Expr ADT
                        // the expected type of the function
                        let inferred = self.root.ctxt.fn_db.query_with_inference(
                            unifier,
                            self.root,
                            &name,
                            ty_args_opt.as_deref(),
                            parsed_args,
                            &self.exp_ty,
                        );
                        match inferred {
                            Ok(op) => op,
                            Err(e) => bail_on!(expr_method, "{}", e),
                        }
                    }
                    // reserved methods (clone & default) are not allowed in the body of the function in the rusmart
                    FuncName::Reserved(_) => bail_on!(method, "reserved method (clone & default)"),
                }
            }
            // SMT-specific macro (i.e., DSL)
            // A macro invocation expression: `format!("{}", q)`.
            Exp::Macro(expr_macro) => {
                let quant = Quantifier::parse(self.root, expr_macro)?;
                match quant {
                    Quantifier::Typed { name, vars, body } => {
                        // parse body
                        // the return type of the body must be a Boolean
                        let mut new_ctxt = self.fork(TypeRef::Boolean);

                        for (var, ty) in &vars {
                            if new_ctxt.vars.insert(var.clone(), ty.into()).is_some() {
                                bail_on!(expr_macro, "duplicated variable binding");
                            }
                        }
                        let quant_body = new_ctxt.convert_expr(unifier, &body)?;

                        // decide return type and opcode
                        let (rty, op) = match name {
                            SysMacroName::Forall => (
                                TypeRef::Boolean,
                                Op::Forall {
                                    vars,
                                    body: quant_body,
                                },
                            ),
                            SysMacroName::Exists => (
                                TypeRef::Boolean,
                                Op::Exists {
                                    vars,
                                    body: quant_body,
                                },
                            ),
                            SysMacroName::Choose => {
                                let rty = if vars.len() == 1 {
                                    let (_, ty) = vars.first().expect("[invariant] non-empty");
                                    ty.into()
                                } else {
                                    TypeRef::Pack(vars.iter().map(|(_, t)| t.into()).collect())
                                };
                                (
                                    rty,
                                    Op::Choose {
                                        vars,
                                        body: quant_body,
                                    },
                                )
                            }
                        };

                        // unify the return type
                        ti_unify!(unifier, &rty, &self.exp_ty, target);

                        // done
                        op
                    }
                    Quantifier::Iterated { name, vars, body } => {
                        // context for body parsing
                        let mut new_ctxt = self.fork(TypeRef::Boolean);
                        // holds the types of the collection expressions for example xs and ys in
                        // forall x in xs, y in ys => x + y == 0
                        let mut var_tys = vec![];
                        // holds the (x, xs) and (y, ys) pairs... but the difference between var_exprs and vars is that the expressions in vars are the original expressions in rust syntax and the expressions in var_exprs are the converted expressions in rusmart
                        let mut var_exprs = vec![];

                        // parse collection expressions
                        for (var, collection) in vars {
                            // tentatively mark the type of the collection as a type variable
                            let sub_ctxt = self.fork(TypeRef::Var(unifier.mk_var()));
                            let sub_expr = sub_ctxt.convert_expr(unifier, &collection)?;
                            let var_ty = match sub_expr.ty() {
                                TypeRef::Seq(_) => TypeRef::Integer, // because in the stdlib in the iterator of Seq we have integer from 0 to n-1 where n is the length of the Seq
                                TypeRef::Set(sub) => sub.as_ref().clone(),
                                TypeRef::Array(key, _) => key.as_ref().clone(),
                                TypeRef::Var(_) => {
                                    bail_on!(&collection, "unable to infer collection type")
                                }
                                // only Seq, Set, and array are allowed
                                _ => bail_on!(&collection, "not a collection type"),
                            };

                            // add the variable declaration to the context of body parsing
                            if new_ctxt.vars.insert(var.clone(), var_ty.clone()).is_some() {
                                bail_on!(expr_macro, "duplicated variable binding");
                            }

                            // save the variable expression and its type (for choose operator)
                            var_tys.push(var_ty);
                            var_exprs.push((var, sub_expr));
                        }

                        // parse the constraint
                        let quant_body = new_ctxt.convert_expr(unifier, &body)?;

                        // decide return type and opcode
                        let (rty, op) = match name {
                            SysMacroName::Forall => (
                                TypeRef::Boolean,
                                Op::IterForall {
                                    vars: var_exprs,
                                    body: quant_body,
                                },
                            ),
                            SysMacroName::Exists => (
                                TypeRef::Boolean,
                                Op::IterExists {
                                    vars: var_exprs,
                                    body: quant_body,
                                },
                            ),
                            SysMacroName::Choose => {
                                let rty = if var_tys.len() == 1 {
                                    var_tys.into_iter().next().unwrap()
                                } else {
                                    TypeRef::Pack(var_tys)
                                };
                                (
                                    rty,
                                    Op::IterChoose {
                                        vars: var_exprs,
                                        body: quant_body,
                                    },
                                )
                            }
                        };

                        // unify the return type
                        ti_unify!(unifier, &rty, &self.exp_ty, target);

                        // done
                        op
                    }
                }
            }
            // Return expressions - unwrap and convert the inner expression
            Exp::Return(expr_return) => {
                match &expr_return.expr {
                    Some(inner_expr) => {
                        // Recursively convert the inner expression - return x becomes x
                        // This returns early from the match, unpacking the Result
                        return self.convert_expr(unifier, inner_expr);
                    }
                    None => {
                        // return without value - not allowed in SMT functions
                        bail_on!(expr_return, "return without value not supported")
                    }
                }
            }

            // array, assign, async, await, binary, break, cast, closure, const, continue, forloop, group, index, infer, let, lit, loop, range, reference, repeat, try, tryblock, unary, unsafe, verbatim, while, yield, are not supported
            _ => bail_on!(target, "invalid expression {:?}", target),
        };

        // build the instruction which encapsulates the expression and the type of the operation
        let inst = Inst {
            op: op.into(), // the `op` that is constructed above and abstracts the expression (Exp), is wrapped as an Box<Op>.
            ty: unifier.refresh_type(&self.exp_ty), // refresh_type is a method of TypeUnifier that replaces any type variables with their inferred types. For the sole expression at the end of a function, the type is never a type variable (because the return type of the function is explicitly mentioned), so it is returned as is. For other expressions (right hand side of a local let declaration) with explicit types, the type is also never a type variable, so it is returned as is. But for expressions (right hand side of a local let declaration) without any type annotations, the type is a type variable, so it is replaced with the inferred type. For other inner expressions, the type may be a type variable, if so, it is replaced with the inferred type. Otherwise, it is returned as is (or in details it is returned with a smaller index inside the same type equivalence group).
        };

        // decide to go with a unit instruction or a block instruction
        // for the right hand side of let declarations, the expression is always a unit instruction
        // for the sole expression at the end of a function, the expression is:
        // - a unit instruction if there are no local let declarations
        // - a block instruction if there are local let declarations
        // finalize the expression tree of the body
        let converted = if self.bindings.is_empty() {
            Expr::Unit(inst)
        } else {
            Expr::Block {
                // the only time when this is a block is for the sole expression at the end of a function with at least one local let declaration. In any other case, (right hand side of a local let declaration, inner expressions, or the sole expression at the end of a function without any local let declarations), this is a unit instruction.
                lets: self.bindings,
                body: inst,
            }
        };
        Ok(converted) // the final expression tree of the body
    }

    /// Parse arguments of a function call
    fn parse_call_arguments(
        &self,
        unifier: &mut TypeUnifier,
        params: &[TypeRef],
        ret_ty: &TypeRef,
        expr_call: &ExprCall,
    ) -> Result<Vec<Expr>> {
        let args = &expr_call.args;

        // parse the arguments
        if params.len() != args.len() {
            bail_on!(args, "argument number mismatch");
        }
        let mut parsed_args = vec![];
        for (param, arg) in params.iter().zip(args) {
            let parsed = self.fork(param.clone()).convert_expr(unifier, arg)?;
            parsed_args.push(parsed);
        }

        // unity the return type
        ti_unify!(unifier, ret_ty, &self.exp_ty, expr_call);

        // done
        Ok(parsed_args)
    }

    /// Parse fields of a record constructor
    /// fields: map of field name to type ref in the struct definition
    /// expr_struct: the expression of the struct in the call (this is passed on to access the field in the call)
    /// Basically, this function checks if in the struct call, all the fields used are defined in the struct definition, the type of the corresponding fields are compatible with the type of the fields in the struct definition, and there are no duplicated fields. If that is the case, the fields of the struct call are returned.
    fn parse_record_fields(
        &self,
        unifier: &mut TypeUnifier,
        fields: &BTreeMap<String, TypeRef>,
        expr_struct: &ExprStruct,
    ) -> Result<BTreeMap<String, Expr>> {
        // get the fields of the struct when it is called
        let args = &expr_struct.fields;

        // parse the arguments
        // if the number of fields in the struct definition and the number of fields in the struct call are different, bail (this will never happen as the rust compiler will catch this)
        if fields.len() != args.len() {
            bail_on!(args, "argument number mismatch");
        }

        // holds the fields of the struct call
        let mut parsed_args = BTreeMap::new();
        // for each field in the struct call
        for arg in args {
            let FieldValue {
                attrs: _,
                member, // member like x in x: 5
                colon_token: _,
                expr: expr_field, // expr like 5 in x: 5
            } = arg;

            // a struct record must have named fields unlike tuple structs
            let field_name = match member {
                Member::Named(ident) => ident.to_string(),
                Member::Unnamed(_) => bail_on!(member, "unnamed field member"), // Expr::Struct should have named fields unline Expr::Call which is for struct tuples
            };

            // check if the field name that is used in the struct call is present in the struct definition
            let parsed_field = match fields.get(&field_name) {
                None => bail_on!(member, "no such field"), // if the field name is not found in the struct definition, bail (this will never happen as the rust compiler will catch this)
                Some(t) => self.fork(t.clone()).convert_expr(unifier, expr_field)?, // otherwise, parse the field expression. The expected type in this case is the type of the field in the struct definition.
            };
            // cannot have duplicated fields in the struct call (this will never happen as the rust compiler will catch this)
            match parsed_args.insert(field_name, parsed_field) {
                None => (),
                Some(_) => bail_on!(member, "duplicated field"),
            }
        }

        // done
        Ok(parsed_args)
    }

    /// Parse arguments of a function call
    fn parse_sys_func_args(
        &self,
        unifier: &mut TypeUnifier,
        fn_name: &SysFuncName,
        operand: TypeRef,
        expr_call: &ExprCall,
    ) -> Result<Op> {
        // parse arguments
        let mut parsed_args = self
            .parse_call_arguments(
                unifier,
                &[operand.clone(), operand.clone()],
                &TypeRef::Boolean,
                expr_call,
            )?
            .into_iter();

        let parsed_lhs = parsed_args.next().unwrap();
        let parsed_rhs = parsed_args.next().unwrap();

        // build the opcode
        let op = match fn_name {
            SysFuncName::Eq => Op::Intrinsic(Box::new(Intrinsic::SmtEq {
                t: operand,
                lhs: parsed_lhs,
                rhs: parsed_rhs,
            })),
            SysFuncName::Ne => Op::Intrinsic(Box::new(Intrinsic::SmtNe {
                t: operand,
                lhs: parsed_lhs,
                rhs: parsed_rhs,
            })),
        };
        Ok(op)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_display_vardecl() {
        use super::*;
        use syn::Ident;

        let ident: Ident = Ident::new("x", proc_macro2::Span::call_site());
        let var: VarName = (&ident).try_into().unwrap();
        let decl = VarDecl::One(var.clone(), TypeRef::Integer);
        assert_eq!(decl.to_string(), "x:Integer");

        let decl = VarDecl::Pack(vec![
            VarDecl::One(var.clone(), TypeRef::Integer),
            VarDecl::One(var, TypeRef::Integer),
        ]);
        assert_eq!(decl.to_string(), "(x:Integer,x:Integer)");
    }
}

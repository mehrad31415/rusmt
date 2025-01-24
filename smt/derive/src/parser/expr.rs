use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use itertools::Itertools;
use syn::{
    Arm, Block, Expr as Exp, ExprBlock, ExprCall, ExprField, ExprIf, ExprMatch, ExprMethodCall,
    ExprParen, ExprStruct, ExprTuple, ExprUnary, FieldValue, Local, LocalInit, Member, Pat,
    PatTuple, PatType, Result, Stmt, UnOp,
};

use crate::parser::adt::{MatchAnalyzer, MatchOrganizer};
use crate::parser::apply::{Kind, TypeFn};
use crate::parser::ctxt::ContextWithSig;
use crate::parser::dsl::{Quantifier, SysMacroName};
use crate::{bail_if_exists, bail_if_missing, bail_if_non_empty, bail_on};
use crate::parser::func::{CastFuncName, FuncName, FuncSig, SysFuncName};
use crate::parser::generics::Generics;
use crate::parser::infer::{ti_unify, TypeRef, TypeUnifier};
use crate::parser::intrinsics::Intrinsic;
use crate::parser::name::{UsrFuncName, UsrTypeName, VarName};
use crate::parser::path::{
    ADTBranch, ExprPathAsCallee, ExprPathAsRecord, ExprPathAsTarget, QualifiedPath,
};
use crate::parser::ty::{CtxtForType, EnumVariant, SysTypeName, TypeBody, TypeDef, TypeTag};

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
    /// ctxt: ExprParserRoot implements the CtxtForType trait, providing the generics and get_type_generics functions
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
        let mut vars = BTreeMap::new(); // vars is only to prevent duplicate declarations of the same variable inside one let statement. Also the vars is added to the overall vars to prevent duplicate declarations of the same variable in the whole function. At the start of parsing the sole let statement, vars is empty.
        let decl = Self::parse_decl(&mut vars, unifier, pat_res, ty_opt.as_ref())?; // parse_decl creates the VarDecl for the pattern
        Ok(Self { vars, decl })
    }
}

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
            Self::One(name, ty) => write!(f, "{}:{}", name, ty),
            Self::Pack(decls) => {
                write!(f, "({})", decls.iter().format(","))
            }
        }
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
    /// Retrieve the kind of the current parsing context (whether the function is an impl or a spec)
    fn kind(&self) -> Kind;

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

    /// Retrieve the function type for a user function on a system type (a.k.a., an intrinsic)
    fn lookup_usr_func_on_sys_type(
        &self,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn>;

    /// Retrieve the function type for a user function on a user type (entirely user-defined method)
    fn lookup_usr_func_on_usr_type(
        &self,
        ty_name: &UsrTypeName,
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
                write!(f, "[{}]", content)
            }
            Self::Record(binds) => {
                let content = binds
                    .iter()
                    .format_with(",", |(k, v), f| f(&format_args!("{}:{}", k, v)));
                write!(f, "{{{}}}", content)
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
    Intrinsic(Intrinsic),
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
                        .format_with(",", |(k, v), p| { p(&format_args!("{}:{}", k, v)) })
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
                        .format_with(",", |(k, v), p| { p(&format_args!("{}:{}", k, v)) })
                )
            }
            Self::AccessSlot { base, slot } => {
                write!(f, "{}.{}", base, slot)
            }
            Self::AccessField { base, field } => {
                write!(f, "{}.{}", base, field)
            }
            Self::Match { heads, combo } => {
                writeln!(f, "match ({}) {{", heads.iter().format(","))?;
                for case in combo {
                    writeln!(f, "  case {}", case)?;
                }
                write!(f, "}}")
            }
            Self::Phi { nodes, default } => {
                writeln!(f, "phi {{")?;
                for node in nodes {
                    writeln!(f, "  {}", node)?;
                }
                writeln!(f, "  default => {}", default)?;
                write!(f, "}}")
            }
            Self::Forall { vars, body } => {
                write!(
                    f,
                    "forall [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{}:{}", n, t))),
                    body
                )
            }
            Self::Exists { vars, body } => {
                write!(
                    f,
                    "exists [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{}:{}", n, t))),
                    body
                )
            }
            Self::Choose { vars, body } => {
                write!(
                    f,
                    "choose [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, t), p| p(&format_args!("{}:{}", n, t))),
                    body
                )
            }
            Self::IterForall { vars, body } => {
                write!(
                    f,
                    "forall [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{} in {}", n, h))),
                    body
                )
            }
            Self::IterExists { vars, body } => {
                write!(
                    f,
                    "exists [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{} in {}", n, h))),
                    body
                )
            }
            Self::IterChoose { vars, body } => {
                write!(
                    f,
                    "choose [{}] {}",
                    vars.iter()
                        .format_with(",", |(n, h), p| p(&format_args!("{} in {}", n, h))),
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
    /// Also used in probe_related_axioms in ctxt.rs
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
            Op::Intrinsic(intrinsic) => match intrinsic {
                // boolean
                Intrinsic::BoolVal(_) => (),
                Intrinsic::BoolNot { val } => val.visit(ty, pre, post)?,
                Intrinsic::BoolAnd { lhs, rhs }
                | Intrinsic::BoolOr { lhs, rhs }
                | Intrinsic::BoolXor { lhs, rhs }
                | Intrinsic::BoolImplies { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // integer
                Intrinsic::IntVal(_) => (),
                Intrinsic::IntLt { lhs, rhs }
                | Intrinsic::IntLe { lhs, rhs }
                | Intrinsic::IntGe { lhs, rhs }
                | Intrinsic::IntGt { lhs, rhs }
                | Intrinsic::IntAdd { lhs, rhs }
                | Intrinsic::IntSub { lhs, rhs }
                | Intrinsic::IntMul { lhs, rhs }
                | Intrinsic::IntDiv { lhs, rhs }
                | Intrinsic::IntRem { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // rational
                Intrinsic::NumVal(_) => (),
                Intrinsic::NumLt { lhs, rhs }
                | Intrinsic::NumLe { lhs, rhs }
                | Intrinsic::NumGe { lhs, rhs }
                | Intrinsic::NumGt { lhs, rhs }
                | Intrinsic::NumAdd { lhs, rhs }
                | Intrinsic::NumSub { lhs, rhs }
                | Intrinsic::NumMul { lhs, rhs }
                | Intrinsic::NumDiv { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // string
                Intrinsic::StrVal(_) => (),
                Intrinsic::StrLt { lhs, rhs }
                | Intrinsic::StrLe { lhs, rhs }
                | Intrinsic::StrGe { lhs, rhs }
                | Intrinsic::StrGt { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // cloak
                Intrinsic::BoxReveal { t, val } | Intrinsic::BoxShield { t, val } => {
                    ty(t)?;
                    val.visit(ty, pre, post)?;
                }
                // seq
                Intrinsic::SeqEmpty { t } => ty(t)?,
                Intrinsic::SeqLength { t, seq } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                }
                Intrinsic::SeqAppend { t, seq, item } | Intrinsic::SeqIncludes { t, seq, item } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                    item.visit(ty, pre, post)?;
                }
                Intrinsic::SeqAt { t, seq, idx } => {
                    ty(t)?;
                    seq.visit(ty, pre, post)?;
                    idx.visit(ty, pre, post)?;
                }
                // set
                Intrinsic::SetEmpty { t } => ty(t)?,
                Intrinsic::SetLength { t, set } => {
                    ty(t)?;
                    set.visit(ty, pre, post)?;
                }
                Intrinsic::SetInsert { t, set, item }
                | Intrinsic::SetRemove { t, set, item }
                | Intrinsic::SetContains { t, set, item } => {
                    ty(t)?;
                    set.visit(ty, pre, post)?;
                    item.visit(ty, pre, post)?;
                }
                // map
                Intrinsic::MapEmpty { k, v } => {
                    ty(k)?;
                    ty(v)?;
                }
                Intrinsic::MapLength { k, v, map } => {
                    ty(k)?;
                    ty(v)?;
                    map.visit(ty, pre, post)?
                }
                Intrinsic::MapGet { k, v, map, key }
                | Intrinsic::MapDel { k, v, map, key }
                | Intrinsic::MapContainsKey { k, v, map, key } => {
                    ty(k)?;
                    ty(v)?;
                    map.visit(ty, pre, post)?;
                    key.visit(ty, pre, post)?;
                }
                Intrinsic::MapPut {
                    k,
                    v,
                    map,
                    key,
                    val,
                } => {
                    ty(k)?;
                    ty(v)?;
                    map.visit(ty, pre, post)?;
                    key.visit(ty, pre, post)?;
                    val.visit(ty, pre, post)?;
                }
                // error
                Intrinsic::ErrFresh => (),
                Intrinsic::ErrMerge { lhs, rhs } => {
                    lhs.visit(ty, pre, post)?;
                    rhs.visit(ty, pre, post)?;
                }
                // smt
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
                    writeln!(f, "{};", binding)?;
                }
                write!(f, "{}", body)
            }
        }
    }
}

#[derive(Clone, Debug)]
/// Root expression parser for each function as a whole
/// ctxt: contains all the marked types, impls, specs, and axioms
/// kind: whether the function is an spec or impl - an axiom is represented as a spec
/// generics, params, and ret_ty: function generics, parameters, and return type derived from the function signature
pub struct ExprParserRoot<'ctx> {
    /// context provider
    ctxt: &'ctx ContextWithSig,
    /// the expression is an spec or impl
    kind: Kind,
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
    /// parent of the parser (this will always remain the same because we are parsing only inside one function - so the context, the kind, and the signature will remain the same)
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
    /// For each separate function (whether impl or spec), a new ExprParserRoot is created
    pub fn new(ctxt: &'ctx ContextWithSig, kind: Kind, sig: &FuncSig) -> Self {
        Self {
            ctxt, // the whole context of the rust file containing all the types, impls, specs, and axioms
            kind, // the kind of the function (impl or spec) - an axiom is represented as a spec
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
    /// The self contains the whole context of the rust file, the kind & signature of the current function
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
        let mut unifier = TypeUnifier::new();

        // derive a parser
        // ExprParserCursor is used to parse the current expression
        // ExprParserRoot is the root parser for the whole function under consideration
        let exp_ty = self.ret_ty.clone(); // the return type of the function is the expected type of ExprParserCursor at the start (the default expected type). This is because ExprParserCursor is used to parse the current expression. Every function body has some optional local let-binding statements and a mandatory sole expression (which is the return value of the function). Note that every function in `rusmart` must have a return value. Therefore, ExprParserCursor `definitely` parses the sole last expression of the body and the expected type of this expression is the function return type. For other expressions (for example right-hand side of a let-binding), the expected type needs to be altered. The is done by `forking` the parser to the defined type.
        let parser = ExprParserCursor {
            root: &self, // the root parser which encapsulates the following details: ContextWithSig (the whole context of the rust file), Kind (spec of impl), Generics (generics), BTreeMap<VarName, TypeRef> (Params), and TypeRef (return type)
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
                    bail_on!(refreshed.to_string(), "incomplete type") // if the type is incomplete, we return an error
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
    /// Retrieve the kind of the current parsing context (whether the function is an impl or a spec)
    fn kind(&self) -> Kind {
        self.kind
    }

    /// Retrieve the definition of a user-defined type from the whole context
    fn get_type_def(&self, name: &UsrTypeName) -> Option<&TypeDef> {
        self.ctxt.get_type_def(name)
    }

    /// Retrieve the signature of a user-defined function given the function name and the kind from the current function (a spec marked function cannot be used in an impl & an impl marked function cannot be used in a spec). So, the kind of the functions are the same.
    fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn> {
        self.ctxt.fn_db.lookup_unqualified(self.kind, name)
    }

    /// Retrieve the function type for a user function on a system type (a.k.a., an intrinsic) and inherenting the kind from the current function
    fn lookup_usr_func_on_sys_type(
        &self,
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.ctxt
            .fn_db
            .lookup_usr_func_on_sys_type(self.kind, ty_name, fn_name)
    }

    /// Retrieve the method type for a user function on a user type (entirely user-defined method) and inherenting the kind from the current function
    fn lookup_usr_func_on_usr_type(
        &self,
        ty_name: &UsrTypeName,
        fn_name: &UsrFuncName,
    ) -> Option<&TypeFn> {
        self.ctxt
            .fn_db
            .lookup_usr_func_on_usr_type(self.kind, ty_name, fn_name)
    }
}

impl<'r, 'ctx: 'r> ExprParserCursor<'r, 'ctx> {
    /// Fork a child parser (this basically redefines the expected type)
    /// The root containing the whole context of the rust file, the kind of the function, and the signature of the function, is retained because we are still processing the same function body
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
    /// The root parser contains the following details: ContextWithSig (the whole context of the rust file), and the Kind (spec of impl) and Function signature (Generics, Params, and Return type)
    /// The expected type is the return type of the function at the start
    /// The variables in scope are the function parameters at the start and will contain all the local variables in the current scope
    /// The new let-bindings created are empty at the start and will contain all the new let-bindings created inside function body
    /// The type unifier is used for type unification
    /// The stmts are the statements in the function body
    ///
    /// Returns:
    ///
    /// * Result<Expr>: the returning expression of the function (impl or spec) - every function should have a return value
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
                    // expecting a unit expression (this is the return value of the function)
                    // so expressions which throw away their return value are not allowed
                    // this is because every expression in rusmart is pure (it should not have side effects meaning modifying the state of the program like mutable variables)
                    // therefore there is no point in having an expression that does not return its value
                    bail_if_exists!(semi_token);

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
                // the root contains the whole context of the rust file, the kind & the signature of the function
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
                    ExprPathAsTarget::EnumUnit(adt) => {
                        // ADTPath encapsulates the enum type name, the variant name, plus the type arguments (if any). The type arguments of the enum are matched with the generics of the enum definition to form the GenericsInstPartial.
                        // The GenericsInstPartial originally contains the matching TypeTag taken from the argument to the Type Parameter taken from the definition
                        // ADTBranch is a struct encapsulating the enum type name and the variant name
                        // Through calling the complete function on the ADTPath, the GenericsInstPartial is converted to GenericsInstFull
                        // The difference is that each TypeTag is converted to TypeRef, if there is no TypeTag (None), it is converted to TypeRef::Var
                        let (branch, inst) = adt.complete(unifier);
                        // self.root is ExprParserRoot which contains the whole context of the rust file, the kind of the function and signature of the function
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
                        // let a : MyEnum<i32> = MyEnum::<i32>::Variant1; // unifies MyEnum<i32> on the left with MyEnum<i32> on the right (in this case, they are compatible)
                        let _ = ti_unify!(unifier, &ret_ty, &self.exp_ty, target);

                        // build the opcode
                        Op::EnumUnit {
                            branch,           // the enum type name and the variant name
                            inst: inst.vec(), // the corresponding typerefs for each type parameter in the GenericsInstFull (this is a list of TypeRef). This still contains TypeRef::Var. So what happens with the result of the unification? (ti_unify!). basically the ti_unify! updates the unifier. In the ExprParserRoot::parse function after converting the statements to an expression, the unifier is used to check if there are any TypeRef::Var left. If there are, it receives the unified type and if still incomplete, it bails.
                        }
                    }
                }
            }
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
                                // a struct is a record type (tuple structs fall under Expr::Call and unit structs cannot exist)
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
                        // expr_struct is the target expression so that if an error occurs defines the span of the error. Note that in ret_ty there are no type parameters.
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

                // parse the receiver without assuming its type (convert the base expression to Expr ADT)
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
                    // any other type is invalid (because it does not have fields to access)
                    // one may assume that we access the inner of Integer, Boolean, etc. but this is not allowed in rusmart as inner is a private field in dt.rs of stdlib
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
                // TODO enable shadowing in rusmart
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
                    // instead it should be written as: if x { if y { do_then } else { do_else } }
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
                        _ => bail_on!(cond, "not a Boolean deref"),
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
                        Exp::Block(else_block) => {
                            let ExprBlock {
                                attrs: _,
                                label,
                                block:
                                    Block {
                                        brace_token: _,
                                        stmts,
                                    },
                            } = else_block;

                            // if the block has a label, bail (labels are not allowed)
                            bail_if_exists!(label);
                            // break and return the last block
                            break self
                                .fork(self.exp_ty.clone())
                                .convert_stmts(unifier, stmts)?;
                        }
                        // the else branch should be an if expression or a block expression
                        _ => bail_on!(else_expr, "expect if or block"),
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
                    // match (1,2) {...} has a tuple base expression
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
                    // parser the headers by tentatively marking their types as variables
                    let converted = self
                        .fork(TypeRef::Var(unifier.mk_var()))
                        .convert_expr(unifier, elem)?;
                    heads.push((converted, elem));
                }

                // organizer encapsulates a list of all the match arms
                // when going over the arms, the arms are added to the organizer
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
                    // elem_pats is a list of patterns in a single case_i (for example match (1, 2) { (1, 2) => 1, (1, 3) => 2 }) for the first arm, elem_pats = [(1, 2)] and for the second arm, elem_pats = [(1, 3)]
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

                    // analyze each atom
                    let mut atoms = vec![];
                    let mut bindings = BTreeMap::new();
                    // elem_pats.iter().zip(heads.iter()) matches the patterns with the head expression
                    // for example match (1, 2) { (x, y) => 1 }. We first take each arm one by one. Now here, the zip makes [(x, 1), (y, 2)] (roughly speaking)
                    for (elem, (head, _)) in elem_pats.iter().zip(heads.iter()) {
                        let (atom, partial) =
                            MatchAnalyzer::analyze_pat_match_head(self.root, unifier, elem, head)?;
                        for (var_name, var_ty) in partial {
                            if bindings.insert(var_name, var_ty).is_some() {
                                bail_on!(elem, "naming conflict");
                            }
                        }
                        atoms.push(atom);
                    }

                    // use the new bindings to analyze the body
                    let mut new_ctxt = self.fork(self.exp_ty.clone());
                    for (var_name, var_ty) in bindings {
                        if new_ctxt.vars.insert(var_name, var_ty).is_some() {
                            bail_on!(case, "naming conflict");
                        }
                    }
                    let body_expr = new_ctxt.convert_expr(unifier, body)?;

                    // save the match arm
                    organizer.add_arm(atoms, body_expr);
                }

                // list options of the head
                let mut heads_exprs = vec![];
                let mut heads_options = vec![];
                for (converted, original) in heads {
                    // retrieve all variants of the ADT
                    let (adt_name, adt_variants) = match unifier.refresh_type(converted.ty()) {
                        TypeRef::User(name, _) => {
                            let adt = match self.root.get_type_def(&name) {
                                None => bail_on!(original, "no such type"),
                                Some(def) => match &def.body {
                                    TypeBody::Enum(adt) => adt,
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
                        _ => bail_on!(original, "not a user-defined type"),
                    };
                    heads_exprs.push(converted);
                    heads_options.push((adt_name, adt_variants));
                }

                // organize the entire match expression with exhaustive expansion
                let combo = organizer.into_organized(expr_match, &heads_options)?;

                // construct the operator
                Op::Match {
                    heads: heads_exprs,
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
                        let parsed_args =
                            self.parse_call_arguments(unifier, &params, &ret_ty, expr_call)?;

                        // build the opcode
                        Op::Procedure {
                            name: fn_name,
                            inst: inst.vec(),
                            args: parsed_args,
                        }
                    }
                    ExprPathAsCallee::FuncWithType(path) => match path {
                        // casts
                        QualifiedPath::CastFromBool => {
                            let intrinsic = Intrinsic::BoolVal(Intrinsic::unpack_lit_bool(args)?);
                            ti_unify!(unifier, &TypeRef::Boolean, &self.exp_ty, target);
                            Op::Intrinsic(intrinsic)
                        }
                        QualifiedPath::CastFromInt => {
                            let intrinsic = Intrinsic::IntVal(Intrinsic::unpack_lit_int(args)?);
                            ti_unify!(unifier, &TypeRef::Integer, &self.exp_ty, target);
                            Op::Intrinsic(intrinsic)
                        }
                        QualifiedPath::CastFromFloat => {
                            let intrinsic = Intrinsic::NumVal(Intrinsic::unpack_lit_float(args)?);
                            ti_unify!(unifier, &TypeRef::Rational, &self.exp_ty, target);
                            Op::Intrinsic(intrinsic)
                        }
                        QualifiedPath::CastFromStr => {
                            let intrinsic = Intrinsic::StrVal(Intrinsic::unpack_lit_str(args)?);
                            ti_unify!(unifier, &TypeRef::Text, &self.exp_ty, target);
                            Op::Intrinsic(intrinsic)
                        }
                        // system function
                        QualifiedPath::SysFuncOnSysType(ty_name, ty_inst, fn_name) => {
                            // derive the operand type
                            let ty_inst = ty_inst.complete(unifier);
                            let operand_ty = match ty_inst.instantiate(&ty_name.as_type_tag()) {
                                None => bail_on!(func, "no such type parameter"),
                                Some(t) => t,
                            };

                            // parse arguments
                            self.parse_sys_func_args(unifier, &fn_name, operand_ty, expr_call)?
                        }
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
                        QualifiedPath::SysFuncOnParamType(ty_name, fn_name) => self
                            .parse_sys_func_args(
                                unifier,
                                &fn_name,
                                TypeRef::Parameter(ty_name),
                                expr_call,
                            )?,
                        // user-defined function on a system type (i.e., intrinsic function)
                        QualifiedPath::UsrFuncOnSysType(ty_name, ty_inst, fn_name, fn_inst) => {
                            // derive type param substitutions
                            let fty =
                                match self.root.lookup_usr_func_on_sys_type(&ty_name, &fn_name) {
                                    None => bail_on!(func, "[invariant] no such function"),
                                    Some(fty) => fty,
                                };

                            // for simplicity, require type generics be the first set of type parameters
                            // TODO: relax this requirement
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

                            // build the opcode
                            let intrinsic =
                                match Intrinsic::new(&ty_name, &fn_name, inst.vec(), parsed_args) {
                                    Ok(parsed) => parsed,
                                    Err(e) => bail_on!(target, "{}", e),
                                };
                            Op::Intrinsic(intrinsic)
                        }
                        // user-defined function on a user-defined type
                        QualifiedPath::UsrFuncOnUsrType(ty_name, ty_inst, fn_name, fn_inst) => {
                            // derive type param substitutions
                            let fty =
                                match self.root.lookup_usr_func_on_usr_type(&ty_name, &fn_name) {
                                    None => bail_on!(func, "[invariant] no such function"),
                                    Some(fty) => fty,
                                };

                            // for simplicity, require type generics be the first set of type parameters
                            // TODO: relax this requirement
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

                            // build the opcode
                            Op::Procedure {
                                name: fn_name.clone(),
                                inst: inst.vec(),
                                args: parsed_args,
                            }
                        }
                    },
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
                    FuncName::Cast(name) => {
                        // none of the cast functions take path arguments
                        bail_if_exists!(turbofish);

                        match name {
                            CastFuncName::Into => {
                                bail_if_non_empty!(args);
                                let (intrinsic, ty) = Intrinsic::parse_literal_into(receiver)?;
                                ti_unify!(unifier, &ty, &self.exp_ty, target);
                                Op::Intrinsic(intrinsic)
                            }
                            CastFuncName::From => bail_on!(expr_method, "unexpected"),
                        }
                    }
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
                        bail_if_exists!(iter.next());
                        let parsed_rhs =
                            self.fork(operand_ty.clone()).convert_expr(unifier, rhs)?;

                        // unity the return type
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

                        Op::Intrinsic(intrinsic)
                    }
                    FuncName::Usr(name) => {
                        // collect type arguments, if any
                        let ty_args_opt = match turbofish {
                            None => None,
                            Some(ty_args) => Some(TypeTag::from_generics(self.root, ty_args)?),
                        };

                        // parse the arguments by tentatively marking their types as variables
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
                    FuncName::Reserved(_) => bail_on!(method, "reserved method"),
                }
            }
            // SMT-specific macro (i.e., DSL)
            Exp::Macro(expr_macro) => {
                let quant = Quantifier::parse(self.root, expr_macro)?;
                match quant {
                    Quantifier::Typed { name, vars, body } => {
                        // early filtering
                        if !matches!(self.root.kind, Kind::Spec) {
                            bail_on!(expr_macro, "only allowed in spec context");
                        }

                        // parse body
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
                                    let (_, ty) = vars.first().unwrap();
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
                        let mut var_tys = vec![];
                        let mut var_exprs = vec![];

                        // parse collection expressions
                        for (var, collection) in vars {
                            let sub_ctxt = self.fork(TypeRef::Var(unifier.mk_var()));
                            let sub_expr = sub_ctxt.convert_expr(unifier, &collection)?;
                            let var_ty = match sub_expr.ty() {
                                TypeRef::Seq(_) => TypeRef::Integer,
                                TypeRef::Set(sub) => sub.as_ref().clone(),
                                TypeRef::Map(key, _) => key.as_ref().clone(),
                                TypeRef::Var(_) => {
                                    bail_on!(&collection, "unable to infer collection type")
                                }
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
            // array, assign, async, await, binary, break, cast, closure, const, continue, forloop, group, index, infer, let, lit, loop, range, reference, repeat, return, try, tryblock, unary, unsafe, verbatim, while, yield, are not supported
            _ => bail_on!(target, "invalid expression {:?}", target),
        };

        // build the instruction which encapsulates the expression and the type of the operation
        let inst = Inst {
            op: op.into(), // the `op` that is constructed above and abstracts the expression (Exp), is wrapped as an Box<Op>.
            ty: unifier.refresh_type(&self.exp_ty), // refresh_type is a method of TypeUnifier that replaces any type variables with their inferred types. For the sole expression, the type is never a type variable (because the return type of the function is explicitly mentioned), so it is returned as is. For other expressions (right hand side of a local let declaration) with explicit types, the type is also never a type variable, so it is returned as is. But for expressions (right hand side of a local let declaration) without any type annotations, the type is a type variable, so it is replaced with the inferred type. For other inner expressions, the type may be a type variable, if so, it is replaced with the inferred type. Otherwise, it is returned as is.
        };

        // decide to go with a unit instruction or a block instruction
        // finalize the expression tree of the body
        let converted = if self.bindings.is_empty() {
            Expr::Unit(inst)
        } else {
            Expr::Block {
                lets: self.bindings,
                body: inst,
            }
        };
        Ok(converted)
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
                Member::Unnamed(_) => bail_on!(member, "unnamed field member"),
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
            SysFuncName::Eq => Op::Intrinsic(Intrinsic::SmtEq {
                t: operand,
                lhs: parsed_lhs,
                rhs: parsed_rhs,
            }),
            SysFuncName::Ne => Op::Intrinsic(Intrinsic::SmtNe {
                t: operand,
                lhs: parsed_lhs,
                rhs: parsed_rhs,
            }),
        };
        Ok(op)
    }
}

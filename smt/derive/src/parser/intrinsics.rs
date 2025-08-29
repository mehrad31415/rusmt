//! System types and functions of Rusmart

use crate::parser::expr::Expr;
use crate::parser::infer::TypeRef;
use crate::parser::name::UsrFuncName;
use crate::parser::ty::SysTypeName;
use crate::{bail_if_exists, bail_if_missing, bail_on};
use anyhow::bail;
use std::fmt::{Display, Formatter};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Expr as Exp, ExprLit, ExprUnary, Lit, Result};

/// Intrinsic procedure
#[derive(Clone, Debug)]
pub enum Intrinsic {
    /// `Boolean::from`
    BoolVal(bool),
    /// `Boolean::not`
    BoolNot { val: Expr },
    /// `Boolean::and`
    BoolAnd { lhs: Expr, rhs: Expr },
    /// `Boolean::or`
    BoolOr { lhs: Expr, rhs: Expr },
    /// `Boolean::xor`
    BoolXor { lhs: Expr, rhs: Expr },
    /// `Boolean::implies`
    BoolImplies { lhs: Expr, rhs: Expr },
    /// `Boolean::iff`
    BoolIff { lhs: Expr, rhs: Expr },
    /// `Integer::from`
    IntVal(i64),
    /// `Integer::lt`
    IntLt { lhs: Expr, rhs: Expr },
    /// `Integer::le`
    IntLe { lhs: Expr, rhs: Expr },
    /// `Integer::ge`
    IntGe { lhs: Expr, rhs: Expr },
    /// `Integer::gt`
    IntGt { lhs: Expr, rhs: Expr },
    /// `Integer::add`
    IntAdd { lhs: Expr, rhs: Expr },
    /// `Integer::sub`
    IntSub { lhs: Expr, rhs: Expr },
    /// `Integer::mul`
    IntMul { lhs: Expr, rhs: Expr },
    /// `Integer::div`
    IntDiv { lhs: Expr, rhs: Expr },
    /// `Integer::rem`
    IntRem { lhs: Expr, rhs: Expr },
    /// `Integer::to_rational`
    IntToRational { val: Expr },
    /// `Integer::pow`
    IntPow { base: Expr, exp: Expr },
    /// `Integer::abs`
    IntAbs { val: Expr },
    /// `Rational::from`
    NumVal(i64),
    /// `Rational::lt`
    NumLt { lhs: Expr, rhs: Expr },
    /// `Rational::le`
    NumLe { lhs: Expr, rhs: Expr },
    /// `Rational::ge`
    NumGe { lhs: Expr, rhs: Expr },
    /// `Rational::gt`
    NumGt { lhs: Expr, rhs: Expr },
    /// `Rational::add`
    NumAdd { lhs: Expr, rhs: Expr },
    /// `Rational::sub`
    NumSub { lhs: Expr, rhs: Expr },
    /// `Rational::mul`
    NumMul { lhs: Expr, rhs: Expr },
    /// `Rational::div`
    NumDiv { lhs: Expr, rhs: Expr },
    /// `Num::pow`
    NumPow { base: Expr, exp: Expr },
    /// `Num::abs`
    NumAbs { val: Expr },
    /// `Num::round`
    NumRound { val: Expr },
    /// `Num::floor`
    NumFloor { val: Expr },
    /// `Num::ceil`
    NumCeil { val: Expr },
    /// `Text::from`
    StrVal(String),
    /// `Text::lt`
    StrLt { lhs: Expr, rhs: Expr },
    /// `Text::le`
    StrLe { lhs: Expr, rhs: Expr },
    /// `Text::gt`
    StrGt { lhs: Expr, rhs: Expr },
    /// `Text::ge`
    StrGe { lhs: Expr, rhs: Expr },
    /// `Text::concat`
    StrConcat { lhs: Expr, rhs: Expr },
    /// `Text::at_index`
    StrAt { seq: Expr, idx: Expr },
    /// `Text::length`
    StrLength { seq: Expr },
    /// `Text::contains`
    StrIncludes { seq: Expr, item: Expr },
    /// `Text::starts_with`
    StrStartsWith { seq: Expr, item: Expr },
    /// `Text::ends_with`
    StrEndsWith { seq: Expr, item: Expr },
    /// `Cloak::shield`
    BoxShield { t: TypeRef, val: Expr },
    /// `Cloak::reveal`
    BoxReveal { t: TypeRef, val: Expr },
    /// `Seq::empty`
    SeqEmpty { t: TypeRef },
    /// `Seq::length`
    SeqLength { t: TypeRef, seq: Expr },
    /// `Seq::append`
    SeqAppend { t: TypeRef, seq: Expr, item: Expr },
    /// `Seq::at_unchecked`
    SeqAt { t: TypeRef, seq: Expr, idx: Expr },
    /// `Seq::includes`
    SeqIncludes { t: TypeRef, seq: Expr, item: Expr },
    /// `Seq::is_empty`
    SeqIsEmpty { t: TypeRef, seq: Expr },
    /// `Set::empty`
    SetEmpty { t: TypeRef },
    /// `Set::length`
    SetLength { t: TypeRef, set: Expr },
    /// `Set::insert`
    SetInsert { t: TypeRef, set: Expr, item: Expr },
    /// `Set::remove`
    SetRemove { t: TypeRef, set: Expr, item: Expr },
    /// `Set::contains`
    SetContains { t: TypeRef, set: Expr, item: Expr },
    /// `Set::intersection`
    SetIntersection { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::union`
    SetUnion { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::difference`
    SetDifference { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_subset`
    SetIsSubset { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `Set::is_empty`
    SetIsEmpty { t: TypeRef, set: Expr },
    /// `Map::empty`
    MapEmpty { k: TypeRef, v: TypeRef },
    /// `Map::length`
    MapLength { k: TypeRef, v: TypeRef, map: Expr },
    /// `Map::put_unchecked`
    MapPut {
        k: TypeRef,
        v: TypeRef,
        map: Expr,
        key: Expr,
        val: Expr,
    },
    /// `Map::get_unchecked`
    MapGet {
        k: TypeRef,
        v: TypeRef,
        map: Expr,
        key: Expr,
    },
    /// `Map::del_unchecked`
    MapDel {
        k: TypeRef,
        v: TypeRef,
        map: Expr,
        key: Expr,
    },
    /// `Map::contains_key`
    MapContainsKey {
        k: TypeRef,
        v: TypeRef,
        map: Expr,
        key: Expr,
    },
    /// `Map::is_empty`
    MapIsEmpty { k: TypeRef, v: TypeRef, map: Expr },
    /// `Error::fresh`
    ErrFresh,
    /// `Error::merge`
    ErrMerge { lhs: Expr, rhs: Expr },
    /// `<any-smt-type>::eq`
    SmtEq { t: TypeRef, lhs: Expr, rhs: Expr },
    /// `<any-smt-type>::ne`
    SmtNe { t: TypeRef, lhs: Expr, rhs: Expr },
}

macro_rules! mk0 {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        Intrinsic::unpack_ty_arg_0($ty_args)?;
        Intrinsic::unpack_expr_0($args)?;
        Intrinsic::$op
    }};
}

// mk1!(BoolNot, ty_args, args) //no type arguments expected for this function.
// the above expands to:
// {
//     Intrinsic::unpack_ty_arg_0(ty_args)?;
//     let e1 = Intrinsic::unpack_expr_1(args)?;
//     Intrinsic::BoolNot { val: e1 }
macro_rules! mk1 {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        Intrinsic::unpack_ty_arg_0($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?; // a.not() expects 1 argument including the receiver
        Intrinsic::$op { val: e1 }
    }};
}

macro_rules! mk2 {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        Intrinsic::unpack_ty_arg_0($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op { lhs: e1, rhs: e2 }
    }};
}

macro_rules! mk0_t {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        Intrinsic::unpack_expr_0($args)?;
        Intrinsic::$op { t: t1 }
    }};
}

macro_rules! mk1_t {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?;
        Intrinsic::$op { t: t1, $n1: e1 }
    }};
}

macro_rules! mk2_t {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident) => {{
        let t1 = Intrinsic::unpack_ty_arg_1($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op {
            t: t1,
            $n1: e1,
            $n2: e2,
        }
    }};
}

macro_rules! mk0_kv {
    ($op:ident, $ty_args:expr, $args:expr) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        Intrinsic::unpack_expr_0($args)?;
        Intrinsic::$op { k: t1, v: t2 }
    }};
}

macro_rules! mk1_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let e1 = Intrinsic::unpack_expr_1($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
        }
    }};
}

macro_rules! mk2_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let (e1, e2) = Intrinsic::unpack_expr_2($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
            $n2: e2,
        }
    }};
}

macro_rules! mk3_kv {
    ($op:ident, $ty_args:expr, $args:expr, $n1:ident, $n2:ident, $n3:ident) => {{
        let (t1, t2) = Intrinsic::unpack_ty_arg_2($ty_args)?;
        let (e1, e2, e3) = Intrinsic::unpack_expr_3($args)?;
        Intrinsic::$op {
            k: t1,
            v: t2,
            $n1: e1,
            $n2: e2,
            $n3: e3,
        }
    }};
}

impl Intrinsic {
    /// Convert an argument list to a boolean literal
    pub fn unpack_lit_bool(args: &Punctuated<Exp, Comma>) -> Result<bool> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Bool(val) => val.value,
                    _ => bail_on!(lit, "not a boolean literal"),
                }
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an argument list to an integer literal
    pub fn unpack_lit_int(args: &Punctuated<Exp, Comma>) -> Result<i64> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Int(val) => match val.token().to_string().parse() {
                        Ok(v) => v,
                        Err(_) => bail_on!(val, "unable to parse"),
                    },
                    _ => bail_on!(lit, "not an integer literal"),
                }
            }
            Exp::Unary(unary) => {
                let ExprUnary { attrs: _, op, expr } = unary;
                let val = match op {
                    syn::UnOp::Neg(_) => -Self::unpack_lit_int(&Punctuated::from_iter(vec![
                        (*expr).as_ref().clone(),
                    ]))?,
                    _ => bail_on!(op, "not a unary negation operator"),
                };
                val
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an argument list to a floating-point literal
    pub fn unpack_lit_float(args: &Punctuated<Exp, Comma>) -> Result<i64> {
        Self::unpack_lit_int(args)
    }

    /// Convert an argument list to a string literal
    pub fn unpack_lit_str(args: &Punctuated<Exp, Comma>) -> Result<String> {
        let mut iter = args.iter();
        let expr = bail_if_missing!(iter.next(), args, "argument");
        let parsed = match expr {
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                match lit {
                    Lit::Str(val) => val.token().to_string(),
                    _ => bail_on!(lit, "not a string literal"),
                }
            }
            _ => bail_on!(expr, "not a literal"),
        };
        bail_if_exists!(iter.next());
        Ok(parsed)
    }

    /// Convert an expression to a literal
    pub fn parse_literal_into(receiver: &Exp) -> Result<(Self, TypeRef)> {
        let (intrinsic, ty) = match receiver {
            // A literal in place of an expression: `1`, `"foo"`.
            Exp::Lit(expr_lit) => {
                let ExprLit { attrs: _, lit } = expr_lit;
                // lit is a Rust literal such as a string or integer or boolean.
                // In rustmart we only have boolean, integer, float, and string literals. So we only need to check for these.
                match lit {
                    Lit::Bool(val) => (Self::BoolVal(val.value), TypeRef::Boolean),
                    Lit::Int(val) => {
                        let parsed = match val.token().to_string().parse() {
                            Ok(v) => v,
                            Err(_) => bail_on!(val, "unable to parse literal integer"),
                        };
                        (Self::IntVal(parsed), TypeRef::Integer)
                    }
                    Lit::Float(val) => {
                        let parsed = match val.token().to_string().parse() {
                            Ok(v) => v,
                            Err(_) => bail_on!(val, "unable to parse literal float"),
                        };
                        (Self::NumVal(parsed), TypeRef::Rational)
                    }
                    Lit::Str(val) => (Self::StrVal(val.token().to_string()), TypeRef::Text),
                    // if not a boolean, integer, float, or string, bail
                    _ => bail_on!(lit, "not an expected literal"),
                }
            }
            // if not a literal, bail
            _ => bail_on!(receiver, "not a literal"),
        };
        Ok((intrinsic, ty))
    }

    /// Create an intrinsic
    pub fn new(
        ty_name: &SysTypeName,
        fn_name: &UsrFuncName,
        ty_args: Vec<TypeRef>,
        args: Vec<Expr>,
    ) -> anyhow::Result<Self> {
        use SysTypeName as Q;

        let intrinsic = match (ty_name, fn_name.as_ref()) {
            // boolean
            (Q::Boolean, "not") => mk1!(BoolNot, ty_args, args),
            (Q::Boolean, "and") => mk2!(BoolAnd, ty_args, args),
            (Q::Boolean, "or") => mk2!(BoolOr, ty_args, args),
            (Q::Boolean, "xor") => mk2!(BoolXor, ty_args, args),
            (Q::Boolean, "implies") => mk2!(BoolImplies, ty_args, args),
            (Q::Boolean, "iff") => mk2!(BoolIff, ty_args, args),
            // integer
            (Q::Integer, "add") => mk2!(IntAdd, ty_args, args),
            (Q::Integer, "sub") => mk2!(IntSub, ty_args, args),
            (Q::Integer, "mul") => mk2!(IntMul, ty_args, args),
            (Q::Integer, "div") => mk2!(IntDiv, ty_args, args),
            (Q::Integer, "rem") => mk2!(IntRem, ty_args, args),
            (Q::Integer, "lt") => mk2!(IntLt, ty_args, args),
            (Q::Integer, "le") => mk2!(IntLe, ty_args, args),
            (Q::Integer, "ge") => mk2!(IntGe, ty_args, args),
            (Q::Integer, "gt") => mk2!(IntGt, ty_args, args),
            (Q::Integer, "to_rational") => mk1!(IntToRational, ty_args, args),
            (Q::Integer, "abs") => mk1!(IntAbs, ty_args, args),
            (Q::Integer, "pow") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (base, exp) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::IntPow { base, exp }
            }
            // rational
            (Q::Rational, "add") => mk2!(NumAdd, ty_args, args),
            (Q::Rational, "sub") => mk2!(NumSub, ty_args, args),
            (Q::Rational, "mul") => mk2!(NumMul, ty_args, args),
            (Q::Rational, "div") => mk2!(NumDiv, ty_args, args),
            (Q::Rational, "lt") => mk2!(NumLt, ty_args, args),
            (Q::Rational, "le") => mk2!(NumLe, ty_args, args),
            (Q::Rational, "ge") => mk2!(NumGe, ty_args, args),
            (Q::Rational, "gt") => mk2!(NumGt, ty_args, args),
            (Q::Rational, "round") => mk1!(NumRound, ty_args, args),
            (Q::Rational, "floor") => mk1!(NumFloor, ty_args, args),
            (Q::Rational, "ceil") => mk1!(NumCeil, ty_args, args),
            (Q::Rational, "abs") => mk1!(NumAbs, ty_args, args),
            (Q::Rational, "pow") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (base, exp) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::NumPow { base, exp }
            }
            // text
            (Q::Text, "lt") => mk2!(StrLt, ty_args, args),
            (Q::Text, "le") => mk2!(StrLe, ty_args, args),
            (Q::Text, "gt") => mk2!(StrGt, ty_args, args),
            (Q::Text, "ge") => mk2!(StrGe, ty_args, args),
            (Q::Text, "concat") => mk2!(StrConcat, ty_args, args),
            (Q::Text, "at_index") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (e1, e2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::StrAt { seq: e1, idx: e2 }
            }
            (Q::Text, "length") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let e1 = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::StrLength { seq: e1 }
            }
            (Q::Text, "contains") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (e1, e2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::StrIncludes { seq: e1, item: e2 }
            }
            (Q::Text, "starts_with") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (e1, e2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::StrStartsWith { seq: e1, item: e2 }
            }
            (Q::Text, "ends_with") => {
                Intrinsic::unpack_ty_arg_0(ty_args)?;
                let (e1, e2) = Intrinsic::unpack_expr_2(args)?;
                Intrinsic::StrEndsWith { seq: e1, item: e2 }
            }
            // cloak
            (Q::Cloak, "shield") => mk1_t!(BoxShield, ty_args, args, val),
            (Q::Cloak, "reveal") => mk1_t!(BoxReveal, ty_args, args, val),
            // seq
            (Q::Seq, "new") => mk0_t!(SeqEmpty, ty_args, args),
            (Q::Seq, "length") => mk1_t!(SeqLength, ty_args, args, seq),
            (Q::Seq, "append") => mk2_t!(SeqAppend, ty_args, args, seq, item),
            (Q::Seq, "at_unchecked") => mk2_t!(SeqAt, ty_args, args, seq, idx),
            (Q::Seq, "includes") => mk2_t!(SeqIncludes, ty_args, args, seq, item),
            (Q::Seq, "is_empty") => {
                let t1 = Intrinsic::unpack_ty_arg_1(ty_args)?;
                let e1 = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::SeqIsEmpty { t: t1, seq: e1 }
            }
            // set
            (Q::Set, "new") => mk0_t!(SetEmpty, ty_args, args),
            (Q::Set, "length") => mk1_t!(SetLength, ty_args, args, set),
            (Q::Set, "insert") => mk2_t!(SetInsert, ty_args, args, set, item),
            (Q::Set, "remove") => mk2_t!(SetRemove, ty_args, args, set, item),
            (Q::Set, "contains") => mk2_t!(SetContains, ty_args, args, set, item),
            (Q::Set, "intersection") => mk2_t!(SetIntersection, ty_args, args, lhs, rhs),
            (Q::Set, "union") => mk2_t!(SetUnion, ty_args, args, lhs, rhs),
            (Q::Set, "difference") => mk2_t!(SetDifference, ty_args, args, lhs, rhs),
            (Q::Set, "is_subset") => mk2_t!(SetIsSubset, ty_args, args, lhs, rhs),
            (Q::Set, "is_empty") => {
                let t1 = Intrinsic::unpack_ty_arg_1(ty_args)?;
                let e1 = Intrinsic::unpack_expr_1(args)?;
                Intrinsic::SetIsEmpty { t: t1, set: e1 }
            }
            // map
            (Q::Map, "new") => mk0_kv!(MapEmpty, ty_args, args),
            (Q::Map, "length") => mk1_kv!(MapLength, ty_args, args, map),
            (Q::Map, "put_unchecked") => mk3_kv!(MapPut, ty_args, args, map, key, val),
            (Q::Map, "get_unchecked") => mk2_kv!(MapGet, ty_args, args, map, key),
            (Q::Map, "del_unchecked") => mk2_kv!(MapDel, ty_args, args, map, key),
            (Q::Map, "contains_key") => mk2_kv!(MapContainsKey, ty_args, args, map, key),
            (Q::Map, "is_empty") => mk1_kv!(MapIsEmpty, ty_args, args, map),
            // error
            (Q::Error, "fresh") => mk0!(ErrFresh, ty_args, args),
            (Q::Error, "merge") => mk2!(ErrMerge, ty_args, args),
            // others
            _ => bail!("no such intrinsic"),
        };
        Ok(intrinsic)
    }

    /// Utility to unpack 0 type argument
    fn unpack_ty_arg_0(ty_args: Vec<TypeRef>) -> anyhow::Result<()> {
        let mut iter = ty_args.into_iter();
        if iter.next().is_some() {
            bail!("expect 0 type argument");
        }
        Ok(())
    }

    /// Utility to unpack 1 type argument
    fn unpack_ty_arg_1(ty_args: Vec<TypeRef>) -> anyhow::Result<TypeRef> {
        let mut iter = ty_args.into_iter();
        let t1 = match iter.next() {
            None => bail!("expect 1 type argument"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail!("expect 1 type argument");
        }
        Ok(t1)
    }

    /// Utility to unpack 2 type arguments
    fn unpack_ty_arg_2(ty_args: Vec<TypeRef>) -> anyhow::Result<(TypeRef, TypeRef)> {
        let mut iter = ty_args.into_iter();
        let t1 = match iter.next() {
            None => bail!("expect 2 type arguments"),
            Some(t) => t,
        };
        let t2 = match iter.next() {
            None => bail!("expect 2 type arguments"),
            Some(t) => t,
        };
        if iter.next().is_some() {
            bail!("expect 2 type arguments");
        }
        Ok((t1, t2))
    }

    /// Utility to unpack 0 argument
    fn unpack_expr_0(exprs: Vec<Expr>) -> anyhow::Result<()> {
        let mut iter = exprs.into_iter();
        if iter.next().is_some() {
            bail!("expect 0 argument");
        }
        Ok(())
    }

    /// Utility to unpack 1 argument
    fn unpack_expr_1(exprs: Vec<Expr>) -> anyhow::Result<Expr> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 1 argument"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 1 argument");
        }
        Ok(e1)
    }

    /// Utility to unpack 2 arguments
    fn unpack_expr_2(exprs: Vec<Expr>) -> anyhow::Result<(Expr, Expr)> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 2 arguments"),
            Some(e) => e,
        };
        let e2 = match iter.next() {
            None => bail!("expect 2 arguments"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 2 arguments");
        }
        Ok((e1, e2))
    }

    /// Utility to unpack 3 arguments
    fn unpack_expr_3(exprs: Vec<Expr>) -> anyhow::Result<(Expr, Expr, Expr)> {
        let mut iter = exprs.into_iter();
        let e1 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        let e2 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        let e3 = match iter.next() {
            None => bail!("expect 3 arguments"),
            Some(e) => e,
        };
        if iter.next().is_some() {
            bail!("expect 3 arguments");
        }
        Ok((e1, e2, e3))
    }
}

impl Display for Intrinsic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BoolVal(v) => write!(f, "{v}"),
            Self::BoolNot { val } => write!(f, "!{val}"),
            Self::BoolAnd { lhs, rhs } => write!(f, "{lhs} & {rhs}"),
            Self::BoolOr { lhs, rhs } => write!(f, "{lhs} | {rhs}"),
            Self::BoolXor { lhs, rhs } => write!(f, "{lhs} ^ {rhs}"),
            Self::BoolImplies { lhs, rhs } => write!(f, "{lhs} => {rhs}"),
            Self::BoolIff { lhs, rhs } => write!(f, "{lhs} <=> {rhs}"),
            Self::IntVal(v) => write!(f, "{v}"),
            Self::NumVal(v) => write!(f, "{v}"),
            Self::IntLt { lhs, rhs } | Self::NumLt { lhs, rhs } | Self::StrLt { lhs, rhs } => {
                write!(f, "{lhs} < {rhs}")
            }
            Self::IntLe { lhs, rhs } | Self::NumLe { lhs, rhs } | Self::StrLe { lhs, rhs } => {
                write!(f, "{lhs} <= {rhs}")
            }
            Self::IntGe { lhs, rhs } | Self::NumGe { lhs, rhs } | Self::StrGe { lhs, rhs } => {
                write!(f, "{lhs} >= {rhs}")
            }
            Self::IntGt { lhs, rhs } | Self::NumGt { lhs, rhs } | Self::StrGt { lhs, rhs } => {
                write!(f, "{lhs} > {rhs}")
            }
            Self::IntAdd { lhs, rhs } | Self::NumAdd { lhs, rhs } => write!(f, "{lhs} + {rhs}"),
            Self::IntSub { lhs, rhs } | Self::NumSub { lhs, rhs } => write!(f, "{lhs} - {rhs}"),
            Self::IntMul { lhs, rhs } | Self::NumMul { lhs, rhs } => write!(f, "{lhs} * {rhs}"),
            Self::IntDiv { lhs, rhs } | Self::NumDiv { lhs, rhs } => write!(f, "{lhs} / {rhs}"),
            Self::IntRem { lhs, rhs } => write!(f, "{lhs} % {rhs}"),
            Self::IntToRational { val } => write!(f, "(rational){val}"),
            Self::IntPow { base, exp } | Self::NumPow { base, exp } => {
                write!(f, "{base} ^ {exp}")
            }
            Self::IntAbs { val } | Self::NumAbs { val } => write!(f, "|{val}|"),
            Self::NumRound { val } => write!(f, "round({val})"),
            Self::NumFloor { val } => write!(f, "floor({val})"),
            Self::NumCeil { val } => write!(f, "ceil({val})"),
            Self::StrVal(v) => write!(f, "{v}"),
            Self::StrConcat { lhs, rhs } => write!(f, "{lhs} ++ {rhs}"),
            Self::StrAt { seq, idx } => write!(f, "{seq}.at({idx})"),
            Self::StrLength { seq } => write!(f, "{seq}.len()"),
            Self::StrIncludes { seq, item } => write!(f, "{seq}.includes({item})"),
            Self::StrStartsWith { seq, item } => write!(f, "{seq}.starts_with({item})"),
            Self::StrEndsWith { seq, item } => write!(f, "{seq}.ends_with({item})"),
            Self::BoxShield { t, val } => write!(f, "&<{t}>({val})"),
            Self::BoxReveal { t, val } => write!(f, "*<{t}>({val})"),
            Self::SeqEmpty { t } => write!(f, "vec<{t}>[]"),
            Self::SeqLength { t, seq } => write!(f, "{seq}.len<{t}>(vec)"),
            Self::SeqAppend { t, seq, item } => write!(f, "{seq}.append<{t}>({item})"),
            Self::SeqAt { t, seq, idx } => write!(f, "{seq}.at<{t}>({idx})"),
            Self::SeqIncludes { t, seq, item } => write!(f, "{seq}.includes<{t}>({item})"),
            Self::SeqIsEmpty { t, seq } => write!(f, "{seq}.is_empty<{t}>()"),
            Self::SetEmpty { t } => write!(f, "set<{t}>[]"),
            Self::SetLength { t, set } => write!(f, "{set}.len<{t}>(set)"),
            Self::SetInsert { t, set, item } => write!(f, "{set}.insert<{t}>({item})"),
            Self::SetRemove { t, set, item } => write!(f, "{set}.remove<{t}>({item})"),
            Self::SetContains { t, set, item } => write!(f, "{set}.contains<{t}>({item})"),
            Self::SetIntersection { t, lhs, rhs } => write!(f, "{lhs} ∩<{t}> {rhs}"),
            Self::SetUnion { t, lhs, rhs } => write!(f, "{lhs} U<{t}> {rhs}"),
            Self::SetDifference { t, lhs, rhs } => write!(f, "{lhs} -<{t}> {rhs}"),
            Self::SetIsSubset { t, lhs, rhs } => write!(f, "{lhs} ⊆<{t}> {rhs}"),
            Self::SetIsEmpty { t, set } => write!(f, "{set}.is_empty<{t}>()"),
            Self::MapEmpty { k, v } => write!(f, "map<{k},{v}>[]"),
            Self::MapLength { k, v, map } => write!(f, "{map}.len<{k},{v}>(map)"),
            Self::MapPut {
                k,
                v,
                map,
                key,
                val,
            } => write!(f, "{map}.put<{k},{v}>({key},{val})"),
            Self::MapGet { k, v, map, key } => write!(f, "{map}.get<{k},{v}>({key})"),
            Self::MapDel { k, v, map, key } => write!(f, "{map}.del<{k},{v}>({key})"),
            Self::MapContainsKey { k, v, map, key } => {
                write!(f, "{map}.contains_key<{k},{v}>({key})")
            }
            Self::MapIsEmpty { k, v, map } => write!(f, "{map}.is_empty<{k},{v}>()"),
            Self::ErrFresh => write!(f, "error"),
            Self::ErrMerge { lhs, rhs } => write!(f, "{lhs} ~ {rhs}"),
            Self::SmtEq { t: _, lhs, rhs } => write!(f, "{lhs} == {rhs}"),
            Self::SmtNe { t: _, lhs, rhs } => write!(f, "{lhs} != {rhs}"),
        }
    }
}

//! This module contains the conversion of RuSmt intrinsics to SMT-LIB format.

use crate::backend::z3::exp::format_expression_renamed;
use crate::backend::z3::fun::{collect_called_functions, format_sort_for_fn};
use crate::backend::z3::sort::{format_sort, get_generic_param_count, resolve_type_name};
use crate::ir::ctxt::IRContext;
use crate::ir::exp::ExpRegistry;
use crate::ir::index::{UsrFunId, UsrSortId};
use crate::ir::intrinsics::Intrinsic;
use crate::ir::sort::{DataType, Sort, Variant};
use num_bigint::BigInt;
use std::collections::{BTreeMap, BTreeSet};

/// Generate an SMT-LIB term for the "null" (don't-care) default of an array value sort.
///
/// Membership lives in `rarr-pres`, so this value carries no meaning -- but it *is*
/// observable, via `select` on an absent key, so it must be the term denoting the
/// value sort's Rust `Default`. For an enum that is the first variant in declaration
/// order, which is what `#[smt_type]` generates.
///
/// Panics if the sort has no writable finite inhabitant — that means a non-well-founded
/// datatype, which Z3 would reject at `declare-datatype` anyway.
pub fn array_null_value(sort: &Sort, ir: &IRContext) -> String {
    canonical_value(sort, ir, &mut BTreeSet::new()).unwrap_or_else(|| {
        panic!(
            "no finite canonical inhabitant for sort `{}`: the datatype is not \
             well-founded, so Z3 cannot declare it either",
            format_sort(sort, ir)
        )
    })
}

/// Recursively build a canonical inhabitant term of `sort`, or `None` if it cannot be
/// built.
///
/// `in_progress` holds the user sorts currently being built; re-entering one is a
/// recursive cycle and returns `None`, so an enum's variant search backtracks to a base
/// variant (well-foundedness guarantees one exists).
fn canonical_value(
    sort: &Sort,
    ir: &IRContext,
    in_progress: &mut BTreeSet<UsrSortId>,
) -> Option<String> {
    let term = match sort {
        Sort::Boolean => "false".to_string(),
        Sort::Integer => "0".to_string(),
        Sort::Real => "0.0".to_string(),
        Sort::String => "\"\"".to_string(),
        Sort::I32 | Sort::U32 => "(_ bv0 32)".to_string(),
        Sort::I64 | Sort::U64 => "(_ bv0 64)".to_string(),
        Sort::F32 => "((_ to_fp 8 24) RNE 0.0)".to_string(),
        Sort::F64 => "((_ to_fp 11 53) RNE 0.0)".to_string(),
        Sort::Path => crate::backend::z3::path::empty_literal(ir), // no marker fired
        Sort::Seq(_) => format!("(as seq.empty {})", format_sort(sort, ir)),
        Sort::Set(inner) => format!(
            "(mk-rset ((as const (Array {} Bool)) false) 0)",
            format_sort(inner, ir)
        ),
        Sort::Array(k, v) => {
            let ks = format_sort(k, ir);
            let vs = format_sort(v, ir);
            let vd = canonical_value(v, ir, in_progress)?;
            format!(
                "(mk-rarr ((as const (Array {ks} {vs})) {vd}) ((as const (Array {ks} Bool)) false) 0)"
            )
        }
        Sort::User(sid) => {
            if in_progress.contains(sid) {
                return None; // recursive cycle — let the caller backtrack
            }
            in_progress.insert(*sid);
            let built = build_user_value(*sid, ir, in_progress);
            in_progress.remove(sid);
            return built;
        }
        Sort::Uninterpreted(_) => {
            panic!(
                "this can't happen: Uninterpreted sorts are monomorphized away before Z3 backend"
            )
        }
    };
    Some(term)
}

/// Build a canonical value for a user sort.
///
/// A `par`-declared datatype shares one constructor symbol across every
/// instantiation, so each use must be qualified with the target sort:
/// `(as Ctor (T A ..))`, or `((as Ctor (T A ..)) f1 ..)` when applied. The datatype's
/// declared arity is exactly `get_generic_param_count(..).0`, which is `0` for
/// non-generic named types *and* for fully concrete unnamed tuples (whose sort name
/// already carries the element types, e.g. `Tuple_Int_Bool`).
fn build_user_value(
    sid: UsrSortId,
    ir: &IRContext,
    in_progress: &mut BTreeSet<UsrSortId>,
) -> Option<String> {
    let name = resolve_type_name(ir, sid);
    let qual = (get_generic_param_count(ir, sid).0 > 0).then(|| format_sort(&Sort::User(sid), ir));
    let qual = qual.as_deref();

    match ir.ty_registry.retrieve(sid) {
        DataType::Tuple(elems) => build_ctor(&format!("mk-{name}"), qual, elems, ir, in_progress),
        DataType::Record(fields) => {
            let sorts: Vec<Sort> = fields.values().cloned().collect();
            build_ctor(&format!("mk-{name}"), qual, &sorts, ir, in_progress)
        }
        DataType::Enum { variants, default } => {
            // Must be the variant `#[smt_type]` gives `Default::default()`, which is
            // the first in declaration order. Anything else and a concrete
            // `V::default()` would disagree with the term Z3 reads at an absent key.
            let ctor = format!("{name}_{default}");
            match variants
                .get(default)
                .expect("the default variant is one of the variants")
            {
                Variant::Unit => Some(apply_ctor(&ctor, qual, &[])),
                Variant::Tuple(slots) => build_ctor(&ctor, qual, slots, ir, in_progress),
                Variant::Record(rec) => {
                    let sorts: Vec<Sort> = rec.values().cloned().collect();
                    build_ctor(&ctor, qual, &sorts, ir, in_progress)
                }
            }
        }
    }
}

/// `Ctor` / `(as Ctor S)` / `(Ctor a b)` / `((as Ctor S) a b)`.
fn apply_ctor(ctor: &str, qual: Option<&str>, args: &[String]) -> String {
    let head = match qual {
        Some(s) => format!("(as {ctor} {s})"),
        None => ctor.to_string(),
    };
    if args.is_empty() {
        head
    } else {
        format!("({head} {})", args.join(" "))
    }
}

/// Apply a constructor to canonical values of its field sorts; `None` if any field
/// cannot be built (propagates a cycle up to the enum's backtrack).
fn build_ctor(
    ctor: &str,
    qual: Option<&str>,
    field_sorts: &[Sort],
    ir: &IRContext,
    in_progress: &mut BTreeSet<UsrSortId>,
) -> Option<String> {
    let mut args = Vec::with_capacity(field_sorts.len());
    for s in field_sorts {
        args.push(canonical_value(s, ir, in_progress)?);
    }
    Some(apply_ctor(ctor, qual, &args))
}

/// Accumulates `let` bindings that share a sub-term used more than once in an
/// intrinsic's expansion.
///
/// Several intrinsics mention an operand many times — `ArrayStore` reads the array
/// five times (`rarr-data`, two `rarr-pres`, two `rarr-card`). Because `fmt` renders
/// operands *inline*, an `n`-deep chain of stores would otherwise emit the base array
/// `5^n` times. Binding the operand once keeps the emitted text linear in the
/// expression depth. Z3 hash-conses either way, so the resulting AST — and therefore
/// the semantics — is identical; what shrinks is the text we generate and Z3 parses.
///
/// Binder names are `__`-prefixed, the convention this backend already reserves for
/// generated binders (see `ctxt.rs`), so they can never collide with a user variable.
/// Nesting is safe without unique suffixes: a nested expansion's `let` sits either in
/// a binding *value* (evaluated in the enclosing scope, since SMT-LIB `let` is
/// non-recursive) or wholly inside its own body, and no rendering ever emits a `__`
/// name free. So an inner binder can only shadow itself.
struct LetScope {
    bindings: Vec<String>,
}

impl LetScope {
    fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Register an operand that the body mentions more than once, returning the token
    /// to use in its place. Single-token operands (a bare parameter name or literal)
    /// are returned verbatim — binding those would only add bytes.
    fn share(&mut self, name: &str, term: String) -> String {
        if !term.starts_with('(') {
            return term;
        }
        let binder = format!("__{name}");
        self.bindings.push(format!("({binder} {term})"));
        binder
    }

    /// Wrap `body` in the accumulated bindings, or return it unchanged if nothing was
    /// worth sharing. The `let` lands exactly where the term sat, so any enclosing
    /// quantifier or `let` still scopes over the operands.
    fn wrap(self, body: String) -> String {
        if self.bindings.is_empty() {
            body
        } else {
            format!("(let ({}) {})", self.bindings.join(" "), body)
        }
    }
}

/// Collect all called functions inside an intrinsic
pub(crate) fn collect_fn_from_intrinsic(
    exp_registry: &ExpRegistry,
    intrinsic: &Intrinsic,
) -> Vec<UsrFunId> {
    let mut called_fns = vec![];

    match intrinsic {
        Intrinsic::BoolVal(_)
        | Intrinsic::IntVal(_)
        | Intrinsic::RealVal(_)
        | Intrinsic::StrVal(_)
        | Intrinsic::StrNew
        | Intrinsic::SeqEmpty { .. }
        | Intrinsic::SetEmpty { .. }
        | Intrinsic::ArrayEmpty { .. }
        | Intrinsic::BvVal { .. }
        | Intrinsic::FloatVal { .. }
        | Intrinsic::FloatNaN { .. }
        | Intrinsic::FloatPosInf { .. }
        | Intrinsic::FloatNegInf { .. }
        | Intrinsic::FloatPosZero { .. }
        | Intrinsic::FloatNegZero { .. }
        | Intrinsic::PathName { .. } => {}
        Intrinsic::BoolNot { val }
        | Intrinsic::IntNeg { val }
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
        | Intrinsic::IntIsGtU32Max { val }
        | Intrinsic::RealNeg { val }
        | Intrinsic::RealAbs { val }
        | Intrinsic::RealRound { val }
        | Intrinsic::RealFloor { val }
        | Intrinsic::RealCeil { val }
        | Intrinsic::RealIsInt { val }
        | Intrinsic::RealToInt { val }
        | Intrinsic::RealToF32 { val }
        | Intrinsic::RealToF64 { val }
        | Intrinsic::StrLen { seq: val }
        | Intrinsic::StrIsEmpty { seq: val }
        | Intrinsic::StrIsDigit { seq: val }
        | Intrinsic::StrToInt { val }
        | Intrinsic::StrFromInt { val }
        | Intrinsic::StrFromCode { val }
        | Intrinsic::StrToCode { val }
        | Intrinsic::SeqUnit { val, .. }
        | Intrinsic::SeqLen { seq: val, .. }
        | Intrinsic::SeqIsEmpty { seq: val, .. }
        | Intrinsic::SetLen { set: val, .. }
        | Intrinsic::SetIsEmpty { set: val, .. }
        | Intrinsic::ArrayLen { arr: val, .. }
        | Intrinsic::ArrayIsEmpty { arr: val, .. }
        | Intrinsic::BvNot { val, .. }
        | Intrinsic::BvNeg { val, .. }
        | Intrinsic::BvRedAnd { val, .. }
        | Intrinsic::BvRedOr { val, .. }
        | Intrinsic::BvToInt { val, .. }
        | Intrinsic::FloatNeg { val, .. }
        | Intrinsic::FloatAbs { val, .. }
        | Intrinsic::FloatSqrt { val, .. }
        | Intrinsic::FloatIsNaN { val, .. }
        | Intrinsic::FloatIsInf { val, .. }
        | Intrinsic::FloatIsZero { val, .. }
        | Intrinsic::FloatIsNormal { val, .. }
        | Intrinsic::FloatIsSubnormal { val, .. }
        | Intrinsic::FloatIsNeg { val, .. }
        | Intrinsic::FloatIsPos { val, .. }
        | Intrinsic::FloatToInt { val, .. }
        | Intrinsic::FloatToReal { val, .. }
        | Intrinsic::FloatToU32 { val, .. }
        | Intrinsic::FloatToI32 { val, .. }
        | Intrinsic::FloatToU64 { val, .. }
        | Intrinsic::FloatToI64 { val, .. }
        | Intrinsic::FloatCeil { val, .. }
        | Intrinsic::FloatFloor { val, .. }
        | Intrinsic::FloatTrunc { val, .. }
        | Intrinsic::FloatNearest { val, .. } => {
            called_fns.append(&mut collect_called_functions(exp_registry, val));
        }
        Intrinsic::BoolAnd { lhs, rhs }
        | Intrinsic::BoolOr { lhs, rhs }
        | Intrinsic::BoolXor { lhs, rhs }
        | Intrinsic::BoolNand { lhs, rhs }
        | Intrinsic::BoolNor { lhs, rhs }
        | Intrinsic::BoolXnor { lhs, rhs }
        | Intrinsic::BoolImplies { lhs, rhs }
        | Intrinsic::BoolIff { lhs, rhs }
        | Intrinsic::IntAdd { lhs, rhs }
        | Intrinsic::IntSub { lhs, rhs }
        | Intrinsic::IntMul { lhs, rhs }
        | Intrinsic::IntDiv { lhs, rhs }
        | Intrinsic::IntDivTrunc { lhs, rhs }
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
        | Intrinsic::IntGe { lhs, rhs }
        | Intrinsic::RealAdd { lhs, rhs }
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
        | Intrinsic::RealGe { lhs, rhs }
        | Intrinsic::StrConcat { lhs, rhs }
        | Intrinsic::StrAt { seq: lhs, idx: rhs }
        | Intrinsic::StrIndexOfDefault { seq: lhs, sub: rhs }
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
        | Intrinsic::StrLt { lhs, rhs }
        | Intrinsic::StrLe { lhs, rhs }
        | Intrinsic::StrGt { lhs, rhs }
        | Intrinsic::StrGe { lhs, rhs }
        | Intrinsic::SeqPush {
            seq: lhs,
            item: rhs,
            ..
        }
        | Intrinsic::SeqConcat { lhs, rhs, .. }
        | Intrinsic::SeqNth {
            seq: lhs, idx: rhs, ..
        }
        | Intrinsic::SeqAtSeq {
            seq: lhs, idx: rhs, ..
        }
        | Intrinsic::SeqIndexOfDefault {
            seq: lhs, sub: rhs, ..
        }
        | Intrinsic::SeqContains {
            seq: lhs,
            item: rhs,
            ..
        }
        | Intrinsic::SeqPrefixOf { lhs, rhs, .. }
        | Intrinsic::SeqSuffixOf { lhs, rhs, .. }
        | Intrinsic::SetInsert {
            set: lhs,
            item: rhs,
            ..
        }
        | Intrinsic::SetRemove {
            set: lhs,
            item: rhs,
            ..
        }
        | Intrinsic::SetContains {
            set: lhs,
            item: rhs,
            ..
        }
        | Intrinsic::SetIsSubset { lhs, rhs, .. }
        | Intrinsic::SetIsProperSubset { lhs, rhs, .. }
        | Intrinsic::SetIsDisjoint { lhs, rhs, .. }
        | Intrinsic::SetHasSize {
            set: lhs,
            size: rhs,
            ..
        }
        | Intrinsic::ArraySelect {
            arr: lhs, key: rhs, ..
        }
        | Intrinsic::ArrayRemove {
            arr: lhs, key: rhs, ..
        }
        | Intrinsic::ArrayContainsKey {
            arr: lhs, key: rhs, ..
        }
        | Intrinsic::BvAnd { lhs, rhs, .. }
        | Intrinsic::BvOr { lhs, rhs, .. }
        | Intrinsic::BvXor { lhs, rhs, .. }
        | Intrinsic::BvNand { lhs, rhs, .. }
        | Intrinsic::BvNor { lhs, rhs, .. }
        | Intrinsic::BvXnor { lhs, rhs, .. }
        | Intrinsic::BvAdd { lhs, rhs, .. }
        | Intrinsic::BvSub { lhs, rhs, .. }
        | Intrinsic::BvMul { lhs, rhs, .. }
        | Intrinsic::BvDiv { lhs, rhs, .. }
        | Intrinsic::BvRem { lhs, rhs, .. }
        | Intrinsic::BvMod { lhs, rhs, .. }
        | Intrinsic::BvShl { lhs, rhs, .. }
        | Intrinsic::BvLshr { lhs, rhs, .. }
        | Intrinsic::BvAshr { lhs, rhs, .. }
        | Intrinsic::BvRotLeft { lhs, rhs, .. }
        | Intrinsic::BvRotRight { lhs, rhs, .. }
        | Intrinsic::BvLt { lhs, rhs, .. }
        | Intrinsic::BvLe { lhs, rhs, .. }
        | Intrinsic::BvGt { lhs, rhs, .. }
        | Intrinsic::BvGe { lhs, rhs, .. }
        | Intrinsic::FloatAdd { lhs, rhs, .. }
        | Intrinsic::FloatSub { lhs, rhs, .. }
        | Intrinsic::FloatMul { lhs, rhs, .. }
        | Intrinsic::FloatDiv { lhs, rhs, .. }
        | Intrinsic::FloatRem { lhs, rhs, .. }
        | Intrinsic::FloatMin { lhs, rhs, .. }
        | Intrinsic::FloatMax { lhs, rhs, .. }
        | Intrinsic::FloatLt { lhs, rhs, .. }
        | Intrinsic::FloatLe { lhs, rhs, .. }
        | Intrinsic::FloatGt { lhs, rhs, .. }
        | Intrinsic::FloatGe { lhs, rhs, .. }
        | Intrinsic::FloatFqEq { lhs, rhs, .. }
        | Intrinsic::PathMerge { lhs, rhs, .. }
        | Intrinsic::SmtEq { lhs, rhs, .. }
        | Intrinsic::SmtNe { lhs, rhs, .. } => {
            called_fns.append(&mut collect_called_functions(exp_registry, lhs));
            called_fns.append(&mut collect_called_functions(exp_registry, rhs));
        }
        Intrinsic::BoolIte { cond, then, else_ } => {
            called_fns.append(&mut collect_called_functions(exp_registry, cond));
            called_fns.append(&mut collect_called_functions(exp_registry, then));
            called_fns.append(&mut collect_called_functions(exp_registry, else_));
        }
        Intrinsic::StrIndexOf { seq, sub, offset } => {
            called_fns.append(&mut collect_called_functions(exp_registry, seq));
            called_fns.append(&mut collect_called_functions(exp_registry, sub));
            called_fns.append(&mut collect_called_functions(exp_registry, offset));
        }
        Intrinsic::StrSubstr { seq, start, len } => {
            called_fns.append(&mut collect_called_functions(exp_registry, seq));
            called_fns.append(&mut collect_called_functions(exp_registry, start));
            called_fns.append(&mut collect_called_functions(exp_registry, len));
        }
        Intrinsic::StrReplace { seq, src, dst }
        | Intrinsic::StrReplaceAll { seq, src, dst }
        | Intrinsic::SeqReplace { seq, src, dst, .. } => {
            called_fns.append(&mut collect_called_functions(exp_registry, seq));
            called_fns.append(&mut collect_called_functions(exp_registry, src));
            called_fns.append(&mut collect_called_functions(exp_registry, dst));
        }
        Intrinsic::SeqExtract {
            seq, offset, len, ..
        } => {
            called_fns.append(&mut collect_called_functions(exp_registry, seq));
            called_fns.append(&mut collect_called_functions(exp_registry, offset));
            called_fns.append(&mut collect_called_functions(exp_registry, len));
        }
        Intrinsic::SeqIndexOf {
            seq, sub, offset, ..
        } => {
            called_fns.append(&mut collect_called_functions(exp_registry, seq));
            called_fns.append(&mut collect_called_functions(exp_registry, sub));
            called_fns.append(&mut collect_called_functions(exp_registry, offset));
        }
        Intrinsic::ArrayStore { arr, key, val, .. } => {
            called_fns.append(&mut collect_called_functions(exp_registry, arr));
            called_fns.append(&mut collect_called_functions(exp_registry, key));
            called_fns.append(&mut collect_called_functions(exp_registry, val));
        }
    }
    called_fns
}

/// Convert an intrinsic operation to SMT-LIB string format.
pub fn format_intrinsic(
    intrinsic: &Intrinsic,
    exp_registry: &ExpRegistry,
    ir: &IRContext,
    rename: &BTreeMap<UsrFunId, String>,
) -> String {
    // Helper to format sub-expressions
    let fmt = |id| format_expression_renamed(exp_registry, id, ir, rename);

    match intrinsic {
        Intrinsic::BoolVal(b) => b.to_string(),
        Intrinsic::BoolNot { val } => format!("(not {})", fmt(*val)),
        Intrinsic::BoolAnd { lhs, rhs } => format!("(and {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolOr { lhs, rhs } => format!("(or {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolXor { lhs, rhs } => format!("(xor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolNand { lhs, rhs } => format!("(not (and {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolNor { lhs, rhs } => format!("(not (or {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolXnor { lhs, rhs } => format!("(not (xor {} {}))", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolImplies { lhs, rhs } => format!("(=> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolIff { lhs, rhs } => format!("(= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BoolIte { cond, then, else_ } => {
            format!("(ite {} {} {})", fmt(*cond), fmt(*then), fmt(*else_))
        }
        Intrinsic::IntVal(n) => n.to_string(),
        Intrinsic::IntNeg { val } => format!("(- {})", fmt(*val)),
        Intrinsic::IntAdd { lhs, rhs } => format!("(+ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntSub { lhs, rhs } => format!("(- {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntMul { lhs, rhs } => format!("(* {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntDiv { lhs, rhs } => format!("(div {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntDivTrunc { lhs, rhs } => {
            let mut lets = LetScope::new();
            let l = lets.share("l", fmt(*lhs));
            let r = lets.share("r", fmt(*rhs));
            lets.wrap(format!(
                "(ite (>= {l} 0) (div {l} {r}) (- (div (- {l}) {r})))"
            ))
        }
        Intrinsic::IntMod { lhs, rhs } => format!("(mod {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntRem { lhs, rhs } => {
            let mut lets = LetScope::new();
            let l = lets.share("l", fmt(*lhs));
            let r = lets.share("r", fmt(*rhs));
            lets.wrap(format!(
                "(- {l} (* {r} (ite (>= {l} 0) (div {l} {r}) (- (div (- {l}) {r})))))"
            ))
        }
        Intrinsic::IntPow { base, exp } => {
            format!("(to_int (^ {} {}))", fmt(*base), fmt(*exp))
        }
        Intrinsic::IntAbs { val } => format!("(abs {})", fmt(*val)),
        Intrinsic::IntDivides { lhs, rhs } => {
            let l = fmt(*lhs);
            let r = fmt(*rhs);
            format!("(= (mod {r} {l}) 0)")
        }
        Intrinsic::IntLt { lhs, rhs } => format!("(< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntLe { lhs, rhs } => format!("(<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntGt { lhs, rhs } => format!("(> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntGe { lhs, rhs } => format!("(>= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::IntToReal { val } => format!("(to_real {})", fmt(*val)),
        Intrinsic::IntToI32 { val } => format!("((_ int2bv 32) {})", fmt(*val)),
        Intrinsic::IntToI64 { val } => format!("((_ int2bv 64) {})", fmt(*val)),
        Intrinsic::IntToU32 { val } => format!("((_ int2bv 32) {})", fmt(*val)),
        Intrinsic::IntToU64 { val } => format!("((_ int2bv 64) {})", fmt(*val)),
        Intrinsic::IntToF32 { val } => format!("((_ to_fp 8 24) RNE (to_real {}))", fmt(*val)),
        Intrinsic::IntToF64 { val } => format!("((_ to_fp 11 53) RNE (to_real {}))", fmt(*val)),
        Intrinsic::IntFromHex { val } => format!("(rusmt_from_hex_str {})", fmt(*val)),
        Intrinsic::IntFromOct { val } => format!("(rusmt_from_oct_str {})", fmt(*val)),
        Intrinsic::IntFromBin { val } => format!("(rusmt_from_bin_str {})", fmt(*val)),
        Intrinsic::IntIsGtI64Max { val } => format!("(> {} 9223372036854775807)", fmt(*val)),
        Intrinsic::IntIsLtI64Min { val } => format!("(< {} (- 9223372036854775808))", fmt(*val)),
        Intrinsic::IntIsGtU64Max { val } => format!("(> {} 18446744073709551615)", fmt(*val)),
        Intrinsic::IntIsLtU64Min { val } => format!("(< {} 0)", fmt(*val)),
        Intrinsic::IntIsLtI32Min { val } => format!("(< {} (- 2147483648))", fmt(*val)),
        Intrinsic::IntIsGtI32Max { val } => format!("(> {} 2147483647)", fmt(*val)),
        Intrinsic::IntIsLtU32Min { val } => format!("(< {} 0)", fmt(*val)),
        Intrinsic::IntIsGtU32Max { val } => format!("(> {} 4294967295)", fmt(*val)),
        Intrinsic::RealVal(r) => format!("(/ {} {})", r.numer(), r.denom()),
        Intrinsic::RealNeg { val } => format!("(- {})", fmt(*val)),
        Intrinsic::RealAdd { lhs, rhs } => format!("(+ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealSub { lhs, rhs } => format!("(- {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealMul { lhs, rhs } => format!("(* {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealDiv { lhs, rhs } => format!("(/ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealPow { base, exp } => format!("(^ {} {})", fmt(*base), fmt(*exp)),
        Intrinsic::RealAbs { val } => format!("(abs {})", fmt(*val)),
        Intrinsic::RealRound { val } => format!("(to_int (+ {} (/ 1.0 2.0)))", fmt(*val)),
        Intrinsic::RealFloor { val } => format!("(to_int {})", fmt(*val)),
        Intrinsic::RealCeil { val } => format!("(- (to_int (- {})))", fmt(*val)),
        Intrinsic::RealIsInt { val } => format!("(is_int {})", fmt(*val)),
        Intrinsic::RealLt { lhs, rhs } => format!("(< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealLe { lhs, rhs } => format!("(<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealGt { lhs, rhs } => format!("(> {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealGe { lhs, rhs } => format!("(>= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::RealToInt { val } => format!("(to_int {})", fmt(*val)),
        Intrinsic::RealToF32 { val } => format!("((_ to_fp 8 24) RNE {})", fmt(*val)),
        Intrinsic::RealToF64 { val } => format!("((_ to_fp 11 53) RNE {})", fmt(*val)),
        Intrinsic::StrVal(s) => format_str_literal(s),
        Intrinsic::StrNew => "\"\"".to_string(),
        Intrinsic::StrLen { seq } => format!("(str.len {})", fmt(*seq)),
        Intrinsic::StrConcat { lhs, rhs } => format!("(str.++ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrAt { seq, idx } => format!("(str.at {} {})", fmt(*seq), fmt(*idx)),
        Intrinsic::StrIndexOf { seq, sub, offset } => {
            format!("(str.indexof {} {} {})", fmt(*seq), fmt(*sub), fmt(*offset))
        }
        Intrinsic::StrIndexOfDefault { seq, sub } => {
            format!("(str.indexof {} {})", fmt(*seq), fmt(*sub))
        }
        Intrinsic::StrSubstr { seq, start, len } => {
            format!("(str.substr {} {} {})", fmt(*seq), fmt(*start), fmt(*len))
        }
        Intrinsic::StrContains { seq, item } => {
            format!("(str.contains {} {})", fmt(*seq), fmt(*item))
        }
        Intrinsic::StrStartsWith { seq, item } => {
            format!("(str.prefixof {} {})", fmt(*item), fmt(*seq))
        }
        Intrinsic::StrEndsWith { seq, item } => {
            format!("(str.suffixof {} {})", fmt(*item), fmt(*seq))
        }
        Intrinsic::StrLe { lhs, rhs } => format!("(str.<= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrLt { lhs, rhs } => format!("(str.< {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::StrGe { lhs, rhs } => format!("(str.<= {} {})", fmt(*rhs), fmt(*lhs)),
        Intrinsic::StrGt { lhs, rhs } => format!("(str.< {} {})", fmt(*rhs), fmt(*lhs)),
        Intrinsic::StrIsDigit { seq } => {
            format!("(str.in_re {} (re.range \"0\" \"9\"))", fmt(*seq))
        }
        Intrinsic::StrToInt { val } => format!("(str.to_int {})", fmt(*val)),
        Intrinsic::StrFromInt { val } => format!("(str.from_int {})", fmt(*val)),
        Intrinsic::StrIsEmpty { seq } => format!("(= (str.len {}) 0)", fmt(*seq)),
        Intrinsic::StrReplace { seq, src, dst } => {
            format!("(str.replace {} {} {})", fmt(*seq), fmt(*src), fmt(*dst))
        }
        Intrinsic::StrReplaceAll { seq, src, dst } => format!(
            "(str.replace_all {} {} {})",
            fmt(*seq),
            fmt(*src),
            fmt(*dst)
        ),
        Intrinsic::StrFromCode { val } => format!("(str.from_code {})", fmt(*val)),
        Intrinsic::StrToCode { val } => format!("(str.to_code {})", fmt(*val)),
        Intrinsic::BvVal { t, val } => {
            let width: u32 = match t {
                Sort::I32 | Sort::U32 => 32,
                Sort::I64 | Sort::U64 => 64,
                _ => panic!("BvVal: Unsupported bitvector type: {t:?}"),
            };
            // SMT-LIB2 requires non-negative value in (_ bvN w).
            // Negative values (e.g. I32::from(-1)) must be converted to their
            // unsigned two's complement: the same bit pattern, just as a positive number.
            // Z3 rejects (_ bv-1 32) with "unknown constant bv-1".
            let unsigned_val = if val.sign() == num_bigint::Sign::Minus {
                val + (BigInt::from(1u64) << width)
            } else {
                val.clone()
            };
            format!("(_ bv{unsigned_val} {width})")
        }
        Intrinsic::BvNot { val, .. } => format!("(bvnot {})", fmt(*val)),
        Intrinsic::BvRedAnd { val, .. } => format!("(= (bvredand {}) (_ bv1 1))", fmt(*val)),
        Intrinsic::BvRedOr { val, .. } => format!("(= (bvredor {}) (_ bv1 1))", fmt(*val)),
        Intrinsic::BvAnd { lhs, rhs, .. } => format!("(bvand {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvOr { lhs, rhs, .. } => format!("(bvor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvXor { lhs, rhs, .. } => format!("(bvxor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvNand { lhs, rhs, .. } => format!("(bvnand {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvNor { lhs, rhs, .. } => format!("(bvnor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvXnor { lhs, rhs, .. } => format!("(bvxnor {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvNeg { val, .. } => format!("(bvneg {})", fmt(*val)),
        Intrinsic::BvAdd { lhs, rhs, .. } => format!("(bvadd {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvSub { lhs, rhs, .. } => format!("(bvsub {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvMul { lhs, rhs, .. } => format!("(bvmul {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvDiv { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvsdiv {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvudiv {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvRem { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvsrem {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvurem {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvMod { t, lhs, rhs } => match t {
            // bvsmod: result has the same sign as the divisor (signed modulo)
            Sort::I32 | Sort::I64 => format!("(bvsmod {} {})", fmt(*lhs), fmt(*rhs)),
            // For unsigned types mod == rem (always non-negative)
            _ => format!("(bvurem {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvShl { lhs, rhs, .. } => format!("(bvshl {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvLshr { lhs, rhs, .. } => format!("(bvlshr {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvAshr { lhs, rhs, .. } => format!("(bvashr {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::BvRotLeft { lhs, rhs, .. } => {
            format!("(ext_rotate_left {} {})", fmt(*lhs), fmt(*rhs))
        }
        Intrinsic::BvRotRight { lhs, rhs, .. } => {
            format!("(ext_rotate_right {} {})", fmt(*lhs), fmt(*rhs))
        }
        Intrinsic::BvLt { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvslt {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvult {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvLe { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvsle {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvule {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvGt { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvsgt {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvugt {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvGe { t, lhs, rhs } => match t {
            Sort::I32 | Sort::I64 => format!("(bvsge {} {})", fmt(*lhs), fmt(*rhs)),
            _ => format!("(bvuge {} {})", fmt(*lhs), fmt(*rhs)),
        },
        Intrinsic::BvToInt { t, val } => match t {
            Sort::I32 => {
                let mut lets = LetScope::new();
                let v = lets.share("v", fmt(*val));
                lets.wrap(format!(
                    "(ite (bvslt {v} (_ bv0 32)) (- (bv2int {v}) 4294967296) (bv2int {v}))"
                ))
            }
            Sort::I64 => {
                let mut lets = LetScope::new();
                let v = lets.share("v", fmt(*val));
                lets.wrap(format!(
                    "(ite (bvslt {v} (_ bv0 64)) (- (bv2int {v}) 18446744073709551616) (bv2int {v}))"
                ))
            }
            _ => format!("(bv2int {})", fmt(*val)),
        },
        Intrinsic::FloatVal { t, val } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!(
                "((_ to_fp {} {}) RNE (/ {} {}))",
                eb,
                sb,
                val.numer(),
                val.denom()
            )
        }
        Intrinsic::FloatAdd { lhs, rhs, .. } => format!("(fp.add RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatSub { lhs, rhs, .. } => format!("(fp.sub RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatMul { lhs, rhs, .. } => format!("(fp.mul RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatDiv { lhs, rhs, .. } => format!("(fp.div RNE {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatNeg { val, .. } => format!("(fp.neg {})", fmt(*val)),
        Intrinsic::FloatAbs { val, .. } => format!("(fp.abs {})", fmt(*val)),
        Intrinsic::FloatRem { lhs, rhs, .. } => format!("(fp.rem {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatSqrt { val, .. } => format!("(fp.sqrt RNE {})", fmt(*val)),
        Intrinsic::FloatMin { lhs, rhs, .. } => format!("(fp.min {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatMax { lhs, rhs, .. } => format!("(fp.max {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatLt { lhs, rhs, .. } => format!("(fp.lt {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatLe { lhs, rhs, .. } => format!("(fp.leq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatGt { lhs, rhs, .. } => format!("(fp.gt {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatGe { lhs, rhs, .. } => format!("(fp.geq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatIsNaN { val, .. } => format!("(fp.isNaN {})", fmt(*val)),
        Intrinsic::FloatIsInf { val, .. } => format!("(fp.isInfinite {})", fmt(*val)),
        Intrinsic::FloatIsZero { val, .. } => format!("(fp.isZero {})", fmt(*val)),
        Intrinsic::FloatIsNormal { val, .. } => format!("(fp.isNormal {})", fmt(*val)),
        Intrinsic::FloatIsSubnormal { val, .. } => format!("(fp.isSubnormal {})", fmt(*val)),
        Intrinsic::FloatIsNeg { val, .. } => format!("(fp.isNegative {})", fmt(*val)),
        Intrinsic::FloatIsPos { val, .. } => format!("(fp.isPositive {})", fmt(*val)),
        Intrinsic::FloatNaN { t } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!("(_ NaN {eb} {sb})")
        }
        Intrinsic::FloatPosInf { t } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!("(_ +oo {eb} {sb})")
        }
        Intrinsic::FloatNegInf { t } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!("(_ -oo {eb} {sb})")
        }
        Intrinsic::FloatPosZero { t } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!("(_ +zero {eb} {sb})")
        }
        Intrinsic::FloatNegZero { t } => {
            let (eb, sb) = match t {
                Sort::F32 => (8, 24),
                _ => (11, 53),
            };
            format!("(_ -zero {eb} {sb})")
        }
        Intrinsic::FloatCeil { val, .. } => format!("(fp.roundToIntegral RTP {})", fmt(*val)),
        Intrinsic::FloatFloor { val, .. } => format!("(fp.roundToIntegral RTN {})", fmt(*val)),
        Intrinsic::FloatTrunc { val, .. } => format!("(fp.roundToIntegral RTZ {})", fmt(*val)),
        Intrinsic::FloatNearest { val, .. } => format!("(fp.roundToIntegral RNE {})", fmt(*val)),
        Intrinsic::FloatFqEq { lhs, rhs, .. } => format!("(fp.eq {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::FloatToInt { val, .. } => format!("(to_int (fp.to_real {}))", fmt(*val)),
        Intrinsic::FloatToReal { val, .. } => format!("(fp.to_real {})", fmt(*val)),
        Intrinsic::FloatToU32 { val, .. } => format!("((_ fp.to_ubv 32) RTZ {})", fmt(*val)),
        Intrinsic::FloatToI32 { val, .. } => format!("((_ fp.to_sbv 32) RTZ {})", fmt(*val)),
        Intrinsic::FloatToU64 { val, .. } => format!("((_ fp.to_ubv 64) RTZ {})", fmt(*val)),
        Intrinsic::FloatToI64 { val, .. } => format!("((_ fp.to_sbv 64) RTZ {})", fmt(*val)),
        Intrinsic::SeqEmpty { t } => format!("(as seq.empty (Seq {}))", format_sort_for_fn(t, ir)),
        Intrinsic::SeqUnit { val, .. } => format!("(seq.unit {})", fmt(*val)),
        Intrinsic::SeqLen { seq, .. } => format!("(seq.len {})", fmt(*seq)),
        Intrinsic::SeqPush { seq, item, .. } => {
            format!("(seq.++ {} (seq.unit {}))", fmt(*seq), fmt(*item))
        }
        Intrinsic::SeqConcat { lhs, rhs, .. } => format!("(seq.++ {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SeqNth { seq, idx, .. } => format!("(seq.nth {} {})", fmt(*seq), fmt(*idx)),
        Intrinsic::SeqAtSeq { seq, idx, .. } => {
            format!("(seq.extract {} {} 1)", fmt(*seq), fmt(*idx))
        }
        Intrinsic::SeqContains { seq, item, .. } => {
            format!("(seq.contains {} (seq.unit {}))", fmt(*seq), fmt(*item))
        }
        Intrinsic::SeqPrefixOf { lhs, rhs, .. } => {
            format!("(seq.prefixof {} {})", fmt(*lhs), fmt(*rhs))
        }
        Intrinsic::SeqSuffixOf { lhs, rhs, .. } => {
            format!("(seq.suffixof {} {})", fmt(*lhs), fmt(*rhs))
        }
        Intrinsic::SeqIsEmpty { seq, .. } => format!("(= (seq.len {}) 0)", fmt(*seq)),
        Intrinsic::SeqExtract {
            seq, offset, len, ..
        } => format!("(seq.extract {} {} {})", fmt(*seq), fmt(*offset), fmt(*len)),
        Intrinsic::SeqIndexOf {
            seq, sub, offset, ..
        } => format!("(seq.indexof {} {} {})", fmt(*seq), fmt(*sub), fmt(*offset)),
        Intrinsic::SeqIndexOfDefault { seq, sub, .. } => {
            format!("(seq.indexof {} {})", fmt(*seq), fmt(*sub))
        }
        Intrinsic::SeqReplace { seq, src, dst, .. } => {
            format!(
                "(seq.replace {} (seq.unit {}) (seq.unit {}))",
                fmt(*seq),
                fmt(*src),
                fmt(*dst)
            )
        }
        Intrinsic::SetEmpty { t } => {
            let ts = format_sort_for_fn(t, ir);
            format!("(mk-rset ((as const (Array {ts} Bool)) false) 0)")
        }
        Intrinsic::SetLen { set, .. } => format!("(rset-card {})", fmt(*set)),
        Intrinsic::SetInsert { set, item, .. } => {
            let mut lets = LetScope::new();
            let s = lets.share("s", fmt(*set));
            let x = lets.share("x", fmt(*item));
            lets.wrap(format!(
                "(mk-rset (store (rset-data {s}) {x} true) (ite (select (rset-data {s}) {x}) (rset-card {s}) (+ (rset-card {s}) 1)))"
            ))
        }
        Intrinsic::SetRemove { set, item, .. } => {
            let mut lets = LetScope::new();
            let s = lets.share("s", fmt(*set));
            let x = lets.share("x", fmt(*item));
            lets.wrap(format!(
                "(mk-rset (store (rset-data {s}) {x} false) (ite (select (rset-data {s}) {x}) (- (rset-card {s}) 1) (rset-card {s})))"
            ))
        }
        Intrinsic::SetContains { set, item, .. } => {
            format!("(select (rset-data {}) {})", fmt(*set), fmt(*item))
        }
        Intrinsic::SetIsEmpty { set, .. } => format!("(= (rset-card {}) 0)", fmt(*set)),
        Intrinsic::SetIsSubset { lhs, rhs, .. } => {
            // `rhs` is mentioned once, so only `lhs` is worth binding.
            let mut lets = LetScope::new();
            let l = lets.share("l", fmt(*lhs));
            let r = fmt(*rhs);
            lets.wrap(format!(
                "(= ((_ map and) (rset-data {l}) (rset-data {r})) (rset-data {l}))"
            ))
        }
        Intrinsic::SetIsProperSubset { lhs, rhs, .. } => {
            let mut lets = LetScope::new();
            let l = lets.share("l", fmt(*lhs));
            let r = lets.share("r", fmt(*rhs));
            lets.wrap(format!(
                "(and (= ((_ map and) (rset-data {l}) (rset-data {r})) (rset-data {l})) (< (rset-card {l}) (rset-card {r})))"
            ))
        }
        Intrinsic::SetIsDisjoint { t, lhs, rhs } => {
            let ts = format_sort_for_fn(t, ir);
            let l = fmt(*lhs);
            let r = fmt(*rhs);
            format!(
                "(= ((_ map and) (rset-data {l}) (rset-data {r})) ((as const (Array {ts} Bool)) false))"
            )
        }
        Intrinsic::SetHasSize { set, size, .. } => {
            format!("(= (rset-card {}) {})", fmt(*set), fmt(*size))
        }
        Intrinsic::ArrayEmpty { k, v } => {
            let (ks, vs) = (format_sort_for_fn(k, ir), format_sort_for_fn(v, ir));
            let null = array_null_value(v, ir);
            format!(
                "(mk-rarr ((as const (Array {ks} {vs})) {null}) ((as const (Array {ks} Bool)) false) 0)"
            )
        }
        Intrinsic::ArrayStore { arr, key, val, .. } => {
            let mut lets = LetScope::new();
            let a = lets.share("a", fmt(*arr));
            let k = lets.share("k", fmt(*key));
            let vl = fmt(*val);
            lets.wrap(format!(
                "(mk-rarr (store (rarr-data {a}) {k} {vl}) (store (rarr-pres {a}) {k} true) (ite (select (rarr-pres {a}) {k}) (rarr-card {a}) (+ (rarr-card {a}) 1)))"
            ))
        }
        Intrinsic::ArrayRemove { arr, key, v, .. } => {
            let mut lets = LetScope::new();
            let a = lets.share("a", fmt(*arr));
            let k = lets.share("k", fmt(*key));
            let null = array_null_value(v, ir);
            lets.wrap(format!(
                "(mk-rarr (store (rarr-data {a}) {k} {null}) (store (rarr-pres {a}) {k} false) (ite (select (rarr-pres {a}) {k}) (- (rarr-card {a}) 1) (rarr-card {a})))"
            ))
        }
        Intrinsic::ArraySelect { arr, key, .. } => {
            format!("(select (rarr-data {}) {})", fmt(*arr), fmt(*key))
        }
        Intrinsic::ArrayContainsKey { arr, key, .. } => {
            format!("(select (rarr-pres {}) {})", fmt(*arr), fmt(*key))
        }
        Intrinsic::ArrayLen { arr, .. } => {
            format!("(rarr-card {})", fmt(*arr))
        }
        Intrinsic::ArrayIsEmpty { arr, .. } => {
            format!("(= (rarr-card {}) 0)", fmt(*arr))
        }
        // A `Path` is a set of marker ids, encoded as a bit-set with one bit per
        // named marker (see `path.rs`). A named marker is the one-hot literal
        // for its bit.
        Intrinsic::PathName { name } => {
            crate::backend::z3::path::marker_literal(ir, rusmt_smt_stdlib::path::marker_id(name))
        }
        // `Path::merge` is set union in the concrete semantics (graceful, non-
        // short-circuiting error accumulation), and `bvor` is exactly that on
        // the bit-set — both operands survive. Nothing here is approximate, so a
        // multi-marker target ("reach all of these on one run") is a satisfiable
        // query about the program rather than an artefact of the encoding.
        Intrinsic::PathMerge { lhs, rhs, .. } => {
            crate::backend::z3::path::merge_expr(&fmt(*lhs), &fmt(*rhs))
        }
        Intrinsic::SmtEq { lhs, rhs, .. } => format!("(= {} {})", fmt(*lhs), fmt(*rhs)),
        Intrinsic::SmtNe { lhs, rhs, .. } => format!("(not (= {} {}))", fmt(*lhs), fmt(*rhs)),
    }
}

/// Highest code point in Z3's string alphabet; `\u{..}` escapes above this are
/// not recognised and would be emitted as literal text.
/// So writing \u{30000} doesn't produce that character — Z3 doesn't recognise the escape and leaves the nine characters \u{30000} sitting there as plain text.
const Z3_MAX_CODE_POINT: u32 = 0x2FFFF;

/// SMT-LIB string literal
fn format_str_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            ' '..='~' if c != '"' && c != '\\' => out.push(c),
            _ => {
                let cp = c as u32;
                assert!(
                    cp <= Z3_MAX_CODE_POINT,
                    "string literal holds U+{cp:04X}, above Z3's alphabet ceiling U+{Z3_MAX_CODE_POINT:04X}"
                );
                out.push_str(&format!("\\u{{{cp:x}}}"));
            }
        }
    }
    out.push('"');
    out
}

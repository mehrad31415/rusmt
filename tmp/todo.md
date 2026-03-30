- Ongoing work:
    - testing parser, IR, and backend (writing unit and integration tests).
    - Some todos are in the codebase and should be removed.

- Topics to be discussed:
    - Maintain the AST (Abstract Syntax Tree) in memory and utilize the Z3 API directly for SMT generation, bypassing the SMT-LIB output format.
    - CyclicUnification is the only type inference error that we have in infer.rs of parser. Type Mismatch, Ambiguous Types, Unresolved Generics, are all panicked. Why not make them errors? In general the choice of bailing, panicking, or returning an error is not clear in the codebase. We should have a clear distinction between these three and use them accordingly.
    - When should a crate be a member of the workspace in the Cargo.toml and when should it only be a dependency?
    - Look at the types that are defined in smt and their respective functions to add the missing ones for expressivity.
    - fn lookup_unqualified(&self, name: &UsrFuncName) -> Option<&TypeFn> in expr.rs of the parser is used to look up the function name in the function database. An impl function can be called inside an impl function. A spec function can be called inside a spec function. An impl function can be called inside a spec function. A spec function CANNOT be called inside an impl function and an error will be thrown.
    - Expand on the number of expressions that can be handled by the parser.
    - The checks we do are sound but are they complete? In general to formally verify a DSL, the software needs to be formally verified itself. Why rust and why not Coq?
    - Why are iterated quantifiers allowed in non-spec but non-iterated are only allowed in spec?
    - Look into the z3 profiler to see where the most computation power is being spent.
    - Look into mbqi to see if you can improve the performance.
    - Get rid of the assert forall for the axioms and convert the axioms to functions in the smt, also get rid of the exists and define them as asserts with declare-const (see if this is better).
    - Right now the current translation to smt is that the spec and the impl are equal but it should be that the spec => impl (implication). Rethink the smt_spec and whether it is need or can we just have axioms and impls.
    - For compound types, if we do not have concrete declarations, we get the incomplete type error; fix this.
    - Add relations to axioms in ctxt.rs of the parser (self annotated axioms).
    - Is the translation for the forall good? is it the case that the forall is not usable in rust? is the default only for pleasing the compiler?
    - Do not continue writing the book, until the design choices are finalized.
    - For simplicity, require type generics be the first set of type parameters; what does this mean and what to do?
    - Look at the smt outputs of the rusmart test files.
    - Write an interpreter for the rego, while, and ebpf language.
    - What does the eq/lt/gt mean in the smt for strings? Is it the same as the rust eq/lt/gt?
    - use tactics and heuristics to improve the performance of the z3 solver.
    - forall erases the environment! also the typing has an error!
    - change the structure of the project with the cvc5, z3 build .... in the deps
    - what happens if the impl and spec do not have the same generics!
    - test the generics more!
    - prg56 up error!
    - Sort::Seq(_) => {} error in expression.rs backend! (pick an element from the seq using the index)

- Remarks:
    - "AE" is in between "A" and "C" in the z3 str encoding.
    - The lengths are not defined in a good way!
    - create a docker image for the project.
    - write documentation for the z3 api rust.
    - write workflows yml for the project.
    - check whether monomorphization happens in the parser or in the backend.


1 - being intrinsic for a TypeParamName in name.rs is not checked for types in the context? it can be checked here 
    let param_name = ident.try_into()?;
    if generics.params.contains(&param_name) {
    Self::Param(param_name) in ty.rs and in generics.rs in the validate_type_param_decl method. The latter is for checking the intrinsic of top level types in the context. and the former is for checking the intrinsic of embedded types in the context.
2 -  db.builtin("contains_key", Q::Map, fn2(map_kv(), k(), Boolean)); // `contains_key` is a function that checks if a map contains a key. It is a binary function, with the signature TypeFn { kind: Kind::Impl, generics: Generics { params: [], }, params: [Map(Box(Parameter(TypeParamName { ident: String::from("K") }), Box(Parameter(TypeParamName { ident: String::from("V") })), Parameter(TypeParamName { ident: String::from("K") })], ret_ty: Boolean, } so the parameters are a map of type K to type V, a key of type K and the return type is a Boolean.
shoudn't the generics be not empty in the above example? in apply.rs
3 - in apply.rs         let fn0 = |rty: TypeTag| TypeFn {
            kind: Kind::Impl,
            generics: empty(),
            params: vec![],
            ret_ty: rty,
        };
depending on the TypeTag the generics should be populated? in the above example, the generics are empty only...
Maybe for system you use the intrinsic         let t = || Parameter(TypeParamName::intrinsic("T")); // gives a Parameter(TypeParamName { ident: String::from("T") }) when called. That is why the generics are empty? so intrinsic types don't count as generics?
4 - Theoretically this is wrong: db.builtin("new", Q::Seq, fn0(seq_t()));  and then having generics empty in apply.rs


Make all of the function return ok than panic!
checkout functional calls and vars in string comparison



are they? the transpilers are still limiting factors to the adoption of formal verification. 

*Related work: ZEN work, K framework, Conformance Testing of Formal Semantics Using Grammar-Based Fuzzing (TAP 2022)*


do the clappy rego part for the lang!

check tarpaulin

cpu cores from initialize







-------------------------------
1 - finish off writing the book 
2 - write new brief report and rusmart programs for documents
3 - change the panic for panic!("cannot convert error sort to Z3 API"), Error !



add linter for equality float in the remark ...

you need to explicitly write the conversion between the types... 

add unsigned integer methods

we only have double tuples ... 

review the unit test cases ...


recheck the test suites

limitations f16 is unstable cannot replicate full functionality of floating z3 c api

The Golden Rule of Annotation: You only need to write a "mark" when you need to tell the transpiler something that isn't already obvious from the Rust code and types alone.


// DCE (dead code elimination).
// linting
// error patterns (based on data)
// K-framework for operation semantics correctness (maybe some test suites)
// common sense properties


maybe give ML LLM Consider this scenario: "Find me a TOML configuration file where the database is enabled, but one of its port numbers is a privileged port (less than 1024)."



there is no way to do rounding modes in f32 and f64 in rust standard library! because they use the hardware floating point operations directly! so we need to implement them ourselves!


❌ Cannot model interpreters with non-RNE rounding

❌ Loses expressiveness


you cannot chain function methods...


Note that in the standard library any of the methods can panic like division by zero or to_u32 conversion failure! If we want to model these
errors then we do that on the interpreter side keeping the standard library as is!


    /// to_bitvec() converts the int to a bitvector of size N if the value fits in N bits.
    pub fn to_bitvec<const N: usize>(self) -> I32 {
        assert!(
            N > 0 && N <= 128,
            "BitVector width must be between 1 and 128"
        );

        // convert the BigInt to i128 type.
        if let Some(val_i128) = self.inner.to_i128() {
            let min_val = if N == 128 {
                i128::MIN
            } else {
                -(1i128 << (N - 1))
            };
            let max_val = if N == 128 {
                i128::MAX
            } else {
                (1i128 << (N - 1)) - 1
            };

            if val_i128 >= min_val && val_i128 <= max_val {
                SMTOption::Some(SymbolicBitVec {
                    inner: val_i128,
                    _phantom: PhantomData,
                })
            } else {
                SMTOption::None
            }
        } else {
            // BigInt is too large to even fit in an i128.
            SMTOption::None
        }
    }

 /// Try to convert to f32
    ///
    /// If the integer is too large to fit in f32, return None
    /// Integers with a magnitude greater than 2^24may lose precision due to rounding.
    pub fn to_f32(self) -> SMTOption<F32> {
        let bigint = self.inner.as_ref();
        let val_f32 = bigint.to_f32();
        let val_f32 = match val_f32 {
            None => return SMTOption::None,
            Some(v) => v,
        };

        let rat = BigRational::from_float(val_f32);
        let rat = match rat {
            None => return SMTOption::None,
            Some(v) => v,
        };
        if rat.is_integer() && rat.to_integer() == *bigint {
            SMTOption::Some(F32::from(val_f32))
        } else {
            SMTOption::None
        }
    }

    /// Try to convert to f64
    ///
    /// If the integer is too large to fit in f64, return None
    /// Integers with a magnitude greater than 2^53 may lose precision due to rounding.
    pub fn to_f64(self) -> SMTOption<F64> {
        let bigint = self.inner.as_ref();
        let val_f64 = bigint.to_f64();
        let val_f64 = match val_f64 {
            None => return SMTOption::None,
            Some(v) => v,
        };

        let rat = BigRational::from_float(val_f64);
        let rat = match rat {
            None => return SMTOption::None,
            Some(v) => v,
        };
        if rat.is_integer() && rat.to_integer() == *bigint {
            SMTOption::Some(F64::from(val_f64))
        } else {
            SMTOption::None
        }
    }

    /// Creates an `Integer` from a hexadecimal string.
    /// The string should not include the "0x" prefix nor any underscores.
    /// Returns `None` if the string contains invalid hex characters.
    pub fn from_hex_str(s: String) -> SMTOption<Self> {
        let without_underscores = s.replace("_".into(), "".into());
        match BigInt::from_str_radix(without_underscores.inner.as_ref(), 16) {
            Ok(val) => SMTOption::Some(Self {
                inner: Intern::new(val),
            }),
            Err(_) => SMTOption::None,
        }
    }

    /// Creates an `Integer` from an octal string.
    /// The string should not include the "0o" prefix.
    pub fn from_oct_str(s: String) -> SMTOption<Self> {
        let without_underscores = s.replace("_".into(), "".into());
        match BigInt::from_str_radix(without_underscores.inner.as_ref(), 8) {
            Ok(val) => SMTOption::Some(Self {
                inner: Intern::new(val),
            }),
            Err(_) => SMTOption::None,
        }
    }

    /// Creates an `Integer` from a binary string.
    /// The string should not include the "0b" prefix.
    pub fn from_bin_str(s: String) -> SMTOption<Self> {
        let without_underscores = s.replace("_".into(), "".into());
        match BigInt::from_str_radix(without_underscores.inner.as_ref(), 2) {
            Ok(val) => SMTOption::Some(Self {
                inner: Intern::new(val),
            }),
            Err(_) => SMTOption::None,
        }
    }

    for real:
//! Real datatype and its operations

use crate::{Boolean, F32, F64, I32, I64, Integer, Real};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::Zero;
use num_traits::cast::ToPrimitive;

/// Real operations
impl Real {
    /// addition
    pub fn add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() + rhs.inner.as_ref()),
        }
    }

    /// multiplication
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() * rhs.inner.as_ref()),
        }
    }

    /// subtraction
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() - rhs.inner.as_ref()),
        }
    }

    /// negation
    pub fn neg(self) -> Self {
        Self {
            inner: Intern::new(-self.inner.as_ref()),
        }
    }

    /// division
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// exponentiation
    pub fn pow(self, exp: Self) -> SMTOption<Self> {
        // Check if exp is an integer
        if !exp.inner.is_integer() {
            return SMTOption::None;
        }
        // Convert exp to i32
        if let Some(e) = exp.inner.to_integer().to_i32() {
            SMTOption::Some(Self {
                inner: Intern::new(self.inner.as_ref().pow(e)),
            })
        } else {
            SMTOption::None
        }
    }

    /// Returns the absolute value of the real number.
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Rounds the real number to the nearest integer.
    pub fn round(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().round().to_integer()),
        }
    }

    /// Floors the real number to the nearest integer less than or equal to the number.
    pub fn floor(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().floor().to_integer()),
        }
    }

    /// Ceils the real number to the nearest integer greater than or equal to the number.
    pub fn ceil(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().ceil().to_integer()),
        }
    }

    /// is integer
    pub fn is_integer(self) -> Boolean {
        self.inner.is_integer().into()
    }
}

/// comparison operations for Real
impl Real {
    /// less than
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() < rhs.inner.as_ref()).into()
    }

    /// less than or equal
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() <= rhs.inner.as_ref()).into()
    }

    /// greater than
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() > rhs.inner.as_ref()).into()
    }

    /// greater than or equal
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() >= rhs.inner.as_ref()).into()
    }
}

/// conversion operations for Real
impl Real {
    /// Lossless & Fallible: Converts a Real to an Integer
    pub fn to_int(self) -> SMTOption<Integer> {
        if !*self.is_integer() {
            return SMTOption::None;
        }

        SMTOption::Some(Integer {
            inner: Intern::new(self.inner.to_integer()),
        })
    }

    /// Try to convert to f32
    ///
    /// If the real is too large to fit in f32, return None
    pub fn to_f32(self) -> SMTOption<F32> {
        let bigrat = self.inner.as_ref();
        let val_f32 = bigrat.to_f32();
        let val_f32 = match val_f32 {
            None => return SMTOption::None,
            Some(v) => v,
        };

        let rat = BigRational::from_float(val_f32);
        let rat = match rat {
            None => return SMTOption::None,
            Some(v) => v,
        };
        if rat == *bigrat {
            SMTOption::Some(F32::from(val_f32))
        } else {
            SMTOption::None
        }
    }

    /// Try to convert to f64
    ///
    /// If the real number is too large to fit in f64, return None
    pub fn to_f64(self) -> SMTOption<F64> {
        let bigrat = self.inner.as_ref();
        let val_f64 = bigrat.to_f64();
        let val_f64 = match val_f64 {
            None => return SMTOption::None,
            Some(v) => v,
        };

        let rat = BigRational::from_float(val_f64);
        let rat = match rat {
            None => return SMTOption::None,
            Some(v) => v,
        };
        if rat == *bigrat {
            SMTOption::Some(F64::from(val_f64))
        } else {
            SMTOption::None
        }
    }

    /// to_bitvec() converts the real to a bitvector of size N if there is no fractional part and the value fits in N bits.
    pub fn to_bitvec<const N: usize>(self) -> SMTOption<SymbolicBitVec<N>> {
        let int_val = self.to_int();
        match int_val {
            SMTOption::Some(iv) => iv.to_bitvec::<N>(),
            SMTOption::None => SMTOption::None,
        }
    }

    /// This allows us to build real numbers from f32
    pub fn try_from_f32(value: f32) -> SMTOption<Self> {
        BigRational::from_float(value)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .map_or_else(|| SMTOption::None, |real_val| SMTOption::Some(real_val))
    }

    /// This allows us to build real numbers from f64
    pub fn try_from_f64(value: f64) -> SMTOption<Self> {
        BigRational::from_float(value)
            .map(|br| Real {
                inner: Intern::new(br),
            })
            .map_or_else(|| SMTOption::None, |real_val| SMTOption::Some(real_val))
    }
}

/// Convert to Real from int literals
/// let a = Real::from(1);
/// let a:Real = 1.into(); // this needs to be annotated
/// let a:Real = From::from(1); // this needs to be annotated
macro_rules! real_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for Real {
            fn from(c: $l) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        }
        $(impl From<$e> for Real {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        })*
    };
}

real_from_literal_int!(i8, i16, i32, i64, i128, isize);
real_from_literal_int!(u8, u16, u32, u64, u128, usize);



 Sequences

    /// `(seq.nth s i)`
    pub fn at(self, i: Integer) -> SMTOption<T> {
        let res = i
            .inner
            .to_usize()
            .and_then(|idx| self.inner.get(idx))
            .map(|wrapped_val| wrapped_val.0);
        match res {
            Some(v) => SMTOption::Some(v),
            None => SMTOption::None,
        }
    }

    /// `(seq.at s i)`
    pub fn at_seq(self, i: Integer) -> SMTOption<Self> {
        if let SMTOption::Some(elem) = self.at(i) {
            SMTOption::Some(Self::unit(elem))
        } else {
            SMTOption::None
        }
    }

    /// `(seq.extract s offset length)`
    pub fn extract(self, offset: Integer, length: Integer) -> SMTOption<Self> {
        let start = offset.inner.to_usize();
        let start = match start {
            None => return SMTOption::None,
            Some(s) => s,
        };
        let len = length.inner.to_usize();
        let len = match len {
            None => return SMTOption::None,
            Some(l) => l,
        };
        let end = start.checked_add(len);
        let end = match end {
            None => return SMTOption::None,
            Some(e) => e,
        };

        if end > self.inner.len() {
            return SMTOption::None;
        }

        let new_vec = self.inner[start..end].to_vec();
        SMTOption::Some(Self {
            inner: Intern::new(new_vec),
        })
    }

    /// `(seq.map f s)`
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(T) -> T,
    {
        let new_vec = self.inner.iter().map(|v| SMTWrap(f(v.0))).collect();
        Self {
            inner: Intern::new(new_vec),
        }
    }

    /// `(seq.contains s (seq.unit e))`
    pub fn contains(self, e: T) -> Boolean {
        self.inner.contains(&SMTWrap(e)).into()
    }

    /// `(seq.prefixof other self)`
    pub fn prefix_of(self, other: Self) -> Boolean {
        other.inner.starts_with(&self.inner).into()
    }

    /// `(seq.suffixof other self)`
    pub fn suffix_of(self, other: Self) -> Boolean {
        other.inner.ends_with(&self.inner).into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<Integer> {
        (0..self.inner.len()).map(Integer::from).collect()
    }

    /// checks if the sequence is empty: `v.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }
}
/// this is a sequence (list) of SMT values of type T where T is a type that implements the SMT trait.
#[macro_export]
/// Example: seq!(Integer::from(1), Integer::from(2));
macro_rules! seq {
    ($($e:expr),*) => {
        {
            let mut seq = Seq::new();
            $(
                seq = seq.append($e);
            )*
            seq
        }
    };
}



set use crate::smt::SMT;
use crate::{Boolean, Integer, Set, dt::SMTWrap};
use internment::Intern;
use std::collections::BTreeSet;

impl<T: SMT> Set<T> {
    /// create an new set: `Set::new()`
    pub fn new() -> Self {
        Self {
            inner: Intern::new(BTreeSet::new()),
        }
    }

    /// return the length of the set: `s.length()`
    pub fn length(self) -> Integer {
        self.inner.len().into()
    }

    /// a non in-place operation to insert an element into the set: `s.insert(e)`
    pub fn insert(self, e: T) -> Self {
        let mut new_set = (*self.inner).clone();
        new_set.insert(SMTWrap(e));
        Self {
            inner: Intern::new(new_set),
        }
    }

    /// a non in-place operation to remove an element from the set: `s.remove(e)`
    pub fn remove(self, e: T) -> Self {
        let mut new_set = (*self.inner).clone();
        new_set.remove(&SMTWrap(e));
        Self {
            inner: Intern::new(new_set),
        }
    }

    /// `v.contains(e)`
    pub fn contains(self, e: T) -> Boolean {
        self.inner.contains(&SMTWrap(e)).into()
    }

    /// iterator
    pub fn iterator(self) -> Vec<T> {
        self.inner.iter().map(|i| i.0).collect()
    }

    /// checks if the set is empty: `s.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// take the intersection of two sets
    pub fn intersection(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.intersection(&other.inner).copied().collect()),
        }
    }

    /// take the union of two sets
    pub fn union(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.union(&other.inner).copied().collect()),
        }
    }

    /// take the difference of two sets (self - other)
    pub fn difference(self, other: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.difference(&other.inner).copied().collect()),
        }
    }

    /// is subset of other (self <= other)
    pub fn is_subset(self, other: Self) -> Boolean {
        self.inner.is_subset(&other.inner).into()
    }

    /// This is a concrete check. Z3's `set.has_size` is a symbolic predicate.
    pub fn has_size(self, k: Integer) -> Boolean {
        self.length().eq(k)
    }

    /// Checks if two sets are disjoint (no common elements)
    pub fn is_disjoint(self, other: Self) -> Boolean {
        self.inner.is_disjoint(&other.inner).into()
    }

    /// Symmetric difference (elements in either but not both)
    pub fn symmetric_difference(self, other: Self) -> Self {
        let diff1 = self.difference(other.clone());
        let diff2 = other.difference(self.clone());
        diff1.union(diff2)
    }

    /// Checks if this is a proper subset (⊂, not ⊆)
    pub fn is_proper_subset(self, other: Self) -> Boolean {
        (self
            .is_subset(other.clone())
            .and(self.length().lt(other.length())))
        .into()
    }

    /// Checks for superset
    pub fn is_superset(self, other: Self) -> Boolean {
        other.is_subset(self).into()
    }
}

#[macro_export]
/// Example: set!(Integer::from(1), Integer::from(2));
macro_rules! set {
    ( $($e:expr),*) => {
        {
            let mut set = Set::new();
            $(
                set = set.insert($e);
            )*
            set
        }
    };
}


float:
use crate::dt::{Boolean, F32, F64, I32, I64, Integer, Real, SMTOption, String, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::FromPrimitive;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

// Common trait for all floating-point types
pub trait SymbolicFloatOps: Sized + SMT {
    /// arithmetic operations
    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    fn div(self, rhs: Self) -> Self;
    fn neg(self) -> Self;
    fn abs(self) -> Self;
    fn rem(self, rhs: Self) -> Self;
    fn sqrt(self) -> Self;
    fn min(self, rhs: Self) -> Self;
    fn max(self, rhs: Self) -> Self;
    /// predicate operations
    fn is_nan(self) -> Boolean;
    fn is_infinite(self) -> Boolean;
    fn is_zero(self) -> Boolean;
    fn is_normal(self) -> Boolean;
    fn is_subnormal(self) -> Boolean;
    fn is_negative(self) -> Boolean;
    fn is_positive(self) -> Boolean;
    /// comparison operations
    fn lt(self, rhs: Self) -> Boolean;
    fn le(self, rhs: Self) -> Boolean;
    fn gt(self, rhs: Self) -> Boolean;
    fn ge(self, rhs: Self) -> Boolean;
    /// constructors
    fn nan() -> Self;
    fn infinity() -> Self;
    fn neg_infinity() -> Self;
    fn pos_zero() -> Self;
    fn neg_zero() -> Self;
    /// conversions
    fn to_integer(self) -> Integer;
    fn to_real(self) -> Real;
    fn to_i32(self) -> I32 {
        // This works by chaining the conversions:
        // 1. Try to convert Float -> Integer (handles NaN/Infinity and truncation).
        // 2. If successful, try to convert Integer -> BitVector (handles overflow).
        let int_val = self.to_integer();
        match int_val {
            SMTOption::Some(int) => int.to_i32(),
            SMTOption::None => SMTOption::None,
        }
    }
    fn to_i64(self) -> SMTOption<I64> {
        let int_val = self.to_integer();
        match int_val {
            SMTOption::Some(int) => int.to_64(),
            SMTOption::None => SMTOption::None,
        }
    }

    fn from_str(s: String) -> SMTOption<Self>;
}

/// Operations for F32.
impl SymbolicFloatOps for F32 {
    /// Creates a Not-a-Number (NaN) value.
    fn nan() -> Self {
        Self { inner: f32::NAN }
    }

    /// Creates a positive infinity value.
    fn infinity() -> Self {
        Self {
            inner: f32::INFINITY,
        }
    }

    /// Creates a negative infinity value.
    fn neg_infinity() -> Self {
        Self {
            inner: f32::NEG_INFINITY,
        }
    }

    /// Creates a positive zero value.
    fn pos_zero() -> Self {
        Self { inner: 0.0f32 }
    }

    /// Creates a negative zero value.
    fn neg_zero() -> Self {
        Self { inner: -0.0f32 }
    }

    // /// addition
    // /// The transpiler should generate the `(fp.add rm t1 t2)` SMT-LIB expression.
    // fn add(self, rm: RoundingMode, rhs: Self) -> Self {
    //     unsafe {
    //         let soft_rm = match rm {
    //             RoundingMode::RNE => 0,
    //             RoundingMode::RTZ => 1,
    //             RoundingMode::RTP => 2,
    //             RoundingMode::RTN => 3,
    //             RoundingMode::RNA => 4,
    //         };

    //         let result = f32_add(self.inner.to_bits(), rhs.inner.to_bits(), soft_rm);

    //         Self {
    //             inner: f32::from_bits(result),
    //         }
    //     }
    // }

    // /// subtraction
    // /// The transpiler should generate `(fp.sub rm t1 t2)`.
    // fn sub(self, rm: RoundingMode, rhs: Self) -> Self {
    //     Self::apply_rounding((self.inner - rhs.inner).into(), rm)
    // }

    // /// multiplication
    // /// The transpiler should generate `(fp.mul rm t1 t2)`.
    // fn mul(self, rm: RoundingMode, rhs: Self) -> Self {
    //     Self::apply_rounding((self.inner * rhs.inner).into(), rm)
    // }

    // /// division
    // /// The transpiler should generate `(fp.div rm t1 t2)`.
    // fn div(self, rm: RoundingMode, rhs: Self) -> Self {
    //     Self::apply_rounding((self.inner / rhs.inner).into(), rm)
    // }

    /// negation
    fn neg(self) -> Self {
        Self { inner: -self.inner }
    }

    /// absolute value
    fn abs(self) -> Self {
        Self {
            inner: self.inner.abs(),
        }
    }

    /// Floating-point remainder. `(fp.rem t1 t2)`
    fn rem(self, rhs: Self) -> Self {
        Self {
            inner: self.inner % rhs.inner,
        }
    }

    /// Floating-point square root. `(fp.sqrt rm t)`
    fn sqrt(self, rm: RoundingMode) -> Self {
        Self::apply_rounding(self.inner.sqrt().into(), rm)
    }

    /// Minimum of floating-point numbers. `(fp.min t1 t2)`
    fn min(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.min(rhs.inner),
        }
    }

    /// Maximum of floating-point numbers. `(fp.max t1 t2)`
    fn max(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.max(rhs.inner),
        }
    }

    /// less than (NaN < anything = false (IEEE 754 semantics))
    fn lt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    /// less than or equal to (NaN <= anything = false (IEEE 754 semantics))
    fn le(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    /// greater than (NaN > anything = false (IEEE 754 semantics))
    fn gt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    /// greater than or equal to (NaN >= anything = false (IEEE 754 semantics))
    fn ge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    /// is NaN `(fp.isNaN X)`
    fn is_nan(self) -> Boolean {
        self.inner.is_nan().into()
    }

    /// is infinite `(fp.isInfinite X)`
    fn is_infinite(self) -> Boolean {
        self.inner.is_infinite().into()
    }

    /// is zero `(fp.isZero X)`
    fn is_zero(self) -> Boolean {
        (self.inner == 0.0f32 || self.inner == -0.0f32).into()
    }

    /// is negative `(fp.isNegative X)`
    fn is_negative(self) -> Boolean {
        self.inner.is_sign_negative().into()
    }

//     /// is positive `(fp.isPositive X)`
//     fn is_positive(self) -> Boolean {
//         self.inner.is_sign_positive().into()
//     }

//     /// is normal `(fp.isNormal t)`
//     fn is_normal(self) -> Boolean {
//         self.inner.is_normal().into()
//     }

//     /// `(fp.isSubnormal t)`
//     fn is_subnormal(self) -> Boolean {
//         self.inner.is_subnormal().into()
//     }

//     /// `Z3_mk_fpa_round_to_integral`
//     fn to_integer(self) -> SMTOption<Integer> {
//         if !self.inner.is_finite() {
//             return SMTOption::None; // Cannot convert NaN or Infinity.
//         }

//         BigInt::from_f32(self.inner.trunc())
//             .map(|bi| Integer {
//                 inner: Intern::new(bi),
//             })
//             .map_or_else(|| SMTOption::None, |int_val| SMTOption::Some(int_val))
//     }

//     /// Converts a float to a Real.
//     fn to_real(self) -> SMTOption<Real> {
//         if !self.inner.is_finite() {
//             return SMTOption::None; // Cannot convert NaN or Infinity.
//         }

//         BigRational::from_float(self.inner)
//             .map(|br| Real {
//                 inner: Intern::new(br),
//             })
//             .map_or_else(|| SMTOption::None, |real_val| SMTOption::Some(real_val))
//     }

//     /// Helper to convert a string representation to a float.
//     fn from_str(s: String) -> SMTOption<Self> {
//         match s.inner.parse::<f32>() {
//             Err(_) => SMTOption::None,
//             Ok(v) => SMTOption::Some(Self { inner: v }),
//         }
//     }

//     /// Rounding helper function
//     fn apply_rounding(val: Self, rm: RoundingMode) -> Self {
//         match rm {
//             // Round Nearest, ties to Even is the default behavior of f64 operations.
//             RoundingMode::RNE => val.inner.round_ties_even().into(),
//             // Round Toward Zero is truncation.
//             RoundingMode::RTZ => val.inner.trunc().into(),
//             // Round Toward Positive Infinity is ceiling.
//             RoundingMode::RTP => val.inner.ceil().into(),
//             // Round Toward Negative Infinity is floor.
//             RoundingMode::RTN => val.inner.floor().into(),
//             // Round to Nearest, ties Away from zero.
//             RoundingMode::RNA => val.inner.round().into(),
//         }
//     }
// }

// /// Constructors for F64.
// impl SymbolicFloatOps for F64 {
//     /// Creates a Not-a-Number (NaN) value.
//     fn nan() -> Self {
//         Self { inner: f64::NAN }
//     }

//     /// Creates a positive infinity value.
//     fn infinity() -> Self {
//         Self {
//             inner: f64::INFINITY,
//         }
//     }

//     /// Creates a negative infinity value.
//     fn neg_infinity() -> Self {
//         Self {
//             inner: f64::NEG_INFINITY,
//         }
//     }

//     /// Creates a positive zero value.
//     fn pos_zero() -> Self {
//         Self { inner: 0.0f64 }
//     }

//     /// Creates a negative zero value.
//     fn neg_zero() -> Self {
//         Self { inner: -0.0f64 }
//     }

//     /// addition
//     /// The transpiler should generate the `(fp.add rm t1 t2)` SMT-LIB expression.
//     fn add(self, rm: RoundingMode, rhs: Self) -> Self {
//         Self::apply_rounding((self.inner + rhs.inner).into(), rm)
//     }

//     /// subtraction
//     /// The transpiler should generate `(fp.sub rm t1 t2)`.
//     fn sub(self, rm: RoundingMode, rhs: Self) -> Self {
//         Self::apply_rounding((self.inner - rhs.inner).into(), rm)
//     }

//     /// multiplication
//     /// The transpiler should generate `(fp.mul rm t1 t2)`.
//     fn mul(self, rm: RoundingMode, rhs: Self) -> Self {
//         Self::apply_rounding((self.inner * rhs.inner).into(), rm)
//     }

//     /// division
//     /// The transpiler should generate `(fp.div rm t1 t2)`.
//     fn div(self, rm: RoundingMode, rhs: Self) -> Self {
//         Self::apply_rounding((self.inner / rhs.inner).into(), rm)
//     }

//     /// negation
//     fn neg(self) -> Self {
//         Self { inner: -self.inner }
//     }

//     /// absolute value
//     fn abs(self) -> Self {
//         Self {
//             inner: self.inner.abs(),
//         }
//     }

//     /// Floating-point remainder. `(fp.rem t1 t2)`
//     fn rem(self, rhs: Self) -> Self {
//         Self {
//             inner: self.inner % rhs.inner,
//         }
//     }

//     /// Floating-point square root. `(fp.sqrt rm t)`
//     fn sqrt(self, rm: RoundingMode) -> Self {
//         Self::apply_rounding(self.inner.sqrt().into(), rm)
//     }

//     /// Minimum of floating-point numbers. `(fp.min t1 t2)`
//     fn min(self, rhs: Self) -> Self {
//         Self {
//             inner: self.inner.min(rhs.inner),
//         }
//     }

//     /// Maximum of floating-point numbers. `(fp.max t1 t2)`
//     fn max(self, rhs: Self) -> Self {
//         Self {
//             inner: self.inner.max(rhs.inner),
//         }
//     }

//     /// less than (NaN < anything = false (IEEE 754 semantics))
//     fn lt(self, rhs: Self) -> Boolean {
//         (self.inner < rhs.inner).into()
//     }

//     /// less than or equal to (NaN <= anything = false (IEEE 754 semantics))
//     fn le(self, rhs: Self) -> Boolean {
//         (self.inner <= rhs.inner).into()
//     }

//     /// greater than (NaN > anything = false (IEEE 754 semantics))
//     fn gt(self, rhs: Self) -> Boolean {
//         (self.inner > rhs.inner).into()
//     }

//     /// greater than or equal to (NaN >= anything = false (IEEE 754 semantics))
//     fn ge(self, rhs: Self) -> Boolean {
//         (self.inner >= rhs.inner).into()
//     }

//     /// is NaN `(fp.isNaN X)`
//     fn is_nan(self) -> Boolean {
//         self.inner.is_nan().into()
//     }

//     /// is infinite `(fp.isInfinite X)`
//     fn is_infinite(self) -> Boolean {
//         self.inner.is_infinite().into()
//     }

//     /// is zero `(fp.isZero X)`
//     fn is_zero(self) -> Boolean {
//         (self.inner == 0.0f64 || self.inner == -0.0f64).into()
//     }

//     /// is negative `(fp.isNegative X)`
//     fn is_negative(self) -> Boolean {
//         self.inner.is_sign_negative().into()
//     }

//     /// is positive `(fp.isPositive X)`
//     fn is_positive(self) -> Boolean {
//         self.inner.is_sign_positive().into()
//     }

//     /// is normal `(fp.isNormal t)`
//     fn is_normal(self) -> Boolean {
//         self.inner.is_normal().into()
//     }

//     /// `(fp.isSubnormal t)`
//     fn is_subnormal(self) -> Boolean {
//         self.inner.is_subnormal().into()
//     }

//     fn to_integer(self) -> SMTOption<Integer> {
//         if !self.inner.is_finite() {
//             return SMTOption::None; // Cannot convert NaN or Infinity.
//         }

//         BigInt::from_f64(self.inner.trunc())
//             .map(|bi| Integer {
//                 inner: Intern::new(bi),
//             })
//             .map_or_else(|| SMTOption::None, |int_val| SMTOption::Some(int_val))
//     }

//     fn to_real(self) -> SMTOption<Real> {
//         if !self.inner.is_finite() {
//             return SMTOption::None; // Cannot convert NaN or Infinity.
//         }

//         BigRational::from_float(self.inner)
//             .map(|br| Real {
//                 inner: Intern::new(br),
//             })
//             .map_or_else(|| SMTOption::None, |real_val| SMTOption::Some(real_val))
//     }

//     fn from_str(s: String) -> SMTOption<Self> {
//         match s.inner.parse::<f64>() {
//             Err(_) => SMTOption::None,
//             Ok(v) => SMTOption::Some(Self { inner: v }),
//         }
//     }

//     fn apply_rounding(val: Self, rm: RoundingMode) -> Self {
//         match rm {
//             // Round Nearest, ties to Even is the default behavior of f64 operations.
//             RoundingMode::RNE => val.inner.round_ties_even().into(),
//             // Round Toward Zero is truncation.
//             RoundingMode::RTZ => val.inner.trunc().into(),
//             // Round Toward Positive Infinity is ceiling.
//             RoundingMode::RTP => val.inner.ceil().into(),
//             // Round Toward Negative Infinity is floor.
//             RoundingMode::RTN => val.inner.floor().into(),
//             // Round to Nearest, ties Away from zero.
//             RoundingMode::RNA => val.inner.round().into(),
//         }
//     }
}

impl SMT for F32 {
    fn _cmp(self, rhs: Self) -> Ordering {
        self.inner.total_cmp(&rhs.inner)
    }
}

impl SMT for F64 {
    fn _cmp(self, rhs: Self) -> Ordering {
        self.inner.total_cmp(&rhs.inner)
    }
}

impl From<f32> for F32 {
    /// from_f32() creates a F32 from a f32.
    fn from(f: f32) -> Self {
        Self { inner: f }
    }
}

impl From<f64> for F64 {
    /// from_f64() creates a F64 from a f64.
    fn from(f: f64) -> Self {
        Self { inner: f }
    }
}

macro_rules! f32_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for F32 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as f32
                }
            }
        }
        $(impl From<$e> for F32 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as f32
                }
            }
        })*
    };
}

f32_from_literal_int!(i8, i16, i32, i64, i128, isize);
f32_from_literal_int!(u8, u16, u32, u64, u128, usize);

macro_rules! f64_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for F64 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as f64,
                }
            }
        }
        $(impl From<$e> for F64 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as f64,
                }
            }
        })*
    };
}

f64_from_literal_int!(i8, i16, i32, i64, i128, isize);
f64_from_literal_int!(u8, u16, u32, u64, u128, usize);



use crate::{Boolean, Integer, Real, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use std::cmp::Ordering;
use std::marker::PhantomData;

/// Bitwise Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// Create a new bit-vector of size N from an i128 value.
    pub fn new(value: i128) -> SMTOption<Self> {
        if N < 128 {
            let max_val = (1i128 << (N - 1)) - 1;
            let min_val = -(1i128 << (N - 1));

            if value >= min_val && value <= max_val {
                SMTOption::Some(Self {
                    inner: value,
                    _phantom: PhantomData,
                })
            } else {
                SMTOption::None
            }
        } else {
            SMTOption::Some(Self {
                inner: value,
                _phantom: PhantomData,
            })
        }
    }

    /// `(bvnot a)`
    pub fn bv_not(self) -> Self {
        Self {
            inner: !self.inner,
            ..self
        }
    }

    /// `(bvredand a)`
    pub fn bv_redand(self) -> Boolean {
        let mask = if N == 128 { -1i128 } else { (1i128 << N) - 1 };
        let canonical: Boolean = ((self.inner & !mask) == 0).into();
        let all_ones: Boolean = ((self.inner & mask) == mask).into();
        all_ones.and(canonical)
    }

    /// `(bvredor a)`
    pub fn bv_redor(self) -> Boolean {
        let mask = if N == 128 { -1i128 } else { (1i128 << N) - 1 };
        let canonical: Boolean = ((self.inner & !mask) == 0).into();
        let any_one: Boolean = ((self.inner & mask) != 0).into();
        any_one.and(canonical)
    }

    /// `(bvand a b)`
    pub fn bv_and(self, rhs: Self) -> Self {
        Self {
            inner: self.inner & rhs.inner,
            _phantom: PhantomData,
        }
    }

    /// `(bvor a b)`
    pub fn bv_or(self, rhs: Self) -> Self {
        Self {
            inner: self.inner | rhs.inner,
            _phantom: PhantomData,
        }
    }

    /// `(bvxor a b)`
    pub fn bv_xor(self, rhs: Self) -> Self {
        Self {
            inner: self.inner ^ rhs.inner,
            _phantom: PhantomData,
        }
    }

    /// `(bvnand a b)`
    pub fn bv_nand(self, rhs: Self) -> Self {
        self.bv_and(rhs).bv_not()
    }

    /// `(bvnor a b)`
    pub fn bv_nor(self, rhs: Self) -> Self {
        self.bv_or(rhs).bv_not()
    }

    /// `(bvxnor a b)`
    pub fn bv_xnor(self, rhs: Self) -> Self {
        self.bv_xor(rhs).bv_not()
    }
}

/// Arithmetic Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvneg a)`
    pub fn bv_neg(self) -> Self {
        Self {
            inner: self.inner.wrapping_neg(),
            ..self
        }
    }

    /// `(bvadd a b)`
    pub fn bv_add(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_add(rhs.inner),
            ..self
        }
    }

    /// `(bvsub a b)`
    pub fn bv_sub(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_sub(rhs.inner),
            ..self
        }
    }

    /// `(bvmul a b)`
    pub fn bv_mul(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_mul(rhs.inner),
            ..self
        }
    }

    /// `(bvsdiv a b)`
    pub fn bv_sdiv(self, rhs: Self) -> SMTOption<Self> {
        let res = self.inner.checked_div(rhs.inner);
        match res {
            Some(v) => SMTOption::Some(Self {
                inner: v,
                _phantom: PhantomData,
            }),
            None => SMTOption::None,
        }
    }

    /// `(bvudiv a b)`
    pub fn bv_udiv(self, rhs: Self) -> SMTOption<Self> {
        let lhs_unsigned = if N == 128 {
            self.inner as u128 // Direct cast for full 128 bits
        } else {
            let mask = (1i128 << N) - 1;
            (self.inner & mask) as u128
        };

        let rhs_unsigned = if N == 128 {
            rhs.inner as u128
        } else {
            let mask = (1i128 << N) - 1;
            (rhs.inner & mask) as u128
        };

        lhs_unsigned
            .checked_div(rhs_unsigned)
            .map(|result| Self::new(result as i128))
            .map_or_else(SMTOption::none, SMTOption::some)
    }

    /// `(bvsrem a b)`
    pub fn bv_srem(self, rhs: Self) -> SMTOption<Self> {
        let res = self.inner.checked_rem(rhs.inner);
        match res {
            Some(v) => SMTOption::Some(Self {
                inner: v,
                _phantom: PhantomData,
            }),
            None => SMTOption::None,
        }
    }

    /// `(bvurem a b)`
    pub fn bv_urem(self, rhs: Self) -> SMTOption<Self> {
        let lhs_unsigned = if N == 128 {
            self.inner as u128 // Direct cast for full 128 bits
        } else {
            let mask = (1i128 << N) - 1;
            (self.inner & mask) as u128
        };

        let rhs_unsigned = if N == 128 {
            rhs.inner as u128
        } else {
            let mask = (1i128 << N) - 1;
            (rhs.inner & mask) as u128
        };

        lhs_unsigned
            .checked_rem(rhs_unsigned)
            .map(|inner| Self::new(inner as i128))
            .map_or_else(|| SMTOption::None, |v| SMTOption::Some(v))
    }

    /// `(bvsmod a b)`
    pub fn bv_smod(self, rhs: Self) -> SMTOption<Self> {
        self.inner
            .checked_rem_euclid(rhs.inner)
            .map(|inner| {
                if rhs.inner < 0 {
                    Self {
                        inner: inner + rhs.inner,
                        ..self
                    }
                } else {
                    Self { inner, ..self }
                }
            })
            .map_or_else(|| SMTOption::None, |v| SMTOption::Some(v))
    }

    /// Z3_mk_bvadd_no_overflow
    pub fn checked_bvadd_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_add(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvsub_no_overflow
    pub fn checked_bvsub_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_sub(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvneg_no_overflow
    pub fn checked_bvneg_no_overflow(self) -> Boolean {
        self.inner.checked_neg().is_some().into()
    }

    /// Z3_mk_bvmul_no_overflow
    pub fn checked_bvmul_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_mul(rhs.inner).is_some().into()
    }

    /// Z3_mk_bvsdiv_no_overflow
    pub fn checked_bvsdiv_no_overflow(self, rhs: Self) -> Boolean {
        self.inner.checked_div(rhs.inner).is_some().into()
    }
}

/// Shift Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvshl a b)`
    pub fn bv_shl(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shl(rhs.inner as u32),
            ..self
        }
    }

    /// `(bvlshr a b)`
    pub fn bv_lshr(self, rhs: Self) -> Self {
        Self {
            inner: ((self.inner as u128).wrapping_shr(rhs.inner as u32)) as i128,
            ..self
        }
    }

    /// `(bvashr a b)`
    pub fn bv_ashr(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.wrapping_shr(rhs.inner as u32),
            ..self
        }
    }

    /// `(rotate_left a b)`
    pub fn bv_rotate_left(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.rotate_left(rhs.inner as u32),
            ..self
        }
    }

    /// `(rotate_right a b)`
    pub fn bv_rotate_right(self, rhs: Self) -> Self {
        Self {
            inner: self.inner.rotate_right(rhs.inner as u32),
            ..self
        }
    }
}

/// Comparison Operators
impl<const N: usize> SymbolicBitVec<N> {
    /// `(bvslt a b)`
    pub fn bv_slt(self, rhs: Self) -> Boolean {
        (self.inner < rhs.inner).into()
    }

    /// `(bvult a b)`
    pub fn bv_ult(self, rhs: Self) -> Boolean {
        let mask = if N < 128 { (1i128 << N) - 1 } else { i128::MAX };
        let lhs_unsigned = (self.inner & mask) as u128;
        let rhs_unsigned = (rhs.inner & mask) as u128;
        (lhs_unsigned < rhs_unsigned).into()
    }

    /// `(bvsle a b)`
    pub fn bv_sle(self, rhs: Self) -> Boolean {
        (self.inner <= rhs.inner).into()
    }

    /// `(bvule a b)`
    pub fn bv_ule(self, rhs: Self) -> Boolean {
        let mask = if N < 128 { (1i128 << N) - 1 } else { i128::MAX };
        let lhs_unsigned = (self.inner & mask) as u128;
        let rhs_unsigned = (rhs.inner & mask) as u128;
        (lhs_unsigned <= rhs_unsigned).into()
    }

    /// `(bvsgt a b)`
    pub fn bv_sgt(self, rhs: Self) -> Boolean {
        (self.inner > rhs.inner).into()
    }

    /// `(bvugt a b)`
    pub fn bv_ugt(self, rhs: Self) -> Boolean {
        let mask = if N < 128 { (1i128 << N) - 1 } else { i128::MAX };
        let lhs_unsigned = (self.inner & mask) as u128;
        let rhs_unsigned = (rhs.inner & mask) as u128;
        (lhs_unsigned > rhs_unsigned).into()
    }

    /// `(bvsge a b)`
    pub fn bv_sge(self, rhs: Self) -> Boolean {
        (self.inner >= rhs.inner).into()
    }

    /// `(bvuge a b)`
    pub fn bv_uge(self, rhs: Self) -> Boolean {
        let mask = if N < 128 { (1i128 << N) - 1 } else { i128::MAX };
        let lhs_unsigned = (self.inner & mask) as u128;
        let rhs_unsigned = (rhs.inner & mask) as u128;
        (lhs_unsigned >= rhs_unsigned).into()
    }
}

/// Conversion Methods
impl<const N: usize> SymbolicBitVec<N> {
    /// to_int() converts the bitvector to a signed integer type.
    /// The conversion is always successful.
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(self.inner)),
        }
    }

    /// to_real() converts the bitvector to a real type.
    /// The conversion is always successful.
    pub fn to_real(self) -> Real {
        self.to_int().to_real()
    }

    /// LOSSY (Rounding): Converts a BitVector to an F32.
    pub fn to_f32(self) -> SMTOption<F32> {
        let f32_val = self.inner as f32;

        if let Some(round_tripped_bigint) = BigInt::from_f32(f32_val) {
            if round_tripped_bigint == BigInt::from(self.inner) {
                return SMTOption::Some(F32::from(f32_val));
            }
        }
        SMTOption::None
    }

    /// LOSSY (Rounding): Converts a BitVector to an F64.
    pub fn to_f64(self) -> SMTOption<F64> {
        let f64_val = self.inner as f64;

        if let Some(round_tripped_bigint) = BigInt::from_f64(f64_val) {
            if round_tripped_bigint == BigInt::from(self.inner) {
                return SMTOption::Some(F64::from(f64_val));
            }
        }
        SMTOption::None
    }
}

macro_rules! i32_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for I32 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for I32 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

i32_from_literal_int!(i8, i16, i32, i64, i128, isize);
i32_from_literal_int!(u8, u16, u32, u64, u128, usize);

macro_rules! i64_from_literal_int {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for I64 {
            fn from(c: $l) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        }
        $(impl From<$e> for I64 {
            fn from(c: $e) -> Self {
                Self {
                    inner: c as i128,
                    _phantom: PhantomData,
                }
            }
        })*
    };
}

/// implement SMT for SymbolicBitVec
impl<const N: usize> SMT for SymbolicBitVec<N> {
    fn _cmp(self, rhs: Self) -> Ordering {
        self.inner.cmp(&rhs.inner)
    }
}

i64_from_literal_int!(i8, i16, i32, i64, i128, isize);
i64_from_literal_int!(u8, u16, u32, u64, u128, usize);











// symbolic edge cases and rules of TOML
// ------------------------------
// 1) TOML is case-sensitive.
// 2) Whitespace means tab (0x09) or space (0x20).
// 3) Newline means LF (0x0A) or CRLF (0x0D 0x0A).
// 4) A hash symbol marks the rest of the line as a comment, except when inside a string.
// 5) Keys are on the left of the equals sign and values are on the right. Whitespace is ignored around key names and values.
// 6) The key, equals sign, and value must be on the same line (though some values can be broken over multiple lines).
// 7) Values must have one of the following types. String Integer Float Boolean Offset Date-Time Local Date-Time Local Date Local Time Array Inline Table
// 8) Whitespace around dot-separated parts is ignored. However, best practice is to not use any extraneous whitespace.
// 9) "" = "blank"     # VALID but discouraged '' = 'blank'     # VALID but discouraged
// 10) Best practice is to use bare keys except when absolutely necessary.
// 11) Indentation is treated as whitespace and ignored.
// 12) Note that bare keys and quoted keys are equivalent: "key" = "value" key = "value" # both are valid and identical
// 13) # This makes the key "fruit" into a table. fruit.apple.smooth = true # So then you can add to the table "fruit" like so: fruit.orange = 2
// 14) apple.type = "fruit" apple.skin = "thin" apple.color = "red" valid and out of order valid
// 15) 3.14159 = "pi" is a dotted key with two parts, both of which are bare keys. valid
// 16) A key may be either bare, quoted, or dotted.
// 17) There are four ways to express strings: basic, multi-line basic, literal, and multi-line literal.
// 18) Multi-line basic strings are surrounded by three quotation marks on each side and allow newlines. A newline immediately following the opening delimiter will be trimmed. All other whitespace and newline characters remain intact.
// 19) TOML parsers should feel free to normalize newline to whatever makes sense for their platform. # On a Unix system, the above multi-line string will most likely be the same as: str2 = "Roses are red\nViolets are blue" # On a Windows system, it will most likely be equivalent to: str3 = "Roses are red\r\nViolets are blue"
// 20) For writing long strings without introducing extraneous whitespace, use a "line ending backslash". When the last non-whitespace character on a line is an unescaped \, it will be trimmed along with all whitespace (including newlines) up to the next non-whitespace character or closing delimiter. All of the escape sequences that are valid for basic strings are also valid for multi-line basic strings. # The following strings are byte-for-byte equivalent: str1 = "The quick brown fox jumps over the lazy dog." str2 = """ The quick brown \ fox jumps over \ the lazy dog.""" str3 = """\ The quick brown \ fox jumps over \ the lazy dog.\ """
// 21) You can write a quotation mark, or two adjacent quotation marks, anywhere inside a multi-line basic string.
// 22) Literal strings are surrounded by single quotes. Like basic strings, they must appear on a single line: # What you see is what you get.
// 23) Note that bare keys are allowed to be composed of only ASCII digits, e.g. 1234, but are always interpreted as strings.
// 24) allow positive Positive numbers may be prefixed with a plus sign. Negative numbers are prefixed with a minus sign.
// 25) Floats should be implemented as IEEE 754 binary64 values.
// 26) Millisecond precision is required... If the value contains greater precision... the additional precision must be truncated, not rounded.
// 27) Arrays can span multiple lines. A terminating comma... is permitted..
// 28) Arrays may contain values of mixed types. However, the following types are considered incompatible and must not be mixed within an array: Date-Time, Local Date-Time, Local Date, Local Time.
// 29) [table-1] key1 = "some string" key2 = 123 [table-2] key1 = "another string" key2 = 456 valid
// 30) Empty tables are allowed and simply have no key/value pairs within them.
// 31) Within the braces, zero or more comma-separated key/value pairs may appear.
// 32) All value types are allowed, including inline tables.
// 33) Integer values -0 and +0 are valid and identical to an unprefixed zero.

// let x = number +1;

// if number == 0 {
//     let x = number + 1;
// } else {
//     let x = number + 1;
// }


// if key1 = key2 {
//     duplicate_key_error();
// } else {
//     if key1.lower_key == key2.lower_key {
//             give_a_valid_file();
//     } else {
//         give_a_valid_file();
//     }
// }


//! DELETE THIS FILE AFTER EDITING!

/// Represents all possible ways that parsing a TOML string can fail.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The input was not valid UTF-8 encoded text.
    InvalidEncoding,
    /// occurs when control characters other than tab (U+0000 to U+0008, U+000A to U+001F, U+007F) are in comments.
    InvalidCharacterInComment,
    /// occurs when whitespace characters other than tab (0x09) or space (0x20) are used.
    InvalidWhitespace,
    /// occurs when newline characters other than LF (0x0A) or CRLF (0x0D 0x0A) are used.
    InvalidNewline,

    /// A general syntax error not covered by a more specific variant.
    InvalidSyntax,
    /// occurs when a key is not bare, quoted, or dotted.
    InvalidKeyFormat,
    /// occurs when a bare key is empty. ( = "no key name" # INVALID)
    MissingKey,
    /// A key/value pair was not followed by a value (e.g., `key =`).
    MissingValue,
    /// A key/value pair was not followed by a newline or EOF.
    MissingNewlineAfterKeyValuePair,
    /// occurs when invalid types are used as values.
    /// valid types: String, Integer, Float, Boolean, Offset Date-Time, Local Date-Time, Local Date, Local Time, Array, Inline Table.
    UnsupportedValueType,

    /// occurs when a bare keys contains other than ASCII letters, ASCII digits, underscores, and dashes (A-Za-z0-9_-).
    InvalidBareKey,
    /// occurs when a dotted key is malformed, for example with a leading, trailing, or consecutive dot.
    InvalidDottedKey,
    /// occurs when multiple key/value pairs with the same key exist in the same table.
    DuplicateKeyInTable,
    /// An attempt was made to define a table that was already finalized as a value
    /// (e.g., `a.b = 1` followed by `[a.b.c]`).
    TableRedefinition,

    /// occurs when a forbidden control character was found within a string. This includes the unescaped:
    /// quotation mark, backslash, and the control characters other than tab (U+0000 to U+0008, U+000A to U+001F, U+007F).
    InvalidControlCharacterInString,
    /// occurs when a string contains an invalid escape sequence (e.g., `\q`).
    InvalidEscapeSequence,
    /// occurs when a Unicode escape sequence (`\uXXXX` or `\UXXXXXXXX`) represents a value
    /// in the invalid surrogate range (U+D800 to U+DFFF).
    InvalidUnicodeScalarValue,
    /// occurs when a string was not properly terminated with a closing quote.
    UnterminatedString,
    /// occurs when # str = """Here are three quotation marks: """."""  # INVALID
    TripleQuoteInMultiLineString,
    /// occurs when # str = "The first newline is " trimmed."  # INVALID
    QuoteInSingleLineString,
    /// occurs when a newline character is found within a single-line (literal) string.
    NewlineInSingleLineString,
    /// occurs when sequences of three or more single quotes are used in a multi-line literal string.
    InvalidSingleQuoteInMultiLineLiteralString,

    /// Each underscore must be surrounded by at least one digit on each side.
    InvalidUnderscoreInNumber,
    /// Leading zeros are not allowed.
    LeadingZeroInNumber,
    /// Non-negative integer values may also be expressed in hexadecimal, octal, or binary. In these formats, leading + is not allowed and leading zeros are allowed (after the prefix).
    InvalidPrefixInNumber,
    /// Invalid Digits for Base
    InvalidDigitForBase,
    /// If an integer cannot be represented losslessly, an error must be thrown.
    IntegerOverflow,

    /// A float must have at least one digit before and after the decimal point.
    InvalidFloatFraction,
    /// A float must have at least one digit in its exponent.
    InvalidFloatExponent,
    /// The fractional part must precede the exponent part.
    FractionExponentOrder,
    /// Underscores must be surrounded by digits
    InvalidUnderscoreInFloat,

    /// A date value did not conform to the RFC 3339 specification.
    InvalidDateFormat,
    /// A time value did not conform to the RFC 3339 specification.
    InvalidTimeFormat,

    /// An array was not properly opened with `[` or closed with `]`.
    UnterminatedArray,
    /// A comma was missing between array elements.
    MissingCommaInArray,
    /// An unexpected character was found inside an array.
    UnexpectedCharacterInArray,
    /// An array element failed to parse as any valid value.
    InvalidArrayElement,
    /// A closing bracket `]` was encountered before any opening bracket `[` .
    UnmatchedArrayCloseBracket,

    /// A table header was not properly opened (`[`) or closed (`]`).
    UnterminatedTableHeader,
    /// A closing bracket `]` was encountered without a matching opening bracket.
    UnmatchedTableHeaderCloseBracket,
    /// A table name was empty or contained invalid characters.
    InvalidTableName,
    /// A table header (e.g., `[my_table]`) was declared more than once.
    DuplicateTable,

    /// An inline table was not properly opened with `{` or closed with `}`.
    UnterminatedInlineTable,
    /// A closing brace `}` was encountered before any opening brace `{`.
    UnmatchedInlineTableCloseBrace,
    /// A comma was missing between key/value pairs in an inline table.
    MissingCommaInInlineTable,
    /// Trailing commas are not allowed in inline tables.
    TrailingCommaInInlineTable,
    /// No newlines are allowed between the curly braces unless they are valid within a value.
    UnexpectedNewlineInInlineTable,
    /// Inline tables are fully self-contained and define all keys and sub-tables within them. Keys and sub-tables cannot be added outside the braces.
    InlineTableRedefinition,

    /// An array of tables was not properly opened with `[[` or closed with `]]`.
    UnterminatedArrayOfTables,
    /// A closing bracket `]]` was encountered without a matching opening bracket `[[`.
    UnmatchedArrayOfTablesCloseBracket,
    /// An table was expected but an array of tables was found, or vice versa.
    MismatchedTableAndArrayOfTables,
    /// Attempting to append to a statically defined array, even if that array is empty, must produce an error at parse time.
    StaticArrayAppend,
    /// A table header was empty (no name between brackets).
    EmptyTableHeader,
    /// occurs [[table]] extra content after closing brackets
    UnexpectedTableHeaderContent,
}
# Errors in TOML Parsing
> All errors have inner 0 and no merging information for now. TODO

# TOML is case-sensitive.
> we have tried to test this by making False, FALSE, TRUE, True variations of boolean values in our test files and they all fail to parse. Only true and false in lowercase are accepted.
> Also we have tested 0X, 0B, 0O variations of integer literals and they all fail to parse. Only lowercase 0x, 0b, 0o are accepted.
> We have tried the case sensitivity when defining table names for example Table and table are treated as different tables. TODO

# A TOML file must be a valid UTF-8 encoded Unicode document.
> This is not tested as we use read_to_string which assumes valid UTF-8 input.

# Date Time
> Millisecond precision is required. Further precision of fractional seconds is implementation-specific. If the value contains greater precision than the implementation can support, the additional precision must be truncated, not rounded.


1 - Add bitvector size I8, U8, I16, U16 and so on (make the bitvector modular) also do it for floating points
2 - Z3_mk_set_complement in sets
3 - Maybe change the whole structure so that IR stores functions that cannot natively be converted to SMT instead of giving it to the backend
4 - Rust: Rounds ties away from zero (2.5 $\to$ 3.0).Wasm/Z3: Rounds ties to even (2.5 $\to$ 2.0). So what do we do? what if the behaviour of z3 is different than the language we are writing the interpreter for using the DSL and that is different than the behaviour of Rust itself? - remove floating point
5 - check the error locations in the IR populate them correctly and fix the intrinsics in the IR and parser and backend and how to the expression handling has been done in the parser and in the IR and in backend (function body)!


        // Z3 prints "success" after every command by default.
        // 500 declarations = 500 "success" lines before the actual sat/unsat result.
        // Turn it off so our response parser only sees the verdict.
        l!(x, "(set-option :print-success false)");
        // When Z3 returns "sat", we need the actual concrete values (e.g., input_0 = "hello"),
        // not just the fact that a solution exists. Without this, (get-model) fails.
        l!(x, "(set-option :produce-models true)");
        // Proof objects explain WHY something is unsatisfiable. Building them requires extra
        // bookkeeping during search, slowing Z3 down. We only care whether sat/unsat, not why.
        l!(x, "(set-option :produce-proofs false)");
        // Unsat cores identify WHICH of your assertions conflict (e.g., "assertions 3 and 7
        // are contradictory"). Computing this requires tracking assertion contributions.
        // We don't debug unsatisfiability — we just move to the next error target.
        l!(x, "(set-option :produce-unsat-cores false)");
        // ── Reproducibility ──
        // Z3 has two layers: an inner SAT solver (raw boolean variables) and an outer SMT solver
        // (theories: integers, strings, sets). Both use randomization to decide things like
        // "which variable to try next" or "which theory to check first."
        //
        // Without fixed seeds, the same input can produce different results across runs:
        //   Monday: sat in 2 seconds (lucky variable ordering)
        //   Tuesday: timeout at 60 seconds (unlucky ordering)
        //
        // Fixed seeds make the search deterministic: same input → same path → same result.
        l!(x, "(set-option :sat.random_seed 42)");
        l!(x, "(set-option :smt.random_seed 42)");
        // ── Parallelism ──
        // Z3 can split its search across multiple threads, each exploring a different part
        // of the search space. If any thread finds a solution, Z3 returns sat.
        // N threads on N cores = each thread gets a dedicated core, no contention.
        l!(x, "(set-option :parallel.enable true)");
        l!(x, "(set-option :parallel.threads.max {})", num_cpu_cores());
        // "Cube and conquer": the main thread works alone, then after `delay` milliseconds
        // splits the problem into subproblems for worker threads.
        //   delay=10: "try alone for 10ms, if still stuck, split across threads"
        //   delay=5000: waits 5s before splitting — if the problem solves in 3s, threads sat idle
        //   delay=0: splits immediately, but splitting has overhead (copying state, coordination)
        // 10ms is a good default for hard problems — start parallelizing quickly.
        l!(x, "(set-option :parallel.conquer.delay 10)");
        // ── SAT Solver: Restarts ──
        // A restart throws away the current guess and starts over, but keeps learned facts.
        // Example: Z3 guesses x=50000. Doesn't work. Nearby values don't work either.
        // Restart: forget the guess, remember "50000 area is bad", try a different region.
        // Each restart is smarter because of accumulated knowledge from previous attempts.
        //
        // 100,000 restarts allowed. Each carries over learned facts, so later restarts are
        // much more targeted than early ones. Too low = gives up before finding a good path.
        // Restarts are sequential (one thread retrying). Conquer is parallel (multiple threads
        // searching different regions). They work together: each thread can restart within
        // its own subproblem.
        l!(x, "(set-option :sat.restart.max 100000)");
        // ── SMT Solver: Arithmetic ──
        //
        // Z3 has multiple arithmetic engines (values: 2, 3, 6).
        //   2 = old Simplex. Linear only (x + y = 5).
        //   3 = adds basic integer support.
        //   6 = newest. Handles mod, div, nonlinear (x * y), mixed int/real.
        // We use 6 because our interpreter uses mod (bitvector ops), integer division,
        // and set membership checks.
        l!(x, "(set-option :smt.arith.solver 6)");
        // ── SMT Solver: Case Splitting ──
        //
        // When Z3 encounters (or A B C), it must decide which branch to explore.
        // Values: 0, 1, 2, 3, 5.
        //   0 = try all branches blindly
        //   3 = relevancy-based: only split on branches relevant to the current goal
        //
        // Example: (assert (or (= x 1) (= x 2) (= x 3))) with (assert (> x 100))
        //   Mode 0: tries x=1, fails. Tries x=2, fails. Tries x=3, fails. Returns unsat.
        //   Mode 3: sees (> x 100) immediately rules out all three. Returns unsat without trying.
        //
        // Mode 3 is critical for us: our interpreter has hundreds of ite branches (every
        // if/match in Rust becomes an ite). Most are irrelevant to any given error target.
        l!(x, "(set-option :smt.case_split 3)");
        // ── SMT Solver: Phase Selection ──
        //
        // When Z3 picks a boolean variable to branch on, should it try true or false first?
        // Values: 0 (always true), 1 (always false), 3 (always false + caching), 4 (random).
        //
        // Example: (assert (=> p (= x 1))) (assert (=> (not p) (= x 2))) (assert (> x 1))
        //   Mode 0 (true first): tries p=true → x=1 → fails (> x 1). Backtracks, tries p=false.
        //   Mode 3 (false first): tries p=false → x=2 → sat immediately.
        //
        // Mode 3 tends to prune faster for verification problems because negative assignments
        // eliminate more possibilities. In our ite chains, the "else" branch is usually the
        // common/fast path, and the "then" branch is the error/special case.
        l!(x, "(set-option :smt.phase_selection 3)");
        // ── Quantifier Handling ──
        //
        // Z3 can't check every integer for (forall ((k Int)) ...). These options control
        // HOW Z3 picks which values to try.
        //
        // MBQI (Model-Based Quantifier Instantiation) — trial and error approach:
        //   1. Z3 GUESSES a model (ignoring the forall)
        //   2. CHECKS: does the forall hold in this model? Looks for a counterexample k.
        //   3. If counterexample found, adds it as a constraint, goes back to step 1.
        //   4. If no counterexample, the model is valid → sat.
        //
        // Example:
        //   (forall ((k Int)) (>= (select arr k) 0))   ; all elements non-negative
        //   (= (select arr 5) (- 3))                    ; but arr[5] = -3
        //
        //   Round 1: guess arr = all zeros. Forall holds. But arr[5]=0 conflicts with arr[5]=-3.
        //   Round 2: fix arr[5]=-3. Check forall with k=5: -3 >= 0? No! Contradiction → unsat.
        //
        // Without MBQI, Z3 stares at the infinite forall and returns "unknown".
        l!(x, "(set-option :smt.mbqi true)");
        //
        // E-matching — forward pattern matching approach (complementary to MBQI):
        //   Z3 looks at terms it already knows (e.g., (select arr 7) from another assertion)
        //   and matches them against quantifier patterns. If (select arr 7) matches the
        //   forall's pattern (select arr k), Z3 instantiates k=7 and checks that case.
        //
        //   MBQI works BACKWARD from models. E-matching works FORWARD from known terms.
        //   Both enabled = two independent strategies to attack quantifiers.
        l!(x, "(set-option :smt.ematching true)");
        //
        // Eager threshold: how aggressively to instantiate quantifiers before they're needed.
        //   Low (0.5) = only instantiate when stuck and need it (conservative)
        //   High (10.0) = instantiate all matching terms immediately (aggressive)
        // We use 10.0 because we have few quantifiers (ArrayIsEmpty) but they're critical.
        // Instantiate eagerly → avoid backtracking later.
        l!(x, "(set-option :smt.qi.eager_threshold 10.0)");
        //
        // Multi-patterns match multiple terms simultaneously in a forall.
        // If Z3 knows 100 (select arr _) terms and 10 (> _ 0) terms, there are 1000 possible
        // combinations. This cap controls how many combinations Z3 is allowed to try.
        // 1000 is generous enough for our use case with few quantifiers.
        l!(x, "(set-option :smt.qi.max_multi_patterns 1000)");
        // ── Auto-configuration ──
        //
        // Z3 normally analyzes your input and auto-tunes its settings. This would override
        // everything above. We disable it because we've tuned specifically for our problem
        // structure: recursive ADTs, strings, sets, bitvectors, and few quantifiers.
        l!(x, "(set-option :smt.auto_config false)");
        l!(x);
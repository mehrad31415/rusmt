//! Real datatype and its operations

use crate::{Boolean, F32, F64, Integer, Real};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;
use num_traits::Zero;
use num_traits::cast::ToPrimitive;

/// Exact integer `q`-th root of `n`, or `None` when `n` is not a perfect `q`-th power
///
/// For `a/b` in lowest terms, `(a/b)^(1/q)` is rational exactly when `a` and `b`
/// are each a perfect `q`-th power — so applying this to numerator and
/// denominator separately decides representability for the whole fraction.
fn exact_nth_root(n: &BigInt, q: u32) -> Option<BigInt> {
    if n.is_negative() && q.is_multiple_of(2) {
        return None; // no real root
    }
    let r = n.nth_root(q);
    (r.pow(q) == *n).then_some(r)
}

/// Is n/d less than, equal to, or greater than 2^k? Requires `d > 0`.
fn cmp_pow2(n: &BigInt, d: &BigInt, k: i64) -> std::cmp::Ordering {
    if k >= 0 {
        n.cmp(&(d << k as usize))
    } else {
        (n << (-k) as usize).cmp(d)
    }
}

/// Round an exact rational to `f32`, rounding to nearest with ties to even.
///
/// This is `((_ to_fp 8 24) RNE x)`, done in **one** rounding step.
///
/// `num_rational` does not implement `to_f32` for `Ratio<BigInt>`, so the
/// `ToPrimitive` default applies: `to_f64()` followed by an `as f32` cast. That
/// rounds *twice*, and double rounding is not single rounding — when the f64 step
/// lands exactly on an f32 midpoint, the cast then applies ties-to-even to a
/// value that was never a tie. Measured divergence from Z3:
/// `x = 1 + 3*2^-24 - 2^-60` is strictly below the midpoint of the adjacent f32
/// values `1+2^-23` and `1+2*2^-23`, so it must round down to `1+2^-23`; the
/// double-rounded path returns `1+2*2^-23` instead.
///
/// `to_f64` needs no such treatment: `num_rational` implements it directly and
/// rounds once.
fn rational_to_f32(r: &BigRational) -> f32 {
    const SIG_BITS: i64 = 24;
    const MIN_EXP: i64 = -149;
    const MAX_EXP: i64 = 104;

    if r.is_zero() {
        return 0.0;
    }
    let negative = r.is_negative();
    let n = r.numer().abs();
    let d = r.denom().clone(); // `BigRational` keeps the denominator positive

    // if n needs Bn bits and d needs Bd bits, then: k0 = Bn − Bd
    let k0 = n.bits() as i64 - d.bits() as i64;
    // 2^(Bn − 1)  ≤  n  <  2^Bn and 2^(Bd − 1)  ≤  d  <  2^Bd
    // n/d  <  2^Bn / 2^(Bd−1)  =  2^(Bn − Bd + 1)
    // n/d  >  2^(Bn−1) / 2^Bd  =  2^(Bn − Bd − 1)
    // 2^(k0 − 1)  <  n/d  <  2^(k0 + 1)
    let k = if cmp_pow2(&n, &d, k0).is_lt() {
        k0 - 1
    } else {
        k0
    };

    let mut e = (k - (SIG_BITS - 1)).max(MIN_EXP);
    let (num, den) = if e >= 0 {
        (n, d << e as usize)
    } else {
        (n << (-e) as usize, d)
    };
    let mut m = &num / &den;
    let twice_rem = (&num - &m * &den) << 1usize;
    if twice_rem > den || (twice_rem == den && m.bit(0)) {
        m += 1;
    }

    // Rounding up can carry out of the binade. Rewriting `2^24 * 2^e` as
    // `2^23 * 2^(e+1)` is an exact identity, not a second rounding.
    if m.bits() as i64 > SIG_BITS {
        m >>= 1;
        e += 1;
    }
    if e > MAX_EXP {
        return if negative {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
    }

    let m = m
        .to_u32()
        .expect("DSL Error: f32 significand fits in 24 bits by construction");
    let sign = if negative { 1u32 << 31 } else { 0 };
    let implicit = 1u32 << (SIG_BITS - 1);
    let bits = if m < implicit {
        // Subnormal (or zero): exponent field 0, no implicit leading bit.
        debug_assert_eq!(e, MIN_EXP);
        sign | m
    } else {
        // value = m * 2^e = 1.f * 2^(e + 23), so the biased field is e + 150.
        sign | (((e + SIG_BITS - 1 + 127) as u32) << (SIG_BITS - 1)) | (m - implicit)
    };
    f32::from_bits(bits)
}

/// Real operations
impl Real {
    /// addition. Emits `(+ n m)`.
    pub fn add(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() + rhs.inner.as_ref()),
        }
    }

    /// multiplication. Emits `(* n m)`.
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() * rhs.inner.as_ref()),
        }
    }

    /// subtraction. Emits `(- n m)`.
    pub fn sub(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() - rhs.inner.as_ref()),
        }
    }

    /// negation. Emits `(- n)`.
    pub fn neg(self) -> Self {
        Self {
            inner: Intern::new(-self.inner.as_ref()),
        }
    }

    /// division. Emits `(/ n d)`.
    ///
    /// Panics when `d` is zero. The Reals theory makes `/` total but places no
    /// constraint on `(/ t 0)`, so there is no value to return; the panic comes
    /// from `num-rational`, not from an explicit check.
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// Exponentiation. Corresponds to Z3: `(^ self exp)`.
    ///
    /// Z3 evaluates `(^ 4.0 0.5)` to `2.0` and `(^ (- 8.0) (/ 1.0 3.0))` to
    /// `(- 2.0)`.
    ///
    /// # Panics
    ///
    /// Where Z3 has a value that a `Real` cannot hold, or where Z3 has no
    /// value at all:
    ///
    /// * **Irrational result** — `(^ 2.0 0.5)` is `√2`, which Z3 represents as
    ///   an algebraic number (`root-obj (+ (^ x 2) (- 2)) 2`). `BigRational`
    ///   is not closed under roots so there is nothing faithful to return.
    /// * **Negative base with an even root** — `(^ (- 2.0) 0.5)` has no real
    ///   value; Z3 does not reduce the term and `check-sat` answers `unknown`,
    ///   so no value can be shown to agree with it.
    /// * **`0^0`** — Z3 leaves `(^ 0.0 0.0)` uninterpreted.
    /// * `p` outside `i32` / `q` outside `u32`.
    ///
    /// Note that `0^x` is `0` in Z3 and here for *every* other exponent —
    /// fractional or integer, negative or positive. `0^0` is the only member of
    /// that family without a value.
    pub fn pow(self, exp: Self) -> Self {
        let e = exp.inner.as_ref();
        if self.inner.as_ref().is_zero() {
            assert!(
                !e.is_zero(),
                "0^0 has no value -- Z3 leaves `(^ 0.0 0.0)` underspecified \
                 (both `(= (^ 0.0 0.0) 0.0)` and `.. 1.0` are sat)"
            );
            return Self {
                inner: Intern::new(BigRational::zero()),
            };
        }

        if e.is_integer() {
            let n = e.to_integer().to_i32().expect(
                "the power is determinate but unaffordable -- an integral \
                     exponent outside i32 gives a number too large to build",
            );
            return Self {
                inner: Intern::new(self.inner.as_ref().pow(n)),
            };
        }

        // Reduced form, with `denom() > 0` guaranteed by `BigRational`.
        let p = e.numer().to_i32().expect(
            "unaffordable -- an exponent numerator outside i32 gives a \
                 number too large to build",
        );
        let q = e.denom().to_u32().expect(
            "a root of index beyond u32 cannot be taken, and for any base other \
             than 0 or ±1 the result would be irrational in any case",
        );

        // `x^p` is exact; the q-th root is where representability can fail.
        let t = self.inner.as_ref().pow(p);
        let root = |n: &BigInt| -> BigInt {
            match exact_nth_root(n, q) {
                Some(root) => root,
                // Two different reasons land here, so name them apart.
                None if n.is_negative() && q % 2 == 0 => panic!(
                    "Real::pow({}, {}) has no real value; Z3 leaves the term \
                     undecided (`check-sat` answers `unknown`), so there is nothing to return",
                    self.inner.as_ref(),
                    e
                ),
                None => panic!(
                    "Real::pow({}, {}) is determinate in Z3 -- a `root-obj` algebraic \
                     number -- but no `BigRational` denotes it",
                    self.inner.as_ref(),
                    e
                ),
            }
        };
        let (num, den) = (root(t.numer()), root(t.denom()));
        Self {
            inner: Intern::new(BigRational::new(num, den)),
        }
    }

    /// Returns the absolute value of the real number. Emits `(abs n)`.
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Rounds to the nearest integer; ties toward +∞ (i.e. floor(x + 0.5)).
    ///
    /// Corresponds to Z3: `(to_int (+ x (/ 1.0 2.0)))`
    pub fn round(self) -> Integer {
        let half =
            BigRational::from_integer(BigInt::from(1)) / BigRational::from_integer(BigInt::from(2));
        Integer {
            inner: Intern::new((self.inner.as_ref() + &half).floor().to_integer()),
        }
    }

    /// Floors the real number to the nearest integer ≤ x (rounds toward −∞).
    ///
    /// Corresponds to Z3: `(to_int x)`
    pub fn floor(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().floor().to_integer()),
        }
    }

    /// Ceils the real number to the nearest integer ≥ x (rounds toward +∞).
    ///
    /// Corresponds to Z3: `(- (to_int (- x)))`
    pub fn ceil(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().ceil().to_integer()),
        }
    }

    /// is integer. Emits `(is_int n)`.
    pub fn is_integer(self) -> Boolean {
        self.inner.is_integer().into()
    }

    /// less than. Emits `(< n m)`.
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() < rhs.inner.as_ref()).into()
    }

    /// less than or equal. Emits `(<= n m)`.
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() <= rhs.inner.as_ref()).into()
    }

    /// greater than. Emits `(> n m)`.
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() > rhs.inner.as_ref()).into()
    }

    /// greater than or equal. Emits `(>= n m)`.
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() >= rhs.inner.as_ref()).into()
    }

    /// Converts a Real to an Integer by rounding toward −∞ (floor).
    ///
    /// Corresponds to Z3: `(to_int x)` — Z3's `to_int` on Real is floor, not truncation.
    /// Example: to_int(-1.5) = -2 (not -1).
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().floor().to_integer()),
        }
    }

    /// convert to f32
    ///
    /// Corresponds to Z3: `((_ to_fp 8 24) RNE x)` — Round to Nearest, ties to Even.
    ///
    /// Total: out-of-range values saturate to `±inf`, and values below the
    /// smallest subnormal to `±0.0`.
    ///
    /// Rounds in a *single* step. See [`rational_to_f32`]: `BigRational`'s
    /// inherited `to_f32` rounds twice and diverges from Z3.
    pub fn to_f32(self) -> F32 {
        F32::from(rational_to_f32(self.inner.as_ref()))
    }

    /// convert to f64
    ///
    /// Corresponds to Z3: `((_ to_fp 11 53) RNE x)`.
    ///
    /// `num_rational` implements this one directly and rounds once, so unlike
    /// [`Self::to_f32`] it needs no help. The `expect` is unreachable: `to_f64` is `None` only for a `0/0` ratio,
    /// and every route into `Real` goes through `BigRational::new` /
    /// `from_integer` / arithmetic, all of which reject a zero denominator first.
    pub fn to_f64(self) -> F64 {
        F64::from(
            self.inner
                .as_ref()
                .to_f64()
                .expect("DSL Error: BigRational.to_f64 should never be None"),
        )
    }
}

/// Convert to Real from int literals
/// let a = Real::from(1);
/// let a:Real = 1.into(); // this needs to be annotated
/// let a:Real = From::from(1); // this needs to be annotated
macro_rules! real_from_literal {
    ($($e:ty),+ $(,)?) => {
        $(impl From<$e> for Real {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigRational::from(BigInt::from(c))),
                }
            }
        })+
    };
}

real_from_literal!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

mod tests {
    #[test]
    fn test_pow_matches_z3_where_rational() {
        use crate::{Real, smt::SMT};

        let half = Real::from(1).div(Real::from(2));
        let quarter = Real::from(1).div(Real::from(4));
        let third = Real::from(1).div(Real::from(3));

        // (^ 2.0 3.0) = 8.0 ; (^ 2.0 (- 2.0)) = (/ 1.0 4.0)
        assert!(*Real::from(2).pow(Real::from(3)).eq(Real::from(8)));
        assert!(*Real::from(2).pow(Real::from(-2)).eq(quarter));
        // (^ 4.0 0.5) = 2.0
        assert!(*Real::from(4).pow(half).eq(Real::from(2)));
        // (^ 8.0 (/ 1.0 3.0)) = 2.0
        assert!(*Real::from(8).pow(third).eq(Real::from(2)));
        // (^ (- 8.0) (/ 1.0 3.0)) = (- 2.0)
        assert!(*Real::from(-8).pow(third).eq(Real::from(-2)));
        // (^ 0.25 (- 0.5)) = 2.0
        assert!(*quarter.pow(half.neg()).eq(Real::from(2)));
        // (^ 9.0 1.5) = 27.0
        let three_halves = Real::from(3).div(Real::from(2));
        assert!(*Real::from(9).pow(three_halves).eq(Real::from(27)));
    }

    /// `(^ 2.0 2.5)` is `root-obj (+ (^ x 2) (- 32)) 2` = sqrt(32): a real value
    /// Z3 has and `BigRational` does not.
    #[test]
    #[should_panic(expected = "no `BigRational` denotes it")]
    fn test_pow_irrational_is_rejected() {
        use crate::Real;
        let five_halves = Real::from(5).div(Real::from(2));
        let _ = Real::from(2).pow(five_halves);
    }

    /// `(^ (- 2.0) 0.5)` has no real value; Z3 leaves the term uninterpreted.
    #[test]
    #[should_panic(expected = "has no real value")]
    fn test_pow_negative_base_even_root_is_rejected() {
        use crate::Real;
        let half = Real::from(1).div(Real::from(2));
        let _ = Real::from(-2).pow(half);
    }

    /// Z3 leaves `(^ 0.0 0.0)` genuinely uninterpreted: measured, both
    /// `(= (^ 0.0 0.0) 0.0)` and `(= (^ 0.0 0.0) 1.0)` are sat.
    #[test]
    #[should_panic(expected = "0^0 has no value")]
    fn test_pow_zero_to_the_zero_is_rejected() {
        use crate::Real;
        let _ = Real::from(0).pow(Real::from(0));
    }

    /// Every other `0^x` is `0.0`.
    #[test]
    fn test_pow_zero_base_matches_z3_at_every_other_exponent() {
        use crate::{Real, smt::SMT};

        let zero = Real::from(0);
        let half = Real::from(1).div(Real::from(2));
        let third = Real::from(1).div(Real::from(3));
        for e in [
            Real::from(2),
            Real::from(1),
            half,
            third,
            half.neg(),
            Real::from(-1),
            Real::from(-2),
        ] {
            assert!(*zero.pow(e).eq(zero), "0^{e:?} should be 0");
        }
    }

    /// `Real` is built from integer literals and `div`, so the value it holds is
    /// the exact rational the source names.
    #[test]
    fn test_real_div_is_exact() {
        use crate::{Real, smt::SMT};

        let a = Real::from(1);
        let b = Real::from(3);
        let c = Real::from(5);

        assert!(*a.div(c).eq(Real::from(2).div(Real::from(10))));
        assert!(*a.div(b).mul(Real::from(3)).eq(Real::from(1)));
        assert!(*a.div(b).eq(a.div(c)).not());
    }

    /// round, floor, ceil, to_int, and is_integer match Z3's behavior.
    #[test]
    fn test_real_rounding_matches_z3() {
        use crate::{Integer, Real, smt::SMT};

        let a = Real::from(-2).div(Real::from(2)); // -1.0
        let b = Real::from(-3).div(Real::from(2)); // -1.5
        let c = Real::from(5).div(Real::from(3)); // 1.666...
        let d = Real::from(3).div(Real::from(2)); // 1.5

        assert!(*a.round().eq(Integer::from(-1)));
        assert!(*b.round().eq(Integer::from(-1)));
        assert!(*c.round().eq(Integer::from(2)));
        assert!(*d.round().eq(Integer::from(2)));

        assert!(*a.floor().eq(Integer::from(-1)));
        assert!(*b.floor().eq(Integer::from(-2)));
        assert!(*c.floor().eq(Integer::from(1)));
        assert!(*d.floor().eq(Integer::from(1)));

        assert!(*a.ceil().eq(Integer::from(-1)));
        assert!(*b.ceil().eq(Integer::from(-1)));
        assert!(*c.ceil().eq(Integer::from(2)));
        assert!(*d.ceil().eq(Integer::from(2)));

        assert!(*a.to_int().eq(Integer::from(-1)));
        assert!(*b.to_int().eq(Integer::from(-2)));
        assert!(*c.to_int().eq(Integer::from(1)));
        assert!(*d.to_int().eq(Integer::from(1)));

        assert!(*a.is_integer());
        assert!(*b.is_integer().not());
        assert!(*c.is_integer().not());
        assert!(*d.is_integer().not());
    }

    /// `to_f32`/`to_f64` are `((_ to_fp 8 24) RNE x)` / `((_ to_fp 11 53) RNE x)`.
    #[test]
    fn test_real_to_float_matches_z3() {
        use crate::{F32, F64, Real, float::FloatOps, smt::SMT};

        // baseline: exactly representable, no rounding involved
        assert!(
            *Real::from(-2)
                .div(Real::from(2))
                .to_f32()
                .eq(F32::from(-1.0))
        );
        assert!(
            *Real::from(-2)
                .div(Real::from(2))
                .to_f64()
                .eq(F64::from(-1.0))
        );

        // rounding: 1/3 is not a binary fraction, so it must round to nearest.
        // (simplify ((_ to_fp 11 53) RNE (/ 1.0 3.0)))
        //   -> (fp #b0 #b01111111101 #x5555555555555)  = 0.3333333333333333
        let third = Real::from(1).div(Real::from(3));
        assert!(*third.to_f64().eq(F64::from(0.3333333333333333)));
        assert!(*third.to_f32().eq(F32::from(0.33333334)));

        // ties-to-even, not ties-away: 2^53+1 is exactly halfway between 2^53
        // and 2^53+2, and only 2^53 has an even mantissa.
        // (simplify ((_ to_fp 11 53) RNE 9007199254740993.0))
        //   -> (fp #b0 #b10000110100 #x0000000000000)  = 2^53
        let tie = Real::from(9007199254740993i64);
        assert!(*tie.to_f64().eq(F64::from(9007199254740992.0)));

        // overflow saturates to infinity, it does not panic.
        // (simplify ((_ to_fp 8 24) RNE 1e40))  -> (_ +oo 8 24)
        let e40 = Real::from(10).pow(Real::from(40));
        assert!(*e40.to_f32().eq(F32::infinity()));
        assert!(*e40.to_f32().is_infinite());
        // 10^40 is still finite in f64, so only the f32 conversion overflows
        assert!(*e40.to_f64().is_infinite().not());

        // (simplify ((_ to_fp 11 53) RNE 1e400)) -> (_ +oo 11 53)
        let e400 = Real::from(10).pow(Real::from(400));
        assert!(*e400.to_f64().eq(F64::infinity()));
        assert!(*e400.neg().to_f64().eq(F64::neg_infinity()));

        // underflow: a tiny negative rational rounds to *negative* zero, and
        // `F64::eq` distinguishes the two zeros (see `smt_float_impl`).
        // (simplify ((_ to_fp 11 53) RNE (- (/ 1.0 1e340)))) -> (_ -zero 11 53)
        let tiny_neg = Real::from(-1).div(Real::from(10).pow(Real::from(400)));
        assert!(*tiny_neg.to_f64().eq(F64::neg_zero()));
        assert!(*tiny_neg.to_f64().eq(F64::pos_zero()).not());

        // and exact zero is *positive* zero -- `Real` has no signed zero.
        assert!(*Real::from(0).to_f64().eq(F64::pos_zero()));
    }

    /// x sits just below the midpoint of two adjacent f32 values, and the lower
    /// neighbour has an odd significand:
    ///     a = 1 + 2^-23     (0x3f800001)
    ///     b = 1 + 2*2^-23   (0x3f800002)
    ///     m = 1 + 3*2^-24   (their midpoint)
    ///     x = m - 2^-60     (strictly below m, so correct rounding gives `a`)
    #[test]
    fn test_real_to_f32_is_single_rounded() {
        use crate::{F32, Real, smt::SMT};

        let x = Real::from(1152921710765277183i64).div(Real::from(2).pow(Real::from(60)));
        assert!(*x.to_f32().eq(F32::from(1.0000001f32)));
    }
}

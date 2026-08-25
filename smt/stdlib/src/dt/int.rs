//! SMT Integer type and operations.

use crate::{Boolean, F32, F64, I32, I64, Integer, Real, String, U32, U64};
use internment::Intern;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Euclid;
use num_traits::Signed;
use num_traits::cast::ToPrimitive;

/// `(_ int2bv N)` is total: it takes the value modulo `2^N`.
fn wrap_u64(v: &BigInt) -> u64 {
    v.rem_euclid(&BigInt::from(1u128 << 64))
        .to_u64()
        .expect("value is in [0, 2^64)")
}

fn wrap_u32(v: &BigInt) -> u32 {
    v.rem_euclid(&BigInt::from(1u64 << 32))
        .to_u32()
        .expect("value is in [0, 2^32)")
}

/// Z3 parses a digit string by folding left, mapping every character that is not
/// a digit of the radix to zero; there is no sign, no prefix and no failure.
fn fold_digits(s: String, radix: u32, digit: fn(char) -> u32) -> BigInt {
    let mut acc = BigInt::from(0);
    for c in s.inner.chars() {
        acc = acc * radix + digit(c);
    }
    acc
}

impl Integer {
    /// addition - Result can grow arbitrarily large but is bounded by available system memory
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

    /// Truncating division, as C, Java and Rust have it.
    ///
    /// Emits `(ite (>= n 0) (div n d) (- (div (- n) d)))`. SMT-LIB offers only
    /// Euclidean `div`, so this is built from it -- there is no `div_trunc`
    /// function in the query, the term is inlined at each use. `n` and `d` are
    /// bound by a `let` when either is a compound expression, since each occurs
    /// more than once.
    ///
    /// Panics when `d` is zero: `(div n 0)` is underspecified.
    pub fn div_trunc(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() / rhs.inner.as_ref()),
        }
    }

    /// Division matching Z3 SMT-LIB `Int` division (Euclidean).
    ///
    /// Z3 ensures: `n = d*q + r` and `0 <= r < |d|` (for `d != 0`).
    /// Corresponds to Z3: `(div n d)`.
    pub fn div(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().div_euclid(rhs.inner.as_ref())),
        }
    }

    /// Modulo matching Z3 SMT-LIB (Euclidean)
    /// Result is ALWAYS non-negative: 0 <= r < |rhs|
    /// Corresponds to Z3 native operator: (mod self rhs)
    pub fn modulo(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().rem_euclid(rhs.inner.as_ref())),
        }
    }

    /// Remainder of the truncating division: zero, or the sign of the dividend.
    ///
    /// Emits `(- n (* d (ite (>= n 0) (div n d) (- (div (- n) d)))))`, inlined
    /// the same way as [`Integer::div_trunc`]; there is no `rem_trunc` function.
    ///
    /// Panics when `d` is zero.
    pub fn rem(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref() % rhs.inner.as_ref()),
        }
    }

    /// exponentiation
    /// corresponds to Z3: (to_int (^ self exp))
    ///
    /// A negative exponent gives the floor of a fraction, which is what the
    /// emitted `to_int` computes: `2^-1` is `0`, `(-2)^-1` is `-1`, `1^-5` is `1`.
    ///
    /// # Panics
    /// On `0^0`, which Z3 leaves unconstrained, and on an exponent beyond `u32`,
    /// which Z3 handles but `BigInt::pow` cannot take -- the result would not fit
    /// in memory regardless.
    pub fn pow(self, exp: Self) -> Self {
        let base = self.inner.as_ref();
        let e = exp.inner.as_ref();
        let (zero, one, minus_one) = (BigInt::from(0), BigInt::from(1), BigInt::from(-1));

        if e.is_negative() {
            // |base^e| <= 1 here, so the floor is one of four values.
            let odd = e % &BigInt::from(2) != zero;
            let floor = if base == &zero {
                zero.clone() // Z3 reads `(^ 0 e)` as 0.0, not as a division by zero
            } else if base == &one {
                one
            } else if base == &minus_one {
                if odd { minus_one } else { one }
            } else if base.is_negative() && odd {
                minus_one // in (-1, 0)
            } else {
                zero.clone() // in (0, 1)
            };
            return Self {
                inner: Intern::new(floor),
            };
        }

        assert!(
            !(e == &zero && base == &zero),
            "0^0 is underspecified in Z3, so there is no value to return"
        );
        let e = e.to_u32().expect(
            "the power is determinate but unaffordable -- an exponent \
                     beyond u32 gives a number of over a billion digits",
        );
        Self {
            inner: Intern::new(base.pow(e)),
        }
    }

    /// absolute value
    ///
    /// corresponds to Z3: (abs self)
    pub fn abs(self) -> Self {
        Self {
            inner: Intern::new(self.inner.as_ref().abs()),
        }
    }

    /// Checks if self divides rhs (i.e., `rhs mod self == 0`).
    /// (= (mod rhs self) 0)
    ///
    /// # Panics
    /// Panics when `self` is zero so guard the call.
    pub fn divides(self, rhs: Self) -> Boolean {
        (rhs.inner.as_ref() % self.inner.as_ref() == BigInt::from(0)).into()
    }

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

    /// to_real() converts the int to a real type.
    /// (to_real x)
    pub fn to_real(self) -> Real {
        Real {
            inner: Intern::new(BigRational::from((
                self.inner.as_ref().clone(),
                BigInt::from(1),
            ))),
        }
    }

    /// to_i32 converts the int to a i32 type.
    ///
    /// ((_ int2bv 32) x)
    pub fn to_i32(self) -> I32 {
        I32 {
            inner: Intern::new(wrap_u32(self.inner.as_ref()) as i32),
        }
    }

    /// to_i64 converts the int to a i64 type.
    ///
    /// ((_ int2bv 64) x)
    pub fn to_i64(self) -> I64 {
        I64 {
            inner: Intern::new(wrap_u64(self.inner.as_ref()) as i64),
        }
    }

    /// to_u32 converts the int to a u32 type.
    ///
    /// ((_ int2bv 32) x)
    pub fn to_u32(self) -> U32 {
        U32 {
            inner: Intern::new(wrap_u32(self.inner.as_ref())),
        }
    }

    /// to_u64 converts the int to a u64 type.
    ///
    /// ((_ int2bv 64) x)
    pub fn to_u64(self) -> U64 {
        U64 {
            inner: Intern::new(wrap_u64(self.inner.as_ref())),
        }
    }

    /// convert to f32
    ///
    /// ((_ to_fp 8 24) RNE (to_real x))
    pub fn to_f32(self) -> F32 {
        F32::from(self.inner.as_ref().to_f32().unwrap())
    }

    /// convert to f64
    ///
    /// ((_ to_fp 11 53) RNE (to_real x))
    pub fn to_f64(self) -> F64 {
        F64::from(self.inner.as_ref().to_f64().unwrap())
    }

    /// Creates an `Integer` from a hexadecimal string, with no `0x` prefix and
    /// no underscores.
    ///
    /// Emits `(rusmt_from_hex_str s)`, defined in the backend preamble as
    ///
    /// ```text
    /// (define-fun rusmt_hex_char_to_int ((s String)) Int
    ///     (ite (and (str.<= "0" s) (str.<= s "9")) (- (str.to_code s) 48)
    ///     (ite (and (str.<= "A" s) (str.<= s "F")) (- (str.to_code s) 55)
    ///     (ite (and (str.<= "a" s) (str.<= s "f")) (- (str.to_code s) 87)
    ///     0))))
    /// (define-fun-rec rusmt_from_hex_str_impl ((s String) (acc Int)) Int
    ///     (ite (= (str.len s) 0)
    ///         acc
    ///         (rusmt_from_hex_str_impl
    ///             (str.substr s 1 (- (str.len s) 1))
    ///             (+ (* acc 16) (rusmt_hex_char_to_int (str.at s 0))))))
    /// (define-fun rusmt_from_hex_str ((s String)) Int (rusmt_from_hex_str_impl s 0))
    /// ```
    ///
    /// A `define-fun` is total, so the nested `ite` needs a final branch and it
    /// must be an `Int`: there is no failure to return. That branch is `0`, so a
    /// character that is not a hex digit contributes nothing and the fold carries
    /// on. Hence `from_hex_str("zz")` is `0` and `from_hex_str("-ff")` is `255` --
    /// there is no sign, no prefix and no error. Validate the string before
    /// calling if the language being specified rejects such input.
    pub fn from_hex_str(s: String) -> Self {
        Self {
            inner: Intern::new(fold_digits(s, 16, |c| match c {
                '0'..='9' => c as u32 - 48,
                'A'..='F' => c as u32 - 55,
                'a'..='f' => c as u32 - 87,
                _ => 0,
            })),
        }
    }

    /// Creates an `Integer` from an octal string, with no `0o` prefix.
    ///
    /// Emits `(rusmt_from_oct_str s)`, defined in the backend preamble as
    ///
    /// ```text
    /// (define-fun rusmt_oct_char_to_int ((s String)) Int
    ///     (ite (and (str.<= "0" s) (str.<= s "7")) (- (str.to_code s) 48) 0))
    /// (define-fun-rec rusmt_from_oct_str_impl ((s String) (acc Int)) Int
    ///     (ite (= (str.len s) 0)
    ///         acc
    ///         (rusmt_from_oct_str_impl
    ///             (str.substr s 1 (- (str.len s) 1))
    ///             (+ (* acc 8) (rusmt_oct_char_to_int (str.at s 0))))))
    /// (define-fun rusmt_from_oct_str ((s String)) Int (rusmt_from_oct_str_impl s 0))
    /// ```
    ///
    /// Non-octal characters contribute `0`, for the reason given on
    /// [`Integer::from_hex_str`].
    pub fn from_oct_str(s: String) -> Self {
        Self {
            inner: Intern::new(fold_digits(s, 8, |c| match c {
                '0'..='7' => c as u32 - 48,
                _ => 0,
            })),
        }
    }

    /// Creates an `Integer` from a binary string, with no `0b` prefix.
    ///
    /// Emits `(rusmt_from_bin_str s)`, defined in the backend preamble as
    ///
    /// ```text
    /// (define-fun-rec rusmt_from_bin_str_impl ((s String) (acc Int)) Int
    ///     (ite (= (str.len s) 0)
    ///         acc
    ///         (rusmt_from_bin_str_impl
    ///             (str.substr s 1 (- (str.len s) 1))
    ///             (+ (* acc 2) (ite (= (str.at s 0) "1") 1 0)))))
    /// (define-fun rusmt_from_bin_str ((s String)) Int (rusmt_from_bin_str_impl s 0))
    /// ```
    ///
    /// Note the test is `= "1"`, so every character other than `1` contributes
    /// `0` -- including `0` itself, and anything else.
    pub fn from_bin_str(s: String) -> Self {
        Self {
            inner: Intern::new(fold_digits(s, 2, |c| u32::from(c == '1'))),
        }
    }

    /// check if greater than i64::MAX
    ///
    /// corresponds to Z3: (> self 9223372036854775807)
    pub fn is_gt_i64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i64::MAX)).into()
    }

    /// check if less than i64::MIN
    ///
    /// corresponds to Z3: (< self (- 9223372036854775808))
    pub fn is_lt_i64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i64::MIN)).into()
    }

    /// check if greater than u64::MAX
    ///
    /// corresponds to Z3: (> self 18446744073709551615)
    pub fn is_gt_u64_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(u64::MAX)).into()
    }

    /// check if less than u64::MIN
    ///
    /// corresponds to Z3: (< self 0)
    pub fn is_lt_u64_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u64::MIN)).into()
    }

    /// check if less than i32::MIN
    ///
    /// corresponds to Z3: (< self (- 2147483648))
    pub fn is_lt_i32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(i32::MIN)).into()
    }

    /// check if greater than i32::MAX
    ///
    /// corresponds to Z3: (> self 2147483647)
    pub fn is_gt_i32_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(i32::MAX)).into()
    }

    /// check if less than u32::MIN
    ///
    /// corresponds to Z3: (< self 0)
    pub fn is_lt_u32_min(self) -> Boolean {
        (self.inner.as_ref() < &BigInt::from(u32::MIN)).into()
    }

    /// check if greater than u32::MAX
    ///
    /// corresponds to Z3: (> self 4294967295)
    pub fn is_gt_u32_max(self) -> Boolean {
        (self.inner.as_ref() > &BigInt::from(u32::MAX)).into()
    }
}

/// Convert to the Integer type from literals
/// let a = Integer::from(1);
/// let a:Integer = 1.into(); // this needs to be annotated
/// let a:Integer = From::from(1); // this needs to be annotated
macro_rules! integer_from_literal {
    ($l:ty $(,$e:ty)* $(,)?) => {
        impl From<$l> for Integer {
            fn from(c: $l) -> Self {
                Self {
                    inner: Intern::new(BigInt::from(c)),
                }
            }
        }
        $(impl From<$e> for Integer {
            fn from(c: $e) -> Self {
                Self {
                    inner: Intern::new(BigInt::from(c)),
                }
            }
        })*
    };
}

integer_from_literal!(i8, i16, i32, i64, i128, isize);
integer_from_literal!(u8, u16, u32, u64, u128, usize);

#[cfg(test)]
mod tests {
    #[test]
    fn test_div() {
        let a1 = super::Integer::from(-7);
        let a2 = super::Integer::from(7);
        let b1 = super::Integer::from(3);
        let b2 = super::Integer::from(-3);
        let c1 = a1.div(b1);
        let c2 = a1.div(b2);
        let c3 = a2.div(b1);
        let c4 = a2.div(b2);
        assert_eq!(c1.inner.as_ref(), &num_bigint::BigInt::from(-3));
        assert_eq!(c2.inner.as_ref(), &num_bigint::BigInt::from(3));
        assert_eq!(c3.inner.as_ref(), &num_bigint::BigInt::from(2));
        assert_eq!(c4.inner.as_ref(), &num_bigint::BigInt::from(-2));
    }

    #[test]
    fn test_div_trunc() {
        let a1 = super::Integer::from(-7);
        let a2 = super::Integer::from(7);
        let b1 = super::Integer::from(3);
        let b2 = super::Integer::from(-3);
        let c1 = a1.div_trunc(b1);
        let c2 = a1.div_trunc(b2);
        let c3 = a2.div_trunc(b1);
        let c4 = a2.div_trunc(b2);
        assert_eq!(c1.inner.as_ref(), &num_bigint::BigInt::from(-2));
        assert_eq!(c2.inner.as_ref(), &num_bigint::BigInt::from(2));
        assert_eq!(c3.inner.as_ref(), &num_bigint::BigInt::from(2));
        assert_eq!(c4.inner.as_ref(), &num_bigint::BigInt::from(-2));
    }

    #[test]
    fn test_modulo() {
        let a1 = super::Integer::from(-7);
        let a2 = super::Integer::from(7);
        let b1 = super::Integer::from(3);
        let b2 = super::Integer::from(-3);
        let c1 = a1.modulo(b1);
        let c2 = a1.modulo(b2);
        let c3 = a2.modulo(b1);
        let c4 = a2.modulo(b2);
        assert_eq!(c1.inner.as_ref(), &num_bigint::BigInt::from(2));
        assert_eq!(c2.inner.as_ref(), &num_bigint::BigInt::from(2));
        assert_eq!(c3.inner.as_ref(), &num_bigint::BigInt::from(1));
        assert_eq!(c4.inner.as_ref(), &num_bigint::BigInt::from(1));
    }

    #[test]
    fn test_rem() {
        let a1 = super::Integer::from(-7);
        let a2 = super::Integer::from(7);
        let b1 = super::Integer::from(3);
        let b2 = super::Integer::from(-3);
        let c1 = a1.rem(b1);
        let c2 = a1.rem(b2);
        let c3 = a2.rem(b1);
        let c4 = a2.rem(b2);
        assert_eq!(c1.inner.as_ref(), &num_bigint::BigInt::from(-1));
        assert_eq!(c2.inner.as_ref(), &num_bigint::BigInt::from(-1));
        assert_eq!(c3.inner.as_ref(), &num_bigint::BigInt::from(1));
        assert_eq!(c4.inner.as_ref(), &num_bigint::BigInt::from(1));
    }
}

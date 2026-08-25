//! String datatype and operations. Both sides count Unicode code points: Rust
//! via `chars()`, Z3 over its alphabet U+0000..U+2FFFF.
//!
//! A literal that is a string written in the source, like the `"é"` in
//! `String::from("é")` is not copied into the SMT script as it stands. Z3's
//! lexer reads a raw byte as a character and it reads `\u{..}` as an escape, so a
//! copied literal can reach Z3 as a different string. The backend spells every
//! character out instead:
//!
//! ```text
//! rust value   emitted           z3 sees
//! é            "\u{e9}"          1 character, not the 2 UTF-8 bytes
//! \u0041       "\u{5c}u0041"     6 characters, not the letter A
//! ```
//!
//! Surrogates are the one case with no shared answer: Z3 admits one as a
//! character, Rust's `char` cannot hold it, so `from_code` panics.

use crate::{Boolean, Integer, String, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
use num_bigint::Sign;
use num_traits::cast::ToPrimitive;

impl From<&str> for String {
    fn from(c: &str) -> Self {
        Self {
            inner: Intern::new(c.to_string()),
        }
    }
}

impl From<std::string::String> for String {
    fn from(c: std::string::String) -> Self {
        Self {
            inner: Intern::new(c),
        }
    }
}

impl String {
    /// Creates a new empty string. Emits `""`.
    pub fn new() -> Self {
        Self {
            inner: Intern::new(std::string::String::new()),
        }
    }

    /// This directly corresponds to the `(str.len s)`.
    pub fn length(self) -> Integer {
        Integer::from(self.inner.chars().count())
    }

    /// `(str.++ s1 s2)`
    pub fn concat(self, rhs: Self) -> Self {
        let mut new_str = self.inner.as_ref().clone();
        new_str.push_str(rhs.inner.as_ref());
        Self {
            inner: Intern::new(new_str),
        }
    }

    /// `(str.at s offset)`
    /// `""` when the index is negative or past the end, as `str.at` gives.
    pub fn at(self, index: Integer) -> Self {
        index
            .inner
            .to_usize()
            .and_then(|idx| self.inner.chars().nth(idx))
            .map_or_else(Self::new, |char_at| Self::from(char_at.to_string()))
    }

    /// `(str.indexof s substr offset)`.
    /// `-1` when the offset is outside `[0, len]` or the needle does not occur at
    /// or after it, as `str.indexof` gives. An offset of exactly `len` is in range.
    pub fn index_of(self, substr: Self, offset: Integer) -> Integer {
        if offset.inner.sign() == Sign::Minus {
            return Integer::from(-1);
        }
        let chars: Vec<char> = self.inner.chars().collect();
        let start_char = offset.inner.to_usize().unwrap_or(usize::MAX);
        if start_char > chars.len() {
            return Integer::from(-1);
        }
        let start_byte: usize = chars[..start_char].iter().map(|c| c.len_utf8()).sum();
        match self.inner[start_byte..].find(substr.inner.as_ref()) {
            Some(found) => Integer::from(self.inner[..(start_byte + found)].chars().count()),
            None => Integer::from(-1),
        }
    }

    /// `(str.indexof s substr)`
    pub fn index_of_default(self, substr: Self) -> Integer {
        self.index_of(substr, Integer::from(0))
    }

    /// `(str.substr s offset length)`
    /// `""` when the offset is outside `[0, len)` or the length is not positive,
    /// otherwise the longest available run, as `str.substr` gives.
    pub fn substr(self, offset: Integer, length: Integer) -> Self {
        if offset.inner.sign() == Sign::Minus || length.inner.sign() != Sign::Plus {
            return Self::new();
        }
        let chars: Vec<char> = self.inner.chars().collect();
        let start_char = offset.inner.to_usize().unwrap_or(usize::MAX);
        if start_char >= chars.len() {
            return Self::new();
        }
        let len = length.inner.to_usize().unwrap_or(usize::MAX);
        let end_char = start_char.saturating_add(len).min(chars.len());
        Self::from(
            chars[start_char..end_char]
                .iter()
                .collect::<std::string::String>(),
        )
    }

    ///`(str.contains self rhs)`
    pub fn contains(self, rhs: Self) -> Boolean {
        self.inner.as_ref().contains(rhs.inner.as_ref()).into()
    }

    /// `(str.prefixof rhs self)`
    pub fn starts_with(self, rhs: Self) -> Boolean {
        self.inner.as_ref().starts_with(rhs.inner.as_ref()).into()
    }

    /// `(str.suffixof rhs self)`
    pub fn ends_with(self, rhs: Self) -> Boolean {
        self.inner.as_ref().ends_with(rhs.inner.as_ref()).into()
    }

    /// `(str.to_int s)`
    /// `-1` unless the string is a non-empty run of ASCII digits, as `str.to_int`
    /// gives; in particular `"-5"` is `-1`, not `-5`.
    pub fn to_int(self) -> Integer {
        let raw = self.inner.as_ref();
        if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit()) {
            return Integer::from(-1);
        }
        Integer {
            inner: Intern::new(raw.parse::<BigInt>().unwrap()),
        }
    }

    /// `(str.from_int i)`
    /// `""` for negative integers, as `str.from_int` gives.
    pub fn from_int(i: Integer) -> Self {
        if i.inner.sign() == Sign::Minus {
            return Self::new();
        }
        Self::from(i.inner.to_string())
    }

    /// Lexicographical less than or equal to: `(str.<= self rhs)`
    /// For example, "a" <= "b" and "aa" <= "ab" and "a" <= "aa" etc.
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() <= rhs.inner.as_ref()).into()
    }

    /// Lexicographical less than: `(str.< self rhs)`
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() < rhs.inner.as_ref()).into()
    }

    /// Lexicographical greater than or equal to: `(str.<= rhs self)`
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() >= rhs.inner.as_ref()).into()
    }

    /// Lexicographical greater than: `(str.< rhs self)`
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() > rhs.inner.as_ref()).into()
    }

    /// `(str.in_re s (re.range "0" "9"))`
    pub fn is_digit(self) -> Boolean {
        (*self.length().eq(Integer::from(1))
            && (self
                .at(Integer::from(0))
                .inner
                .as_ref()
                .chars()
                .next()
                .unwrap()
                .is_ascii_digit()))
        .into()
    }

    /// checks if the string is empty. Emits `(= (str.len s) 0)`.
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// `(str.from_code i)`
    /// `""` outside Z3's alphabet `[0, 0x2FFFF]`, as `str.from_code` gives.
    /// Panics on a surrogate: Z3 admits one as a character, Rust's `char` cannot
    /// hold it, so there is no value to return.
    pub fn from_code(code: Integer) -> Self {
        if code.inner.sign() == Sign::Minus {
            return Self::new();
        }
        let Some(raw_val) = code.inner.to_u32() else {
            return Self::new();
        };
        if raw_val > 0x2FFFF {
            return Self::new();
        }
        let c = char::from_u32(raw_val).unwrap_or_else(|| {
            panic!(
                "U+{raw_val:04X} is a surrogate -- Z3 admits it as a character, \
                 no Rust `char` denotes it"
            )
        });
        Self::from(c.to_string())
    }

    /// `(str.to_code s)`
    /// `-1` unless the string is exactly one character, as `str.to_code` gives.
    pub fn to_code(self) -> Integer {
        let mut chars = self.inner.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Integer::from(c as u32),
            _ => Integer::from(-1),
        }
    }

    /// iterator, yielding one-character strings (`Seq::iterator` yields indices).
    /// Having this method means that the string can be used in the expression macros for iterating over the string (forall, exists, choose).
    pub fn iterator(self) -> Vec<Self> {
        self.inner
            .chars()
            .map(|c| String::from(c.to_string()))
            .collect()
    }

    /// `(str.replace s src dst)`.
    pub fn replace(self, src: Self, dst: Self) -> Self {
        let replaced = self
            .inner
            .replacen(src.inner.as_ref(), dst.inner.as_ref(), 1);
        Self::from(replaced)
    }

    /// `(str.replace_all s src dst)`
    pub fn replace_all(self, src: Self, dst: Self) -> Self {
        if src.inner.as_ref().is_empty() {
            return self;
        }
        let replaced = self.inner.replace(src.inner.as_ref(), dst.inner.as_ref());
        Self::from(replaced)
    }
}

mod tests {
    #[test]
    fn test_string_to_code() {
        use crate::{Integer, String, smt::SMT};
        // test to code
        let s1 = String::from("😀");
        let s2 = String::from("é");
        assert!(*s1.to_code().eq(Integer::from(128512)));
        assert!(*s2.to_code().eq(Integer::from(233)));
    }

    #[test]
    fn test_string_length2() {
        use crate::{Integer, String, smt::SMT};
        let s1 = String::from("😀");
        let s2 = String::from("Hello");
        let s3 = String::from("é");
        assert!(*s1.length().eq(Integer::from(1)));
        assert!(*s2.length().eq(Integer::from(5)));
        assert!(*s3.length().eq(Integer::from(1)));
    }

    #[test]
    fn from_code_outside_z3s_alphabet_is_the_empty_string() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from_code(Integer::from(0x30000)).eq(String::new()));
        assert!(*String::from_code(Integer::from(-1)).eq(String::new()));
    }

    #[test]
    #[should_panic(expected = "is a surrogate")]
    fn from_code_on_a_surrogate_has_no_rust_value() {
        use crate::{Integer, String};
        let _ = String::from_code(Integer::from(0xD800));
    }

    #[test]
    fn test_string_at() {
        use crate::{Integer, String, smt::SMT};
        let s1 = String::from("Hello");
        let s2 = String::from("😀");
        let s3 = String::from("é");
        assert!(*s1.at(Integer::from(0)).eq(String::from("H")));
        assert!(*s2.at(Integer::from(0)).eq(String::from("😀")));
        assert!(*s3.at(Integer::from(0)).eq(String::from("é")));
    }

    #[test]
    fn test_index_of() {
        use crate::{Integer, String, smt::SMT};

        assert!(
            *String::from("Hello")
                .index_of(String::from("lo"), Integer::from(0))
                .eq(Integer::from(3))
        );
        assert!(
            *String::from("Hello")
                .index_of(String::from("l"), Integer::from(3))
                .eq(Integer::from(3))
        );
        assert!(
            *String::from("Hello")
                .index_of(String::from("He"), Integer::from(0))
                .eq(Integer::from(0))
        );
        assert!(
            *String::from("Hello")
                .index_of(String::from("lo"), Integer::from(0))
                .eq(Integer::from(3))
        );
        assert!(
            *String::from("Hello")
                .index_of(String::from(""), Integer::from(0))
                .eq(Integer::from(0))
        );
        assert!(
            *String::from("Hello")
                .index_of(String::from(""), Integer::from(3))
                .eq(Integer::from(3))
        );
        assert!(
            *String::from("abcabc")
                .index_of(String::from("b"), Integer::from(0))
                .eq(Integer::from(1))
        );
        assert!(
            *String::from("abcabc")
                .index_of(String::from("b"), Integer::from(2))
                .eq(Integer::from(4))
        );
    }

    #[test]
    fn test_index_of_not_found() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .index_of(String::from("xyz"), Integer::from(0))
                .eq(Integer::from(-1))
        );
        // an offset of exactly the length is in range and finds the empty needle
        assert!(
            *String::from("Hello")
                .index_of(String::new(), Integer::from(5))
                .eq(Integer::from(5))
        );
    }

    #[test]
    fn test_index_of_offset_out_of_bounds() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .index_of(String::from("l"), Integer::from(10))
                .eq(Integer::from(-1))
        );
    }

    #[test]
    fn test_index_of_negative_offset() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .index_of(String::from("l"), Integer::from(-1))
                .eq(Integer::from(-1))
        );
    }

    #[test]
    fn test_index_of_subsequence_longer_than_sequence() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .index_of(String::from("HelloWorld"), Integer::from(0))
                .eq(Integer::from(-1))
        );
    }

    #[test]
    fn test_substr() {
        use crate::{Integer, String, smt::SMT};

        assert!(
            *String::from("Hello")
                .substr(Integer::from(1), Integer::from(3))
                .eq(String::from("ell"))
        );
        assert!(
            *String::from("Hello")
                .substr(Integer::from(0), Integer::from(5))
                .eq(String::from("Hello"))
        );
        assert!(
            *String::from("Hello")
                .substr(Integer::from(0), Integer::from(2))
                .eq(String::from("He"))
        );
        assert!(
            *String::from("Hello")
                .substr(Integer::from(3), Integer::from(100))
                .eq(String::from("lo"))
        );
        assert!(
            *String::from("Hello")
                .substr(Integer::from(2), Integer::from(0))
                .eq(String::from(""))
        );
        assert!(
            *String::from("Hello")
                .substr(Integer::from(4), Integer::from(1))
                .eq(String::from("o"))
        );
        assert!(
            *String::from("")
                .substr(Integer::from(0), Integer::from(0))
                .eq(String::from(""))
        );
    }

    #[test]
    fn test_substr_negative_offset() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .substr(Integer::from(-1), Integer::from(3))
                .eq(String::new())
        );
    }

    #[test]
    fn test_substr_offset_beyond_length() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .substr(Integer::from(10), Integer::from(1))
                .eq(String::new())
        );
    }

    #[test]
    fn test_substr_negative_length() {
        use crate::{Integer, String, smt::SMT};
        assert!(
            *String::from("Hello")
                .substr(Integer::from(0), Integer::from(-1))
                .eq(String::new())
        );
    }

    #[test]
    fn test_contains() {
        use crate::String;

        assert!(*String::from("Hello").contains(String::from("ell")));
        assert!(*String::from("Hello").contains(String::from("Hello")));
        assert!(*String::from("Hello").contains(String::from("")));
        assert!(*String::from("Hello").contains(String::from("xyz")).not());
        assert!(*String::from("").contains(String::from("a")).not());
        assert!(*String::from("").contains(String::from("")));
    }

    #[test]
    fn test_starts_with_ends_with() {
        use crate::String;

        assert!(*String::from("Hello").starts_with(String::from("He")));
        assert!(*String::from("Hello").starts_with(String::from("lo")).not());
        assert!(*String::from("Hello").starts_with(String::from("")));
        assert!(*String::from("Hello").starts_with(String::from("Hello")));
        assert!(
            *String::from("Hello")
                .starts_with(String::from("Helloo"))
                .not()
        );

        assert!(*String::from("Hello").ends_with(String::from("lo")));
        assert!(*String::from("Hello").ends_with(String::from("He")).not());
        assert!(*String::from("Hello").ends_with(String::from("")));
        assert!(*String::from("Hello").ends_with(String::from("Hello")));
        assert!(
            *String::from("Hello")
                .ends_with(String::from("HHello"))
                .not()
        );
    }

    #[test]
    fn test_is_digit() {
        use crate::String;

        assert!(*String::from("0").is_digit());
        assert!(*String::from("5").is_digit());
        assert!(*String::from("9").is_digit());
        assert!(*String::from("a").is_digit().not());
        assert!(*String::from("A").is_digit().not());
        assert!(*String::from("").is_digit().not());
        assert!(*String::from("12").is_digit().not());
        assert!(*String::from(" ").is_digit().not());
    }

    #[test]
    fn test_str_to_int() {
        use crate::{Integer, String, smt::SMT};

        assert!(*String::from("123").to_int().eq(Integer::from(123)));
        assert!(*String::from("0").to_int().eq(Integer::from(0)));
        assert!(*String::from("-5").to_int().eq(Integer::from(-1)));
    }

    #[test]
    fn test_str_to_int_empty() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from("").to_int().eq(Integer::from(-1)));
    }

    #[test]
    fn test_str_to_int_non_integer() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from("abc").to_int().eq(Integer::from(-1)));
    }

    #[test]
    fn test_str_to_int_non_integer2() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from("12a").to_int().eq(Integer::from(-1)));
    }

    #[test]
    fn test_str_to_int_non_integer3() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from(" 5").to_int().eq(Integer::from(-1)));
    }

    #[test]
    fn test_str_from_int() {
        use crate::{Integer, String, smt::SMT};

        assert!(*String::from_int(Integer::from(123)).eq(String::from("123")));
        assert!(*String::from_int(Integer::from(0)).eq(String::from("0")));
        assert!(*String::from_int(Integer::from(-5)).eq(String::new()));
        assert!(*String::from_int(Integer::from(-1)).eq(String::new()));
    }

    #[test]
    fn test_is_empty() {
        use crate::String;

        assert!(*String::from("").is_empty());
        assert!(*String::from("a").is_empty().not());
        assert!(*String::from("Hello").is_empty().not());
        assert!(*String::from(" ").is_empty().not());
    }

    #[test]
    fn test_replace() {
        use crate::{String, smt::SMT};

        assert!(
            *String::from("abcabc")
                .replace(String::from("b"), String::from("X"))
                .eq(String::from("aXcabc"))
        );
        assert!(
            *String::from("Hello")
                .replace(String::from("xyz"), String::from("X"))
                .eq(String::from("Hello"))
        );
        assert!(
            *String::from("Hello")
                .replace(String::from(""), String::from("X"))
                .eq(String::from("XHello"))
        );
        assert!(
            *String::from("Hello")
                .replace(String::from("l"), String::from(""))
                .eq(String::from("Helo"))
        );
        assert!(
            *String::from("")
                .replace(String::from("a"), String::from("b"))
                .eq(String::from(""))
        );
    }

    #[test]
    fn test_replace_all() {
        use crate::{String, smt::SMT};

        assert!(
            *String::from("abcabc")
                .replace_all(String::from("b"), String::from("X"))
                .eq(String::from("aXcaXc"))
        );
        assert!(
            *String::from("Hello")
                .replace_all(String::from("xyz"), String::from("X"))
                .eq(String::from("Hello"))
        );
        assert!(
            *String::from("Hello")
                .replace_all(String::from(""), String::from("X"))
                .eq(String::from("Hello"))
        );
        assert!(
            *String::from("Hello")
                .replace_all(String::from("l"), String::from(""))
                .eq(String::from("Heo"))
        );
    }

    #[test]
    fn test_from_code() {
        use crate::{Integer, String, smt::SMT};

        assert!(*String::from_code(Integer::from(72)).eq(String::from("H")));
        assert!(*String::from_code(Integer::from(48)).eq(String::from("0")));
        assert!(*String::from_code(Integer::from(32)).eq(String::from(" ")));
        assert!(*String::from_code(Integer::from(97)).eq(String::from("a")));
    }

    #[test]
    #[should_panic]
    fn test_from_code_invalid() {
        use crate::{Integer, String};
        // surrogate range — not a valid Unicode scalar
        String::from_code(Integer::from(0xD800));
    }

    #[test]
    fn test_to_code() {
        use crate::{Integer, String, smt::SMT};

        assert!(*String::from("H").to_code().eq(Integer::from(72)));
        assert!(*String::from("0").to_code().eq(Integer::from(48)));
        assert!(*String::from(" ").to_code().eq(Integer::from(32)));
        assert!(*String::from("a").to_code().eq(Integer::from(97)));
    }

    #[test]
    fn test_to_code_empty() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from("").to_code().eq(Integer::from(-1)));
    }

    #[test]
    fn test_to_code_multi_char() {
        use crate::{Integer, String, smt::SMT};
        assert!(*String::from("ab").to_code().eq(Integer::from(-1)));
    }
}

//! String datatype and operations (these operations are ASCII-only sound).
//! Z3's string theory counts UTF-8 bytes, while Rust's string operations count Unicode code points.

use crate::{Boolean, Integer, String, smt::SMT};
use internment::Intern;
use num_bigint::BigInt;
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
    /// Creates a new empty string `String::new()`
    /// transpiles to `(declare-const s String) (assert (= s ""))`
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
    pub fn at(self, index: Integer) -> Self {
        index
            .inner
            .to_usize()
            .and_then(|idx| self.inner.chars().nth(idx))
            .map(|char_at| Self::from(char_at.to_string()))
            .unwrap()
    }

    /// `(str.indexof s substr offset)`.
    pub fn index_of(self, substr: Self, offset: Integer) -> Integer {
        let start_char = offset.inner.to_usize().unwrap();

        let start_byte = self.inner.char_indices().nth(start_char).unwrap().0;

        let found_byte_idx = self.inner[start_byte..]
            .find(substr.inner.as_ref())
            .unwrap();
        Integer::from(self.inner[..(start_byte + found_byte_idx)].chars().count())
    }

    /// `(str.indexof s substr)`
    pub fn index_of_default(self, substr: Self) -> Integer {
        self.index_of(substr, Integer::from(0))
    }

    /// `(str.substr s offset length)`
    /// Returns a substring starting at `offset` with the given `length`.
    ///
    /// # Panics
    /// - Panics if `offset` or `length` are negative or cannot be converted to usize
    /// - Panics if `offset` is beyond the string length
    ///
    /// Note: If `length` extends beyond the string, returns available characters (does not panic).
    pub fn substr(self, offset: Integer, length: Integer) -> Self {
        let start_char = offset.inner.to_usize().unwrap();
        let len = length.inner.to_usize().unwrap();

        let chars: Vec<char> = self.inner.chars().collect();
        let end_char = (start_char + len).min(chars.len());
        let substring: std::string::String = chars[start_char..end_char].iter().collect();

        Self::from(substring)
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
    /// Z3's str.to_int only handles non-negative decimal integers & returns -1 for any string that is not a non-negative integer (including negative numbers).
    /// but this method panics in case of non integer strings and gives the correct result for negative integers.
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.as_ref().parse::<BigInt>().unwrap()),
        }
    }

    /// `(str.from_int i)`
    /// Z3's str.from_int returns "" for negative integers.
    pub fn from_int(i: Integer) -> Self {
        Self::from(i.inner.to_string())
    }

    /// Lexicographical less than or equal to
    /// For example, "a" <= "b" and "aa" <= "ab" and "a" <= "aa" etc.
    pub fn le(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() <= rhs.inner.as_ref()).into()
    }

    /// Lexicographical less than
    pub fn lt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() < rhs.inner.as_ref()).into()
    }

    /// Lexicographical greater than or equal to
    pub fn ge(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() >= rhs.inner.as_ref()).into()
    }

    /// Lexicographical greater than
    pub fn gt(self, rhs: Self) -> Boolean {
        (self.inner.as_ref() > rhs.inner.as_ref()).into()
    }

    /// `(str.is_digit s)`
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

    /// checks if the string is empty: `v.is_empty()`
    /// transpiles to `(= s "")`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// str.from_code i
    pub fn from_code(code: Integer) -> Self {
        let raw_val = *code.to_u32().inner;
        let c = char::from_u32(raw_val).unwrap();
        Self::from(c.to_string())
    }

    /// str.to_code s
    pub fn to_code(self) -> Integer {
        if self.inner.as_ref().is_empty() {
            panic!("Cannot get code point of an empty string");
        }

        if self.inner.as_ref().chars().count() > 1 {
            panic!("Cannot get code point of a string with more than one character");
        }

        let c = self.inner.as_ref().chars().next().unwrap();
        Integer::from(c as u32)
    }

    /// iterator
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
    fn test_string_length() {
        use crate::{Integer, String, smt::SMT};
        // test to code
        let s = String::from("😀");
        assert!(*s.to_code().eq(Integer::from(128512)));
    }

    #[test]
    fn test_string_length2() {
        use crate::{Integer, String, smt::SMT};
        let s1 = String::from("😀");
        let s2 = String::from("Hello");
        assert!(*s1.length().eq(Integer::from(1)));
        assert!(*s2.length().eq(Integer::from(5)));
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
    #[should_panic]
    fn test_index_of_not_found() {
        use crate::{Integer, String};
        String::from("Hello").index_of(String::from("xyz"), Integer::from(0));
    }

    #[test]
    #[should_panic]
    fn test_index_of_offset_out_of_bounds() {
        use crate::{Integer, String};
        String::from("Hello").index_of(String::from("l"), Integer::from(10));
    }

    #[test]
    #[should_panic]
    fn test_index_of_negative_offset() {
        use crate::{Integer, String};
        String::from("Hello").index_of(String::from("l"), Integer::from(-1));
    }

    #[test]
    #[should_panic]
    fn test_index_of_subsequence_longer_than_sequence() {
        use crate::{Integer, String};
        String::from("Hello").index_of(String::from("HelloWorld"), Integer::from(0));
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
    #[should_panic]
    fn test_substr_negative_offset() {
        use crate::{Integer, String};
        String::from("Hello").substr(Integer::from(-1), Integer::from(3));
    }

    #[test]
    #[should_panic]
    fn test_substr_offset_beyond_length() {
        use crate::{Integer, String};
        String::from("Hello").substr(Integer::from(10), Integer::from(1));
    }

    #[test]
    #[should_panic]
    fn test_substr_negative_length() {
        use crate::{Integer, String};
        String::from("Hello").substr(Integer::from(0), Integer::from(-1));
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
        assert!(*String::from("-5").to_int().eq(Integer::from(-5)));
    }

    #[test]
    #[should_panic]
    fn test_str_to_int_empty() {
        use crate::String;
        String::from("").to_int();
    }

    #[test]
    #[should_panic]
    fn test_str_to_int_non_integer() {
        use crate::String;
        String::from("abc").to_int();
    }

    #[test]
    #[should_panic]
    fn test_str_to_int_non_integer2() {
        use crate::String;
        String::from("12a").to_int();
    }

    #[test]
    #[should_panic]
    fn test_str_to_int_non_integer3() {
        use crate::String;
        String::from(" 5").to_int();
    }

    #[test]
    fn test_str_from_int() {
        use crate::{Integer, String, smt::SMT};

        assert!(*String::from_int(Integer::from(123)).eq(String::from("123")));
        assert!(*String::from_int(Integer::from(0)).eq(String::from("0")));
        assert!(*String::from_int(Integer::from(-5)).eq(String::from("-5")));
        assert!(*String::from_int(Integer::from(-1)).eq(String::from("-1")));
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
    #[should_panic(expected = "Cannot get code point of an empty string")]
    fn test_to_code_empty() {
        use crate::String;
        String::from("").to_code();
    }

    #[test]
    #[should_panic(expected = "Cannot get code point of a string with more than one character")]
    fn test_to_code_multi_char() {
        use crate::String;
        String::from("ab").to_code();
    }
}

//! String datatype and operations

use crate::{Boolean, Integer, String, U32, smt::SMT};
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

    /// This directly corresponds to the `(str.len s)`
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
    /// Returns the character at the given index.
    /// # Panics
    /// Panics if the index is out of bounds.
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
    pub fn to_int(self) -> Integer {
        Integer {
            inner: Intern::new(self.inner.parse::<BigInt>().unwrap()),
        }
    }

    /// `(str.from_int i)`
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
        self.length().eq(Integer::from(1)).and(
            self.at(Integer::from(0))
                .inner
                .as_ref()
                .chars()
                .next()
                .unwrap()
                .is_ascii_digit()
                .into(),
        )
    }

    /// str.from_code i
    pub fn from_code(code: U32) -> Self {
        let raw_val = *code.inner;
        let c = char::from_u32(raw_val).unwrap();
        Self::from(c.to_string())
    }

    /// str.to_code s
    pub fn to_code(self) -> U32 {
        if self.inner.as_ref().is_empty() {
            panic!("Cannot get code point of an empty string");
        }

        if self.inner.as_ref().chars().count() > 1 {
            panic!("Cannot get code point of a string with more than one character");
        }

        let c = self.inner.as_ref().chars().next().unwrap();
        U32::from(c as u32)
    }

    /// checks if the string is empty: `v.is_empty()`
    /// transpiles to `(= s "")`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
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
        let replaced = self.inner.replace(src.inner.as_ref(), dst.inner.as_ref());
        Self::from(replaced)
    }
}

mod tests {
    #[test]
    fn test_string_length() {
        use crate::{String, U32, smt::SMT};
        // test to code
        let s = String::from("😀");
        assert!(*s.to_code().eq(U32::from(128512)));
    }
}

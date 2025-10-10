use crate::{Boolean, Integer, String};
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
    /// Creates a new empty string
    pub fn new() -> Self {
        Self {
            inner: Intern::new(std::string::String::new()),
        }
    }

    /// This directly corresponds to the `(str.len s)`
    pub fn length(self) -> Integer {
        Integer::from(self.inner.chars().count())
    }

    /// append a character to the end of the string
    pub fn append(self, c: char) -> Self {
        let mut new_str = self.inner.as_ref().clone();
        new_str.push(c);
        Self {
            inner: Intern::new(new_str),
        }
    }

    /// `(str.++ s1 s2)`
    pub fn concat(self, rhs: Self) -> Self {
        let mut new_str = self.inner.as_ref().clone();
        new_str.push_str(rhs.inner.as_ref());
        Self {
            inner: Intern::new(new_str),
        }
    }

    /// This can be modeled by the transpiler as `(seq.at s i)` for the character,
    /// wrapped in an `(ite ...)` expression to handle the out-of-bounds case.
    pub fn at(self, index: Integer) -> Option<Self> {
        index
            .inner
            .to_usize()
            .and_then(|idx| self.inner.chars().nth(idx))
            .map(|char_at| Self::from(char_at.to_string()))
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

    /// checks if the string is empty: `v.is_empty()`
    pub fn is_empty(self) -> Boolean {
        self.inner.is_empty().into()
    }

    /// iterator
    pub fn to_chars(self) -> Vec<Self> {
        self.inner
            .chars()
            .map(|c| Self::from(c.to_string()))
            .collect()
    }

    /// `(seq.extract s offset length)`
    pub fn substr(self, offset: Integer, length: Integer) -> Option<Self> {
        let start = offset.inner.to_usize()?;
        let len = length.inner.to_usize()?;

        let collected: std::string::String = self.inner.chars().skip(start).take(len).collect();

        // If the collected length is less than requested, the range was out of bounds.
        if collected.chars().count() < len {
            None
        } else {
            Some(Self::from(collected))
        }
    }

    /// `(str.replace s src dst)`.
    pub fn replace(self, src: Self, dst: Self) -> Self {
        let replaced = self
            .inner
            .replacen(src.inner.as_ref(), dst.inner.as_ref(), 1);
        Self::from(replaced)
    }

    /// `(str.indexof s substr offset)`.
    pub fn index_of(self, substr: Self, offset: Integer) -> Option<Integer> {
        let start_char = offset.inner.to_usize()?;

        let start_byte = match self.inner.char_indices().nth(start_char) {
            Some((byte_idx, _)) => byte_idx,
            None if start_char == self.inner.chars().count() => self.inner.len(),
            None => None?,
        };

        if let Some(found_byte_idx) = self.inner[start_byte..].find(substr.inner.as_ref()) {
            let total_byte_idx = start_byte + found_byte_idx;
            Some(Integer::from(self.inner[..total_byte_idx].chars().count()))
        } else {
            None
        }
    }

    /// `(str.to_int s)`
    pub fn to_int(self) -> Option<Integer> {
        match self.inner.parse::<BigInt>() {
            Ok(val) => Some(Integer {
                inner: Intern::new(val),
            }),
            Err(_) => None,
        }
    }

    /// `(str.from_int i)`
    pub fn from_int(i: Integer) -> Self {
        Self::from(i.inner.to_string())
    }
}

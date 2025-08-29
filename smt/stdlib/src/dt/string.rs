use crate::{Boolean, Integer, Text, order_operator};
use internment::Intern;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;

impl From<&'static str> for Text {
    fn from(c: &'static str) -> Self {
        Self {
            inner: Intern::new(c.to_string()),
        }
    }
}

impl Text {
    pub fn at_index(self, index: Integer) -> Self {
        let idx = index
            .inner
            .as_ref()
            .to_usize()
            .expect("Index out of bounds");
        let char_at = self.inner.as_ref().chars().nth(idx).unwrap_or('\0');
        Self {
            inner: Intern::new(char_at.to_string()),
        }
    }

    pub fn concat(self, rhs: Self) -> Self {
        Self {
            inner: Intern::new(format!("{}{}", self.inner.as_ref(), rhs.inner.as_ref())),
        }
    }

    pub fn length(self) -> Integer {
        Integer {
            inner: Intern::new(BigInt::from(self.inner.as_ref().len())),
        }
    }

    pub fn contains(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner.as_ref().contains(rhs.inner.as_ref()),
        }
    }

    pub fn starts_with(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner.as_ref().starts_with(rhs.inner.as_ref()),
        }
    }

    pub fn ends_with(self, rhs: Self) -> Boolean {
        Boolean {
            inner: self.inner.as_ref().ends_with(rhs.inner.as_ref()),
        }
    }
}

order_operator!(Text, lt, le, ge, gt);

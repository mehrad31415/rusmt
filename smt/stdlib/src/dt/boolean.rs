use crate::Boolean;
use std::ops::Deref;

impl Deref for Boolean {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<bool> for Boolean {
    fn from(c: bool) -> Self {
        Self { inner: c }
    }
}

/// true
pub const TRUE: Boolean = Boolean { inner: true };
/// false
pub const FALSE: Boolean = Boolean { inner: false };

/// All methods are defined such that they take ownership of the self parameter.
impl Boolean {
    /// logical NOT
    pub fn not(self) -> Self {
        Self { inner: !self.inner }
    }

    /// logical AND
    pub fn and(self, rhs: Self) -> Self {
        Self {
            inner: self.inner && rhs.inner,
        }
    }

    /// logical OR
    pub fn or(self, rhs: Self) -> Self {
        Self {
            inner: self.inner || rhs.inner,
        }
    }

    /// logical XOR
    pub fn xor(self, rhs: Self) -> Self {
        Self {
            inner: self.inner ^ rhs.inner,
        }
    }

    /// logical IMPLIES
    pub fn implies(self, rhs: Self) -> Self {
        Self {
            inner: !self.inner || rhs.inner,
        }
    }

    /// logical IFF
    pub fn iff(self, rhs: Self) -> Self {
        Self {
            inner: (self.inner && rhs.inner) || (!self.inner && !rhs.inner),
        }
    }
}

//! SMT Boolean type and operations.

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
    /// NOT
    pub fn not(self) -> Self {
        Self { inner: !self.inner }
    }

    /// AND
    pub fn and(self, rhs: Self) -> Self {
        Self {
            inner: self.inner && rhs.inner,
        }
    }

    /// OR
    pub fn or(self, rhs: Self) -> Self {
        Self {
            inner: self.inner || rhs.inner,
        }
    }

    /// XOR
    pub fn xor(self, rhs: Self) -> Self {
        Self {
            inner: self.inner ^ rhs.inner,
        }
    }

    /// NAND: NOT AND
    pub fn nand(self, rhs: Self) -> Self {
        self.and(rhs).not()
    }

    /// NOR: NOT OR
    pub fn nor(self, rhs: Self) -> Self {
        self.or(rhs).not()
    }

    /// XNOR: NOT XOR (equivalence)
    pub fn xnor(self, rhs: Self) -> Self {
        self.xor(rhs).not()
    }

    /// IMPLIES
    pub fn implies(self, rhs: Self) -> Self {
        Self {
            inner: !self.inner || rhs.inner,
        }
    }

    /// IFF
    pub fn iff(self, rhs: Self) -> Self {
        Self {
            inner: self.inner == rhs.inner,
        }
    }
}

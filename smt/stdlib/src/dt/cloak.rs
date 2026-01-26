//! SMT Cloak type to prevent cyclic dependencies in ADTs.

use crate::{Cloak, dt::SMTWrap, smt::SMT};
use internment::Intern;

/// Cloak is used to prevent cyclic dependencies in Abstract Data Types (ADTs).
/// ILLEGAL - The size of Statement is infinite!
/// pub enum Statement {
///     Assign(String, Expr),
///     Sequence(Statement, Statement),
/// }
/// LEGAL
/// pub enum Statement {
///     Assign(String, Expr),
///     Sequence(Cloak<Statement>, Cloak<Statement>),
/// }
impl<T: SMT> Cloak<T> {
    /// `Cloak::shield(Integer::from(1))`
    pub fn shield(t: T) -> Self {
        Self {
            inner: Intern::new(SMTWrap(t)),
        }
    }
    /// `let a = Cloak::shield(Integer::from(1)).reveal()` is of type Integer with value 1
    pub fn reveal(self) -> T {
        self.inner.0
    }
}

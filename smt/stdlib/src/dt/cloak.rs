use crate::{Cloak, dt::SMTWrap, smt::SMT};
use internment::Intern;

impl<T: SMT> Cloak<T> {
    /// operation: `Cloak::shield(Integer::from(1))`
    pub fn shield(t: T) -> Self {
        Self {
            inner: Intern::new(SMTWrap(t)),
        }
    }
    /// operation: `let a = Cloak::shield(Integer::from(1)).reveal(); // a is of type Integer with value 1`
    pub fn reveal(self) -> T {
        self.inner.0
    }
}

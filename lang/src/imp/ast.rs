//! Abstract syntax for IMP/WHILE.
//!
//! ```text
//! Aexp (a) ::= n | X | a0 + a1 | a0 - a1 | a0 * a1 | a0 / a1
//! Bexp (b) ::= true | false | a0 == a1 | a0 <= a1 | not b | b0 and b1 | b0 or b1
//! Com  (c) ::= skip | X := a | c0 ; c1
//!            | if b then c0 else c1
//!            | while b do c
//! ```

use rusmt_smt_remark_derive::smt_type;
use rusmt_smt_stdlib::smt::SMT;
use rusmt_smt_stdlib::{Cloak, I64, String};

/// Arithmetic expression.
#[smt_type]
pub enum Aexp {
    /// Integer literal `n` (signed 64-bit).
    Num(I64),
    /// Variable reference `X` (location identifier).
    Var(String),
    /// `a0 + a1`
    Add(Cloak<Aexp>, Cloak<Aexp>),
    /// `a0 - a1`
    Sub(Cloak<Aexp>, Cloak<Aexp>),
    /// `a0 * a1`
    Mul(Cloak<Aexp>, Cloak<Aexp>),
    /// `a0 / a1` -- this is an extension to the original IMP/WHILE language.
    Div(Cloak<Aexp>, Cloak<Aexp>),
}

/// Boolean expression.
#[smt_type]
pub enum Bexp {
    /// `true`
    True,
    /// `false`
    False,
    /// `a0 == a1`
    Eq(Cloak<Aexp>, Cloak<Aexp>),
    /// `a0 <= a1`
    Le(Cloak<Aexp>, Cloak<Aexp>),
    /// `not b`
    Not(Cloak<Bexp>),
    /// `b0 and b1`
    And(Cloak<Bexp>, Cloak<Bexp>),
    /// `b0 or b1`
    Or(Cloak<Bexp>, Cloak<Bexp>),
}

/// Command.
#[smt_type]
pub enum Com {
    /// `skip`
    Skip,
    /// `X := a`
    Assign(String, Cloak<Aexp>),
    /// `c0 ; c1`
    Seq(Cloak<Com>, Cloak<Com>),
    /// `if b then c0 else c1`
    If(Cloak<Bexp>, Cloak<Com>, Cloak<Com>),
    /// `while b do c`
    While(Cloak<Bexp>, Cloak<Com>),
}

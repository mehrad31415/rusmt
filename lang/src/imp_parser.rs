//! # Grammar
//!
//! Whitespace is insignificant; `//` starts a comment that runs to end of line.
//! Within each block, operators listed earlier bind *looser* (lower precedence):
//!
//! ```text
//! program ::= com EOF
//!
//! com     ::= stmt (";" stmt)*                      // ";" separates statements
//! stmt    ::= "skip"
//!           | "if" bexp "then" stmt "else" stmt
//!           | "while" bexp "do" stmt
//!           | "(" com ")"                           // grouping / blocks
//!           | ident ":=" aexp                       // assignment
//!
//! aexp    ::= sum
//! sum     ::= product (("+" | "-") product)*        // left-associative
//! product ::= factor (("*" | "/") factor)*          // left-associative
//! factor  ::= "(" aexp ")" | "-"? number | ident
//!
//! bexp    ::= disj
//! disj    ::= conj ("or" conj)*                     // left-associative
//! conj    ::= neg  ("and" neg)*                     // left-associative
//! neg     ::= "not" neg | atom
//! atom    ::= "true" | "false"
//!           | "(" bexp ")"                          // parenthesised boolean
//!           | aexp ("==" | "<=") aexp               // comparison
//! ```
//!

use crate::imp::ast::{Aexp, Bexp, Com};
use rusmt_smt_stdlib::{Array, Cloak, I64, String as SmtString, smt::SMT};

/// Every parsing step yields the node it built, or a human-readable error
/// (a message plus the byte offset where parsing stuck).
type ParseResult<T> = Result<T, std::string::String>;

/// Tokens for the IMP parser.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Eof,
    Semicolon,
    Skip,
    If,
    Then,
    Else,
    While,
    Do,
    LParen,
    RParen,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Or,
    And,
    Not,
    True,
    False,
    EqEq,
    Leq,
    Ident(String),
    Number(i64),
}

/// Lex the source code into a vector of tokens.
fn lex(src: &str) -> ParseResult<Vec<Token>> {
    let mut chars = src.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        match c {
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }

            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else {
                    tokens.push(Token::Slash);
                }
            }
            ':' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Assign);
                } else {
                    return Err(format!(
                        "expected ':=', got ':' followed by {:?}",
                        chars.peek()
                    ));
                }
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqEq);
                } else {
                    return Err(format!(
                        "expected '==', got '=' followed by {:?}",
                        chars.peek()
                    ));
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Leq);
                } else {
                    return Err(format!(
                        "expected '<=', got '<' followed by {:?}",
                        chars.peek()
                    ));
                }
            }

            '0'..='9' => {
                let mut digits = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        digits.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let n: i64 = digits
                    .parse()
                    .map_err(|_| format!("integer literal out of range: {digits}"))?;
                tokens.push(Token::Number(n));
            }

            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(match ident.as_str() {
                    "skip" => Token::Skip,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "do" => Token::Do,
                    "true" => Token::True,
                    "false" => Token::False,
                    "not" => Token::Not,
                    "and" => Token::And,
                    "or" => Token::Or,
                    _ => Token::Ident(ident),
                });
            }

            _ => return Err(format!("unexpected character: {:?}", c)),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

/// A parser over the IMP tokens.
struct Parser<'a> {
    toks: &'a Vec<Token>,
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser with the given tokens.
    fn new(toks: &'a Vec<Token>) -> Self {
        Parser { toks, pos: 0 }
    }

    /// Get the current token.
    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    /// Consume and return the current token.
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }

    /// Check if the current token is the same as the given token.
    fn at(&self, t: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(t)
    }

    /// Consume the current token if it is the same as the given token.
    fn eat(&mut self, t: &Token) -> bool {
        if self.at(t) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current token if it is the same as the given token.
    fn expect(&mut self, t: Token) -> ParseResult<()> {
        if self.at(&t) {
            self.bump();
            Ok(())
        } else {
            Err(format!("expected {:?}, found {:?}", t, self.peek()))
        }
    }

    /// Parse an arithmetic expression.
    /// the grammar is aexp ::= sum
    fn aexp(&mut self) -> ParseResult<Aexp> {
        self.sum()
    }

    /// Parse a sum expression.
    /// the grammar is sum ::= product (("+" | "-") product)*
    fn sum(&mut self) -> ParseResult<Aexp> {
        let mut acc = self.product()?;
        loop {
            if self.eat(&Token::Plus) {
                acc = Aexp::Add(shield(acc), shield(self.product()?));
            } else if self.eat(&Token::Minus) {
                acc = Aexp::Sub(shield(acc), shield(self.product()?));
            } else {
                return Ok(acc);
            }
        }
    }

    /// Parse a product expression.
    /// the grammar is product ::= factor (("*" | "/") factor)*
    fn product(&mut self) -> ParseResult<Aexp> {
        let mut acc = self.factor()?;
        loop {
            if self.eat(&Token::Star) {
                acc = Aexp::Mul(shield(acc), shield(self.factor()?));
            } else if self.eat(&Token::Slash) {
                acc = Aexp::Div(shield(acc), shield(self.factor()?));
            } else {
                return Ok(acc);
            }
        }
    }

    /// Parse a factor expression.
    /// the grammar is factor ::= "(" aexp ")" | "-"? number | ident
    fn factor(&mut self) -> ParseResult<Aexp> {
        if self.eat(&Token::LParen) {
            let inner = self.aexp()?;
            self.expect(Token::RParen)?;
            return Ok(inner);
        }
        if self.eat(&Token::Minus) {
            let next = self.bump();
            if let Token::Number(n) = next {
                return Ok(Aexp::Num(I64::from(-n)));
            }
            return Err(format!("expected a number after `-`, found {:?}", next));
        }
        let tok = self.peek().clone();
        match tok {
            Token::Number(n) => {
                self.bump();
                Ok(Aexp::Num(I64::from(n)))
            }
            Token::Ident(name) => {
                self.bump();
                Ok(Aexp::Var(SmtString::from(name.as_str())))
            }
            _ => Err(format!(
                "expected a number, a variable, or `(`, found {:?}",
                self.peek()
            )),
        }
    }

    /// Run `f`; on failure, rewind to where we started and yield `None`. This is
    /// the only place the parser backtracks — see [`Parser::atom`].
    fn attempt<T>(&mut self, f: impl FnOnce(&mut Self) -> ParseResult<T>) -> Option<T> {
        let start = self.pos;
        match f(self) {
            Ok(value) => Some(value),
            Err(_) => {
                self.pos = start;
                None
            }
        }
    }

    /// Parse a command.
    /// the grammar is com ::= stmt (";" stmt)*
    fn com(&mut self) -> ParseResult<Com> {
        let mut acc = self.stmt()?;
        while self.eat(&Token::Semicolon) {
            let next = self.stmt()?;
            acc = Com::Seq(shield(acc), shield(next));
        }
        Ok(acc)
    }

    /// Parse a statement.
    /// the grammar is stmt ::= "skip" | "if" bexp "then" stmt "else" stmt | "while" bexp "do" stmt | "(" com ")" | ident ":=" aexp
    fn stmt(&mut self) -> ParseResult<Com> {
        if self.eat(&Token::Skip) {
            return Ok(Com::Skip);
        }
        if self.eat(&Token::If) {
            let cond = self.bexp()?;
            self.expect(Token::Then)?;
            let then_branch = self.stmt()?;
            self.expect(Token::Else)?;
            let else_branch = self.stmt()?;
            return Ok(Com::If(
                shield(cond),
                shield(then_branch),
                shield(else_branch),
            ));
        }
        if self.eat(&Token::While) {
            let cond = self.bexp()?;
            self.expect(Token::Do)?;
            let body = self.stmt()?;
            return Ok(Com::While(shield(cond), shield(body)));
        }
        if self.eat(&Token::LParen) {
            let inner = self.com()?;
            self.expect(Token::RParen)?;
            return Ok(inner);
        }

        // The only statement left is an assignment `ident := aexp`.
        if let Token::Ident(name) = self.peek().clone() {
            self.bump();
            self.expect(Token::Assign)?;
            let rhs = self.aexp()?;
            return Ok(Com::Assign(SmtString::from(name.as_str()), shield(rhs)));
        }
        Err(format!(
            "expected a statement (skip, assignment, if, while, or `(`)"
        ))
    }

    /// Parse a boolean expression.
    /// the grammar is bexp ::= disj
    fn bexp(&mut self) -> ParseResult<Bexp> {
        self.disj()
    }

    /// Parse a disjunction expression.
    /// the grammar is disj ::= conj ("or" conj)*
    fn disj(&mut self) -> ParseResult<Bexp> {
        let mut acc = self.conj()?;
        while self.eat(&Token::Or) {
            acc = Bexp::Or(shield(acc), shield(self.conj()?));
        }
        Ok(acc)
    }

    /// Parse a conjunction expression.
    /// the grammar is conj ::= neg ("and" neg)*
    fn conj(&mut self) -> ParseResult<Bexp> {
        let mut acc = self.neg()?;
        while self.eat(&Token::And) {
            acc = Bexp::And(shield(acc), shield(self.neg()?));
        }
        Ok(acc)
    }

    /// Parse a negation expression.
    /// the grammar is neg ::= "not" neg | atom
    fn neg(&mut self) -> ParseResult<Bexp> {
        if self.eat(&Token::Not) {
            return Ok(Bexp::Not(shield(self.neg()?)));
        }
        self.atom()
    }

    /// Parse an atom expression.
    /// the grammar is atom ::= "true" | "false" | "(" bexp ")" | aexp ("==" | "<=") aexp
    fn atom(&mut self) -> ParseResult<Bexp> {
        if self.eat(&Token::True) {
            return Ok(Bexp::True);
        }
        if self.eat(&Token::False) {
            return Ok(Bexp::False);
        }
        // A leading `(` is ambiguous: it can open a parenthesised boolean
        //     "(" bexp ")"
        // or be the first arithmetic operand of a comparison
        //     "(" aexp ")" ("==" | "<=") aexp        e.g.  (a + b) == c
        // Try the boolean reading first; if it does not parse, rewind and fall
        // through to the comparison (whose lhs aexp may itself be parenthesised).
        if self.at(&Token::LParen) {
            if let Some(b) = self.attempt(|p| {
                p.expect(Token::LParen)?;
                let b = p.bexp()?;
                p.expect(Token::RParen)?;
                Ok(b)
            }) {
                return Ok(b);
            }
        }
        // comparison: aexp ("==" | "<=") aexp
        let lhs = self.aexp()?;
        if self.eat(&Token::EqEq) {
            Ok(Bexp::Eq(shield(lhs), shield(self.aexp()?)))
        } else if self.eat(&Token::Leq) {
            Ok(Bexp::Le(shield(lhs), shield(self.aexp()?)))
        } else {
            Err(format!("expected `==` or `<=`"))
        }
    }
}

fn shield<T: SMT>(node: T) -> Cloak<T> {
    Cloak::shield(node)
}

/// Parse a complete IMP program; the entire input must be consumed.
pub fn parse_imp_source(src: &str) -> ParseResult<Com> {
    let tokens = lex(src)?;
    let mut p = Parser::new(&tokens);
    let prog = p.com()?;
    p.expect(Token::Eof)?;
    Ok(prog)
}

/// Pretty-print a final IMP store as `var = value` lines, sorted by key.
pub fn format_store(store: Array<SmtString, I64>) -> std::string::String {
    let mut entries: Vec<(std::string::String, i64)> = store
        .iterator()
        .into_iter()
        .map(|k| {
            let v = store.select(k);
            let key_str: std::string::String = format!("{:?}", k)
                .trim_start_matches("String { inner: \"")
                .trim_end_matches("\" }")
                .to_owned();
            let val_i64: i64 = format!("{:?}", v)
                .trim_start_matches("I64 { inner: ")
                .trim_end_matches(" }")
                .parse()
                .unwrap_or(0);
            (key_str, val_i64)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = std::string::String::new();
    for (k, v) in entries {
        out.push_str(&format!("{} = {}\n", k, v));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Com {
        parse_imp_source(src).unwrap_or_else(|e| panic!("expected Ok on {src:?}, got Err: {e}"))
    }

    fn parse_err(src: &str) -> String {
        match parse_imp_source(src) {
            Err(e) => e,
            Ok(_) => panic!("expected Err on {src:?}, got Ok"),
        }
    }

    #[test]
    fn bare_ident_in_assign() {
        parse_ok("x := y");
    }
    #[test]
    fn ident_in_arith() {
        parse_ok("x := x + 5");
    }
    #[test]
    fn unary_minus() {
        parse_ok("x := -5");
    }
    #[test]
    fn simple_assign() {
        parse_ok("x := 5");
    }
    #[test]
    fn paren_lhs_compare() {
        parse_ok("if (x + y) == z then skip else skip");
    }
    #[test]
    fn paren_bool() {
        parse_ok("if (true) then skip else skip");
    }
    #[test]
    fn full_conditional() {
        parse_ok("if x == 0 then skip else x := x - 1");
    }

    // error case
    #[test]
    fn trailing_minus_is_error_not_panic() {
        let _ = parse_err("x := -");
    }
}

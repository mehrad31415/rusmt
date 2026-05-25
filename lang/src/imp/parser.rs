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

/// Parse a complete IMP program; the entire input must be consumed.
pub fn parse_imp_source(src: &str) -> ParseResult<Com> {
    let mut p = Parser::new(src.as_bytes());
    let program = p.com()?;
    p.skip_trivia();
    if p.pos != p.src.len() {
        return Err(p.error("unexpected trailing input"));
    }
    Ok(program)
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

/// A cursor over the source bytes; `pos` is the index of the next unread byte.
struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a [u8]) -> Self {
        Parser { src, pos: 0 }
    }

    /// The next raw byte, without consuming it.
    fn peek_raw(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    /// True if the next raw byte exists and satisfies `pred`.
    fn peek_is(&self, pred: impl Fn(u8) -> bool) -> bool {
        matches!(self.peek_raw(), Some(b) if pred(b))
    }

    /// Skip whitespace and `// ...` line comments. Called by every token helper,
    /// so the grammar rules below can ignore layout entirely.
    fn skip_trivia(&mut self) {
        loop {
            while self.peek_is(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r')) {
                self.pos += 1;
            }
            if self.src[self.pos..].starts_with(b"//") {
                while self.peek_is(|b| b != b'\n') {
                    self.pos += 1;
                }
                continue; // a comment may be followed by more trivia
            }
            break;
        }
    }

    /// Build an error message anchored at the current position.
    fn error(&self, what: &str) -> std::string::String {
        let end = (self.pos + 16).min(self.src.len());
        let near = std::string::String::from_utf8_lossy(&self.src[self.pos..end]);
        format!("{what} at byte {} (near {near:?})", self.pos)
    }

    /// Consume a fixed symbol (operator/punctuation) if it is next.
    fn eat_symbol(&mut self, sym: &str) -> bool {
        self.skip_trivia();
        if self.src[self.pos..].starts_with(sym.as_bytes()) {
            self.pos += sym.len();
            true
        } else {
            false
        }
    }

    /// Is `sym` next? (Skips trivia but does not consume the symbol.)
    fn at_symbol(&mut self, sym: &str) -> bool {
        self.skip_trivia();
        self.src[self.pos..].starts_with(sym.as_bytes())
    }

    /// Consume `sym`, or report an error naming it.
    fn expect_symbol(&mut self, sym: &str) -> ParseResult<()> {
        if self.eat_symbol(sym) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{sym}`")))
        }
    }

    /// Consume a keyword if it is next. Unlike a symbol, a keyword must be a
    /// whole word: `if` must not match the start of the identifier `iffy`.
    fn eat_keyword(&mut self, kw: &str) -> bool {
        self.skip_trivia();
        if !self.src[self.pos..].starts_with(kw.as_bytes()) {
            return false;
        }
        let after = self.pos + kw.len();
        if matches!(self.src.get(after), Some(&b) if is_ident_cont(b)) {
            return false; // it's a longer identifier, not this keyword
        }
        self.pos = after;
        true
    }

    /// Consume `kw`, or report an error naming it.
    fn expect_keyword(&mut self, kw: &str) -> ParseResult<()> {
        if self.eat_keyword(kw) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{kw}`")))
        }
    }

    /// Consume an identifier and return it, or `None` if the next token is not
    /// one. Reserved keywords are never identifiers.
    fn eat_ident(&mut self) -> Option<std::string::String> {
        self.skip_trivia();
        let start = self.pos;
        if !self.peek_is(is_ident_start) {
            return None;
        }
        self.pos += 1;
        while self.peek_is(is_ident_cont) {
            self.pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..self.pos])
            .expect("identifier bytes are ASCII")
            .to_owned();
        if is_keyword(&word) {
            self.pos = start; // hand it back; the caller wanted a name
            return None;
        }
        Some(word)
    }

    /// Consume an integer literal `"-"? digits`. Returns `None` when the next
    /// token is not a number; errors when a `-` has no digits after it or the
    /// value does not fit in `i64`.
    fn eat_number(&mut self) -> ParseResult<Option<i64>> {
        self.skip_trivia();
        let start = self.pos;
        let negative = self.peek_raw() == Some(b'-');
        let digits_start = if negative { start + 1 } else { start };
        let mut end = digits_start;
        while matches!(self.src.get(end), Some(&b) if b.is_ascii_digit()) {
            end += 1;
        }
        if end == digits_start {
            return if negative {
                Err(self.error("expected digits after `-`"))
            } else {
                Ok(None) // not a number at all
            };
        }
        // Parse the sign together with the digits so that `i64::MIN` is accepted.
        let text = std::str::from_utf8(&self.src[start..end]).expect("number bytes are ASCII");
        let value: i64 = text
            .parse()
            .map_err(|e| self.error(&format!("integer literal out of range: {e}")))?;
        self.pos = end;
        Ok(Some(value))
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

    fn com(&mut self) -> ParseResult<Com> {
        let mut acc = self.stmt()?;
        while self.eat_symbol(";") {
            let next = self.stmt()?;
            acc = Com::Seq(shield(acc), shield(next));
        }
        Ok(acc)
    }

    fn stmt(&mut self) -> ParseResult<Com> {
        if self.eat_keyword("skip") {
            return Ok(Com::Skip);
        }
        if self.eat_keyword("if") {
            let cond = self.bexp()?;
            self.expect_keyword("then")?;
            let then_branch = self.stmt()?;
            self.expect_keyword("else")?;
            let else_branch = self.stmt()?;
            return Ok(Com::If(
                shield(cond),
                shield(then_branch),
                shield(else_branch),
            ));
        }
        if self.eat_keyword("while") {
            let cond = self.bexp()?;
            self.expect_keyword("do")?;
            let body = self.stmt()?;
            return Ok(Com::While(shield(cond), shield(body)));
        }
        if self.eat_symbol("(") {
            let inner = self.com()?;
            self.expect_symbol(")")?;
            return Ok(inner);
        }
        // The only statement left is an assignment `ident := aexp`.
        if let Some(name) = self.eat_ident() {
            self.expect_symbol(":=")?;
            let rhs = self.aexp()?;
            return Ok(Com::Assign(SmtString::from(name.as_str()), shield(rhs)));
        }
        Err(self.error("expected a statement (skip, assignment, if, while, or `(`)"))
    }

    fn aexp(&mut self) -> ParseResult<Aexp> {
        self.sum()
    }

    fn sum(&mut self) -> ParseResult<Aexp> {
        let mut acc = self.product()?;
        loop {
            if self.eat_symbol("+") {
                acc = Aexp::Add(shield(acc), shield(self.product()?));
            } else if self.eat_symbol("-") {
                acc = Aexp::Sub(shield(acc), shield(self.product()?));
            } else {
                return Ok(acc);
            }
        }
    }

    fn product(&mut self) -> ParseResult<Aexp> {
        let mut acc = self.factor()?;
        loop {
            if self.eat_symbol("*") {
                acc = Aexp::Mul(shield(acc), shield(self.factor()?));
            } else if self.eat_symbol("/") {
                acc = Aexp::Div(shield(acc), shield(self.factor()?));
            } else {
                return Ok(acc);
            }
        }
    }

    fn factor(&mut self) -> ParseResult<Aexp> {
        if self.eat_symbol("(") {
            let inner = self.aexp()?;
            self.expect_symbol(")")?;
            return Ok(inner);
        }
        if let Some(n) = self.eat_number()? {
            return Ok(Aexp::Num(I64::from(n)));
        }
        if let Some(name) = self.eat_ident() {
            return Ok(Aexp::Var(SmtString::from(name.as_str())));
        }
        Err(self.error("expected a number, a variable, or `(`"))
    }

    fn bexp(&mut self) -> ParseResult<Bexp> {
        self.disj()
    }

    fn disj(&mut self) -> ParseResult<Bexp> {
        let mut acc = self.conj()?;
        while self.eat_keyword("or") {
            acc = Bexp::Or(shield(acc), shield(self.conj()?));
        }
        Ok(acc)
    }

    fn conj(&mut self) -> ParseResult<Bexp> {
        let mut acc = self.neg()?;
        while self.eat_keyword("and") {
            acc = Bexp::And(shield(acc), shield(self.neg()?));
        }
        Ok(acc)
    }

    fn neg(&mut self) -> ParseResult<Bexp> {
        if self.eat_keyword("not") {
            return Ok(Bexp::Not(shield(self.neg()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> ParseResult<Bexp> {
        if self.eat_keyword("true") {
            return Ok(Bexp::True);
        }
        if self.eat_keyword("false") {
            return Ok(Bexp::False);
        }
        // A leading `(` is ambiguous: it can open a parenthesised boolean
        //     "(" bexp ")"
        // or be the first arithmetic operand of a comparison
        //     "(" aexp ")" ("==" | "<=") aexp        e.g.  (a + b) == c
        // Try the boolean reading first; if it does not parse, rewind and fall
        // through to the comparison (whose lhs aexp may itself be parenthesised).
        if self.at_symbol("(") {
            if let Some(b) = self.attempt(|p| {
                p.expect_symbol("(")?;
                let b = p.bexp()?;
                p.expect_symbol(")")?;
                Ok(b)
            }) {
                return Ok(b);
            }
        }
        // comparison: aexp ("==" | "<=") aexp
        let lhs = self.aexp()?;
        if self.eat_symbol("==") {
            Ok(Bexp::Eq(shield(lhs), shield(self.aexp()?)))
        } else if self.eat_symbol("<=") {
            Ok(Bexp::Le(shield(lhs), shield(self.aexp()?)))
        } else {
            Err(self.error("expected `==` or `<=`"))
        }
    }
}

fn shield<T: SMT>(node: T) -> Cloak<T> {
    Cloak::shield(node)
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "skip" | "if" | "then" | "else" | "while" | "do" | "true" | "false" | "not" | "and" | "or"
    )
}

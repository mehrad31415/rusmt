//! Render a synthesized Z3 model of `input_0 : Com` back into `.imp` source.
//!
//! Synthesis writes a raw Z3 model to `synthesis/imp/<backend>/target_*/response.txt`.
//! For a `sat` result the only relevant definition is `input_0` — the synthesized
//! program.

use crate::imp::ast::{Aexp, Bexp, Com};
use rusmt_smt_stdlib::{Cloak, I64, String as SmtString, smt::SMT};
use std::collections::HashMap;

/// Render a whole `response.txt`: `Some(source)` for a `sat` model, `None` for
/// `unsat`/`unknown`/`timeout`/malformed input.
pub fn render_response(response_text: &str) -> Option<String> {
    if response_text.lines().next()?.trim() != "sat" {
        return None;
    }
    model_to_com(response_text).ok().map(|c| com_to_source(&c))
}

/// Parse a Z3 model, extract the `input_0` definition, and rebuild it as a [`Com`].
///
/// Variable names and bitvector literals are kept verbatim from the model; the
/// readable rewriting (name canonicalization, signed decode) happens in
/// [`com_to_source`].
pub fn model_to_com(model_text: &str) -> Result<Com, String> {
    let value = extract_input_0(model_text).ok_or("input_0 not found in model")?;
    Ok(build_com(&resolve(&value, &HashMap::new())))
}

/// Pretty-print a [`Com`] as `.imp` source accepted by `parse_imp_source`.
pub fn com_to_source(c: &Com) -> String {
    Printer::default().com(c)
}

/// Locate the `input_0` value in either serialization: the text backend emits
/// `(define-fun input_0 () Com <value>)`, the API backend emits `input_0 -> <value>`.
fn extract_input_0(model: &str) -> Option<Sexp> {
    if let Some(idx) = model.find("(define-fun input_0") {
        match Reader::new(&model[idx..]).read() {
            // (define-fun input_0 () Com <value>)
            // - items[0] = Atom("define-fun")
            // - items[1] = Atom("input_0")
            // - items[2] = List([]) - ()
            // - items[3] = Atom("Com") - return type
            // - items[4] = program
            Sexp::List(items) => items.into_iter().nth(4),
            _ => None,
        }
    } else {
        let idx = model.find("input_0 ->")?;
        Some(Reader::new(&model[idx + "input_0 ->".len()..]).read())
    }
}

#[derive(Clone)]
enum Sexp {
    Atom(String),
    Str(String),
    List(Vec<Sexp>),
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(s: &'a str) -> Self {
        Reader {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn read(&mut self) -> Sexp {
        self.skip_ws();
        match self.bytes[self.pos] {
            b'(' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    if self.bytes[self.pos] == b')' {
                        self.pos += 1;
                        return Sexp::List(items);
                    }
                    items.push(self.read());
                }
            }
            b'"' => {
                self.pos += 1;
                let mut content = Vec::new();
                loop {
                    match self.bytes[self.pos] {
                        // SMT-LIB escapes a literal quote by doubling it.
                        b'"' if self.bytes.get(self.pos + 1) == Some(&b'"') => {
                            content.push(b'"');
                            self.pos += 2;
                        }
                        b'"' => {
                            self.pos += 1;
                            break;
                        }
                        c => {
                            content.push(c);
                            self.pos += 1;
                        }
                    }
                }
                Sexp::Str(String::from_utf8(content).unwrap())
            }
            _ => {
                let start = self.pos;
                while self.pos < self.bytes.len() {
                    let c = self.bytes[self.pos];
                    if c.is_ascii_whitespace() || c == b'(' || c == b')' || c == b'"' {
                        break;
                    }
                    self.pos += 1;
                }
                let atom = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
                Sexp::Atom(atom.to_owned())
            }
        }
    }
}

/// Inline Z3 `let` bindings: `a!1` style names are shared-subterm aliases, not AST
/// constructors, so substitute each bound value where its name appears.
/// get rid of the let binbdings and substitue the variable with its value.
fn resolve(s: &Sexp, env: &HashMap<String, Sexp>) -> Sexp {
    match s {
        Sexp::Atom(a) => env.get(a).cloned().unwrap_or_else(|| s.clone()),
        Sexp::Str(_) => s.clone(),
        Sexp::List(items) => {
            if let [Sexp::Atom(head), Sexp::List(bindings), body] = items.as_slice() {
                if head == "let" {
                    let mut inner = env.clone();
                    for b in bindings {
                        if let Sexp::List(pair) = b {
                            if let [Sexp::Atom(name), val] = pair.as_slice() {
                                inner.insert(name.clone(), resolve(val, env)); // Bindings in one `let` are evaluated in the enclosing scope (parallel); nesting is handled by recursion.
                            }
                        }
                    }
                    return resolve(body, &inner);
                }
            }
            Sexp::List(items.iter().map(|x| resolve(x, env)).collect())
        }
    }
}

fn head_args(s: &Sexp) -> (&str, &[Sexp]) {
    match s {
        Sexp::Atom(a) => (a.as_str(), &[]),
        Sexp::List(items) => match &items[0] {
            Sexp::Atom(a) => (a.as_str(), &items[1..]),
            _ => unreachable!("constructor head is always an atom"),
        },
        Sexp::Str(_) => unreachable!("a string is never a constructor application"),
    }
}

fn build_com(s: &Sexp) -> Com {
    let (head, args) = head_args(s);
    match head {
        "Com_Skip" => Com::Skip,
        "Com_Assign" => Com::Assign(build_string(&args[0]), shield(build_aexp(&args[1]))),
        "Com_Seq" => Com::Seq(shield(build_com(&args[0])), shield(build_com(&args[1]))),
        "Com_If" => Com::If(
            shield(build_bexp(&args[0])),
            shield(build_com(&args[1])),
            shield(build_com(&args[2])),
        ),
        "Com_While" => Com::While(shield(build_bexp(&args[0])), shield(build_com(&args[1]))),
        other => unreachable!("unknown Com constructor: {other}"),
    }
}

fn build_aexp(s: &Sexp) -> Aexp {
    let (head, args) = head_args(s);
    match head {
        "Aexp_Num" => Aexp::Num(I64::from(decode_bv(&args[0]))),
        "Aexp_Var" => Aexp::Var(build_string(&args[0])),
        "Aexp_Add" => Aexp::Add(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        "Aexp_Sub" => Aexp::Sub(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        "Aexp_Mul" => Aexp::Mul(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        "Aexp_Div" => Aexp::Div(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        other => unreachable!("unknown Aexp constructor: {other}"),
    }
}

fn build_bexp(s: &Sexp) -> Bexp {
    let (head, args) = head_args(s);
    match head {
        "Bexp_True" => Bexp::True,
        "Bexp_False" => Bexp::False,
        "Bexp_Eq" => Bexp::Eq(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        "Bexp_Le" => Bexp::Le(shield(build_aexp(&args[0])), shield(build_aexp(&args[1]))),
        "Bexp_Not" => Bexp::Not(shield(build_bexp(&args[0]))),
        "Bexp_And" => Bexp::And(shield(build_bexp(&args[0])), shield(build_bexp(&args[1]))),
        "Bexp_Or" => Bexp::Or(shield(build_bexp(&args[0])), shield(build_bexp(&args[1]))),
        other => unreachable!("unknown Bexp constructor: {other}"),
    }
}

fn build_string(s: &Sexp) -> SmtString {
    match s {
        Sexp::Str(content) => SmtString::from(content.as_str()),
        _ => unreachable!("constructor field expected a string literal"),
    }
}

/// `#x…` is a 64-bit literal; decode as two's-complement signed `i64`
/// (`#x8000000000000000` -> `i64::MIN`, `#xffffffffffffffff` -> `-1`).
fn decode_bv(s: &Sexp) -> i64 {
    let Sexp::Atom(a) = s else {
        unreachable!("Aexp_Num operand is a bitvector literal");
    };
    u64::from_str_radix(a.strip_prefix("#x").unwrap(), 16).unwrap() as i64
}

fn shield<T: SMT>(node: T) -> Cloak<T> {
    Cloak::shield(node)
}

#[derive(Default)]
struct Printer {
    names: HashMap<String, String>,
    next: usize,
}

impl Printer {
    /// Map each distinct original name to a valid IMP identifier, reusing the same
    /// canonical name for repeated occurrences. Z3 picks degenerate names (empty,
    /// `!0!`, …); the specific name does not affect which marker is reached, so
    /// renaming them to `v0`, `v1`, … keeps the program parseable and replayable.
    fn name(&mut self, s: &SmtString) -> String {
        let raw = name_str(s);
        if let Some(canon) = self.names.get(&raw) {
            return canon.clone();
        }
        let canon = if is_valid_ident(&raw) {
            raw.clone()
        } else {
            let c = format!("v{}", self.next);
            self.next += 1;
            c
        };
        self.names.insert(raw, canon.clone());
        canon
    }

    fn com(&mut self, c: &Com) -> String {
        match c {
            Com::Seq(a, b) => format!("{} ; {}", self.stmt(&a.reveal()), self.stmt(&b.reveal())),
            _ => self.stmt(c),
        }
    }

    /// A command in statement position.
    fn stmt(&mut self, c: &Com) -> String {
        match c {
            Com::Skip => "skip".to_owned(),
            Com::Assign(x, a) => format!("{} := {}", self.name(x), self.aexp(&a.reveal())),
            Com::If(b, t, e) => format!(
                "if {} then {} else {}",
                self.bexp(&b.reveal()),
                self.stmt(&t.reveal()),
                self.stmt(&e.reveal()),
            ),
            Com::While(b, body) => {
                format!(
                    "while {} do {}",
                    self.bexp(&b.reveal()),
                    self.stmt(&body.reveal())
                )
            }
            Com::Seq(..) => format!("({})", self.com(c)),
        }
    }

    fn aexp(&mut self, a: &Aexp) -> String {
        match a {
            Aexp::Num(n) => i64_of(n).to_string(),
            Aexp::Var(x) => self.name(x),
            Aexp::Add(l, r) => format!("({} + {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
            Aexp::Sub(l, r) => format!("({} - {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
            Aexp::Mul(l, r) => format!("({} * {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
            Aexp::Div(l, r) => format!("({} / {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
        }
    }

    fn bexp(&mut self, b: &Bexp) -> String {
        match b {
            Bexp::True => "true".to_owned(),
            Bexp::False => "false".to_owned(),
            Bexp::Eq(l, r) => format!("({} == {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
            Bexp::Le(l, r) => format!("({} <= {})", self.aexp(&l.reveal()), self.aexp(&r.reveal())),
            Bexp::Not(x) => format!("(not {})", self.bexp(&x.reveal())),
            Bexp::And(l, r) => format!(
                "({} and {})",
                self.bexp(&l.reveal()),
                self.bexp(&r.reveal())
            ),
            Bexp::Or(l, r) => format!("({} or {})", self.bexp(&l.reveal()), self.bexp(&r.reveal())),
        }
    }
}

/// Read the inner value of an `I64` / `String`; the stdlib exposes no accessor, so
/// recover it from the `Debug` form, matching `parser::format_store`.
fn i64_of(n: &I64) -> i64 {
    format!("{n:?}")
        .trim_start_matches("I64 { inner: ")
        .trim_end_matches(" }")
        .parse()
        .unwrap()
}

fn name_str(s: &SmtString) -> String {
    format!("{s:?}")
        .trim_start_matches("String { inner: \"")
        .trim_end_matches("\" }")
        .to_owned()
}

fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !is_keyword(name)
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "skip" | "if" | "then" | "else" | "while" | "do" | "true" | "false" | "not" | "and" | "or"
    )
}

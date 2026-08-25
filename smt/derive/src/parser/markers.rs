//! Whole-program marker-name rules.
//!
//! A marker name denotes one branch, so it has exactly one standalone
//! `Path::named`. Inside a `Path::merge` the same call is a *reference* to a
//! marker declared elsewhere, and every referenced name must have such a
//! declaration.
//!
//! Scanning happens over each parsed file, before monomorphization, so every
//! site is seen exactly once. `rusmt_smt_remark::marker` applies the same rules
//! to a single `#[smt_fn]` body, which reports them in the editor; this is the
//! authority, since only here is the whole program visible.

use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, ExprPath, File, Lit, Result};

/// Where a marker was written.
#[derive(Clone, Debug)]
pub struct Site {
    file: PathBuf,
    line: usize,
    column: usize,
}

impl Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.file.display(),
            self.line,
            self.column + 1
        )
    }
}

/// A marker name written as `"x"` or `String::from("x")`, if `expr` is one.
fn marker_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Some(s.value()),
        Expr::Call(ExprCall { func, args, .. }) => {
            let is_string_from = matches!(
                func.as_ref(),
                Expr::Path(ExprPath { path, .. })
                    if path.segments.len() >= 2
                        && path.segments[path.segments.len() - 2].ident == "String"
                        && path.segments[path.segments.len() - 1].ident == "from"
            );
            if !is_string_from {
                return None;
            }
            marker_name(args.iter().next()?)
        }
        _ => None,
    }
}

/// The name in a `Path::named(..)` call, if `expr` is one.
fn path_named_call(expr: &Expr) -> Option<String> {
    let Expr::Call(ExprCall { func, args, .. }) = expr else {
        return None;
    };
    let Expr::Path(ExprPath { path, .. }) = func.as_ref() else {
        return None;
    };
    let mut segments = path.segments.iter().rev();
    if segments.next()?.ident != "named" || segments.next()?.ident != "Path" {
        return None;
    }
    marker_name(args.iter().next()?)
}

/// Declarations and merge references collected across every scanned file.
#[derive(Default, Debug)]
pub struct MarkerScan {
    declared: BTreeMap<String, Vec<Site>>,
    referenced: BTreeMap<String, Vec<Site>>,
}

impl MarkerScan {
    /// Collect every marker site in one parsed file.
    pub fn scan(&mut self, file: &File, path: &Path) {
        let mut visitor = FileVisitor {
            scan: self,
            path: path.to_path_buf(),
        };
        visitor.visit_file(file);
    }

    /// Enforce the whole-program rules over everything scanned so far.
    pub fn validate(&self) -> Result<()> {
        // Two distinct names hashing to one id would share a bit and be
        // indistinguishable to both the query and concrete replay.
        let mut by_id: BTreeMap<usize, &str> = BTreeMap::new();
        for name in self.declared.keys().chain(self.referenced.keys()) {
            let id = rusmt_smt_stdlib::path::marker_id(name);
            match by_id.get(&id) {
                Some(prev) if *prev != name.as_str() => {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!(
                            "marker id collision: `{prev}` and `{name}` both hash \
                             to {id}, so they would share one bit and could not be \
                             told apart. Rename one of them."
                        ),
                    ));
                }
                _ => {
                    by_id.insert(id, name);
                }
            }
        }

        for (name, sites) in &self.declared {
            if sites.len() > 1 {
                let listed: Vec<String> = sites.iter().map(Site::to_string).collect();
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "marker `{name}` is declared at {} sites:\n  {}\n\
                         a marker name denotes one branch, and both sites set the \
                         same bit, so only one of them can ever be targeted. \
                         Rename one, or use `Path::merge` if a single branch \
                         raises both.",
                        sites.len(),
                        listed.join("\n  ")
                    ),
                ));
            }
        }

        for (name, sites) in &self.referenced {
            if !self.declared.contains_key(name) {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "marker `{name}` is only used inside `Path::merge`, at {}\n\
                         a merge combines markers that each have their own branch. \
                         Give `{name}` a standalone `Path::named`, or name the \
                         combined condition with a single new marker instead.",
                        sites[0]
                    ),
                ));
            }
        }

        Ok(())
    }
}

struct FileVisitor<'s> {
    scan: &'s mut MarkerScan,
    path: PathBuf,
}

impl FileVisitor<'_> {
    fn site(&self, expr: &Expr) -> Site {
        let start = expr.span().start();
        Site {
            file: self.path.clone(),
            line: start.line,
            column: start.column,
        }
    }
}

impl<'ast> Visit<'ast> for FileVisitor<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Some(name) = path_named_call(expr) {
            let site = self.site(expr);
            self.scan.declared.entry(name).or_default().push(site);
            return;
        }
        visit::visit_expr(self, expr);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method != "merge" {
            visit::visit_expr_method_call(self, call);
            return;
        }
        for operand in std::iter::once(call.receiver.as_ref()).chain(call.args.iter()) {
            match path_named_call(operand) {
                Some(name) => {
                    let site = self.site(operand);
                    self.scan.referenced.entry(name).or_default().push(site);
                }
                None => self.visit_expr(operand),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(sources: &[(&str, &str)]) -> Result<()> {
        let mut scan = MarkerScan::default();
        for (name, src) in sources {
            let file = syn::parse_file(src).expect("test source parses");
            scan.scan(&file, Path::new(name));
        }
        scan.validate()
    }

    #[test]
    fn one_declaration_per_name_is_accepted() {
        scan(&[(
            "a.rs",
            r#"fn f() { Path::named("x"); Path::named(String::from("y")); }"#,
        )])
        .expect("distinct names are fine");
    }

    #[test]
    fn a_duplicate_across_two_files_is_rejected() {
        let err = scan(&[
            ("a.rs", r#"fn f() { Path::named("dup"); }"#),
            ("b.rs", r#"fn g() { Path::named(String::from("dup")); }"#),
        ])
        .expect_err("a name declared in two files must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("declared at 2 sites"), "{msg}");
        assert!(msg.contains("a.rs:1:10"), "{msg}");
        assert!(msg.contains("b.rs:1:10"), "{msg}");
    }

    #[test]
    fn a_merge_reference_needs_a_standalone_declaration() {
        let err = scan(&[(
            "a.rs",
            r#"fn f() { Path::named("x").merge(Path::named("only_merged")); Path::named("x"); }"#,
        )])
        .expect_err("a merge-only marker must be rejected");
        assert!(err.to_string().contains("only used inside"), "{err}");
    }

    #[test]
    fn a_merge_reference_with_a_declaration_is_accepted() {
        scan(&[
            (
                "a.rs",
                r#"fn f() { Path::named("x").merge(Path::named("y")); }"#,
            ),
            ("b.rs", r#"fn g() { Path::named("x"); Path::named("y"); }"#),
        ])
        .expect("both merged names have their own branch");
    }

    #[test]
    fn a_merge_operand_is_not_a_second_declaration() {
        scan(&[(
            "a.rs",
            r#"fn f() { Path::named("x"); Path::named("x").merge(Path::named("y")); Path::named("y"); }"#,
        )])
        .expect("merge operands are references, not declarations");
    }
}

//! Marker-name checks over a single `#[smt_fn]` body.
//!
//! Reports what one function can see, so an author gets it in the editor. The
//! transpiler repeats these checks over the whole program, which is where a
//! duplicate spanning two functions or two files is caught.

use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, ExprPath, Lit, Result};

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

#[derive(Default)]
struct Scan {
    /// Standalone declarations: name -> first span seen.
    declared: HashMap<String, proc_macro2::Span>,
    error: Option<syn::Error>,
}

impl Scan {
    /// Record a standalone declaration, rejecting a repeat of the same name.
    fn declare(&mut self, name: String, expr: &Expr) {
        if self.declared.contains_key(&name) && self.error.is_none() {
            self.error = Some(syn::Error::new(
                expr.span(),
                format!(
                    "marker `{name}` is already declared in this function; a name \
                     denotes one branch, so two branches cannot share it. Rename \
                     one, or use `Path::merge` if a single branch raises both."
                ),
            ));
        }
        self.declared.entry(name).or_insert_with(|| expr.span());
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Some(name) = path_named_call(expr) {
            self.declare(name, expr);
            return;
        }
        visit::visit_expr(self, expr);
    }

    /// `Path::named` inside a `merge` is a reference to a marker declared
    /// elsewhere, not a declaration, so its operands are skipped here.
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method != "merge" {
            visit::visit_expr_method_call(self, call);
            return;
        }
        for operand in std::iter::once(call.receiver.as_ref()).chain(call.args.iter()) {
            if path_named_call(operand).is_none() {
                self.visit_expr(operand);
            }
        }
    }
}

/// Reject a marker name declared twice in the same function body.
pub fn check_body(block: &syn::Block) -> Result<()> {
    let mut scan = Scan::default();
    scan.visit_block(block);
    match scan.error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn check(block: syn::Block) -> Result<()> {
        check_body(&block)
    }

    #[test]
    fn distinct_names_are_accepted() {
        assert!(
            check(parse_quote! {{
                if a { Path::named("x") } else { Path::named(String::from("y")) }
            }})
            .is_ok()
        );
    }

    #[test]
    fn a_repeated_standalone_name_is_rejected() {
        let err = check(parse_quote! {{
            if a { Path::named("x") } else { Path::named("x") }
        }})
        .expect_err("a repeated standalone marker must be rejected");
        assert!(err.to_string().contains("already declared"), "{err}");
    }

    #[test]
    fn the_two_literal_forms_are_the_same_name() {
        let err = check(parse_quote! {{
            if a { Path::named("x") } else { Path::named(String::from("x")) }
        }})
        .expect_err("the wrapped and bare forms name the same marker");
        assert!(err.to_string().contains("already declared"), "{err}");
    }

    #[test]
    fn a_merge_operand_is_a_reference_not_a_declaration() {
        // `a_zero` is declared once and referenced once inside the merge.
        assert!(
            check(parse_quote! {{
                if a {
                    Path::named("a_zero").merge(Path::named("b_zero"))
                } else {
                    Path::named("a_zero")
                }
            }})
            .is_ok()
        );
    }

    #[test]
    fn nested_merges_are_all_references() {
        assert!(
            check(parse_quote! {{
                Path::named("x")
                    .merge(Path::named("y"))
                    .merge(Path::named("z"))
            }})
            .is_ok()
        );
    }

    #[test]
    fn a_marker_nested_deep_in_the_body_is_still_seen() {
        let err = check(parse_quote! {{
            let f = |v| match v {
                Some(_) => Path::named("deep"),
                None => { while c { return Path::named("deep"); } Path::named("other") }
            };
        }})
        .expect_err("the walk must reach every expression");
        assert!(err.to_string().contains("already declared"), "{err}");
    }

    #[test]
    fn a_non_marker_named_call_is_ignored() {
        assert!(
            check(parse_quote! {{
                Other::named("x");
                Path::other("x");
                Path::named("x")
            }})
            .is_ok()
        );
    }
}

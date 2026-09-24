//! Owned execution summaries for transparent project callback wrappers.
//!
//! Only a single direct parameter invocation is supported. More complex bodies
//! use ordinary lexical analysis and retain callback uncertainty.

use syn::{Expr, FnArg, Pat, Stmt};

pub(super) fn invoked_parameter(signature: &syn::Signature, body: &syn::Block) -> Option<usize> {
    if signature.asyncness.is_some() {
        return None;
    }
    let [Stmt::Expr(expr, _)] = body.stmts.as_slice() else {
        return None;
    };
    let Expr::Call(call) = unwrapped(expr)? else {
        return None;
    };
    let Expr::Path(path) = unwrapped(&call.func)? else {
        return None;
    };
    let name = path.path.get_ident()?;
    signature.inputs.iter().position(|input| {
        matches!(input, FnArg::Typed(input) if matches!(&*input.pat, Pat::Ident(pattern)
            if pattern.ident == *name && pattern.subpat.is_none() && pattern.by_ref.is_none()))
    })
}

fn unwrapped(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Paren(paren) => unwrapped(&paren.expr),
        Expr::Group(group) => unwrapped(&group.expr),
        Expr::Return(returned) => unwrapped(returned.expr.as_deref()?),
        _ => Some(expr),
    }
}

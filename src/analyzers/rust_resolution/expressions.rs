//! Pure, bounded expression type propagation over the workspace declarations.
use super::body::{Body, unknown};
use super::index::Lookup;
use super::types::{TypeFact, UnknownReason};
use syn::{Expr, GenericArgument, PathArguments};

impl<'a> Body<'a> {
    pub(super) fn infer(&self, expr: &Expr) -> TypeFact {
        self.infer_at(expr, 0)
    }

    fn infer_at(&self, expr: &Expr, depth: usize) -> TypeFact {
        if depth >= 32 {
            return TypeFact::Unknown(UnknownReason::AnalysisLimit);
        }
        match expr {
            Expr::Path(p) => self.path_fact(&p.path),
            Expr::Struct(s) => self.path_type(&s.path),
            Expr::Reference(r) => TypeFact::Reference {
                mutable: r.mutability.is_some(),
                inner: Box::new(self.infer_at(&r.expr, depth + 1)),
            },
            Expr::Paren(p) => self.infer_at(&p.expr, depth + 1),
            Expr::Group(g) => self.infer_at(&g.expr, depth + 1),
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => {
                match self.infer_at(&u.expr, depth + 1) {
                    TypeFact::Reference { inner, .. } => *inner,
                    _ => unknown(),
                }
            }
            Expr::Tuple(t) => TypeFact::Tuple(
                t.elems
                    .iter()
                    .map(|e| self.infer_at(e, depth + 1))
                    .collect(),
            ),
            Expr::Field(f) => self
                .index
                .field_type(&self.infer_at(&f.base, depth + 1), &f.member),
            Expr::Call(c) => self.call_result(c),
            Expr::MethodCall(m) => self.method_result(m, depth),
            Expr::Await(a) => match self.infer_at(&a.base, depth + 1) {
                TypeFact::Future(output) => *output,
                _ => unknown(),
            },
            Expr::Block(b) => self.block_result(&b.block, depth),
            Expr::If(i) => self.if_result(i, depth),
            _ => unknown(),
        }
    }

    fn path_fact(&self, path: &syn::Path) -> TypeFact {
        if let Some(ident) = path.get_ident() {
            if self
                .callable
                .const_parameters
                .iter()
                .any(|name| ident == name)
            {
                return unknown();
            }
            if let Some(fact) = self.bindings.get(&ident.to_string()) {
                return fact;
            }
            if ident == "Self" {
                return self.callable.owner.clone().unwrap_or_else(unknown);
            }
        }
        if self.path_shadowed(path) {
            return unknown();
        }
        self.index
            .value_from_path(path, &self.callable.context, &self.substitutions)
            .unwrap_or_else(unknown)
    }

    pub(super) fn lookup_path(&self, path: &syn::ExprPath) -> Lookup<'a> {
        if self.path_shadowed(&path.path) {
            return Lookup::default();
        }
        if path.path.segments.len() == 1
            && path
                .path
                .get_ident()
                .is_some_and(|i| self.bindings.get(&i.to_string()).is_some())
        {
            return Lookup::default();
        }
        if path
            .path
            .segments
            .first()
            .is_some_and(|s| s.ident == "Self")
            && let (Some(owner), Some(method)) = (&self.callable.owner, path.path.segments.last())
        {
            return self.index.lookup_associated(
                owner,
                &method.ident.to_string(),
                &self.callable.context,
            );
        }
        if self.substitutions.is_empty() {
            self.index
                .lookup_call(&path.path, path.qself.as_ref(), &self.callable.context)
        } else {
            self.index.lookup_call_with_substitutions(
                &path.path,
                path.qself.as_ref(),
                &self.callable.context,
                &self.substitutions,
            )
        }
    }

    fn path_shadowed(&self, path: &syn::Path) -> bool {
        path.leading_colon.is_none()
            && path
                .segments
                .first()
                .is_some_and(|s| self.bindings.item_shadowed(&s.ident.to_string()))
    }

    fn path_type(&self, path: &syn::Path) -> TypeFact {
        self.declared_type(&syn::Type::Path(syn::TypePath {
            qself: None,
            path: path.clone(),
        }))
    }

    fn call_result(&self, call: &syn::ExprCall) -> TypeFact {
        let Expr::Path(path) = &*call.func else {
            return unknown();
        };
        let lookup = self.lookup_path(path);
        let owner = self.call_owner(path);
        let args = path
            .path
            .segments
            .last()
            .map(|s| self.arguments(&s.arguments))
            .unwrap_or_default();
        self.lookup_result(lookup, owner.as_ref(), &args)
    }

    fn call_owner(&self, path: &syn::ExprPath) -> Option<TypeFact> {
        if let Some(qself) = &path.qself {
            return Some(self.declared_type(&qself.ty));
        }
        let mut owner = path.path.clone();
        owner.segments.pop();
        if owner.segments.is_empty() {
            return None;
        }
        owner.segments.pop_punct();
        Some(self.path_type(&owner))
    }

    fn method_result(&self, method: &syn::ExprMethodCall, depth: usize) -> TypeFact {
        let receiver = self.infer_at(&method.receiver, depth + 1);
        let lookup = self.index.lookup_methods(
            &receiver,
            &method.method.to_string(),
            &self.callable.context,
        );
        let args = method
            .turbofish
            .as_ref()
            .map(|a| self.arguments(&PathArguments::AngleBracketed(a.clone())))
            .unwrap_or_default();
        self.lookup_result(lookup, Some(&receiver), &args)
    }

    fn lookup_result(
        &self,
        lookup: Lookup<'_>,
        owner: Option<&TypeFact>,
        args: &[TypeFact],
    ) -> TypeFact {
        match lookup.candidates.as_slice() {
            [callable] if lookup.justified => self.index.return_type(callable, owner, args),
            _ => unknown(),
        }
    }

    fn arguments(&self, arguments: &PathArguments) -> Vec<TypeFact> {
        match arguments {
            PathArguments::AngleBracketed(a) => a
                .args
                .iter()
                .filter_map(|a| match a {
                    GenericArgument::Type(t) => Some(self.declared_type(t)),
                    GenericArgument::Const(c) => {
                        Some(TypeFact::Const(quote::quote!(#c).to_string()))
                    }
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn block_result(&self, block: &syn::Block, depth: usize) -> TypeFact {
        // Bindings introduced within a block are handled by the lexical visitor.
        // Only a declaration-free tail can be inferred without executing a scope.
        match block.stmts.as_slice() {
            [syn::Stmt::Expr(expr, None)] => self.infer_at(expr, depth + 1),
            _ => unknown(),
        }
    }

    fn if_result(&self, expr: &syn::ExprIf, depth: usize) -> TypeFact {
        let left = self.block_result(&expr.then_branch, depth + 1);
        let right = expr
            .else_branch
            .as_ref()
            .map(|(_, e)| self.infer_at(e, depth + 1))
            .unwrap_or_else(unknown);
        if left == right { left } else { unknown() }
    }
}

//! Pure, bounded expression type propagation over the workspace declarations.
use super::body::{Body, unknown};
use super::index::Lookup;
use super::types::{PrimitiveType, TypeFact, UnknownReason};
use syn::{Expr, PathArguments};

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
            Expr::Lit(literal) => literal_fact(&literal.lit),
            Expr::Struct(s) => self.path_type(&s.path),
            Expr::Reference(r) => TypeFact::Reference {
                mutable: r.mutability.is_some(),
                inner: Box::new(self.infer_at(&r.expr, depth + 1)),
            },
            Expr::Paren(p) => self.infer_at(&p.expr, depth + 1),
            Expr::Group(g) => self.infer_at(&g.expr, depth + 1),
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => {
                self.infer_at(&u.expr, depth + 1).dereferenced()
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

    fn call_result(&self, call: &syn::ExprCall) -> TypeFact {
        let Expr::Path(path) = &*call.func else {
            return unknown();
        };
        if self.path_shadowed(&path.path) {
            return unknown();
        }
        if let Some(query) = self.associated_query(path) {
            let lookup = self
                .index
                .lookup_associated_query(&query, &self.callable.context);
            return self.lookup_result(lookup, Some(&query.owner), &query.arguments);
        }
        let args = path
            .path
            .segments
            .last()
            .map(|s| self.arguments(&s.arguments))
            .unwrap_or_default();
        self.index
            .value_call_result(&path.path, call.args.len(), &self.callable.context, &args)
            .unwrap_or_else(unknown)
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

    pub(super) fn arguments(&self, arguments: &PathArguments) -> Vec<TypeFact> {
        self.scoped_arguments(arguments, 0)
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

fn literal_fact(literal: &syn::Lit) -> TypeFact {
    let primitive = match literal {
        syn::Lit::Int(value) => PrimitiveType::from_name(value.suffix()),
        syn::Lit::Float(value) => PrimitiveType::from_name(value.suffix()),
        syn::Lit::Bool(_) => Some(PrimitiveType::Bool),
        syn::Lit::Char(_) => Some(PrimitiveType::Char),
        syn::Lit::Byte(_) => Some(PrimitiveType::U8),
        syn::Lit::Str(_) => {
            return TypeFact::Reference {
                mutable: false,
                inner: Box::new(TypeFact::Primitive(PrimitiveType::Str)),
            };
        }
        _ => None,
    };
    primitive.map(TypeFact::Primitive).unwrap_or_else(unknown)
}

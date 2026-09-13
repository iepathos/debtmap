//! Per-body context and conversion to shared graph outcomes.
use super::bindings::Bindings;
use super::index::{Callable, Lookup, WorkspaceIndex};
use super::types::{TypeFact, UnknownReason};
use crate::priority::call_graph::{
    CallEdgeProvenance, CallGraph, CallSite, CallType, ResolutionOutcome, UncertainCall,
    UncertaintyReason,
};
use std::collections::HashMap;
use syn::{spanned::Spanned, visit::Visit};

pub(super) struct Body<'a> {
    pub index: &'a WorkspaceIndex,
    pub callable: &'a Callable,
    pub graph: &'a mut CallGraph,
    pub bindings: Bindings,
    pub substitutions: HashMap<String, TypeFact>,
}

pub(super) fn unknown() -> TypeFact {
    TypeFact::Unknown(UnknownReason::UnsupportedTypeOperation)
}

impl<'a> Body<'a> {
    pub fn new(
        index: &'a WorkspaceIndex,
        callable: &'a Callable,
        graph: &'a mut CallGraph,
    ) -> Self {
        let substitutions = callable.substitutions.clone();
        Self {
            index,
            callable,
            graph,
            bindings: Bindings::default(),
            substitutions,
        }
    }

    pub fn analyze(mut self, signature: &syn::Signature, block: &syn::Block) {
        for input in &signature.inputs {
            self.bind_parameter(input);
        }
        self.visit_block(block);
    }

    fn bind_parameter(&mut self, input: &syn::FnArg) {
        match input {
            syn::FnArg::Typed(pat) => {
                self.bind_pattern(&pat.pat, self.declared_type(&pat.ty), true);
            }
            syn::FnArg::Receiver(receiver) => {
                self.bindings
                    .insert("self".into(), self.self_type(receiver), true);
            }
        }
    }

    fn self_type(&self, receiver: &syn::Receiver) -> TypeFact {
        if receiver.colon_token.is_some() {
            return self.declared_type(&receiver.ty);
        }
        let owner = self.callable.owner.clone().unwrap_or_else(unknown);
        if receiver.reference.is_some() {
            TypeFact::Reference {
                mutable: receiver.mutability.is_some(),
                inner: Box::new(owner),
            }
        } else {
            owner
        }
    }

    pub fn declared_type(&self, ty: &syn::Type) -> TypeFact {
        let mut shadow = TypeShadows {
            bindings: &self.bindings,
            found: false,
        };
        shadow.visit_type(ty);
        if shadow.found {
            return unknown();
        }
        self.index
            .type_from_syn(ty, &self.callable.context, &self.substitutions)
    }

    pub fn record(
        &mut self,
        lookup: Lookup<'_>,
        expr: &syn::Expr,
        query: String,
        receiver: Option<TypeFact>,
    ) {
        let position = match expr {
            syn::Expr::MethodCall(method) => method.method.span(),
            _ => expr.span(),
        }
        .start();
        let site = CallSite {
            file: self.callable.id.file.clone(),
            line: position.line,
            column: Some(position.column),
        };
        let candidates: Vec<_> = lookup.candidates.iter().map(|c| c.id.clone()).collect();
        if lookup.justified && candidates.len() == 1 {
            self.record_resolved(candidates[0].clone(), site, lookup.provenance);
        } else {
            let reason = uncertainty_reason(receiver.as_ref(), candidates.len());
            self.graph.record_uncertain_call(UncertainCall {
                caller: self.callable.id.clone(),
                call_site: site,
                call_ordinal: None,
                lexical_module: self.callable.context.module.join("::"),
                call_type: CallType::Direct,
                query,
                receiver: receiver.map(|r| format!("{r:?}")),
                candidates,
                reason,
            });
        }
    }

    fn record_resolved(
        &mut self,
        target: crate::priority::call_graph::FunctionId,
        site: CallSite,
        provenance: CallEdgeProvenance,
    ) {
        self.graph.add_resolution(
            self.callable.id.clone(),
            CallType::Direct,
            ResolutionOutcome::Resolved {
                target,
                provenance,
                confidence: if provenance == CallEdgeProvenance::AstDirect {
                    100
                } else {
                    95
                },
                call_site: Some(site),
            },
        );
    }
}

fn uncertainty_reason(receiver: Option<&TypeFact>, count: usize) -> UncertaintyReason {
    if let Some(reason) = receiver.and_then(fact_reason) {
        return match reason {
            UnknownReason::UnknownReceiver => UncertaintyReason::UnknownReceiver,
            UnknownReason::AmbiguousDeclaration => UncertaintyReason::AmbiguousDeclaration,
            UnknownReason::UnsupportedTypeOperation => UncertaintyReason::UnsupportedTypeOperation,
            UnknownReason::UnavailableDefinition => UncertaintyReason::UnavailableDefinition,
            UnknownReason::AnalysisLimit => UncertaintyReason::AnalysisLimit,
        };
    }
    match count {
        0 => UncertaintyReason::UnavailableDefinition,
        1 => UncertaintyReason::UnknownReceiver,
        _ => UncertaintyReason::AmbiguousDeclaration,
    }
}

fn fact_reason(fact: &TypeFact) -> Option<&UnknownReason> {
    match fact {
        TypeFact::Unknown(reason) | TypeFact::Uncertain { reason, .. } => Some(reason),
        TypeFact::Reference { inner, .. } => fact_reason(inner),
        _ => None,
    }
}

struct TypeShadows<'a> {
    bindings: &'a Bindings,
    found: bool,
}

impl<'ast> Visit<'ast> for TypeShadows<'_> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        self.found |= ty
            .path
            .segments
            .first()
            .is_some_and(|s| self.bindings.item_shadowed(&s.ident.to_string()));
        syn::visit::visit_type_path(self, ty);
    }
}

//! Per-body context and conversion to shared graph outcomes.
use super::bindings::Bindings;
use super::index::{Callable, Lookup, WorkspaceIndex};
use super::types::{TypeFact, UnknownReason};
use crate::analysis::effect_evidence::{
    EffectAssessment, EffectDependency, EffectProvenance, ObservedEffect, ObservedEffectKind,
    UnresolvedBehavior, UnresolvedReason,
};
use crate::priority::call_graph::{
    CallEdgeProvenance, CallGraph, CallSite, CallType, ResolutionOutcome, UncertainCall,
    UncertaintyReason,
};
use std::collections::HashMap;
use syn::{spanned::Spanned, visit::Visit};

mod closures;
mod scoped_types;
mod type_shadows;

pub(super) struct Body<'a> {
    pub index: &'a WorkspaceIndex,
    pub callable: &'a Callable,
    pub graph: &'a mut CallGraph,
    pub bindings: Bindings,
    pub substitutions: HashMap<String, TypeFact>,
    pub(super) assessment: EffectAssessment,
    pub(super) reachability_only: bool,
    bound_closures: Vec<closures::BoundClosure>,
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
            assessment: EffectAssessment::complete(),
            reachability_only: false,
            bound_closures: Vec::new(),
        }
    }

    pub fn analyze(mut self, signature: &syn::Signature, block: &syn::Block) {
        for input in &signature.inputs {
            self.bind_parameter(input);
        }
        self.visit_block(block);
        self.graph
            .record_effect_assessment(self.callable.id.clone(), self.assessment);
    }

    fn bind_parameter(&mut self, input: &syn::FnArg) {
        match input {
            syn::FnArg::Typed(pat) => {
                let fact = self.declared_type(&pat.ty);
                self.record_owned_drop(&fact, pat.pat.span());
                self.bind_pattern(&pat.pat, fact, true);
            }
            syn::FnArg::Receiver(receiver) => {
                self.record_owned_drop(&self.self_type(receiver), receiver.span());
                self.bindings
                    .insert("self".into(), self.self_type(receiver), true);
            }
        }
    }

    pub(super) fn record_owned_drop(&mut self, fact: &TypeFact, span: proc_macro2::Span) {
        if !super::models::may_dispatch_drop(fact) {
            return;
        }
        let position = span.start();
        let provenance = EffectProvenance::source(
            self.callable.id.clone(),
            position.line,
            Some(position.column),
        );
        self.join_assessment(
            EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
                reason: UnresolvedReason::UnsupportedDispatch,
                detail: "owned value may run an unmodeled destructor".into(),
                provenance,
            }),
        );
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
            let reason = lookup
                .reason
                .unwrap_or_else(|| uncertainty_reason(receiver.as_ref(), candidates.len()));
            self.record_unresolved(&site, &query, unresolved_reason(reason));
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
        if self.reachability_only {
            self.graph.add_resolution(
                self.callable.id.clone(),
                CallType::Callback,
                ResolutionOutcome::Resolved {
                    target,
                    provenance,
                    confidence: 100,
                    call_site: Some(site),
                },
            );
            return;
        }
        let mut effect_provenance =
            EffectProvenance::source(self.callable.id.clone(), site.line, site.column);
        effect_provenance.identity = Some(target.name.clone());
        effect_provenance.dependency = Some(target.clone());
        let assessment = std::mem::replace(&mut self.assessment, EffectAssessment::complete());
        self.assessment = assessment.with_dependency(EffectDependency {
            target: target.clone(),
            provenance: effect_provenance,
        });
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

    pub(super) fn record_reference(&mut self, lookup: Lookup<'_>, expr: &syn::Expr) {
        let position = expr.span().start();
        let site = CallSite {
            file: self.callable.id.file.clone(),
            line: position.line,
            column: Some(position.column),
        };
        let candidates: Vec<_> = lookup
            .candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect();
        if !lookup.justified || candidates.len() != 1 {
            if !candidates.is_empty() {
                self.graph.record_uncertain_call(UncertainCall {
                    caller: self.callable.id.clone(),
                    call_site: site,
                    call_ordinal: None,
                    lexical_module: self.callable.context.module.join("::"),
                    call_type: CallType::Callback,
                    query: quote::quote!(#expr).to_string(),
                    receiver: None,
                    candidates,
                    reason: lookup
                        .reason
                        .unwrap_or(UncertaintyReason::AmbiguousDeclaration),
                });
            }
            return;
        }
        self.graph.add_resolution(
            self.callable.id.clone(),
            CallType::Callback,
            ResolutionOutcome::Resolved {
                target: candidates[0].clone(),
                provenance: lookup.provenance,
                confidence: 100,
                call_site: Some(site),
            },
        );
    }

    pub(super) fn record_indirect_invocation(&mut self, expr: &syn::Expr) {
        let position = expr.span().start();
        let site = CallSite {
            file: self.callable.id.file.clone(),
            line: position.line,
            column: Some(position.column),
        };
        self.record_unresolved(
            &site,
            "indirect callable invocation",
            UnresolvedReason::CallbackInvocation,
        );
        self.graph.record_uncertain_call(UncertainCall {
            caller: self.callable.id.clone(),
            call_site: site,
            call_ordinal: None,
            lexical_module: self.callable.context.module.join("::"),
            call_type: CallType::Callback,
            query: "indirect callable invocation".into(),
            receiver: None,
            candidates: Vec::new(),
            reason: UncertaintyReason::UnavailableDefinition,
        });
    }

    pub(super) fn provenance(
        &self,
        expr: &syn::Expr,
        identity: impl Into<String>,
    ) -> EffectProvenance {
        let position = expr.span().start();
        let mut provenance = EffectProvenance::source(
            self.callable.id.clone(),
            position.line,
            Some(position.column),
        );
        provenance.identity = Some(identity.into());
        provenance
    }

    pub(super) fn join_assessment(&mut self, evidence: EffectAssessment) {
        let assessment = std::mem::replace(&mut self.assessment, EffectAssessment::complete());
        self.assessment = assessment.merge(&evidence);
    }

    pub(super) fn record_observed_effect(
        &mut self,
        expr: &syn::Expr,
        kind: ObservedEffectKind,
        detail: impl Into<String>,
    ) {
        let detail = detail.into();
        let provenance = self.provenance(expr, detail.clone());
        self.join_assessment(EffectAssessment::complete().with_effect(ObservedEffect {
            kind,
            detail,
            provenance,
        }));
    }

    pub(super) fn record_unsupported_syntax(&mut self, expr: &syn::Expr, detail: &str) {
        let provenance = self.provenance(expr, detail);
        self.join_assessment(
            EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
                reason: UnresolvedReason::UnsupportedSyntax,
                detail: detail.to_string(),
                provenance,
            }),
        );
    }

    fn record_unresolved(&mut self, site: &CallSite, query: &str, reason: UnresolvedReason) {
        let mut provenance =
            EffectProvenance::source(self.callable.id.clone(), site.line, site.column);
        provenance.identity = Some(query.to_string());
        let assessment = std::mem::replace(&mut self.assessment, EffectAssessment::complete());
        self.assessment = assessment.with_unresolved(UnresolvedBehavior {
            reason,
            detail: query.to_string(),
            provenance,
        });
    }
}

fn unresolved_reason(reason: UncertaintyReason) -> UnresolvedReason {
    match reason {
        UncertaintyReason::UnknownReceiver => UnresolvedReason::UnknownReceiver,
        UncertaintyReason::AmbiguousDeclaration => UnresolvedReason::AmbiguousTarget,
        UncertaintyReason::UnsupportedTypeOperation => UnresolvedReason::UnsupportedDispatch,
        UncertaintyReason::UnavailableDefinition => UnresolvedReason::UnresolvedCall,
        UncertaintyReason::AnalysisLimit => UnresolvedReason::UnsupportedSyntax,
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
        TypeFact::Dynamic(bounds) | TypeFact::BoundedGeneric { traits: bounds, .. } => {
            bounds.iter().find_map(|bound| bound.uncertainty_reason())
        }
        _ => None,
    }
}

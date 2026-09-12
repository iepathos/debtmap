//! Represented reference adjustments and constrained propagation.

use super::*;

pub(super) fn strip_references(fact: &TypeFact) -> &TypeFact {
    match fact {
        TypeFact::Reference { inner, .. } => strip_references(inner),
        TypeFact::Uncertain { constraint, .. } => strip_references(constraint),
        _ => fact,
    }
}

pub(super) fn receiver_adjustment_known(
    receiver: &TypeFact,
    call: &Callable,
    dotted: bool,
) -> bool {
    if !dotted {
        return true;
    }
    let Some(parameter) = call.signature.receiver() else {
        return false;
    };
    if parameter.colon_token.is_some() {
        return false;
    }
    match receiver {
        TypeFact::Reference { .. } => {
            parameter.reference.is_some()
                && (parameter.mutability.is_none() || references_allow_mutable_borrow(receiver))
        }
        _ => true,
    }
}

fn references_allow_mutable_borrow(receiver: &TypeFact) -> bool {
    match receiver {
        TypeFact::Reference { mutable, inner } => {
            *mutable && references_allow_mutable_borrow(inner)
        }
        _ => true,
    }
}

pub(super) fn with_uncertainty(fact: TypeFact, reason: UnknownReason) -> TypeFact {
    match fact {
        TypeFact::Reference { mutable, inner } => TypeFact::Reference {
            mutable,
            inner: Box::new(with_uncertainty(*inner, reason)),
        },
        TypeFact::Future(inner) => TypeFact::Future(Box::new(with_uncertainty(*inner, reason))),
        constraint => TypeFact::Uncertain {
            constraint: Box::new(constraint),
            reason,
        },
    }
}

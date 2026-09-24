//! Reconcile explicit types with partial initializer facts without losing constraints.
use super::super::types::{TypeFact, UnknownReason};

pub(super) fn compatible(declared: &TypeFact, value: &TypeFact) -> bool {
    compatible_at(declared, value, 0)
}

fn compatible_at(declared: &TypeFact, value: &TypeFact, depth: usize) -> bool {
    if depth >= 32 {
        return false;
    }
    match (declared, value) {
        (_, TypeFact::Unknown(_) | TypeFact::Generic(_) | TypeFact::BoundedGeneric { .. }) => true,
        (TypeFact::Tuple(left), TypeFact::Tuple(right)) => {
            compatible_components(left, right, depth)
        }
        (
            TypeFact::Nominal {
                declaration: left,
                arguments: left_args,
            },
            TypeFact::Nominal {
                declaration: right,
                arguments: right_args,
            },
        ) => left == right && compatible_components(left_args, right_args, depth),
        (
            TypeFact::Reference {
                mutable: left_mut,
                inner: left,
            },
            TypeFact::Reference {
                mutable: right_mut,
                inner: right,
            },
        ) => (!left_mut || *right_mut) && compatible_at(left, right, depth + 1),
        _ => declared == value,
    }
}

fn compatible_components(left: &[TypeFact], right: &[TypeFact], depth: usize) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| compatible_at(left, right, depth + 1))
}

pub(super) fn annotated_fact(annotation: TypeFact, value: &TypeFact, depth: usize) -> TypeFact {
    if depth < 32 {
        if let (TypeFact::Tuple(expected), TypeFact::Tuple(actual)) = (&annotation, value)
            && expected.len() == actual.len()
        {
            return TypeFact::Tuple(
                expected
                    .iter()
                    .zip(actual)
                    .map(|(expected, actual)| annotated_fact(expected.clone(), actual, depth + 1))
                    .collect(),
            );
        }
        if compatible(&annotation, value) {
            return annotation;
        }
    }
    TypeFact::Uncertain {
        constraint: Box::new(annotation),
        reason: if depth >= 32 {
            UnknownReason::AnalysisLimit
        } else {
            UnknownReason::UnsupportedTypeOperation
        },
    }
}

pub(super) fn tuple_facts(fact: TypeFact) -> Vec<TypeFact> {
    match fact {
        TypeFact::Tuple(facts) => facts,
        TypeFact::Uncertain { constraint, reason } => tuple_facts(*constraint)
            .into_iter()
            .map(|constraint| TypeFact::Uncertain {
                constraint: Box::new(constraint),
                reason: reason.clone(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

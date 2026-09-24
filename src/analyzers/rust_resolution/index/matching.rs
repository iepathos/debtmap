//! Structural type matching and direct generic bindings.

use super::*;

/// A represented owner constraint can admit candidates; unknown owners cannot.
pub(super) fn constrained_owner_compatible(pattern: &TypeFact, receiver: &TypeFact) -> bool {
    match strip_references(pattern) {
        TypeFact::Ambiguous(owners) => owners
            .iter()
            .any(|owner| constrained_owner_compatible(owner, receiver)),
        TypeFact::Nominal { .. }
        | TypeFact::Primitive(_)
        | TypeFact::Generic(_)
        | TypeFact::BoundedGeneric { .. } => owner_compatible(pattern, receiver),
        _ => false,
    }
}

pub(super) fn owner_compatible(pattern: &TypeFact, receiver: &TypeFact) -> bool {
    types_match(strip_references(pattern), strip_references(receiver), false)
        && repeated_arguments_agree(strip_references(pattern), strip_references(receiver), false)
}

pub(super) fn owner_match_known(pattern: &TypeFact, receiver: &TypeFact) -> bool {
    types_match(strip_references(pattern), strip_references(receiver), true)
        && repeated_arguments_agree(strip_references(pattern), strip_references(receiver), true)
}

fn types_match(pattern: &TypeFact, actual: &TypeFact, require_known: bool) -> bool {
    match (pattern, actual) {
        (TypeFact::Generic(_) | TypeFact::BoundedGeneric { .. }, _) => true,
        (TypeFact::Unknown(_), _)
        | (_, TypeFact::Generic(_) | TypeFact::BoundedGeneric { .. } | TypeFact::Unknown(_)) => {
            !require_known
        }
        (TypeFact::Uncertain { constraint, .. }, actual) => {
            !require_known && types_match(constraint, actual, false)
        }
        (pattern, TypeFact::Uncertain { constraint, .. }) => {
            !require_known && types_match(pattern, constraint, false)
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
        ) => left == right && arguments_match(left_args, right_args, require_known),
        (
            TypeFact::Reference {
                mutable: left,
                inner: left_inner,
            },
            TypeFact::Reference {
                mutable: right,
                inner: right_inner,
            },
        ) => left == right && types_match(left_inner, right_inner, require_known),
        (TypeFact::Tuple(left), TypeFact::Tuple(right)) => {
            arguments_match(left, right, require_known)
        }
        (left, right) => left == right && (!require_known || left.is_known()),
    }
}

fn arguments_match(pattern: &[TypeFact], actual: &[TypeFact], require_known: bool) -> bool {
    pattern.len() == actual.len()
        && pattern
            .iter()
            .zip(actual)
            .all(|(pattern, actual)| types_match(pattern, actual, require_known))
}

pub(super) fn infer_substitutions(
    pattern: &TypeFact,
    receiver: &TypeFact,
    substitutions: &mut Substitutions,
) {
    match (pattern, receiver) {
        (TypeFact::Generic(name) | TypeFact::BoundedGeneric { name, .. }, actual) => {
            substitutions.insert(name.clone(), actual.clone());
        }
        (
            TypeFact::Nominal {
                arguments: expected,
                ..
            },
            TypeFact::Nominal {
                arguments: actual, ..
            },
        ) => {
            for (pattern, receiver) in expected.iter().zip(actual) {
                infer_substitutions(pattern, receiver, substitutions);
            }
        }
        (TypeFact::Tuple(expected), TypeFact::Tuple(actual)) => {
            for (pattern, receiver) in expected.iter().zip(actual) {
                infer_substitutions(pattern, receiver, substitutions);
            }
        }
        (
            TypeFact::Reference { inner: pattern, .. },
            TypeFact::Reference {
                inner: receiver, ..
            },
        ) => {
            infer_substitutions(pattern, receiver, substitutions);
        }
        _ => {}
    }
}

fn repeated_arguments_agree(pattern: &TypeFact, actual: &TypeFact, require_known: bool) -> bool {
    let mut arguments = Vec::new();
    collect_generic_arguments(pattern, actual, &mut arguments);
    let mut previous = HashMap::new();
    arguments
        .into_iter()
        .all(|(name, argument)| match previous.insert(name, argument) {
            None => true,
            Some(previous) if require_known => argument == previous && !contains_unknown(argument),
            Some(previous) => types_match(previous, argument, false),
        })
}

fn collect_generic_arguments<'p, 'a>(
    pattern: &'p TypeFact,
    actual: &'a TypeFact,
    arguments: &mut Vec<(&'p str, &'a TypeFact)>,
) {
    match (pattern, actual) {
        (TypeFact::Generic(name) | TypeFact::BoundedGeneric { name, .. }, _) => {
            arguments.push((name, actual))
        }
        (
            TypeFact::Nominal {
                arguments: pattern, ..
            },
            TypeFact::Nominal {
                arguments: actual, ..
            },
        )
        | (TypeFact::Tuple(pattern), TypeFact::Tuple(actual)) => {
            for (pattern, actual) in pattern.iter().zip(actual) {
                collect_generic_arguments(pattern, actual, arguments);
            }
        }
        (TypeFact::Reference { inner: pattern, .. }, TypeFact::Reference { inner: actual, .. }) => {
            collect_generic_arguments(pattern, actual, arguments)
        }
        _ => {}
    }
}

fn contains_unknown(fact: &TypeFact) -> bool {
    match fact {
        TypeFact::Unknown(_)
        | TypeFact::Uncertain { .. }
        | TypeFact::Ambiguous(_)
        | TypeFact::UnavailablePath(_) => true,
        TypeFact::Reference { inner, .. } | TypeFact::Future(inner) => contains_unknown(inner),
        TypeFact::Nominal { arguments, .. } | TypeFact::Tuple(arguments) => {
            arguments.iter().any(contains_unknown)
        }
        _ => false,
    }
}

use super::*;

#[test]
fn reference_projection_preserves_outer_uncertainty_and_candidate_alternatives() {
    let leaf = TypeFact::UnavailablePath(vec!["Local".into()]);
    let fact = TypeFact::Uncertain {
        reason: UnknownReason::AmbiguousDeclaration,
        constraint: Box::new(TypeFact::Ambiguous(vec![
            TypeFact::Reference {
                mutable: false,
                inner: Box::new(leaf.clone()),
            },
            TypeFact::Reference {
                mutable: true,
                inner: Box::new(TypeFact::Primitive(PrimitiveType::U32)),
            },
        ])),
    };
    assert_eq!(
        fact.dereferenced(),
        TypeFact::Uncertain {
            reason: UnknownReason::AmbiguousDeclaration,
            constraint: Box::new(TypeFact::Ambiguous(vec![
                leaf,
                TypeFact::Primitive(PrimitiveType::U32)
            ])),
        }
    );
}

#[test]
fn unsupported_projection_cannot_broaden_an_unavailable_path() {
    let fact = TypeFact::Uncertain {
        constraint: Box::new(TypeFact::UnavailablePath(vec!["Local".into()])),
        reason: UnknownReason::UnsupportedTypeOperation,
    };
    assert_eq!(fact.clone().dereferenced(), fact);
    assert!(matches!(
        TypeFact::Primitive(PrimitiveType::U32).dereferenced(),
        TypeFact::Unknown(UnknownReason::UnsupportedTypeOperation)
    ));
}

//! Structured Rust type facts. Display names never establish declaration identity.

use std::path::PathBuf;

/// A declaration is identified by its source location and lexical module.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct DeclarationId {
    pub file: PathBuf,
    pub module: Vec<String>,
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum UnknownReason {
    UnknownReceiver,
    AmbiguousDeclaration,
    UnsupportedTypeOperation,
    UnavailableDefinition,
    AnalysisLimit,
}

/// Facts deliberately represent unavailable knowledge instead of inventing names.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeFact {
    Nominal {
        declaration: DeclarationId,
        arguments: Vec<TypeFact>,
    },
    Reference {
        mutable: bool,
        inner: Box<TypeFact>,
    },
    Generic(String),
    BoundedGeneric {
        name: String,
        traits: Vec<Vec<String>>,
    },
    Ambiguous(Vec<TypeFact>),
    UnavailablePath(Vec<String>),
    Uncertain {
        constraint: Box<TypeFact>,
        reason: UnknownReason,
    },
    SelfType,
    Tuple(Vec<TypeFact>),
    Future(Box<TypeFact>),
    Const(String),
    Dynamic(Vec<Vec<String>>),
    Unknown(UnknownReason),
}

/// The source of a fact is independent of how its type is represented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactOrigin {
    Declaration,
    Propagation,
}

impl TypeFact {
    pub fn unknown() -> Self {
        Self::Unknown(UnknownReason::UnknownReceiver)
    }

    /// Only references represented in the source can be peeled implicitly.
    pub fn nominal(&self) -> Option<(&DeclarationId, &[TypeFact])> {
        match self {
            Self::Nominal {
                declaration,
                arguments,
            } => Some((declaration, arguments)),
            Self::Reference { inner, .. } => inner.nominal(),
            Self::Uncertain { constraint, .. } => constraint.nominal(),
            _ => None,
        }
    }

    pub fn is_known(&self) -> bool {
        match self {
            Self::Unknown(_)
            | Self::Generic(_)
            | Self::SelfType
            | Self::Dynamic(_)
            | Self::BoundedGeneric { .. }
            | Self::Ambiguous(_)
            | Self::UnavailablePath(_)
            | Self::Uncertain { .. } => false,
            Self::Reference { inner, .. } | Self::Future(inner) => inner.is_known(),
            Self::Nominal { arguments, .. } | Self::Tuple(arguments) => {
                arguments.iter().all(Self::is_known)
            }
            Self::Const(_) => true,
        }
    }

    pub fn is_uncertain(&self) -> bool {
        match self {
            Self::Uncertain { .. } | Self::Ambiguous(_) => true,
            Self::Reference { inner, .. } => inner.is_uncertain(),
            _ => false,
        }
    }

    pub fn uncertainty_reason(&self) -> Option<UnknownReason> {
        match self {
            Self::Uncertain { reason, .. } => Some(reason.clone()),
            Self::Ambiguous(_) => Some(UnknownReason::AmbiguousDeclaration),
            Self::Reference { inner, .. } => inner.uncertainty_reason(),
            _ => None,
        }
    }

    pub fn has_nominal_candidates(&self) -> bool {
        match self {
            Self::Nominal { .. } => true,
            Self::Reference { inner, .. }
            | Self::Uncertain {
                constraint: inner, ..
            } => inner.has_nominal_candidates(),
            Self::Ambiguous(candidates) => candidates.iter().any(Self::has_nominal_candidates),
            _ => false,
        }
    }
}

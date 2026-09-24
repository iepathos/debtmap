//! Small identity-based standard-library effect registry.

use super::types::{ModelType, TypeFact};
use crate::analysis::effect_evidence::{
    EffectAssessment, EffectProvenance, ObservedEffect, ObservedEffectKind, UnresolvedBehavior,
    UnresolvedReason,
};

pub(super) fn modeled_type(path: &[String], arguments: Vec<TypeFact>) -> Option<TypeFact> {
    let rooted = path
        .iter()
        .find(|part| part.as_str() != "::")
        .is_some_and(|part| matches!(part.as_str(), "std" | "core" | "alloc"));
    let prelude =
        path.len() == 1 && matches!(path[0].as_str(), "Vec" | "String" | "Option" | "Result");
    if !rooted && !prelude {
        return None;
    }
    let canonical = canonical_suffix(path);
    let kind = match canonical.as_slice() {
        ["Vec"] | ["vec", "Vec"] => ModelType::Vec,
        ["String"] | ["string", "String"] => ModelType::String,
        ["Option"] | ["option", "Option"] => ModelType::Option,
        ["Result"] | ["result", "Result"] => ModelType::Result,
        ["collections", "HashMap"] => ModelType::HashMap,
        ["collections", "HashSet"] => ModelType::HashSet,
        ["fs", "File"] => ModelType::File,
        ["net", "TcpStream"] => ModelType::TcpStream,
        ["io", "Cursor"] => ModelType::Cursor,
        _ => return None,
    };
    Some(TypeFact::Modeled { kind, arguments })
}

pub(super) fn method(
    receiver: &TypeFact,
    name: &str,
    callback_resolved: bool,
    provenance: EffectProvenance,
) -> Option<EffectAssessment> {
    let kind = modeled_receiver(receiver)?;
    let borrowed = modeled_receiver_is_reference(receiver);
    let dispatch_provenance = provenance.clone();
    let assessment = match (kind, name) {
        (ModelType::Vec, "len" | "is_empty" | "get" | "as_slice")
        | (ModelType::String, "len" | "is_empty" | "as_str")
        | (ModelType::Option | ModelType::Result, "is_some" | "is_none" | "is_ok" | "is_err")
        | (ModelType::Option | ModelType::Result, "unwrap" | "expect" | "as_ref") => {
            EffectAssessment::complete()
        }
        (ModelType::Vec, "push" | "pop" | "insert" | "remove" | "clear")
        | (ModelType::String, "push" | "push_str" | "clear")
        | (ModelType::Option, "take" | "replace") => mutation(name, borrowed, provenance),
        (ModelType::String, "contains") => unresolved_dispatch(name, provenance),
        (ModelType::HashMap | ModelType::HashSet, "len" | "is_empty") => {
            EffectAssessment::complete()
        }
        (ModelType::HashMap | ModelType::HashSet, "get" | "contains" | "contains_key") => {
            unresolved_dispatch(name, provenance)
        }
        (ModelType::HashMap | ModelType::HashSet, "insert" | "remove" | "clear") => {
            mutation(name, borrowed, provenance.clone())
                .join(&unresolved_dispatch(name, provenance))
        }
        (ModelType::Option | ModelType::Result, "map" | "and_then" | "or_else" | "inspect") => {
            if callback_resolved {
                EffectAssessment::complete()
            } else {
                callback(name, provenance)
            }
        }
        (ModelType::Cursor, "read" | "read_exact" | "write" | "write_all" | "seek") => {
            mutation(name, borrowed, provenance.clone())
                .join(&unresolved_dispatch(name, provenance))
        }
        (ModelType::File | ModelType::TcpStream, "read" | "read_exact" | "read_to_end") => {
            external_io(ObservedEffectKind::ExternalRead, name, provenance)
        }
        (ModelType::File | ModelType::TcpStream, "write" | "write_all" | "flush") => {
            external_io(ObservedEffectKind::ExternalWrite, name, provenance)
        }
        _ => return None,
    };
    Some(if receiver_may_dispatch_drop(receiver) {
        assessment.join(&unresolved_dispatch(
            "generic value destruction",
            dispatch_provenance,
        ))
    } else {
        assessment
    })
}

/// Destruction of these owned values can run an unavailable user implementation.
pub(super) fn may_dispatch_drop(fact: &TypeFact) -> bool {
    match fact {
        TypeFact::Reference { .. } | TypeFact::Primitive(_) | TypeFact::Const(_) => false,
        TypeFact::Modeled {
            kind: ModelType::String,
            ..
        } => false,
        TypeFact::Modeled {
            kind: ModelType::File | ModelType::TcpStream | ModelType::Cursor,
            ..
        } => true,
        TypeFact::Modeled { arguments, .. } => {
            arguments.is_empty() || arguments.iter().any(may_dispatch_drop)
        }
        TypeFact::Tuple(items) => items.iter().any(may_dispatch_drop),
        TypeFact::Nominal { .. }
        | TypeFact::Generic(_)
        | TypeFact::BoundedGeneric { .. }
        | TypeFact::Dynamic(_)
        | TypeFact::UnavailablePath(_)
        | TypeFact::SelfType => true,
        TypeFact::Uncertain { constraint, .. } => may_dispatch_drop(constraint),
        TypeFact::Ambiguous(items) => items.iter().any(may_dispatch_drop),
        TypeFact::Future(_) | TypeFact::Unknown(_) => false,
    }
}

fn receiver_may_dispatch_drop(fact: &TypeFact) -> bool {
    match fact {
        TypeFact::Reference { inner, .. } => receiver_may_dispatch_drop(inner),
        _ => may_dispatch_drop(fact),
    }
}

pub(super) fn executes_callback(receiver: &TypeFact, name: &str) -> bool {
    matches!(
        modeled_receiver(receiver),
        Some(ModelType::Option | ModelType::Result)
    ) && matches!(name, "map" | "and_then" | "or_else" | "inspect")
}

pub(super) fn function(path: &[String], provenance: EffectProvenance) -> Option<EffectAssessment> {
    if !is_explicit_std_path(path) {
        return None;
    }
    let canonical = canonical_suffix(path);
    match canonical.as_slice() {
        ["fs", "read"] | ["fs", "read_to_string"] | ["fs", "metadata"] => Some(external_io(
            ObservedEffectKind::ExternalRead,
            &canonical.join("::"),
            provenance,
        )),
        ["fs", "write"]
        | ["fs", "remove_file"]
        | ["fs", "create_dir"]
        | ["fs", "create_dir_all"] => Some(external_io(
            ObservedEffectKind::ExternalWrite,
            &canonical.join("::"),
            provenance,
        )),
        ["env", "var"] | ["env", "vars"] | ["env", "current_dir"] => Some(effect(
            ObservedEffectKind::ExternalRead,
            &canonical.join("::"),
            provenance,
        )),
        ["env", "set_var"] | ["env", "remove_var"] | ["env", "set_current_dir"] => Some(effect(
            ObservedEffectKind::ExternalWrite,
            &canonical.join("::"),
            provenance,
        )),
        ["time", "SystemTime", "now"] => Some(effect(
            ObservedEffectKind::Nondeterminism,
            &canonical.join("::"),
            provenance,
        )),
        _ => None,
    }
}

fn is_explicit_std_path(path: &[String]) -> bool {
    path.iter()
        .find(|part| part.as_str() != "::")
        .is_some_and(|part| part == "std")
}

pub(super) fn console_macro(name: &str, provenance: EffectProvenance) -> Option<EffectAssessment> {
    matches!(name, "print" | "println" | "eprint" | "eprintln" | "dbg").then(|| {
        external_io(ObservedEffectKind::ExternalWrite, name, provenance.clone()).merge(
            &unresolved_dispatch("console formatting arguments", provenance),
        )
    })
}

fn modeled_receiver(receiver: &TypeFact) -> Option<ModelType> {
    match receiver {
        TypeFact::Modeled { kind, .. } => Some(*kind),
        TypeFact::Reference { inner, .. } => modeled_receiver(inner),
        _ => None,
    }
}

/// Only built-in argument conversions are modeled; arbitrary AsRef and other
/// generic implementations cannot inherit a library operation's completeness.
pub(super) fn supported_argument(fact: &TypeFact) -> bool {
    match fact {
        TypeFact::Primitive(_)
        | TypeFact::Modeled {
            kind: ModelType::String,
            ..
        } => true,
        TypeFact::Reference { inner, .. } => supported_argument(inner),
        _ => false,
    }
}

fn modeled_receiver_is_reference(receiver: &TypeFact) -> bool {
    match receiver {
        TypeFact::Reference { .. } => true,
        TypeFact::Uncertain { constraint, .. } => modeled_receiver_is_reference(constraint),
        _ => false,
    }
}

fn canonical_suffix(path: &[String]) -> Vec<&str> {
    let start = path
        .iter()
        .position(|part| !matches!(part.as_str(), "::" | "std" | "core" | "alloc"))
        .unwrap_or(path.len());
    path[start..].iter().map(String::as_str).collect()
}

fn effect(
    kind: ObservedEffectKind,
    detail: &str,
    mut provenance: EffectProvenance,
) -> EffectAssessment {
    provenance.identity = Some(format!("std::{detail}"));
    EffectAssessment::complete().with_effect(ObservedEffect {
        kind,
        detail: detail.into(),
        provenance,
    })
}

fn local_mutation(detail: &str, provenance: EffectProvenance) -> EffectAssessment {
    effect(ObservedEffectKind::LocalMutation, detail, provenance)
}

fn mutation(detail: &str, borrowed: bool, provenance: EffectProvenance) -> EffectAssessment {
    if borrowed {
        return EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
            reason: UnresolvedReason::UnsupportedDispatch,
            detail: format!("modeled mutation {detail} has unresolved target ownership"),
            provenance,
        });
    }
    local_mutation(detail, provenance)
}

fn external_io(
    kind: ObservedEffectKind,
    detail: &str,
    provenance: EffectProvenance,
) -> EffectAssessment {
    effect(kind, detail, provenance.clone()).join(&effect(
        ObservedEffectKind::Io,
        detail,
        provenance,
    ))
}

fn unresolved_dispatch(detail: &str, provenance: EffectProvenance) -> EffectAssessment {
    EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
        reason: UnresolvedReason::UnsupportedDispatch,
        detail: format!("modeled operation {detail} may dispatch user code"),
        provenance,
    })
}

fn callback(detail: &str, provenance: EffectProvenance) -> EffectAssessment {
    EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
        reason: UnresolvedReason::CallbackInvocation,
        detail: format!("modeled operation {detail} invokes a callback"),
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::effect_evidence::EffectClassification;
    use crate::priority::call_graph::FunctionId;

    fn source() -> EffectProvenance {
        EffectProvenance::source(
            FunctionId::new("src/lib.rs".into(), "f".into(), 1),
            2,
            Some(3),
        )
    }

    #[test]
    fn unqualified_module_spelling_cannot_establish_std_identity() {
        for path in [
            "fs::File",
            "io::Cursor",
            "net::TcpStream",
            "collections::HashMap",
        ] {
            let segments: Vec<_> = path.split("::").map(str::to_owned).collect();
            assert!(modeled_type(&segments, Vec::new()).is_none(), "{path}");
            let rooted: Vec<_> = std::iter::once("std".to_owned()).chain(segments).collect();
            assert!(modeled_type(&rooted, Vec::new()).is_some(), "{path}");
        }
    }

    #[test]
    fn memory_and_external_operations_remain_distinct() {
        let cursor = TypeFact::Modeled {
            kind: ModelType::Cursor,
            arguments: vec![],
        };
        let file = TypeFact::Modeled {
            kind: ModelType::File,
            arguments: vec![],
        };
        assert_eq!(
            method(&cursor, "write", false, source())
                .unwrap()
                .classification(),
            EffectClassification::Unknown
        );
        assert_eq!(
            method(&file, "write", false, source())
                .unwrap()
                .classification(),
            EffectClassification::Impure
        );
    }

    #[test]
    fn hash_lookup_retains_generic_dispatch_uncertainty() {
        let map = TypeFact::Modeled {
            kind: ModelType::HashMap,
            arguments: vec![],
        };
        assert_eq!(
            method(&map, "get", false, source())
                .unwrap()
                .classification(),
            EffectClassification::Unknown
        );
    }
}

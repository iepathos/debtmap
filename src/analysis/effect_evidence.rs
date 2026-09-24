//! Shared, owned evidence for Rust effect analysis.
//!
//! The model describes supported program effects. It deliberately does not claim
//! termination, panic freedom, allocation success, or timing independence.

use crate::priority::call_graph::FunctionId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An effect directly observed by a supported analysis or model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedEffectKind {
    LocalMutation,
    ExternalRead,
    ExternalWrite,
    Io,
    Nondeterminism,
}

/// Why the analysis cannot make a completeness claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedReason {
    UnresolvedCall,
    AmbiguousTarget,
    UnknownReceiver,
    UnsupportedSyntax,
    CallbackInvocation,
    UnavailableBody,
    UnsupportedDispatch,
    LegacyEvidence,
}

/// Compact source identity shared by effects and uncertainty records.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EffectProvenance {
    pub owner: FunctionId,
    pub line: usize,
    pub column: Option<usize>,
    pub identity: Option<String>,
    pub dependency: Option<FunctionId>,
}

impl EffectProvenance {
    pub fn source(owner: FunctionId, line: usize, column: Option<usize>) -> Self {
        Self {
            owner,
            line,
            column,
            identity: None,
            dependency: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservedEffect {
    pub kind: ObservedEffectKind,
    pub detail: String,
    pub provenance: EffectProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnresolvedBehavior {
    pub reason: UnresolvedReason,
    pub detail: String,
    pub provenance: EffectProvenance,
}

/// A project definition whose assessment contributes when this call executes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EffectDependency {
    pub target: FunctionId,
    pub provenance: EffectProvenance,
}

/// Internal classification. Unknown is intentionally distinct from Impure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClassification {
    StrictlyPure,
    LocallyPure,
    ReadOnly,
    Impure,
    Unknown,
}

/// Normalized evidence for one definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "AssessmentRecords", into = "AssessmentRecords")]
pub struct EffectAssessment {
    observed: BTreeMap<ObservedKey, ObservedEffect>,
    unresolved: BTreeMap<UnresolvedKey, UnresolvedBehavior>,
    #[serde(default)]
    dependencies: BTreeMap<DependencyKey, EffectDependency>,
    complete: bool,
}

/// Store records as lists so both JSON and binary formats support source identities.
/// Rebuilding the indexes on read also normalizes duplicate evidence.
#[derive(Serialize, Deserialize)]
struct AssessmentRecords {
    observed: Vec<ObservedEffect>,
    unresolved: Vec<UnresolvedBehavior>,
    dependencies: Vec<EffectDependency>,
    complete: bool,
}

impl From<EffectAssessment> for AssessmentRecords {
    fn from(assessment: EffectAssessment) -> Self {
        Self {
            observed: assessment.observed.into_values().collect(),
            unresolved: assessment.unresolved.into_values().collect(),
            dependencies: assessment.dependencies.into_values().collect(),
            complete: assessment.complete,
        }
    }
}

impl From<AssessmentRecords> for EffectAssessment {
    fn from(records: AssessmentRecords) -> Self {
        let initial = Self {
            complete: records.complete,
            ..Self::complete()
        };
        records
            .observed
            .into_iter()
            .fold(initial, Self::with_effect)
            .add_records(records.unresolved, records.dependencies)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SourceKey {
    owner: FunctionId,
    line: usize,
    column: Option<usize>,
    identity: Option<String>,
}

impl From<&EffectProvenance> for SourceKey {
    fn from(value: &EffectProvenance) -> Self {
        Self {
            owner: value.owner.clone(),
            line: value.line,
            column: value.column,
            identity: value.identity.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct ObservedKey {
    source: SourceKey,
    kind: ObservedEffectKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct UnresolvedKey {
    source: SourceKey,
    reason: UnresolvedReason,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct DependencyKey {
    source: SourceKey,
    target: FunctionId,
}

impl Default for EffectAssessment {
    fn default() -> Self {
        Self::unknown(
            UnresolvedReason::UnavailableBody,
            "effect evidence is absent",
        )
    }
}

impl EffectAssessment {
    fn add_records(
        self,
        unresolved: Vec<UnresolvedBehavior>,
        dependencies: Vec<EffectDependency>,
    ) -> Self {
        dependencies.into_iter().fold(
            unresolved.into_iter().fold(self, Self::with_unresolved),
            Self::with_dependency,
        )
    }

    pub fn complete() -> Self {
        Self {
            observed: BTreeMap::new(),
            unresolved: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            complete: true,
        }
    }

    pub fn unknown(reason: UnresolvedReason, detail: impl Into<String>) -> Self {
        let owner = FunctionId::new("<unknown>".into(), "<unknown>".into(), 0);
        Self::unknown_for(owner, reason, detail)
    }

    pub fn unknown_for(
        owner: FunctionId,
        reason: UnresolvedReason,
        detail: impl Into<String>,
    ) -> Self {
        let line = owner.line;
        let column = owner.column;
        Self::complete().with_unresolved(UnresolvedBehavior {
            reason,
            detail: detail.into(),
            provenance: EffectProvenance::source(owner, line, column),
        })
    }

    pub fn with_effect(mut self, effect: ObservedEffect) -> Self {
        let key = ObservedKey {
            source: SourceKey::from(&effect.provenance),
            kind: effect.kind,
        };
        insert_minimum(&mut self.observed, key, effect);
        self
    }

    pub fn with_unresolved(mut self, behavior: UnresolvedBehavior) -> Self {
        self.complete = false;
        let key = UnresolvedKey {
            source: SourceKey::from(&behavior.provenance),
            reason: behavior.reason,
        };
        insert_minimum(&mut self.unresolved, key, behavior);
        self
    }

    pub fn with_dependency(mut self, dependency: EffectDependency) -> Self {
        let key = DependencyKey {
            source: SourceKey::from(&dependency.provenance),
            target: dependency.target.clone(),
        };
        insert_minimum(&mut self.dependencies, key, dependency);
        self
    }

    pub fn join(&self, other: &Self) -> Self {
        self.clone().merge(other)
    }

    /// Consume the accumulated evidence, avoiding a full copy at every join.
    pub(crate) fn merge(mut self, other: &Self) -> Self {
        self.complete &= other.complete;
        for (key, effect) in &other.observed {
            insert_minimum(&mut self.observed, key.clone(), effect.clone());
        }
        for (key, behavior) in &other.unresolved {
            insert_minimum(&mut self.unresolved, key.clone(), behavior.clone());
        }
        for (key, dependency) in &other.dependencies {
            insert_minimum(&mut self.dependencies, key.clone(), dependency.clone());
        }
        self
    }

    pub fn observed(&self) -> impl Iterator<Item = &ObservedEffect> {
        self.observed.values()
    }

    pub fn unresolved(&self) -> impl Iterator<Item = &UnresolvedBehavior> {
        self.unresolved.values()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &EffectDependency> {
        self.dependencies.values()
    }

    pub(crate) fn remap_identities(&self, identity: impl Fn(&FunctionId) -> FunctionId) -> Self {
        let provenance = |source: &EffectProvenance| EffectProvenance {
            owner: identity(&source.owner),
            line: source.line,
            column: source.column,
            identity: source.identity.clone(),
            dependency: source.dependency.as_ref().map(&identity),
        };
        let initial = Self {
            complete: self.complete,
            ..Self::complete()
        };
        self.observed()
            .fold(initial, |assessment, effect| {
                assessment.with_effect(ObservedEffect {
                    kind: effect.kind,
                    detail: effect.detail.clone(),
                    provenance: provenance(&effect.provenance),
                })
            })
            .join(
                &self
                    .unresolved()
                    .fold(EffectAssessment::complete(), |assessment, behavior| {
                        assessment.with_unresolved(UnresolvedBehavior {
                            reason: behavior.reason,
                            detail: behavior.detail.clone(),
                            provenance: provenance(&behavior.provenance),
                        })
                    }),
            )
            .join(&self.dependencies().fold(
                EffectAssessment::complete(),
                |assessment, dependency| {
                    assessment.with_dependency(EffectDependency {
                        target: identity(&dependency.target),
                        provenance: provenance(&dependency.provenance),
                    })
                },
            ))
    }

    pub fn is_complete(&self) -> bool {
        self.complete && self.unresolved.is_empty()
    }

    pub fn classification(&self) -> EffectClassification {
        let has = |kind| self.observed.values().any(|effect| effect.kind == kind);
        if has(ObservedEffectKind::ExternalWrite)
            || has(ObservedEffectKind::Io)
            || has(ObservedEffectKind::Nondeterminism)
        {
            return EffectClassification::Impure;
        }
        if !self.is_complete() {
            return EffectClassification::Unknown;
        }
        if has(ObservedEffectKind::ExternalRead) {
            return EffectClassification::ReadOnly;
        }
        if has(ObservedEffectKind::LocalMutation) {
            return EffectClassification::LocallyPure;
        }
        EffectClassification::StrictlyPure
    }
}

fn insert_minimum<K, V>(records: &mut BTreeMap<K, V>, key: K, value: V)
where
    K: Ord,
    V: Ord + Clone,
{
    records
        .entry(key)
        .and_modify(|current| {
            if value < *current {
                *current = value.clone();
            }
        })
        .or_insert(value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn provenance() -> EffectProvenance {
        EffectProvenance::source(
            FunctionId::new("src/lib.rs".into(), "f".into(), 3),
            4,
            Some(2),
        )
    }

    fn effect(kind: ObservedEffectKind) -> ObservedEffect {
        ObservedEffect {
            kind,
            detail: format!("{kind:?}"),
            provenance: provenance(),
        }
    }

    fn unresolved() -> UnresolvedBehavior {
        UnresolvedBehavior {
            reason: UnresolvedReason::UnresolvedCall,
            detail: "missing".into(),
            provenance: provenance(),
        }
    }

    fn populated_assessment() -> EffectAssessment {
        EffectAssessment::complete()
            .with_effect(effect(ObservedEffectKind::ExternalRead))
            .with_unresolved(unresolved())
            .with_dependency(EffectDependency {
                target: FunctionId::new("src/io.rs".into(), "read".into(), 7),
                provenance: provenance(),
            })
    }

    #[test]
    fn nonempty_evidence_round_trips_through_json_and_postcard() {
        let assessment = populated_assessment();
        let json = serde_json::to_string(&assessment).unwrap();
        let restored: EffectAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, assessment);
        let binary = postcard::to_allocvec(&assessment).unwrap();
        let restored: EffectAssessment = postcard::from_bytes(&binary).unwrap();
        assert_eq!(restored, assessment);
    }

    #[test]
    fn deserialization_normalizes_duplicate_records_and_incompleteness() {
        let assessment = populated_assessment();
        let mut value = serde_json::to_value(&assessment).unwrap();
        let observed = value["observed"].as_array_mut().unwrap();
        observed.push(observed[0].clone());
        value["complete"] = serde_json::Value::Bool(true);
        let restored: EffectAssessment = serde_json::from_value(value).unwrap();
        assert_eq!(restored, assessment);
    }

    #[test]
    fn absent_evidence_and_identity_remapping_do_not_claim_completeness() {
        #[derive(Deserialize)]
        struct LegacyRecord {
            #[serde(default)]
            assessment: EffectAssessment,
        }
        let legacy: LegacyRecord = serde_json::from_str("{}").unwrap();
        assert_eq!(
            legacy.assessment.classification(),
            EffectClassification::Unknown
        );
        let incomplete = EffectAssessment {
            complete: false,
            ..EffectAssessment::complete()
        };
        assert_eq!(incomplete.remap_identities(Clone::clone), incomplete);
    }

    fn assessment_strategy() -> impl Strategy<Value = EffectAssessment> {
        prop::collection::vec((0u8..7, 0usize..5, 0usize..3), 0..20).prop_map(|records| {
            records.into_iter().fold(
                EffectAssessment::complete(),
                |assessment, (kind, line, column)| {
                    let mut source = provenance();
                    source.line = line;
                    source.column = Some(column);
                    add_generated_record(assessment, kind, source)
                },
            )
        })
    }

    fn add_generated_record(
        assessment: EffectAssessment,
        kind: u8,
        source: EffectProvenance,
    ) -> EffectAssessment {
        if kind == 6 {
            return assessment.with_dependency(EffectDependency {
                target: FunctionId::new("src/callee.rs".into(), "callee".into(), source.line),
                provenance: source,
            });
        }
        let kinds = [
            ObservedEffectKind::LocalMutation,
            ObservedEffectKind::ExternalRead,
            ObservedEffectKind::ExternalWrite,
            ObservedEffectKind::Io,
            ObservedEffectKind::Nondeterminism,
        ];
        match kinds.get(usize::from(kind)) {
            Some(&kind) => assessment.with_effect(ObservedEffect {
                kind,
                detail: format!("{kind:?}"),
                provenance: source,
            }),
            None => assessment.with_unresolved(UnresolvedBehavior {
                provenance: source,
                ..unresolved()
            }),
        }
    }

    fn qualifies_for_discount(assessment: &EffectAssessment) -> bool {
        matches!(
            assessment.classification(),
            EffectClassification::StrictlyPure | EffectClassification::LocallyPure
        )
    }

    proptest! {
        #[test]
        fn evidence_joins_obey_semilattice_laws(
            a in assessment_strategy(), b in assessment_strategy(), c in assessment_strategy()
        ) {
            prop_assert_eq!(a.join(&b), b.join(&a));
            prop_assert_eq!(a.join(&a), a.clone());
            prop_assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
        }

        #[test]
        fn joining_preserves_all_observed_source_effects(
            a in assessment_strategy(), b in assessment_strategy()
        ) {
            let joined = a.join(&b);
            for key in a.observed.keys().chain(b.observed.keys()) {
                prop_assert!(joined.observed.contains_key(key));
            }
        }

        #[test]
        fn adding_uncertainty_cannot_create_a_purity_discount(a in assessment_strategy()) {
            let incomplete = a.with_unresolved(unresolved());
            prop_assert!(!qualifies_for_discount(&incomplete));
            prop_assert!(!incomplete.is_complete());
        }

        #[test]
        fn serialization_is_independent_of_join_order(
            a in assessment_strategy(), b in assessment_strategy()
        ) {
            prop_assert_eq!(
                serde_json::to_string(&a.join(&b)).unwrap(),
                serde_json::to_string(&b.join(&a)).unwrap()
            );
        }
    }

    #[test]
    fn classification_follows_evidence_contract() {
        for (assessment, expected) in [
            (
                EffectAssessment::complete(),
                EffectClassification::StrictlyPure,
            ),
            (
                EffectAssessment::complete().with_effect(effect(ObservedEffectKind::LocalMutation)),
                EffectClassification::LocallyPure,
            ),
            (
                EffectAssessment::complete().with_effect(effect(ObservedEffectKind::ExternalRead)),
                EffectClassification::ReadOnly,
            ),
            (
                EffectAssessment::complete().with_effect(effect(ObservedEffectKind::ExternalWrite)),
                EffectClassification::Impure,
            ),
            (
                EffectAssessment::complete().with_effect(effect(ObservedEffectKind::Io)),
                EffectClassification::Impure,
            ),
            (
                EffectAssessment::complete()
                    .with_effect(effect(ObservedEffectKind::Nondeterminism)),
                EffectClassification::Impure,
            ),
            (
                EffectAssessment::complete().with_unresolved(unresolved()),
                EffectClassification::Unknown,
            ),
            (
                EffectAssessment::complete()
                    .with_effect(effect(ObservedEffectKind::ExternalRead))
                    .with_unresolved(unresolved()),
                EffectClassification::Unknown,
            ),
            (
                EffectAssessment::complete()
                    .with_effect(effect(ObservedEffectKind::Io))
                    .with_unresolved(unresolved()),
                EffectClassification::Impure,
            ),
        ] {
            assert_eq!(assessment.classification(), expected);
        }
    }

    #[test]
    fn join_is_associative_commutative_idempotent_and_deduplicated() {
        let a = EffectAssessment::complete().with_effect(effect(ObservedEffectKind::LocalMutation));
        let b = EffectAssessment::complete().with_effect(effect(ObservedEffectKind::ExternalRead));
        let c = EffectAssessment::complete().with_unresolved(unresolved());
        assert_eq!(a.join(&b), b.join(&a));
        assert_eq!(a.join(&a), a);
        assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
        assert_eq!(a.join(&a).observed().count(), 1);
    }

    #[test]
    fn wording_and_dependency_do_not_defeat_source_identity_deduplication() {
        let first = effect(ObservedEffectKind::Io);
        let mut second = first.clone();
        second.detail = "different wording".into();
        second.provenance.dependency = Some(FunctionId::new("src/io.rs".into(), "write".into(), 9));
        let joined = EffectAssessment::complete()
            .with_effect(first)
            .join(&EffectAssessment::complete().with_effect(second));
        assert_eq!(joined.observed().count(), 1);
    }

    #[test]
    fn dependencies_deduplicate_and_remap_all_definition_identities() {
        let owner = FunctionId::new("src/lib.rs".into(), "caller".into(), 3);
        let target = FunctionId::new("src/lib.rs".into(), "target".into(), 8);
        let mut source = EffectProvenance::source(owner.clone(), 4, Some(2));
        source.dependency = Some(target.clone());
        let dependency = EffectDependency {
            target: target.clone(),
            provenance: source,
        };
        let assessment = EffectAssessment::complete()
            .with_dependency(dependency.clone())
            .join(&EffectAssessment::complete().with_dependency(dependency));
        assert_eq!(assessment.dependencies().count(), 1);

        let remapped = assessment.remap_identities(|id| FunctionId {
            name: format!("canonical::{}", id.name),
            ..id.clone()
        });
        let dependency = remapped.dependencies().next().unwrap();
        assert_eq!(dependency.target.name, "canonical::target");
        assert_eq!(dependency.provenance.owner.name, "canonical::caller");
        assert_eq!(
            dependency
                .provenance
                .dependency
                .as_ref()
                .map(|id| id.name.as_str()),
            Some("canonical::target")
        );
    }

    #[test]
    fn uncertainty_never_qualifies_for_a_discount() {
        let known =
            EffectAssessment::complete().with_effect(effect(ObservedEffectKind::LocalMutation));
        assert_eq!(known.classification(), EffectClassification::LocallyPure);
        assert_eq!(
            known.with_unresolved(unresolved()).classification(),
            EffectClassification::Unknown
        );
    }
}

//! Inter-Procedural Purity Propagation
//!
//! This module implements two-phase purity analysis that propagates purity information
//! from callees to callers, enabling whole-program purity inference.
//!
//! # Architecture
//!
//! - **Phase 1**: Intrinsic analysis of each function in isolation using PurityAnalyzer
//! - **Phase 2**: Bottom-up propagation of purity through the call graph
//!
//! # Example
//!
//! ```ignore
//! let propagator = PurityPropagator::new(call_graph, purity_analyzer);
//! propagator.propagate(&functions)?;
//! ```

mod cache;
mod call_graph_adapter;
mod known_pure_functions;
mod metrics;

pub use cache::{PURITY_MODEL_VERSION, PurityCache, hash_deps, hash_deps_with_assessments};
pub use call_graph_adapter::PurityCallGraphAdapter;
pub use known_pure_functions::{
    CalleeEvidence, CalleePurity, aggregate_callee_purity, resolve_callee_purity,
};

use crate::analysis::effect_evidence::{
    EffectAssessment, EffectClassification, ObservedEffect, UnresolvedBehavior, UnresolvedReason,
};
use crate::analysis::purity_analysis::{PurityAnalysis, PurityAnalyzer, PurityLevel};
use crate::core::FunctionMetrics;
use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use dashmap::DashMap;
use std::collections::{BTreeMap, BTreeSet};

/// Result of purity propagation for a function
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PurityResult {
    pub level: PurityLevel,
    pub confidence: f64,
    pub reason: PurityReason,
    /// Authoritative internal assessment. `level` is a compatibility projection.
    #[serde(default)]
    pub assessment: EffectAssessment,
}

/// Reason for purity classification
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PurityReason {
    /// Function has no side effects or calls
    Intrinsic,

    /// All dependencies are pure
    PropagatedFromDeps { depth: usize },

    /// Has side effects
    SideEffects { effects: Vec<String> },

    /// Part of recursive cycle with side effects
    RecursiveWithSideEffects,

    /// Part of recursive cycle but otherwise pure
    RecursivePure,

    /// Unknown dependencies
    UnknownDeps { count: usize },

    /// Evidence-backed result with observed effects and unresolved behavior kept separate.
    Evidence {
        observed: Vec<String>,
        unresolved: Vec<String>,
    },
}

impl PurityResult {
    /// Convert from existing PurityAnalysis (phase 1 result)
    pub fn from_analysis(analysis: PurityAnalysis) -> Self {
        let reason = if !analysis.violations.is_empty() {
            PurityReason::SideEffects {
                effects: analysis
                    .violations
                    .iter()
                    .map(|v| v.description())
                    .collect(),
            }
        } else {
            PurityReason::Intrinsic
        };

        Self {
            level: analysis.purity,
            confidence: 1.0,
            reason,
            assessment: EffectAssessment::unknown(
                UnresolvedReason::LegacyEvidence,
                "legacy purity analysis has no semantic effect evidence",
            ),
        }
    }
}

/// Purity propagator for whole-program analysis
pub struct PurityPropagator {
    /// Cache of function purity results
    cache: DashMap<FunctionId, PurityResult>,

    /// Call graph adapter for dependency tracking
    call_graph: PurityCallGraphAdapter,

    /// Existing purity analyzer for intrinsic analysis (phase 1)
    #[allow(dead_code)]
    purity_analyzer: PurityAnalyzer,
}

impl PurityPropagator {
    /// Create a new purity propagator
    pub fn new(call_graph: PurityCallGraphAdapter, purity_analyzer: PurityAnalyzer) -> Self {
        Self {
            cache: DashMap::new(),
            call_graph,
            purity_analyzer,
        }
    }

    /// Propagate purity information through all functions
    pub fn propagate(&mut self, functions: &[FunctionMetrics]) -> Result<()> {
        self.cache.clear();
        let mut intrinsic = BTreeMap::new();
        for func in functions {
            let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line)
                .with_column(func.column);
            let initial = self.analyze_intrinsic_purity(func, &func_id)?;
            intrinsic.insert(func_id, initial);
        }
        let assessments = intrinsic
            .iter()
            .map(|(id, result)| (id.clone(), result.assessment.clone()))
            .collect();
        for (id, assessment) in propagate_assessment_map(&assessments) {
            let confidence = intrinsic
                .get(&id)
                .map(|result| result.confidence)
                .unwrap_or(1.0);
            self.cache
                .insert(id, result_from_assessment(assessment, confidence));
        }

        Ok(())
    }

    /// Analyze intrinsic purity using existing PurityAnalyzer
    fn analyze_intrinsic_purity(
        &self,
        func: &FunctionMetrics,
        func_id: &FunctionId,
    ) -> Result<PurityResult> {
        Ok(self
            .call_graph
            .effect_assessment(func_id)
            .map(|assessment| result_from_assessment(assessment, 1.0))
            .unwrap_or_else(|| metrics::intrinsic_purity(func)))
    }

    /// Apply each definition's complete purity fact without losing refined levels.
    pub(crate) fn apply_results(&self, metrics: &[FunctionMetrics]) -> Vec<FunctionMetrics> {
        metrics
            .iter()
            .map(|metric| {
                let id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
                    .with_column(metric.column);
                self.get_result(&id)
                    .map(|result| result.apply_to_metric(metric))
                    .unwrap_or_else(|| metric.clone())
            })
            .collect()
    }

    /// Get the purity result for a function
    pub fn get_result(&self, func_id: &FunctionId) -> Option<PurityResult> {
        self.cache.get(func_id).map(|r| r.clone())
    }

    pub(crate) fn assessments(&self) -> BTreeMap<FunctionId, EffectAssessment> {
        self.cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.assessment.clone()))
            .collect()
    }
}

/// Propagate the same typed summaries used by scoring and reporting.
pub(crate) fn propagate_graph_assessments(
    graph: &crate::priority::call_graph::CallGraph,
) -> BTreeMap<FunctionId, EffectAssessment> {
    if graph.effect_assessments_are_propagated() {
        return graph
            .effect_assessments()
            .map(|(id, assessment)| (id.clone(), assessment.clone()))
            .collect();
    }
    let intrinsic = graph
        .effect_assessments()
        .map(|(id, assessment)| (id.clone(), assessment.clone()))
        .collect();
    propagate_assessment_map(&intrinsic)
}

fn propagate_assessment_map(
    intrinsic: &BTreeMap<FunctionId, EffectAssessment>,
) -> BTreeMap<FunctionId, EffectAssessment> {
    let mut current = intrinsic.clone();
    let dependents = intrinsic.iter().fold(
        BTreeMap::<FunctionId, BTreeSet<FunctionId>>::new(),
        |mut result, (caller, assessment)| {
            for dependency in assessment.dependencies() {
                if intrinsic.contains_key(&dependency.target) {
                    result
                        .entry(dependency.target.clone())
                        .or_default()
                        .insert(caller.clone());
                }
            }
            result
        },
    );
    for component in invocation_components(intrinsic) {
        let mut pending = component.clone();
        while let Some(id) = pending.pop_first() {
            let Some(base) = intrinsic.get(&id) else {
                continue;
            };
            let assessment = base
                .dependencies()
                .fold(base.clone(), |result, dependency| {
                    let summary = current
                        .get(&dependency.target)
                        .map(|callee| dependency_assessment(dependency, callee))
                        .unwrap_or_else(|| missing_dependency(dependency));
                    result.merge(&summary)
                });
            if current.get(&id) != Some(&assessment) {
                current.insert(id.clone(), assessment);
                if let Some(callers) = dependents.get(&id) {
                    pending.extend(callers.intersection(&component).cloned());
                }
            }
        }
    }
    current
}

/// Callee-first components; recursion is solved only by monotonically joining facts.
fn invocation_components(
    intrinsic: &BTreeMap<FunctionId, EffectAssessment>,
) -> Vec<BTreeSet<FunctionId>> {
    let mut graph = petgraph::graph::DiGraph::<&FunctionId, ()>::new();
    let nodes: BTreeMap<_, _> = intrinsic
        .keys()
        .map(|id| (id, graph.add_node(id)))
        .collect();
    for (id, assessment) in intrinsic {
        for dependency in assessment.dependencies() {
            if let Some(target) = nodes.get(&dependency.target) {
                graph.add_edge(nodes[id], *target, ());
            }
        }
    }
    petgraph::algo::kosaraju_scc(&graph)
        .into_iter()
        .map(|component| {
            component
                .into_iter()
                .map(|node| graph[node].clone())
                .collect()
        })
        .collect()
}

fn dependency_assessment(
    dependency: &crate::analysis::effect_evidence::EffectDependency,
    callee: &EffectAssessment,
) -> EffectAssessment {
    let observed_kinds: BTreeSet<_> = callee.observed().map(|effect| effect.kind).collect();
    let observed = observed_kinds
        .into_iter()
        .fold(EffectAssessment::complete(), |result, kind| {
            let mut provenance = dependency.provenance.clone();
            provenance.dependency = Some(dependency.target.clone());
            result.with_effect(ObservedEffect {
                kind,
                detail: format!("{kind:?} in {}", dependency.target.name),
                provenance,
            })
        });
    let unresolved_reasons: BTreeSet<_> = callee
        .unresolved()
        .map(|unresolved| unresolved.reason)
        .collect();
    unresolved_reasons
        .into_iter()
        .fold(observed, |result, reason| {
            let mut provenance = dependency.provenance.clone();
            provenance.dependency = Some(dependency.target.clone());
            result.with_unresolved(UnresolvedBehavior {
                reason,
                detail: format!("{reason:?} in {}", dependency.target.name),
                provenance,
            })
        })
}

fn missing_dependency(
    dependency: &crate::analysis::effect_evidence::EffectDependency,
) -> EffectAssessment {
    EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
        reason: UnresolvedReason::UnavailableBody,
        detail: format!("body unavailable for {}", dependency.target.name),
        provenance: dependency.provenance.clone(),
    })
}

fn result_from_assessment(assessment: EffectAssessment, confidence: f64) -> PurityResult {
    let level = match assessment.classification() {
        EffectClassification::StrictlyPure => PurityLevel::StrictlyPure,
        EffectClassification::LocallyPure => PurityLevel::LocallyPure,
        EffectClassification::ReadOnly => PurityLevel::ReadOnly,
        EffectClassification::Impure | EffectClassification::Unknown => PurityLevel::Impure,
    };
    let observed = assessment
        .observed()
        .map(|effect| effect.detail.clone())
        .collect();
    let unresolved = assessment
        .unresolved()
        .map(|behavior| behavior.detail.clone())
        .collect();
    PurityResult {
        level,
        confidence,
        reason: PurityReason::Evidence {
            observed,
            unresolved,
        },
        assessment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::effect_evidence::{
        EffectDependency, EffectProvenance, ObservedEffectKind,
    };

    fn id(name: &str, line: usize) -> FunctionId {
        FunctionId::new("src/lib.rs".into(), name.into(), line)
    }

    fn assessment(classification: EffectClassification, owner: &FunctionId) -> EffectAssessment {
        let provenance = EffectProvenance::source(owner.clone(), owner.line, owner.column);
        match classification {
            EffectClassification::StrictlyPure => EffectAssessment::complete(),
            EffectClassification::LocallyPure => {
                effect(ObservedEffectKind::LocalMutation, provenance)
            }
            EffectClassification::ReadOnly => effect(ObservedEffectKind::ExternalRead, provenance),
            EffectClassification::Impure => effect(ObservedEffectKind::Io, provenance),
            EffectClassification::Unknown => {
                EffectAssessment::complete().with_unresolved(UnresolvedBehavior {
                    reason: UnresolvedReason::UnsupportedDispatch,
                    detail: "unknown".into(),
                    provenance,
                })
            }
        }
    }

    fn effect(kind: ObservedEffectKind, provenance: EffectProvenance) -> EffectAssessment {
        EffectAssessment::complete().with_effect(ObservedEffect {
            kind,
            detail: format!("{kind:?}"),
            provenance,
        })
    }

    #[test]
    fn every_caller_callee_combination_preserves_the_lattice_and_unknown() {
        let classes = [
            EffectClassification::StrictlyPure,
            EffectClassification::LocallyPure,
            EffectClassification::ReadOnly,
            EffectClassification::Impure,
            EffectClassification::Unknown,
        ];
        let caller = id("caller", 1);
        let callee = id("callee", 2);
        let dependency = EffectDependency {
            target: callee.clone(),
            provenance: EffectProvenance::source(caller.clone(), 3, Some(4)),
        };
        for left in classes {
            for right in classes {
                let propagated = assessment(left, &caller).join(&dependency_assessment(
                    &dependency,
                    &assessment(right, &callee),
                ));
                let expected = if left == EffectClassification::Impure
                    || right == EffectClassification::Impure
                {
                    EffectClassification::Impure
                } else if left == EffectClassification::Unknown
                    || right == EffectClassification::Unknown
                {
                    EffectClassification::Unknown
                } else if left == EffectClassification::ReadOnly
                    || right == EffectClassification::ReadOnly
                {
                    EffectClassification::ReadOnly
                } else if left == EffectClassification::LocallyPure
                    || right == EffectClassification::LocallyPure
                {
                    EffectClassification::LocallyPure
                } else {
                    EffectClassification::StrictlyPure
                };
                assert_eq!(
                    propagated.classification(),
                    expected,
                    "{left:?} + {right:?}"
                );
            }
        }
    }

    #[test]
    fn missing_dependency_is_unknown_not_impure() {
        let dependency = EffectDependency {
            target: id("missing", 9),
            provenance: EffectProvenance::source(id("caller", 1), 3, Some(4)),
        };
        assert_eq!(
            missing_dependency(&dependency).classification(),
            EffectClassification::Unknown
        );
    }

    #[test]
    fn propagation_keeps_source_facts_once_and_uses_compact_dependency_summaries() {
        let caller = id("caller", 1);
        let callee = id("callee", 10);
        let dependency = EffectDependency {
            target: callee.clone(),
            provenance: EffectProvenance::source(caller, 3, Some(4)),
        };
        let callee_assessment =
            [11, 12]
                .into_iter()
                .fold(EffectAssessment::complete(), |assessment, line| {
                    assessment.with_effect(ObservedEffect {
                        kind: ObservedEffectKind::Io,
                        detail: format!("I/O at line {line}"),
                        provenance: EffectProvenance::source(callee.clone(), line, Some(8)),
                    })
                });

        let propagated = dependency_assessment(&dependency, &callee_assessment);

        assert_eq!(callee_assessment.observed().count(), 2);
        let summaries: Vec<_> = propagated.observed().collect();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].provenance.owner.name, "caller");
        assert_eq!(
            summaries[0].provenance.dependency.as_ref(),
            Some(&dependency.target)
        );
        assert_eq!(summaries[0].detail, "Io in callee");
    }
}

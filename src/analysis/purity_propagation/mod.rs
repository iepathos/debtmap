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

pub use cache::PurityCache;
pub use call_graph_adapter::PurityCallGraphAdapter;
pub use known_pure_functions::{
    aggregate_callee_purity, resolve_callee_purity, CalleeEvidence, CalleePurity,
};

use crate::analysis::purity_analysis::{PurityAnalysis, PurityAnalyzer, PurityLevel};
use crate::core::FunctionMetrics;
use crate::priority::call_graph::FunctionId;
use anyhow::{anyhow, Result};
use dashmap::DashMap;

/// Result of purity propagation for a function
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PurityResult {
    pub level: PurityLevel,
    pub confidence: f64,
    pub reason: PurityReason,
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

struct DependencyPuritySummary {
    all_deps_pure: bool,
    aggregated_confidence: f64,
    impure_reasons: Vec<String>,
    max_depth: usize,
    unknown_count: usize,
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
        // Phase 1: Initial purity analysis using existing PurityAnalyzer
        for func in functions {
            let initial = self.analyze_intrinsic_purity(func)?;
            let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
            self.cache.insert(func_id, initial);
        }

        // Phase 2: Propagate purity bottom-up
        let sorted = self.call_graph.topological_sort()?;

        for func_id in sorted {
            self.propagate_for_function(&func_id)?;
        }

        Ok(())
    }

    /// Analyze intrinsic purity using existing PurityAnalyzer
    fn analyze_intrinsic_purity(&self, func: &FunctionMetrics) -> Result<PurityResult> {
        // For now, use existing purity information if available
        if let (Some(is_pure), Some(confidence)) = (func.is_pure, func.purity_confidence) {
            let level = if is_pure {
                PurityLevel::StrictlyPure
            } else {
                PurityLevel::Impure
            };
            return Ok(PurityResult {
                level,
                confidence: confidence as f64,
                reason: PurityReason::Intrinsic,
            });
        }

        // Default to impure with low confidence if no information available
        Ok(PurityResult {
            level: PurityLevel::Impure,
            confidence: 0.3,
            reason: PurityReason::UnknownDeps { count: 0 },
        })
    }

    /// Propagate purity for a single function
    ///
    /// Uses call graph and known pure std functions (Spec 261) to propagate
    /// purity information from callees to callers.
    fn propagate_for_function(&mut self, func_id: &FunctionId) -> Result<()> {
        let result = self.cached_result(func_id)?;
        let deps = self.call_graph.get_dependencies(func_id);
        let result = if self.call_graph.is_in_cycle(func_id) {
            propagate_recursive_result(result)
        } else {
            let callee_evidence = self.callee_evidence(&deps);
            let summary = self.dependency_summary(&deps, &callee_evidence);
            propagate_dependency_result(result, summary)
        };

        self.cache.insert(func_id.clone(), result);
        Ok(())
    }

    fn cached_result(&self, func_id: &FunctionId) -> Result<PurityResult> {
        self.cache
            .get(func_id)
            .map(|result| result.clone())
            .ok_or_else(|| anyhow!("Function not in cache"))
    }

    fn callee_evidence(&self, deps: &[FunctionId]) -> Vec<CalleeEvidence> {
        deps.iter()
            .map(|dep_id| CalleeEvidence {
                callee_name: dep_id.name.clone(),
                callee_purity: resolve_callee_purity(
                    &dep_id.name,
                    None,
                    self.cached_callee_purity(dep_id),
                ),
            })
            .collect()
    }

    fn cached_callee_purity(&self, dep_id: &FunctionId) -> Option<(bool, f64)> {
        self.cache.get(dep_id).map(|result| {
            let is_pure = result.level == PurityLevel::StrictlyPure;
            (is_pure, result.confidence)
        })
    }

    fn dependency_summary(
        &self,
        deps: &[FunctionId],
        evidence: &[CalleeEvidence],
    ) -> DependencyPuritySummary {
        let (all_deps_pure, aggregated_confidence, impure_reasons) =
            aggregate_callee_purity(evidence);

        DependencyPuritySummary {
            all_deps_pure,
            aggregated_confidence,
            impure_reasons,
            max_depth: self.max_dependency_depth(deps),
            unknown_count: count_unknown_dependencies(evidence),
        }
    }

    fn max_dependency_depth(&self, deps: &[FunctionId]) -> usize {
        deps.iter()
            .filter_map(|dep_id| self.cache.get(dep_id))
            .filter_map(|result| propagated_depth(&result.reason))
            .max()
            .unwrap_or(0)
    }

    /// Get the purity result for a function
    pub fn get_result(&self, func_id: &FunctionId) -> Option<PurityResult> {
        self.cache.get(func_id).map(|r| r.clone())
    }
}

fn propagate_recursive_result(mut result: PurityResult) -> PurityResult {
    if is_recursion_eligible_pure(&result.level) {
        result.reason = PurityReason::RecursivePure;
        result.confidence *= 0.7;
        return result;
    }

    result.level = PurityLevel::Impure;
    result.reason = PurityReason::RecursiveWithSideEffects;
    result.confidence = 0.95;
    result
}

fn propagate_dependency_result(
    result: PurityResult,
    summary: DependencyPuritySummary,
) -> PurityResult {
    if has_pure_dependencies(&result, &summary) {
        return propagated_pure_result(result, &summary);
    }

    if !summary.impure_reasons.is_empty() {
        return propagated_impure_result(summary);
    }

    if summary.unknown_count > 0 {
        return propagated_unknown_result(result, &summary);
    }

    result
}

fn is_recursion_eligible_pure(level: &PurityLevel) -> bool {
    matches!(level, PurityLevel::StrictlyPure | PurityLevel::LocallyPure)
}

fn has_pure_dependencies(result: &PurityResult, summary: &DependencyPuritySummary) -> bool {
    summary.all_deps_pure
        && result.level != PurityLevel::Impure
        && summary.impure_reasons.is_empty()
}

fn propagated_pure_result(
    mut result: PurityResult,
    summary: &DependencyPuritySummary,
) -> PurityResult {
    let depth = summary.max_depth + 1;
    let depth_confidence = 0.9_f64.powi(depth as i32);

    result.level = PurityLevel::StrictlyPure;
    result.reason = PurityReason::PropagatedFromDeps { depth };
    result.confidence =
        (result.confidence * depth_confidence * summary.aggregated_confidence).clamp(0.5, 1.0);
    result
}

fn propagated_impure_result(summary: DependencyPuritySummary) -> PurityResult {
    PurityResult {
        level: PurityLevel::Impure,
        confidence: summary.aggregated_confidence,
        reason: PurityReason::SideEffects {
            effects: summary.impure_reasons,
        },
    }
}

fn propagated_unknown_result(
    mut result: PurityResult,
    summary: &DependencyPuritySummary,
) -> PurityResult {
    result.reason = PurityReason::UnknownDeps {
        count: summary.unknown_count,
    };
    result.confidence = (result.confidence * summary.aggregated_confidence).clamp(0.3, 1.0);
    result
}

fn propagated_depth(reason: &PurityReason) -> Option<usize> {
    match reason {
        PurityReason::PropagatedFromDeps { depth } => Some(*depth),
        _ => None,
    }
}

fn count_unknown_dependencies(evidence: &[CalleeEvidence]) -> usize {
    evidence
        .iter()
        .filter(|entry| matches!(entry.callee_purity, CalleePurity::Unknown))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(level: PurityLevel, confidence: f64) -> PurityResult {
        PurityResult {
            level,
            confidence,
            reason: PurityReason::Intrinsic,
        }
    }

    fn summary(
        all_deps_pure: bool,
        aggregated_confidence: f64,
        impure_reasons: Vec<String>,
        max_depth: usize,
        unknown_count: usize,
    ) -> DependencyPuritySummary {
        DependencyPuritySummary {
            all_deps_pure,
            aggregated_confidence,
            impure_reasons,
            max_depth,
            unknown_count,
        }
    }

    #[test]
    fn recursive_pure_result_keeps_purity_with_penalty() {
        let propagated = propagate_recursive_result(result(PurityLevel::StrictlyPure, 0.9));

        assert_eq!(propagated.level, PurityLevel::StrictlyPure);
        assert_eq!(propagated.reason, PurityReason::RecursivePure);
        assert!((propagated.confidence - 0.63).abs() < f64::EPSILON);
    }

    #[test]
    fn recursive_impure_result_marks_side_effect_cycle() {
        let propagated = propagate_recursive_result(result(PurityLevel::Impure, 0.4));

        assert_eq!(propagated.level, PurityLevel::Impure);
        assert_eq!(propagated.reason, PurityReason::RecursiveWithSideEffects);
        assert!((propagated.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn pure_dependencies_set_depth_and_confidence() {
        let propagated = propagate_dependency_result(
            result(PurityLevel::LocallyPure, 0.95),
            summary(true, 0.9, Vec::new(), 2, 0),
        );

        assert_eq!(propagated.level, PurityLevel::StrictlyPure);
        assert_eq!(
            propagated.reason,
            PurityReason::PropagatedFromDeps { depth: 3 }
        );
        assert!(propagated.confidence < 0.95);
    }

    #[test]
    fn impure_dependencies_override_current_result() {
        let propagated = propagate_dependency_result(
            result(PurityLevel::StrictlyPure, 0.95),
            summary(
                false,
                0.95,
                vec!["Calls impure function: write".into()],
                0,
                0,
            ),
        );

        assert_eq!(propagated.level, PurityLevel::Impure);
        assert_eq!(
            propagated.reason,
            PurityReason::SideEffects {
                effects: vec!["Calls impure function: write".into()]
            }
        );
    }

    #[test]
    fn unknown_dependencies_reduce_confidence_without_changing_level() {
        let propagated = propagate_dependency_result(
            result(PurityLevel::StrictlyPure, 0.8),
            summary(true, 0.9, Vec::new(), 0, 2),
        );

        assert_eq!(propagated.level, PurityLevel::StrictlyPure);
        assert_eq!(
            propagated.reason,
            PurityReason::PropagatedFromDeps { depth: 1 }
        );
    }

    #[test]
    fn counts_unknown_dependencies_from_evidence() {
        let evidence = vec![
            CalleeEvidence {
                callee_name: "external".into(),
                callee_purity: CalleePurity::Unknown,
            },
            CalleeEvidence {
                callee_name: "len".into(),
                callee_purity: CalleePurity::KnownPure,
            },
        ];

        assert_eq!(count_unknown_dependencies(&evidence), 1);
    }
}

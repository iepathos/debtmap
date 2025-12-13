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
        // Get current purity result
        let mut result = self
            .cache
            .get(func_id)
            .ok_or_else(|| anyhow!("Function not in cache"))?
            .clone();

        // Get all dependencies
        let deps = self.call_graph.get_dependencies(func_id);

        // Check if function is in a cycle (recursive)
        if self.call_graph.is_in_cycle(func_id) {
            // Distinguish between pure recursion and recursion with side effects
            if result.level == PurityLevel::StrictlyPure || result.level == PurityLevel::LocallyPure
            {
                // Pure structural recursion (e.g., factorial, tree traversal)
                // Keep pure but reduce confidence due to recursion complexity
                result.reason = PurityReason::RecursivePure;
                result.confidence *= 0.7; // Penalty for recursion
            } else {
                // Recursion with side effects is impure
                result.level = PurityLevel::Impure;
                result.reason = PurityReason::RecursiveWithSideEffects;
                result.confidence = 0.95;
            }
            self.cache.insert(func_id.clone(), result);
            return Ok(());
        }

        // Build callee evidence using known pure functions (Spec 261)
        let mut callee_evidence = Vec::new();

        for dep_id in &deps {
            // Get cached purity if available
            let cached_purity = self.cache.get(dep_id).map(|r| {
                let is_pure = r.level == PurityLevel::StrictlyPure;
                (is_pure, r.confidence)
            });

            // Resolve callee purity using known pure std functions
            let callee_purity = resolve_callee_purity(&dep_id.name, None, cached_purity);

            callee_evidence.push(CalleeEvidence {
                callee_name: dep_id.name.clone(),
                callee_purity,
            });
        }

        // Aggregate purity from all callees
        let (all_deps_pure, aggregated_confidence, impure_reasons) =
            aggregate_callee_purity(&callee_evidence);

        // Track propagation depth for pure deps
        let max_depth = deps
            .iter()
            .filter_map(|dep_id| self.cache.get(dep_id))
            .filter_map(|r| {
                if let PurityReason::PropagatedFromDeps { depth } = r.reason {
                    Some(depth)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        // Count unknown dependencies (not in cache and not known pure)
        let unknown_count = callee_evidence
            .iter()
            .filter(|e| matches!(e.callee_purity, CalleePurity::Unknown))
            .count();

        // Update purity based on callee analysis
        if all_deps_pure && result.level != PurityLevel::Impure && impure_reasons.is_empty() {
            result.level = PurityLevel::StrictlyPure;
            result.reason = PurityReason::PropagatedFromDeps {
                depth: max_depth + 1,
            };

            // Combine confidence from depth and callee analysis
            let depth_confidence = 0.9_f64.powi((max_depth + 1) as i32);
            result.confidence =
                (result.confidence * depth_confidence * aggregated_confidence).clamp(0.5, 1.0);
        } else if !impure_reasons.is_empty() {
            // Has impure callees
            result.level = PurityLevel::Impure;
            result.reason = PurityReason::SideEffects {
                effects: impure_reasons,
            };
            result.confidence = aggregated_confidence;
        } else if unknown_count > 0 {
            result.reason = PurityReason::UnknownDeps {
                count: unknown_count,
            };
            result.confidence = (result.confidence * aggregated_confidence).clamp(0.3, 1.0);
        }

        self.cache.insert(func_id.clone(), result);
        Ok(())
    }

    /// Get the purity result for a function
    pub fn get_result(&self, func_id: &FunctionId) -> Option<PurityResult> {
        self.cache.get(func_id).map(|r| r.clone())
    }
}

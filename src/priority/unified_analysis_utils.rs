//! Utility operations for UnifiedAnalysis.
//!
//! This module provides methods for managing debt items, accessing data flow
//! information, and performing auxiliary operations on UnifiedAnalysis instances.

use super::{FileDebtItem, UnifiedAnalysis, UnifiedDebtItem};
use crate::data_flow::{DataFlowGraph, IoOperation, PurityInfo};
use crate::priority::call_graph::FunctionId;
use std::cmp::Ordering;

// Pure comparison functions for zero-copy sorting (spec 204)

/// Compare debt items by score (pure function).
/// Returns descending order (highest scores first).
fn compare_debt_items_by_score(a: &UnifiedDebtItem, b: &UnifiedDebtItem) -> Ordering {
    b.unified_score
        .final_score
        .partial_cmp(&a.unified_score.final_score)
        .unwrap_or(Ordering::Equal)
}

/// Compare file items by score (pure function).
/// Returns descending order (highest scores first).
fn compare_file_items_by_score(a: &FileDebtItem, b: &FileDebtItem) -> Ordering {
    b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
}

/// Extension trait providing utility operations for UnifiedAnalysis
pub trait UnifiedAnalysisUtils {
    /// Get timing information for the analysis phases
    fn timings(&self) -> Option<&crate::builders::parallel_unified_analysis::AnalysisPhaseTimings>;

    /// Add a file-level debt item
    fn add_file_item(&mut self, item: FileDebtItem);

    /// Add a function-level debt item
    fn add_item(&mut self, item: UnifiedDebtItem);

    /// Sort all items by priority score
    fn sort_by_priority(&mut self);

    /// Get a reference to the data flow graph
    fn data_flow_graph(&self) -> &DataFlowGraph;

    /// Get a mutable reference to the data flow graph
    fn data_flow_graph_mut(&mut self) -> &mut DataFlowGraph;

    /// Populate the data flow graph with purity analysis data
    fn populate_purity_analysis(&mut self, metrics: &[crate::core::FunctionMetrics]);

    /// Add an I/O operation to the data flow graph
    fn add_io_operation(&mut self, func_id: FunctionId, operation: IoOperation);

    /// Add variable dependencies to the data flow graph
    fn add_variable_dependencies(
        &mut self,
        func_id: FunctionId,
        variables: std::collections::HashSet<String>,
    );

    /// Apply file context adjustments to all debt item scores (spec 166)
    ///
    /// Adjusts scores based on file context (test vs production).
    /// Test files receive reduced scores to avoid false positives.
    fn apply_file_context_adjustments(
        &mut self,
        file_contexts: &std::collections::HashMap<std::path::PathBuf, crate::analysis::FileContext>,
    );
}

impl UnifiedAnalysisUtils for UnifiedAnalysis {
    fn timings(&self) -> Option<&crate::builders::parallel_unified_analysis::AnalysisPhaseTimings> {
        self.timings.as_ref()
    }

    fn add_file_item(&mut self, item: FileDebtItem) {
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();

        // Filter out items below minimum thresholds
        // Items with score 0.0 are "non-debt" and should always be excluded
        if item.score <= 0.0 || item.score < min_score {
            return;
        }

        // Check for duplicates before adding
        let is_duplicate = self
            .file_items
            .iter()
            .any(|existing| existing.metrics.path == item.metrics.path);

        if !is_duplicate {
            self.file_items.push_back(item);
        }
    }

    fn add_item(&mut self, item: UnifiedDebtItem) {
        use crate::priority::filter_config::ItemFilterConfig;
        use crate::priority::filter_predicates::*;

        self.stats.total_items_processed += 1;

        // Items with score 0.0 are "non-debt" and should always be filtered out,
        // regardless of item type (including god objects)
        if item.unified_score.final_score <= 0.0 {
            self.stats.filtered_by_score += 1;
            return;
        }

        // God objects bypass other filters as they represent critical architectural issues (spec 207)
        let is_god_object = item
            .god_object_indicators
            .as_ref()
            .is_some_and(|indicators| indicators.is_god_object);

        if !is_god_object {
            // Get unified filter configuration (spec 243: single-stage filtering)
            let config = ItemFilterConfig::from_environment();

            // Apply filters using pure predicates
            if !meets_score_threshold(&item, config.min_score) {
                self.stats.filtered_by_score += 1;
                return;
            }

            if !meets_risk_threshold(&item, config.min_risk) {
                self.stats.filtered_by_risk += 1;
                return;
            }

            if !meets_complexity_thresholds(&item, config.min_cyclomatic, config.min_cognitive) {
                self.stats.filtered_by_complexity += 1;
                return;
            }
        }

        // Check for duplicates (applies to all items including god objects)
        if self
            .items
            .iter()
            .any(|existing| is_duplicate_of(&item, existing))
        {
            self.stats.filtered_as_duplicate += 1;
            return;
        }

        // Item passed all filters
        self.items.push_back(item);
        self.stats.items_added += 1;
    }

    fn sort_by_priority(&mut self) {
        // Sort function items by score (highest first) - zero-copy with im::Vector (spec 204)
        self.items.sort_by(compare_debt_items_by_score);

        // Sort file items by score (highest first) - zero-copy with im::Vector (spec 204)
        self.file_items.sort_by(compare_file_items_by_score);
    }

    fn data_flow_graph(&self) -> &DataFlowGraph {
        &self.data_flow_graph
    }

    fn data_flow_graph_mut(&mut self) -> &mut DataFlowGraph {
        &mut self.data_flow_graph
    }

    fn populate_purity_analysis(&mut self, metrics: &[crate::core::FunctionMetrics]) {
        for metric in metrics {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

            let purity_info = PurityInfo {
                is_pure: metric.is_pure.unwrap_or(false),
                confidence: metric.purity_confidence.unwrap_or(0.0),
                impurity_reasons: if !metric.is_pure.unwrap_or(false) {
                    vec!["Function may have side effects".to_string()]
                } else {
                    vec![]
                },
            };

            self.data_flow_graph.set_purity_info(func_id, purity_info);
        }
    }

    fn add_io_operation(&mut self, func_id: FunctionId, operation: IoOperation) {
        self.data_flow_graph.add_io_operation(func_id, operation);
    }

    fn add_variable_dependencies(
        &mut self,
        func_id: FunctionId,
        variables: std::collections::HashSet<String>,
    ) {
        self.data_flow_graph
            .add_variable_dependencies(func_id, variables);
    }

    fn apply_file_context_adjustments(
        &mut self,
        file_contexts: &std::collections::HashMap<std::path::PathBuf, crate::analysis::FileContext>,
    ) {
        use crate::priority::scoring::file_context_scoring::apply_context_adjustments;

        // Apply adjustments to all items
        self.items = self
            .items
            .iter()
            .map(|item| {
                // Get the file context for this item
                if let Some(context) = file_contexts.get(&item.location.file) {
                    // Apply context adjustment to the final score
                    let adjusted_score =
                        apply_context_adjustments(item.unified_score.final_score, context);

                    // Create a new item with the adjusted score and file context
                    let mut adjusted_item = item.clone();
                    adjusted_item.unified_score.final_score = adjusted_score.max(0.0);
                    adjusted_item.file_context = Some(context.clone());
                    adjusted_item
                } else {
                    // No context available, keep original item
                    item.clone()
                }
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{
        DetectionType, GodObjectAnalysis, GodObjectConfidence, SplitAnalysisMethod,
    };
    use crate::priority::call_graph::CallGraph;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_god_object_item(score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "[file-scope]".to_string(),
                line: 1,
            },
            debt_type: DebtType::GodObject {
                methods: 50,
                fields: Some(20),
                responsibilities: 10,
                god_object_score: 85.0,
                lines: 500,
            },
            unified_score: UnifiedScore {
                final_score: score,
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            cyclomatic_complexity: 65,
            cognitive_complexity: 6,
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Split".to_string(),
                rationale: "God object".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 0,
            function_length: 500,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: Some(GodObjectAnalysis {
                is_god_object: true,
                method_count: 50,
                weighted_method_count: None,
                field_count: 20,
                responsibility_count: 10,
                lines_of_code: 500,
                complexity_sum: 100,
                god_object_score: 85.0,
                recommended_splits: vec![],
                confidence: GodObjectConfidence::Probable,
                responsibilities: vec!["data".to_string()],
                responsibility_method_counts: HashMap::new(),
                purity_distribution: None,
                module_structure: None,
                detection_type: DetectionType::GodFile,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                visibility_breakdown: None,
                domain_count: 1,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,
                trait_method_summary: None,
            }),
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn test_god_object_with_zero_score_is_filtered() {
        // Bug reproduction: god objects with score 0.0 should NOT be added
        // because score 0.0 means "this is not debt"
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create a god object with score 0.0 (marked as "not debt")
        let zero_score_god_object = create_god_object_item(0.0);

        analysis.add_item(zero_score_god_object);

        // The item should NOT be added because score 0.0 means "not debt"
        assert_eq!(
            analysis.items.len(),
            0,
            "God object with score 0.0 should be filtered out (score 0.0 = not debt)"
        );
    }

    #[test]
    fn test_god_object_with_positive_score_is_included() {
        // God objects with positive scores should still be included
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create a god object with a positive score
        let god_object = create_god_object_item(50.0);

        analysis.add_item(god_object);

        // The item SHOULD be added because it has a positive score
        assert_eq!(
            analysis.items.len(),
            1,
            "God object with positive score should be included"
        );
    }
}

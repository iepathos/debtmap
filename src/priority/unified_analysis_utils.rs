//! Utility operations for UnifiedAnalysis.
//!
//! This module provides methods for managing debt items, accessing data flow
//! information, and performing auxiliary operations on UnifiedAnalysis instances.

use super::{DebtType, FileDebtItem, UnifiedAnalysis, UnifiedDebtItem};
use crate::data_flow::{DataFlowGraph, IoOperation, PurityInfo};
use crate::priority::call_graph::FunctionId;

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
}

impl UnifiedAnalysisUtils for UnifiedAnalysis {
    fn timings(&self) -> Option<&crate::builders::parallel_unified_analysis::AnalysisPhaseTimings> {
        self.timings.as_ref()
    }

    fn add_file_item(&mut self, item: FileDebtItem) {
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();

        // Filter out items below minimum thresholds
        if item.score < min_score {
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
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();
        let min_cyclomatic = crate::config::get_minimum_cyclomatic_complexity();
        let min_cognitive = crate::config::get_minimum_cognitive_complexity();
        let min_risk = crate::config::get_minimum_risk_score();

        // Filter out items below minimum thresholds
        if item.unified_score.final_score < min_score {
            return;
        }

        // Check risk score threshold for Risk debt types
        if let DebtType::Risk { risk_score, .. } = &item.debt_type {
            if *risk_score < min_risk {
                return;
            }
        }

        // Filter out trivial functions based on configured complexity thresholds.
        // Test-related items are exempt as they have different complexity characteristics.
        if !matches!(
            item.debt_type,
            DebtType::TestComplexityHotspot { .. }
                | DebtType::TestTodo { .. }
                | DebtType::TestDuplication { .. }
        ) {
            // Enforce cyclomatic complexity threshold
            if item.cyclomatic_complexity < min_cyclomatic {
                return;
            }

            // Enforce cognitive complexity threshold
            if item.cognitive_complexity < min_cognitive {
                return;
            }
        }

        // Check for duplicates before adding
        // Two items are considered duplicates if they have the same location and debt type
        let is_duplicate = self.items.iter().any(|existing| {
            existing.location.file == item.location.file
                && existing.location.line == item.location.line
                && std::mem::discriminant(&existing.debt_type)
                    == std::mem::discriminant(&item.debt_type)
        });

        if !is_duplicate {
            self.items.push_back(item);
        }
    }

    fn sort_by_priority(&mut self) {
        // Sort function items by score (highest first)
        let mut items_vec: Vec<UnifiedDebtItem> = self.items.iter().cloned().collect();
        items_vec.sort_by(|a, b| {
            b.unified_score
                .final_score
                .partial_cmp(&a.unified_score.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.items = items_vec.into_iter().collect();

        // Sort file items by score (highest first)
        let mut file_items_vec: Vec<FileDebtItem> = self.file_items.iter().cloned().collect();
        file_items_vec.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.file_items = file_items_vec.into_iter().collect();
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
}

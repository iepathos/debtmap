pub mod call_graph;
pub mod coverage_propagation;
pub mod debt_aggregator;
pub mod external_api_detector;
pub mod formatter;
pub mod formatter_markdown;
pub mod parallel_call_graph;
pub mod score_formatter;
pub mod scoring;
pub mod semantic_classifier;
pub mod unified_scorer;

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
pub use formatter::{format_priorities, OutputFormat};
pub use formatter_markdown::format_priorities_markdown;
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use unified_scorer::{calculate_unified_priority, Location, UnifiedDebtItem, UnifiedScore};

use im::Vector;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub total_impact: ImpactMetrics,
    pub total_debt_score: f64,
    pub call_graph: CallGraph,
    pub data_flow_graph: crate::data_flow::DataFlowGraph,
    pub overall_coverage: Option<f64>,
}

// Single function analysis for evidence-based risk calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnalysis {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
    pub function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub is_pure: Option<bool>,
    pub purity_confidence: Option<f32>,
    pub nesting_depth: u32,
    pub is_test: bool,
    pub visibility: FunctionVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactMetrics {
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub related_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtType {
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
    },
    DeadCode {
        visibility: FunctionVisibility,
        cyclomatic: u32,
        cognitive: u32,
        usage_hints: Vec<String>,
    },
    Duplication {
        instances: u32,
        total_lines: u32,
    },
    Risk {
        risk_score: f64,
        factors: Vec<String>,
    },
    // Test-specific debt types
    TestComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        threshold: u32,
    },
    TestTodo {
        priority: crate::core::Priority,
        reason: Option<String>,
    },
    TestDuplication {
        instances: u32,
        total_lines: u32,
        similarity: f64,
    },
    ErrorSwallowing {
        pattern: String,
        context: Option<String>,
    },
    // Resource Management debt types
    AllocationInefficiency {
        pattern: String,
        impact: String,
    },
    StringConcatenation {
        loop_type: String,
        iterations: Option<u32>,
    },
    NestedLoops {
        depth: u32,
        complexity_estimate: String,
    },
    BlockingIO {
        operation: String,
        context: String,
    },
    SuboptimalDataStructure {
        current_type: String,
        recommended_type: String,
    },
    // Organization debt types
    GodObject {
        responsibility_count: u32,
        complexity_score: f64,
    },
    FeatureEnvy {
        external_class: String,
        usage_ratio: f64,
    },
    PrimitiveObsession {
        primitive_type: String,
        domain_concept: String,
    },
    MagicValues {
        value: String,
        occurrences: u32,
    },
    // Testing quality debt types
    AssertionComplexity {
        assertion_count: u32,
        complexity_score: f64,
    },
    FlakyTestPattern {
        pattern_type: String,
        reliability_impact: String,
    },
    // Resource management debt types
    AsyncMisuse {
        pattern: String,
        performance_impact: String,
    },
    ResourceLeak {
        resource_type: String,
        cleanup_missing: String,
    },
    CollectionInefficiency {
        collection_type: String,
        inefficiency_type: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

impl UnifiedAnalysis {
    pub fn new(call_graph: CallGraph) -> Self {
        // Create DataFlowGraph from the CallGraph
        let data_flow_graph = crate::data_flow::DataFlowGraph::from_call_graph(call_graph.clone());

        Self {
            items: Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 0.0,
            call_graph,
            data_flow_graph,
            overall_coverage: None,
        }
    }

    pub fn add_item(&mut self, item: UnifiedDebtItem) {
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

        // For non-test items, also check complexity thresholds
        // This helps filter out trivial functions that aren't really debt
        if !matches!(
            item.debt_type,
            DebtType::TestComplexityHotspot { .. }
                | DebtType::TestTodo { .. }
                | DebtType::TestDuplication { .. }
        ) && item.cyclomatic_complexity <= min_cyclomatic
            && item.cognitive_complexity <= min_cognitive
        {
            // Skip trivial functions unless they have other significant issues
            // (like being completely untested critical paths)
            if item.unified_score.coverage_factor < 8.0 {
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

    pub fn sort_by_priority(&mut self) {
        let mut items_vec: Vec<UnifiedDebtItem> = self.items.iter().cloned().collect();
        items_vec.sort_by(|a, b| {
            b.unified_score
                .final_score
                .partial_cmp(&a.unified_score.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.items = items_vec.into_iter().collect();
    }

    pub fn calculate_total_impact(&mut self) {
        let mut coverage_improvement = 0.0;
        let mut lines_reduction = 0;
        let mut complexity_reduction = 0.0;
        let mut risk_reduction = 0.0;
        let mut _functions_to_test = 0;
        let mut total_debt_score = 0.0;

        for item in &self.items {
            // Sum up all final scores as the total debt score
            total_debt_score += item.unified_score.final_score;

            // Only count functions that actually need testing
            if item.expected_impact.coverage_improvement > 0.0 {
                _functions_to_test += 1;
                // Each function contributes a small amount to overall coverage
                // Estimate based on function count (rough approximation)
                coverage_improvement += item.expected_impact.coverage_improvement / 100.0;
            }
            lines_reduction += item.expected_impact.lines_reduction;
            complexity_reduction += item.expected_impact.complexity_reduction;
            risk_reduction += item.expected_impact.risk_reduction;
        }

        // Coverage improvement is the estimated overall project coverage gain
        // Assuming tested functions represent a portion of the codebase
        coverage_improvement = (coverage_improvement * 5.0).min(100.0); // Scale factor for visibility

        // Total complexity reduction (sum of all reductions)
        let total_complexity_reduction = complexity_reduction;

        self.total_debt_score = total_debt_score;
        self.total_impact = ImpactMetrics {
            coverage_improvement,
            lines_reduction,
            complexity_reduction: total_complexity_reduction,
            risk_reduction,
        };
    }

    pub fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        self.items.iter().take(n).cloned().collect()
    }

    pub fn get_bottom_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        let total_items = self.items.len();
        if total_items <= n {
            self.items.clone()
        } else {
            self.items.iter().skip(total_items - n).cloned().collect()
        }
    }

    /// Get a reference to the data flow graph
    pub fn data_flow_graph(&self) -> &crate::data_flow::DataFlowGraph {
        &self.data_flow_graph
    }

    /// Get a mutable reference to the data flow graph
    pub fn data_flow_graph_mut(&mut self) -> &mut crate::data_flow::DataFlowGraph {
        &mut self.data_flow_graph
    }

    /// Populate the data flow graph with purity analysis data from function metrics
    pub fn populate_purity_analysis(&mut self, metrics: &[crate::core::FunctionMetrics]) {
        use crate::data_flow::PurityInfo;
        use crate::priority::call_graph::FunctionId;

        for metric in metrics {
            let func_id = FunctionId {
                file: metric.file.clone(),
                name: metric.name.clone(),
                line: metric.line,
            };

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

    /// Add I/O operation detected during analysis
    pub fn add_io_operation(
        &mut self,
        func_id: call_graph::FunctionId,
        operation: crate::data_flow::IoOperation,
    ) {
        self.data_flow_graph.add_io_operation(func_id, operation);
    }

    /// Add variable dependencies for a function
    pub fn add_variable_dependencies(
        &mut self,
        func_id: call_graph::FunctionId,
        variables: std::collections::HashSet<String>,
    ) {
        self.data_flow_graph
            .add_variable_dependencies(func_id, variables);
    }
}

#[cfg(test)]
mod tests {}

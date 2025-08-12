pub mod call_graph;
pub mod coverage_propagation;
pub mod external_api_detector;
pub mod formatter;
pub mod semantic_classifier;
pub mod unified_scorer;

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use formatter::{format_priorities, OutputFormat};
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use unified_scorer::{calculate_unified_priority, UnifiedDebtItem, UnifiedScore};

use im::Vector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub total_impact: ImpactMetrics,
    pub total_debt_score: f64,
    pub call_graph: CallGraph,
    pub overall_coverage: Option<f64>,
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
    Orchestration {
        delegates_to: Vec<String>,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

impl UnifiedAnalysis {
    pub fn new(call_graph: CallGraph) -> Self {
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
            overall_coverage: None,
        }
    }

    pub fn add_item(&mut self, item: UnifiedDebtItem) {
        self.items.push_back(item);
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
}

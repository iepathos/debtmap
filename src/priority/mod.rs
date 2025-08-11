pub mod call_graph;
pub mod coverage_propagation;
pub mod formatter;
pub mod semantic_classifier;
pub mod unified_scorer;

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use formatter::{format_priorities, OutputFormat};
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use unified_scorer::{calculate_unified_priority, UnifiedDebtItem, UnifiedScore};

use im::Vector;

#[derive(Debug, Clone)]
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub total_impact: ImpactMetrics,
    pub call_graph: CallGraph,
}

#[derive(Debug, Clone)]
pub struct ImpactMetrics {
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

#[derive(Debug, Clone)]
pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub related_items: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DebtType {
    TestingGap {
        coverage: f64,
        complexity: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
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
            call_graph,
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

        for item in &self.items {
            coverage_improvement += item.expected_impact.coverage_improvement;
            lines_reduction += item.expected_impact.lines_reduction;
            complexity_reduction += item.expected_impact.complexity_reduction;
            risk_reduction += item.expected_impact.risk_reduction;
        }

        self.total_impact = ImpactMetrics {
            coverage_improvement,
            lines_reduction,
            complexity_reduction,
            risk_reduction,
        };
    }

    pub fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        self.items.iter().take(n).cloned().collect()
    }
}

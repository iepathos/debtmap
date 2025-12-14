//! Core output types for unified debt output (spec 108)
//!
//! Provides the main `UnifiedOutput`, `OutputMetadata`, `DebtSummary` types
//! and the `UnifiedDebtItemOutput` enum.

use super::cohesion::CohesionSummary;
use super::file_item::FileDebtItemOutput;
use super::func_item::FunctionDebtItemOutput;
use crate::priority::DebtItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unified output format with consistent structure for all debt items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedOutput {
    pub format_version: String,
    pub metadata: OutputMetadata,
    pub summary: DebtSummary,
    pub items: Vec<UnifiedDebtItemOutput>,
}

/// Metadata about the analysis run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub debtmap_version: String,
    pub generated_at: String,
    pub project_root: Option<PathBuf>,
    pub analysis_type: String,
}

/// Summary statistics for the entire codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtSummary {
    pub total_items: usize,
    pub total_debt_score: f64,
    pub debt_density: f64,
    pub total_loc: usize,
    pub by_type: TypeBreakdown,
    pub by_category: std::collections::HashMap<String, usize>,
    pub score_distribution: ScoreDistribution,
    /// Codebase-wide cohesion statistics (spec 198)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion: Option<CohesionSummary>,
}

/// Breakdown by item type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeBreakdown {
    #[serde(rename = "File")]
    pub file: usize,
    #[serde(rename = "Function")]
    pub function: usize,
}

/// Distribution of items by score range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistribution {
    pub critical: usize, // >= 100
    pub high: usize,     // >= 50
    pub medium: usize,   // >= 20
    pub low: usize,      // < 20
}

/// Unified debt item with consistent structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UnifiedDebtItemOutput {
    File(Box<FileDebtItemOutput>),
    Function(Box<FunctionDebtItemOutput>),
}

impl UnifiedDebtItemOutput {
    /// Assert all invariants hold for this debt item (spec 230)
    ///
    /// Called in debug builds before serialization to catch bugs early.
    /// Zero cost in release builds.
    #[cfg(debug_assertions)]
    pub fn assert_invariants(&self) {
        match self {
            UnifiedDebtItemOutput::File(f) => f.assert_invariants(),
            UnifiedDebtItemOutput::Function(f) => f.assert_invariants(),
        }
    }

    /// No-op in release builds for zero overhead
    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn assert_invariants(&self) {}

    /// Get the score of this debt item
    pub fn score(&self) -> f64 {
        match self {
            UnifiedDebtItemOutput::File(f) => f.score,
            UnifiedDebtItemOutput::Function(f) => f.score,
        }
    }

    pub fn from_debt_item(item: &DebtItem, include_scoring_details: bool) -> Self {
        Self::from_debt_item_with_call_graph(item, include_scoring_details, None)
    }

    /// Convert legacy DebtItem to unified format with optional call graph for cohesion (spec 198)
    pub fn from_debt_item_with_call_graph(
        item: &DebtItem,
        include_scoring_details: bool,
        call_graph: Option<&crate::priority::CallGraph>,
    ) -> Self {
        use super::cohesion::build_cohesion_output;

        match item {
            DebtItem::File(file_item) => {
                // Calculate cohesion if call graph is available (spec 198)
                let cohesion = call_graph.and_then(|cg| {
                    crate::organization::calculate_file_cohesion(&file_item.metrics.path, cg)
                        .map(|r| build_cohesion_output(&r))
                });
                UnifiedDebtItemOutput::File(Box::new(
                    FileDebtItemOutput::from_file_item_with_cohesion(
                        file_item,
                        include_scoring_details,
                        cohesion,
                    ),
                ))
            }
            DebtItem::Function(func_item) => UnifiedDebtItemOutput::Function(Box::new(
                FunctionDebtItemOutput::from_function_item(func_item, include_scoring_details),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::unified::dependencies::{Dependencies, RecommendationOutput};
    use crate::output::unified::file_item::{FileImpactOutput, FileMetricsOutput};
    use crate::output::unified::func_item::{FunctionImpactOutput, FunctionMetricsOutput};
    use crate::output::unified::location::UnifiedLocation;
    use crate::output::unified::priority::Priority;
    use crate::priority::{DebtType, FunctionRole};

    /// Helper to create a function debt item for testing
    fn create_test_function_item(
        file: &str,
        line: usize,
        function: &str,
        score: f64,
    ) -> UnifiedDebtItemOutput {
        UnifiedDebtItemOutput::Function(Box::new(FunctionDebtItemOutput {
            score,
            category: "TestCategory".to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: file.to_string(),
                line: Some(line),
                function: Some(function.to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
                length: 50,
                nesting_depth: 3,
                coverage: Some(0.8),
                uncovered_lines: None,
                entropy_score: None,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
            },
            function_role: FunctionRole::PureLogic,
            purity_analysis: None,
            dependencies: Dependencies {
                upstream_count: 0,
                downstream_count: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
            },
            recommendation: RecommendationOutput {
                action: "Test action".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
        }))
    }

    /// Helper to create a file debt item for testing
    fn create_test_file_item(file: &str, score: f64) -> UnifiedDebtItemOutput {
        UnifiedDebtItemOutput::File(Box::new(FileDebtItemOutput {
            score,
            category: "Architecture".to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: file.to_string(),
                line: None,
                function: None,
                file_context_label: None,
            },
            metrics: FileMetricsOutput {
                lines: 500,
                functions: 20,
                classes: 1,
                avg_complexity: 8.0,
                max_complexity: 15,
                total_complexity: 160,
                coverage: 0.7,
                uncovered_lines: 150,
            },
            god_object_indicators: None,
            dependencies: None,
            anti_patterns: None,
            cohesion: None,
            recommendation: RecommendationOutput {
                action: "Refactor file".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FileImpactOutput {
                complexity_reduction: 10.0,
                maintainability_improvement: 0.2,
                test_effort: 5.0,
            },
            scoring_details: None,
        }))
    }

    #[test]
    fn test_unified_debt_item_output_score() {
        let func_item = create_test_function_item("a.rs", 10, "foo", 75.5);
        let file_item = create_test_file_item("b.rs", 42.0);

        assert_eq!(func_item.score(), 75.5);
        assert_eq!(file_item.score(), 42.0);
    }
}

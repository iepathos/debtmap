use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Metadata about the comparison
    pub metadata: ComparisonMetadata,

    /// Target item comparison (if target specified)
    pub target_item: Option<TargetComparison>,

    /// Project-wide health comparison
    pub project_health: ProjectHealthComparison,

    /// New critical debt items (regressions)
    pub regressions: Vec<RegressionItem>,

    /// Resolved debt items (improvements)
    pub improvements: Vec<ImprovementItem>,

    /// Summary statistics
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetadata {
    pub comparison_date: String,
    pub before_file: String,
    pub after_file: String,
    pub target_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetComparison {
    pub location: String,
    pub before: TargetMetrics,
    pub after: Option<TargetMetrics>,
    pub improvements: ImprovementMetrics,
    pub status: TargetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetMetrics {
    pub score: f64,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub coverage: f64,
    pub function_length: usize,
    pub nesting_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementMetrics {
    pub score_reduction_pct: f64,
    pub complexity_reduction_pct: f64,
    pub coverage_improvement_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetStatus {
    /// Target item completely resolved (not in after)
    Resolved,
    /// Target item improved
    Improved,
    /// Target item unchanged
    Unchanged,
    /// Target item regressed (got worse)
    Regressed,
    /// Target item not found in before
    NotFoundBefore,
    /// Target item not found in either
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealthComparison {
    pub before: ProjectMetrics,
    pub after: ProjectMetrics,
    pub changes: ProjectChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub total_debt_score: f64,
    pub total_items: usize,
    pub critical_items: usize,      // score >= 60
    pub high_priority_items: usize, // score >= 40
    pub average_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectChanges {
    pub debt_score_change: f64,
    pub debt_score_change_pct: f64,
    pub items_change: i32,
    pub critical_items_change: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionItem {
    pub location: String,
    pub score: f64,
    pub debt_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementItem {
    pub location: String,
    pub before_score: f64,
    pub after_score: Option<f64>, // None if resolved
    pub improvement_type: ImprovementType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    Resolved,
    ScoreReduced,
    ComplexityReduced,
    CoverageImproved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub target_improved: bool,
    pub new_critical_count: usize,
    pub resolved_count: usize,
    pub overall_debt_trend: DebtTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DebtTrend {
    Improving,  // debt decreased
    Stable,     // debt unchanged
    Regressing, // debt increased
}

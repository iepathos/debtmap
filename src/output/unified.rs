//! Unified output format that provides consistent structure for File and Function debt items
//!
//! This module implements spec 108, providing a normalized JSON output format where:
//! - All items have consistent top-level fields (type, score, category, priority, location)
//! - Score is at the same path for both File and Function items
//! - Location structure is unified (file, line, function)
//! - Simplifies filtering and sorting across item types

use crate::priority::{
    DebtItem, DebtType, FileDebtItem, FunctionRole, GodObjectIndicators, UnifiedAnalysisQueries,
    UnifiedDebtItem,
};
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

/// Priority level based on score
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical, // >= 100
    High,     // >= 50
    Medium,   // >= 20
    Low,      // < 20
}

impl Priority {
    fn from_score(score: f64) -> Self {
        if score >= 100.0 {
            Priority::Critical
        } else if score >= 50.0 {
            Priority::High
        } else if score >= 20.0 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

/// Unified location structure for all debt items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_context_label: Option<String>, // "TEST FILE" or "PROBABLE TEST" for test files (spec 166)
}

/// File-level debt item in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItemOutput {
    pub score: f64,
    pub category: String,
    pub priority: Priority,
    pub location: UnifiedLocation,
    pub metrics: FileMetricsOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub god_object_indicators: Option<GodObjectIndicators>,
    pub recommendation: RecommendationOutput,
    pub impact: FileImpactOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_details: Option<FileScoringDetails>,
}

/// File metrics in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetricsOutput {
    pub lines: usize,
    pub functions: usize,
    pub classes: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub total_complexity: u32,
    pub coverage: f64,
    pub uncovered_lines: usize,
}

/// File impact metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImpactOutput {
    pub complexity_reduction: f64,
    pub maintainability_improvement: f64,
    pub test_effort: f64,
}

/// File scoring details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScoringDetails {
    pub file_size_score: f64,
    pub function_count_score: f64,
    pub complexity_score: f64,
    pub coverage_penalty: f64,
}

/// Function-level debt item in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDebtItemOutput {
    pub score: f64,
    pub category: String,
    pub priority: Priority,
    pub location: UnifiedLocation,
    pub metrics: FunctionMetricsOutput,
    pub debt_type: DebtType,
    pub function_role: FunctionRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_analysis: Option<PurityAnalysis>,
    pub dependencies: Dependencies,
    pub recommendation: RecommendationOutput,
    pub impact: FunctionImpactOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_details: Option<FunctionScoringDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjusted_complexity: Option<AdjustedComplexity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity_pattern: Option<String>,
}

/// Adjusted complexity based on entropy analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComplexity {
    pub dampened_cyclomatic: f64,
    pub dampening_factor: f64,
}

/// Function metrics in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetricsOutput {
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub length: usize,
    pub nesting_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncovered_lines: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_score: Option<f64>,
}

/// Purity analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effects: Option<Vec<String>>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    pub upstream_count: usize,
    pub downstream_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream_callers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream_callees: Vec<String>,
}

/// Recommendation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationOutput {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementation_steps: Vec<String>,
}

/// Function impact metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionImpactOutput {
    pub coverage_improvement: f64,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

/// Function scoring details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionScoringDetails {
    pub coverage_score: f64,
    pub complexity_score: f64,
    pub dependency_score: f64,
    pub base_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_dampening: Option<f64>,
    pub role_multiplier: f64,
    pub final_score: f64,
}

/// Convert legacy DebtItem to unified format
impl UnifiedDebtItemOutput {
    pub fn from_debt_item(item: &DebtItem, include_scoring_details: bool) -> Self {
        match item {
            DebtItem::File(file_item) => UnifiedDebtItemOutput::File(Box::new(
                FileDebtItemOutput::from_file_item(file_item, include_scoring_details),
            )),
            DebtItem::Function(func_item) => UnifiedDebtItemOutput::Function(Box::new(
                FunctionDebtItemOutput::from_function_item(func_item, include_scoring_details),
            )),
        }
    }
}

impl FileDebtItemOutput {
    fn from_file_item(item: &FileDebtItem, include_scoring_details: bool) -> Self {
        let score = item.score;
        FileDebtItemOutput {
            score,
            category: categorize_file_debt(item),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: item.metrics.path.to_string_lossy().to_string(),
                line: None,
                function: None,
                file_context_label: None, // File-level debt doesn't need test file tags
            },
            metrics: FileMetricsOutput {
                lines: item.metrics.total_lines,
                functions: item.metrics.function_count,
                classes: item.metrics.class_count,
                avg_complexity: item.metrics.avg_complexity,
                max_complexity: item.metrics.max_complexity,
                total_complexity: item.metrics.total_complexity,
                coverage: item.metrics.coverage_percent,
                uncovered_lines: item.metrics.uncovered_lines,
            },
            god_object_indicators: Some(item.metrics.god_object_indicators.clone()),
            recommendation: RecommendationOutput {
                action: item.recommendation.clone(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FileImpactOutput {
                complexity_reduction: item.impact.complexity_reduction,
                maintainability_improvement: item.impact.maintainability_improvement,
                test_effort: item.impact.test_effort,
            },
            scoring_details: if include_scoring_details {
                Some(calculate_file_scoring_details(item))
            } else {
                None
            },
        }
    }
}

impl FunctionDebtItemOutput {
    fn from_function_item(item: &UnifiedDebtItem, include_scoring_details: bool) -> Self {
        let score = item.unified_score.final_score;
        let complexity_pattern = extract_complexity_pattern(
            &item.recommendation.rationale,
            &item.recommendation.primary_action,
        );
        FunctionDebtItemOutput {
            score,
            category: crate::priority::DebtCategory::from_debt_type(&item.debt_type).to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: item.location.file.to_string_lossy().to_string(),
                line: Some(item.location.line),
                function: Some(item.location.function.clone()),
                file_context_label: item.file_context.as_ref().map(|ctx| {
                    use crate::priority::scoring::file_context_scoring::context_label;
                    context_label(ctx).to_string()
                }),
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: item.cyclomatic_complexity,
                cognitive_complexity: item.cognitive_complexity,
                length: item.function_length,
                nesting_depth: item.nesting_depth,
                coverage: item.transitive_coverage.as_ref().map(|c| c.transitive),
                uncovered_lines: None, // Not currently tracked
                entropy_score: item.entropy_details.as_ref().map(|e| e.entropy_score),
            },
            debt_type: item.debt_type.clone(),
            function_role: item.function_role,
            purity_analysis: item.is_pure.map(|is_pure| PurityAnalysis {
                is_pure,
                confidence: item.purity_confidence.unwrap_or(0.0),
                side_effects: None,
            }),
            dependencies: Dependencies {
                upstream_count: item.upstream_dependencies,
                downstream_count: item.downstream_dependencies,
                upstream_callers: item.upstream_callers.clone(),
                downstream_callees: item.downstream_callees.clone(),
            },
            recommendation: RecommendationOutput {
                action: item.recommendation.primary_action.clone(),
                priority: None,
                implementation_steps: item.recommendation.implementation_steps.clone(),
            },
            impact: FunctionImpactOutput {
                coverage_improvement: item.expected_impact.coverage_improvement,
                complexity_reduction: item.expected_impact.complexity_reduction,
                risk_reduction: item.expected_impact.risk_reduction,
            },
            scoring_details: if include_scoring_details {
                Some(FunctionScoringDetails {
                    coverage_score: item.unified_score.coverage_factor,
                    complexity_score: item.unified_score.complexity_factor,
                    dependency_score: item.unified_score.dependency_factor,
                    base_score: item.unified_score.complexity_factor
                        + item.unified_score.coverage_factor
                        + item.unified_score.dependency_factor,
                    entropy_dampening: item.entropy_details.as_ref().map(|e| e.dampening_factor),
                    role_multiplier: item.unified_score.role_multiplier,
                    final_score: item.unified_score.final_score,
                })
            } else {
                None
            },
            adjusted_complexity: item.entropy_details.as_ref().map(|e| AdjustedComplexity {
                dampened_cyclomatic: e.adjusted_complexity as f64,
                dampening_factor: e.dampening_factor,
            }),
            complexity_pattern,
        }
    }
}

/// Extract complexity pattern from recommendation text
fn extract_complexity_pattern(rationale: &str, action: &str) -> Option<String> {
    // Check for moderate complexity (preventive)
    if action.contains("Maintain current low complexity")
        || action.contains("approaching thresholds")
    {
        return Some("ModerateComplexity".to_string());
    }

    // Check for specific patterns in the rationale
    if rationale.contains("Deep nesting") || rationale.contains("nesting is primary issue") {
        Some("DeepNesting".to_string())
    } else if rationale.contains("Many decision points")
        || rationale.contains("branches) drive cyclomatic")
    {
        Some("HighBranching".to_string())
    } else if rationale.contains("State machine pattern") {
        Some("StateMachine".to_string())
    } else if rationale.contains("High token entropy")
        || rationale.contains("inconsistent structure")
    {
        Some("ChaoticStructure".to_string())
    } else if action.contains("Clean dispatcher pattern") || rationale.contains("dispatcher") {
        Some("Dispatcher".to_string())
    } else if rationale.contains("repetitive validation")
        || rationale.contains("Repetitive validation")
    {
        Some("RepetitiveValidation".to_string())
    } else if rationale.contains("coordinator") || rationale.contains("orchestrat") {
        Some("Coordinator".to_string())
    } else if rationale.contains("nesting and branching") || action.contains("two-phase approach") {
        Some("MixedComplexity".to_string())
    } else {
        None
    }
}

fn categorize_file_debt(_item: &FileDebtItem) -> String {
    // File-level debt is always architecture-related (large files, god modules)
    "Architecture".to_string()
}

fn calculate_file_scoring_details(item: &FileDebtItem) -> FileScoringDetails {
    // Simplified scoring calculation - actual implementation may vary
    let file_size_score = (item.metrics.total_lines as f64 / 100.0).min(50.0);
    let function_count_score = (item.metrics.function_count as f64 / 2.0).min(30.0);
    let complexity_score = (item.metrics.avg_complexity * 2.0).min(20.0);
    let coverage_penalty = (1.0 - item.metrics.coverage_percent) * 20.0;

    FileScoringDetails {
        file_size_score,
        function_count_score,
        complexity_score,
        coverage_penalty,
    }
}

/// Convert analysis results to unified output format
pub fn convert_to_unified_format(
    analysis: &crate::priority::UnifiedAnalysis,
    include_scoring_details: bool,
) -> UnifiedOutput {
    use std::collections::HashMap;

    // Get all debt items sorted by score
    let all_items = analysis.get_top_mixed_priorities(usize::MAX);

    // Convert to unified format
    let unified_items: Vec<UnifiedDebtItemOutput> = all_items
        .iter()
        .map(|item| UnifiedDebtItemOutput::from_debt_item(item, include_scoring_details))
        .collect();

    // Calculate summary statistics from filtered items
    let mut file_count = 0;
    let mut function_count = 0;
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut score_dist = ScoreDistribution {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };

    // Calculate total debt score from filtered items only
    let total_debt_score: f64 = all_items.iter().map(|item| item.score()).sum();

    for item in &unified_items {
        match item {
            UnifiedDebtItemOutput::File(f) => {
                file_count += 1;
                *category_counts.entry(f.category.clone()).or_insert(0) += 1;
                match f.priority {
                    Priority::Critical => score_dist.critical += 1,
                    Priority::High => score_dist.high += 1,
                    Priority::Medium => score_dist.medium += 1,
                    Priority::Low => score_dist.low += 1,
                }
            }
            UnifiedDebtItemOutput::Function(f) => {
                function_count += 1;
                *category_counts.entry(f.category.clone()).or_insert(0) += 1;
                match f.priority {
                    Priority::Critical => score_dist.critical += 1,
                    Priority::High => score_dist.high += 1,
                    Priority::Medium => score_dist.medium += 1,
                    Priority::Low => score_dist.low += 1,
                }
            }
        }
    }

    // Recalculate debt density from filtered items
    let debt_density = if analysis.total_lines_of_code > 0 {
        (total_debt_score / analysis.total_lines_of_code as f64) * 1000.0
    } else {
        0.0
    };

    UnifiedOutput {
        format_version: "2.0".to_string(),
        metadata: OutputMetadata {
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_root: None,
            analysis_type: "unified".to_string(),
        },
        summary: DebtSummary {
            total_items: unified_items.len(),
            total_debt_score,
            debt_density,
            total_loc: analysis.total_lines_of_code,
            by_type: TypeBreakdown {
                file: file_count,
                function: function_count,
            },
            by_category: category_counts,
            score_distribution: score_dist,
        },
        items: unified_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_score() {
        assert!(matches!(Priority::from_score(150.0), Priority::Critical));
        assert!(matches!(Priority::from_score(75.0), Priority::High));
        assert!(matches!(Priority::from_score(35.0), Priority::Medium));
        assert!(matches!(Priority::from_score(10.0), Priority::Low));
    }

    #[test]
    fn test_unified_location_serialization() {
        let loc = UnifiedLocation {
            file: "test.rs".to_string(),
            line: Some(42),
            function: Some("test_function".to_string()),
            file_context_label: None,
        };

        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("\"file\":\"test.rs\""));
        assert!(json.contains("\"line\":42"));
        assert!(json.contains("\"function\":\"test_function\""));
    }

    #[test]
    fn test_file_location_omits_optional_fields() {
        let loc = UnifiedLocation {
            file: "test.rs".to_string(),
            line: None,
            function: None,
            file_context_label: None,
        };

        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("\"file\":\"test.rs\""));
        assert!(!json.contains("\"line\""));
        assert!(!json.contains("\"function\""));
    }
}

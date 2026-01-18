//! File-level debt item output types and conversions (spec 108)
//!
//! Provides `FileDebtItemOutput` struct and conversion from `FileDebtItem`.

use super::anti_patterns::{build_anti_patterns, AntiPatternOutput};
use super::cohesion::CohesionOutput;
use super::coupling::{build_file_dependencies, FileDependencies};
use super::format::{assert_ratio_invariants, assert_score_invariants};
use super::format::{round_ratio, round_score};
use super::location::UnifiedLocation;
use super::priority::{assert_priority_invariants, Priority};
use crate::priority::{DebtType, FileDebtItem};
use serde::{Deserialize, Serialize};

/// File-level debt item in unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItemOutput {
    pub score: f64,
    pub category: String,
    pub priority: Priority,
    pub location: UnifiedLocation,
    pub metrics: FileMetricsOutput,
    /// Debt type classification for file-level issues
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debt_type: Option<DebtType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub god_object_indicators: Option<crate::priority::GodObjectIndicators>,
    /// File-level dependency metrics (spec 201)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<FileDependencies>,
    /// Anti-pattern detection results (spec 197)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_patterns: Option<AntiPatternOutput>,
    /// File-level cohesion metrics (spec 198)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohesion: Option<CohesionOutput>,
    pub impact: FileImpactOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring_details: Option<FileScoringDetails>,
}

impl FileDebtItemOutput {
    /// Assert all invariants hold for this file debt item (spec 230)
    #[cfg(debug_assertions)]
    pub fn assert_invariants(&self) {
        assert_score_invariants(self.score, "file.score");
        assert_priority_invariants(&self.priority, self.score);
        assert_ratio_invariants(self.metrics.coverage, "file.metrics.coverage");

        if let Some(ref cohesion) = self.cohesion {
            assert_ratio_invariants(cohesion.score, "file.cohesion.score");
        }

        if let Some(ref deps) = self.dependencies {
            assert_ratio_invariants(deps.instability, "file.dependencies.instability");
        }
    }

    /// No-op in release builds
    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn assert_invariants(&self) {}

    /// Convert from FileDebtItem without cohesion data
    #[allow(dead_code)]
    pub fn from_file_item(item: &FileDebtItem, include_scoring_details: bool) -> Self {
        Self::from_file_item_with_cohesion(item, include_scoring_details, None)
    }

    pub fn from_file_item_with_cohesion(
        item: &FileDebtItem,
        include_scoring_details: bool,
        cohesion: Option<CohesionOutput>,
    ) -> Self {
        let score = item.score;

        // Build file dependencies if coupling data is present (spec 201)
        let dependencies = build_file_dependencies(&item.metrics);

        // Build anti-pattern output if present in god object analysis (spec 197)
        let anti_patterns = build_anti_patterns(&item.metrics);

        // Apply rounding for clean output (spec 230)
        let rounded_score = round_score(score);
        let rounded_coverage = round_ratio(item.metrics.coverage_percent);
        let rounded_avg_complexity = round_score(item.metrics.avg_complexity);

        // Round cohesion if present
        let cohesion = cohesion.map(|mut c| {
            c.score = round_ratio(c.score);
            c
        });

        // Derive debt type from file metrics
        let debt_type = derive_file_debt_type(item);

        FileDebtItemOutput {
            score: rounded_score,
            category: categorize_file_debt(item),
            priority: Priority::from_score(rounded_score),
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
                avg_complexity: rounded_avg_complexity,
                max_complexity: item.metrics.max_complexity,
                total_complexity: item.metrics.total_complexity,
                coverage: rounded_coverage,
                uncovered_lines: item.metrics.uncovered_lines,
                distribution: None, // Populated separately when distribution metrics are available
            },
            debt_type,
            god_object_indicators: item.metrics.god_object_analysis.clone().map(|a| a.into()),
            dependencies,
            anti_patterns,
            cohesion,
            impact: FileImpactOutput {
                complexity_reduction: round_ratio(item.impact.complexity_reduction),
                maintainability_improvement: round_ratio(item.impact.maintainability_improvement),
                test_effort: round_ratio(item.impact.test_effort),
            },
            scoring_details: if include_scoring_details {
                Some(calculate_file_scoring_details(item))
            } else {
                None
            },
        }
    }
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
    /// Distribution metrics for complexity analysis (Spec 268)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<DistributionMetricsOutput>,
}

/// Distribution metrics for file-scope complexity analysis (Spec 268)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetricsOutput {
    /// Number of functions in the file
    pub function_count: usize,
    /// Highest cyclomatic complexity among all functions
    pub max_function_complexity: u32,
    /// Average cyclomatic complexity per function
    pub avg_function_complexity: f64,
    /// Median cyclomatic complexity (robust to outliers)
    pub median_complexity: u32,
    /// Number of functions exceeding the complexity threshold (15)
    pub exceeding_threshold: usize,
    /// Classification: "Concentrated", "Mixed", or "Distributed"
    pub distribution_type: String,
    /// Human-readable explanation of the classification
    pub classification_explanation: String,
    /// Production code lines (excluding test modules)
    pub production_loc: usize,
    /// Test code lines (inside #[cfg(test)] modules)
    pub test_loc: usize,
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

fn categorize_file_debt(_item: &FileDebtItem) -> String {
    // File-level debt is always architecture-related (large files, god modules)
    "Architecture".to_string()
}

/// Derive the debt type for a file based on its metrics.
///
/// Returns GodObject if god object analysis indicates it's a god object,
/// or if raw metrics exceed god object thresholds.
fn derive_file_debt_type(item: &FileDebtItem) -> Option<DebtType> {
    // Check if this file has god object analysis and is classified as a god object
    if let Some(ref analysis) = item.metrics.god_object_analysis {
        if analysis.is_god_object {
            return Some(DebtType::GodObject {
                methods: analysis.method_count as u32,
                fields: if analysis.field_count > 0 {
                    Some(analysis.field_count as u32)
                } else {
                    None
                },
                responsibilities: analysis.responsibility_count as u32,
                god_object_score: analysis.god_object_score,
                lines: analysis.lines_of_code as u32,
            });
        }
    }

    // Fallback: infer god object from raw metrics if thresholds exceeded
    // Same thresholds used in file_analysis.rs for consistency
    let is_large_file = item.metrics.total_lines > 2000;
    let has_many_functions = item.metrics.function_count > 50;

    if is_large_file || has_many_functions {
        let god_object_score = (item.metrics.function_count as f64 / 50.0).min(2.0) * 100.0;
        return Some(DebtType::GodObject {
            methods: item.metrics.function_count as u32,
            fields: None,
            responsibilities: 0,
            god_object_score,
            lines: item.metrics.total_lines as u32,
        });
    }

    // No specific debt type identified for this file
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_debt_item_serialization_roundtrip() {
        use super::super::cohesion::CohesionClassification;

        let item = FileDebtItemOutput {
            score: 75.25,
            category: "Architecture".to_string(),
            priority: Priority::from_score(75.25),
            location: UnifiedLocation {
                file: "big_file.rs".to_string(),
                line: None,
                function: None,
                file_context_label: None,
            },
            metrics: FileMetricsOutput {
                lines: 500,
                functions: 25,
                classes: 0,
                avg_complexity: 8.5,
                max_complexity: 15,
                total_complexity: 212,
                coverage: 0.65,
                uncovered_lines: 175,
                distribution: None,
            },
            debt_type: Some(DebtType::GodObject {
                methods: 25,
                fields: None,
                responsibilities: 5,
                god_object_score: 75.25,
                lines: 500,
            }),
            god_object_indicators: None,
            dependencies: None,
            anti_patterns: None,
            cohesion: Some(CohesionOutput {
                score: 0.45,
                internal_calls: 10,
                external_calls: 15,
                classification: CohesionClassification::Medium,
                functions_analyzed: 25,
            }),
            impact: FileImpactOutput {
                complexity_reduction: 0.3,
                maintainability_improvement: 0.4,
                test_effort: 0.5,
            },
            scoring_details: None,
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: FileDebtItemOutput = serde_json::from_str(&json).unwrap();

        // Key fields should be preserved
        assert_eq!(item.score, deserialized.score);
        assert!(matches!(deserialized.priority, Priority::High));
        assert_eq!(item.metrics.coverage, deserialized.metrics.coverage);
    }
}

//! Diagnostic reporting and analysis output for complexity results.
//!
//! This module provides types and utilities for generating human-readable diagnostic
//! reports from complexity analysis results. It transforms raw analysis data into
//! actionable insights, recommendations, and formatted output.
//!
//! # Components
//!
//! - [`crate::analysis::diagnostics::InsightGenerator`]: Generates insights from complexity attribution data
//! - [`crate::analysis::diagnostics::RecommendationEngine`]: Produces prioritized refactoring recommendations
//! - [`crate::analysis::diagnostics::DiagnosticReporter`]: Formats analysis results into various output formats
//!
//! # Effect-Based API
//!
//! The [`effects`] submodule provides effect-based wrappers that integrate with
//! the stillwater effect system for configuration access and testability.

use crate::analysis::attribution::ComplexityAttribution;
use crate::analysis::multi_pass::{ComplexityRecommendation, MultiPassResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Effect-based wrappers for diagnostic generation with configuration access.
pub mod effects;
/// Insight generation from complexity attribution data.
pub mod insights;
/// Recommendation strategies for complexity reduction.
pub mod recommendations;
/// Report formatting and output generation.
pub mod reporter;

pub use insights::InsightGenerator;
pub use recommendations::RecommendationEngine;
pub use reporter::DiagnosticReporter;

/// Complete diagnostic report containing analysis results and recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// High-level summary of complexity metrics and key findings.
    pub summary: ComplexitySummary,
    /// Detailed breakdown of complexity by category (logical, formatting, patterns).
    pub detailed_attribution: DetailedAttribution,
    /// Prioritized list of recommendations for reducing complexity.
    pub recommendations: Vec<ComplexityRecommendation>,
    /// Comparison with a previous version, if available.
    pub comparative_analysis: Option<ComparativeAnalysis>,
    /// Performance metrics from the analysis process itself.
    pub performance_metrics: Option<AnalysisPerformanceMetrics>,
}

/// Summary of complexity analysis results.
///
/// Provides high-level metrics comparing raw and normalized complexity,
/// along with key findings from the analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexitySummary {
    /// Original complexity score before normalization.
    pub raw_complexity: u32,
    /// Complexity score after normalization adjustments.
    pub normalized_complexity: u32,
    /// Percentage reduction from raw to normalized complexity.
    pub complexity_reduction: f32,
    /// Percentage of complexity attributed to formatting choices.
    pub formatting_impact: f32,
    /// Confidence level (0.0-100.0) of pattern recognition in the analysis.
    pub pattern_recognition: f32,
    /// List of notable findings from the complexity analysis.
    pub key_findings: Vec<String>,
}

/// Detailed attribution breakdown showing how complexity is distributed
/// across different categories (logical, formatting, and pattern-based).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedAttribution {
    /// Breakdown of complexity from logical constructs (conditionals, loops, etc.).
    pub logical_breakdown: AttributionBreakdown,
    /// Breakdown of complexity from formatting choices (nesting, line length, etc.).
    pub formatting_breakdown: AttributionBreakdown,
    /// Breakdown of complexity from recognized code patterns.
    pub pattern_breakdown: AttributionBreakdown,
    /// Sum of all attributed complexity across categories.
    pub total_attribution: u32,
    /// Confidence level (0.0-1.0) in the accuracy of the attribution.
    pub confidence_level: f32,
}

/// Breakdown of a specific attribution type (logical, formatting, or pattern).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionBreakdown {
    /// Name of this attribution category (e.g., "logical", "formatting").
    pub category: String,
    /// Total complexity points attributed to this category.
    pub total: u32,
    /// Percentage of total complexity this category represents (0.0-100.0).
    pub percentage: f32,
    /// Individual components contributing to this category's complexity.
    pub components: Vec<AttributionComponent>,
}

/// Individual attribution component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionComponent {
    /// Name or description of this component (e.g., "nested conditionals").
    pub name: String,
    /// Complexity points contributed by this component.
    pub contribution: u32,
    /// Source location in "file:line" format.
    pub location: String,
    /// Actionable suggestions for reducing this component's complexity.
    pub suggestions: Vec<String>,
}

/// Comparative analysis between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeAnalysis {
    /// Complexity score before the change.
    pub before_complexity: u32,
    /// Complexity score after the change.
    pub after_complexity: u32,
    /// Percentage improvement (positive means reduction in complexity).
    pub improvement_percentage: f32,
    /// Detailed descriptions of individual changes.
    pub changes: Vec<ChangeDescription>,
}

/// Description of a specific change between two analysis versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDescription {
    /// Type of change (e.g., "added", "removed", "modified").
    pub change_type: String,
    /// Description of the change's impact on complexity.
    pub impact: String,
    /// Source location where the change occurred in "file:line" format.
    pub location: String,
}

/// Performance metrics for the analysis process.
///
/// Tracks timing for each phase of analysis to help identify
/// bottlenecks and optimize performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPerformanceMetrics {
    /// Total wall-clock time for the entire analysis in milliseconds.
    pub total_time_ms: u64,
    /// Time spent on raw complexity analysis in milliseconds.
    pub raw_analysis_ms: u64,
    /// Time spent on normalized complexity analysis in milliseconds.
    pub normalized_analysis_ms: u64,
    /// Time spent on complexity attribution in milliseconds.
    pub attribution_ms: u64,
    /// Time spent generating the report in milliseconds.
    pub reporting_ms: u64,
    /// Peak memory usage during analysis in megabytes.
    pub memory_used_mb: f32,
}

/// Detail level for diagnostic reports
#[derive(Debug, Clone, PartialEq)]
pub enum DetailLevel {
    Summary,
    Standard,
    Comprehensive,
    Debug,
}

impl DetailLevel {
    pub fn includes_attribution(&self) -> bool {
        !matches!(self, DetailLevel::Summary)
    }

    pub fn includes_recommendations(&self) -> bool {
        matches!(
            self,
            DetailLevel::Standard | DetailLevel::Comprehensive | DetailLevel::Debug
        )
    }

    pub fn includes_source_mapping(&self) -> bool {
        matches!(self, DetailLevel::Comprehensive | DetailLevel::Debug)
    }

    pub fn includes_performance(&self) -> bool {
        matches!(self, DetailLevel::Debug)
    }
}

/// Output format for reports
#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Markdown,
    Html,
    Text,
}

impl OutputFormat {
    pub fn file_extension(&self) -> &str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Yaml => "yaml",
            OutputFormat::Markdown => "md",
            OutputFormat::Html => "html",
            OutputFormat::Text => "txt",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "JSON"),
            OutputFormat::Yaml => write!(f, "YAML"),
            OutputFormat::Markdown => write!(f, "Markdown"),
            OutputFormat::Html => write!(f, "HTML"),
            OutputFormat::Text => write!(f, "Plain Text"),
        }
    }
}

/// Generate a summary from multi-pass results
pub fn generate_summary(result: &MultiPassResult) -> ComplexitySummary {
    let raw = result.raw_complexity.total_complexity;
    let normalized = result.normalized_complexity.total_complexity;

    let complexity_reduction = if raw > 0 {
        ((raw - normalized) as f32 / raw as f32) * 100.0
    } else {
        0.0
    };

    let formatting_impact = if result.attribution.logical_complexity.total > 0 {
        (result.attribution.formatting_artifacts.total as f32
            / result.attribution.logical_complexity.total as f32)
            * 100.0
    } else {
        0.0
    };

    let pattern_recognition = result.attribution.pattern_complexity.confidence * 100.0;

    let mut key_findings = Vec::new();

    if complexity_reduction > 10.0 {
        key_findings.push(format!(
            "Normalization reduces complexity by {:.1}%",
            complexity_reduction
        ));
    }

    if formatting_impact > 20.0 {
        key_findings.push(format!(
            "Formatting contributes {:.1}% to complexity",
            formatting_impact
        ));
    }

    if pattern_recognition > 70.0 {
        key_findings.push("High pattern recognition confidence".to_string());
    }

    for insight in &result.insights {
        if insight.impact_level == crate::analysis::multi_pass::ImpactLevel::High {
            key_findings.push(insight.description.clone());
        }
    }

    ComplexitySummary {
        raw_complexity: raw,
        normalized_complexity: normalized,
        complexity_reduction,
        formatting_impact,
        pattern_recognition,
        key_findings,
    }
}

/// Generate detailed attribution from results
pub fn generate_detailed_attribution(attribution: &ComplexityAttribution) -> DetailedAttribution {
    let total = attribution.logical_complexity.total
        + attribution.formatting_artifacts.total
        + attribution.pattern_complexity.total;

    let logical_breakdown = AttributionBreakdown {
        category: "Logical Structure".to_string(),
        total: attribution.logical_complexity.total,
        percentage: if total > 0 {
            (attribution.logical_complexity.total as f32 / total as f32) * 100.0
        } else {
            0.0
        },
        components: attribution
            .logical_complexity
            .breakdown
            .iter()
            .map(|c| AttributionComponent {
                name: c.description.clone(),
                contribution: c.contribution,
                location: format!("{}:{}", c.location.file, c.location.line),
                suggestions: c.suggestions.clone(),
            })
            .collect(),
    };

    let formatting_breakdown = AttributionBreakdown {
        category: "Formatting Artifacts".to_string(),
        total: attribution.formatting_artifacts.total,
        percentage: if total > 0 {
            (attribution.formatting_artifacts.total as f32 / total as f32) * 100.0
        } else {
            0.0
        },
        components: attribution
            .formatting_artifacts
            .breakdown
            .iter()
            .map(|c| AttributionComponent {
                name: c.description.clone(),
                contribution: c.contribution,
                location: format!("{}:{}", c.location.file, c.location.line),
                suggestions: c.suggestions.clone(),
            })
            .collect(),
    };

    let pattern_breakdown = AttributionBreakdown {
        category: "Pattern Recognition".to_string(),
        total: attribution.pattern_complexity.total,
        percentage: if total > 0 {
            (attribution.pattern_complexity.total as f32 / total as f32) * 100.0
        } else {
            0.0
        },
        components: attribution
            .pattern_complexity
            .breakdown
            .iter()
            .map(|c| AttributionComponent {
                name: c.description.clone(),
                contribution: c.contribution,
                location: format!("{}:{}", c.location.file, c.location.line),
                suggestions: c.suggestions.clone(),
            })
            .collect(),
    };

    let confidence_level = (attribution.logical_complexity.confidence
        + attribution.formatting_artifacts.confidence
        + attribution.pattern_complexity.confidence)
        / 3.0;

    DetailedAttribution {
        logical_breakdown,
        formatting_breakdown,
        pattern_breakdown,
        total_attribution: total,
        confidence_level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_level_includes() {
        assert!(!DetailLevel::Summary.includes_attribution());
        assert!(DetailLevel::Standard.includes_attribution());
        assert!(DetailLevel::Standard.includes_recommendations());
        assert!(!DetailLevel::Standard.includes_source_mapping());
        assert!(DetailLevel::Comprehensive.includes_source_mapping());
        assert!(DetailLevel::Debug.includes_performance());
    }

    #[test]
    fn test_output_format_extension() {
        assert_eq!(OutputFormat::Json.file_extension(), "json");
        assert_eq!(OutputFormat::Yaml.file_extension(), "yaml");
        assert_eq!(OutputFormat::Markdown.file_extension(), "md");
        assert_eq!(OutputFormat::Html.file_extension(), "html");
        assert_eq!(OutputFormat::Text.file_extension(), "txt");
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(format!("{}", OutputFormat::Json), "JSON");
        assert_eq!(format!("{}", OutputFormat::Markdown), "Markdown");
    }

    #[test]
    fn test_generate_summary() {
        use crate::analysis::attribution::AttributedComplexity;
        use crate::analysis::multi_pass::{AnalysisType, ComplexityResult};

        let result = MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: 20,
                cognitive_complexity: 15,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: 15,
                cognitive_complexity: 12,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: AttributedComplexity {
                    total: 12,
                    breakdown: vec![],
                    confidence: 0.9,
                },
                formatting_artifacts: AttributedComplexity {
                    total: 5,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                pattern_complexity: AttributedComplexity {
                    total: 3,
                    breakdown: vec![],
                    confidence: 0.75,
                },
                source_mappings: vec![],
            },
            insights: vec![],
            recommendations: vec![],
            performance_metrics: None,
        };

        let summary = generate_summary(&result);
        assert_eq!(summary.raw_complexity, 20);
        assert_eq!(summary.normalized_complexity, 15);
        assert!(summary.complexity_reduction > 0.0);
    }
}

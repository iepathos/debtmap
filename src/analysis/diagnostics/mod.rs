use crate::analysis::attribution::ComplexityAttribution;
use crate::analysis::multi_pass::{ComplexityRecommendation, MultiPassResult};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod insights;
pub mod recommendations;
pub mod reporter;

pub use insights::InsightGenerator;
pub use recommendations::RecommendationEngine;
pub use reporter::DiagnosticReporter;

/// Complete diagnostic report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub summary: ComplexitySummary,
    pub detailed_attribution: DetailedAttribution,
    pub recommendations: Vec<ComplexityRecommendation>,
    pub comparative_analysis: Option<ComparativeAnalysis>,
    pub performance_metrics: Option<AnalysisPerformanceMetrics>,
}

/// Summary of complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexitySummary {
    pub raw_complexity: u32,
    pub normalized_complexity: u32,
    pub complexity_reduction: f32,
    pub formatting_impact: f32,
    pub pattern_recognition: f32,
    pub key_findings: Vec<String>,
}

/// Detailed attribution breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedAttribution {
    pub logical_breakdown: AttributionBreakdown,
    pub formatting_breakdown: AttributionBreakdown,
    pub pattern_breakdown: AttributionBreakdown,
    pub total_attribution: u32,
    pub confidence_level: f32,
}

/// Breakdown of a specific attribution type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionBreakdown {
    pub category: String,
    pub total: u32,
    pub percentage: f32,
    pub components: Vec<AttributionComponent>,
}

/// Individual attribution component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionComponent {
    pub name: String,
    pub contribution: u32,
    pub location: String,
    pub suggestions: Vec<String>,
}

/// Comparative analysis between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeAnalysis {
    pub before_complexity: u32,
    pub after_complexity: u32,
    pub improvement_percentage: f32,
    pub changes: Vec<ChangeDescription>,
}

/// Description of a specific change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDescription {
    pub change_type: String,
    pub impact: String,
    pub location: String,
}

/// Performance metrics for the analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPerformanceMetrics {
    pub total_time_ms: u64,
    pub raw_analysis_ms: u64,
    pub normalized_analysis_ms: u64,
    pub attribution_ms: u64,
    pub reporting_ms: u64,
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

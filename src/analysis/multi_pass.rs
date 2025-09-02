use crate::analyzers::Analyzer;
use crate::complexity::semantic_normalizer::SemanticNormalizer;
use crate::core::{FunctionMetrics, Language};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use super::attribution::{AttributionEngine, ComplexityAttribution};
use super::diagnostics::{DetailLevel, DiagnosticReport, DiagnosticReporter, OutputFormat};

/// Core multi-pass analysis engine
pub struct MultiPassAnalyzer {
    raw_analyzer: Box<dyn Analyzer>,
    normalized_analyzer: Box<dyn Analyzer>,
    attribution_engine: AttributionEngine,
    diagnostic_reporter: DiagnosticReporter,
}

impl MultiPassAnalyzer {
    pub fn new(options: MultiPassOptions) -> Self {
        Self {
            raw_analyzer: create_raw_analyzer(options.language),
            normalized_analyzer: create_normalized_analyzer(options.language),
            attribution_engine: AttributionEngine::new(),
            diagnostic_reporter: DiagnosticReporter::new(
                options.output_format.clone(),
                options.detail_level.clone(),
            ),
        }
    }

    pub fn from_analyzer(base: Box<dyn Analyzer>, options: MultiPassOptions) -> Self {
        Self {
            raw_analyzer: base,
            normalized_analyzer: create_normalized_analyzer(options.language),
            attribution_engine: AttributionEngine::new(),
            diagnostic_reporter: DiagnosticReporter::new(
                options.output_format.clone(),
                options.detail_level.clone(),
            ),
        }
    }

    pub fn analyze(&self, source: &AnalysisUnit) -> Result<MultiPassResult> {
        let start_time = Instant::now();
        let start_memory = Self::get_memory_usage_mb();

        // First pass: Raw complexity analysis
        let raw_start = Instant::now();
        let raw_result = self.analyze_raw(&source.raw_source)?;
        let raw_time = raw_start.elapsed().as_millis() as u64;

        // Second pass: Normalized complexity analysis
        let normalized_start = Instant::now();
        let normalized_result = self.analyze_normalized(&source.normalized_source)?;
        let normalized_time = normalized_start.elapsed().as_millis() as u64;

        // Attribution analysis
        let attribution_start = Instant::now();
        let attribution = self
            .attribution_engine
            .attribute(&raw_result, &normalized_result);
        let attribution_time = attribution_start.elapsed().as_millis() as u64;

        // Generate insights
        let insights = self.generate_insights(&attribution);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&attribution, &insights);

        let total_time = start_time.elapsed().as_millis() as u64;
        let end_memory = Self::get_memory_usage_mb();
        let memory_used = (end_memory - start_memory).max(0.0);

        // Create performance metrics
        let performance_metrics = AnalysisPerformanceMetrics {
            raw_analysis_time_ms: raw_time,
            normalized_analysis_time_ms: normalized_time,
            attribution_time_ms: attribution_time,
            total_time_ms: total_time,
            memory_used_mb: memory_used,
        };

        // Validate performance requirements (25% overhead limit)
        let single_pass_estimate = raw_time; // Estimate single-pass time as raw analysis time
        let overhead_percentage = if single_pass_estimate > 0 {
            ((total_time as f64 - single_pass_estimate as f64) / single_pass_estimate as f64)
                * 100.0
        } else {
            0.0
        };

        if overhead_percentage > 25.0 {
            eprintln!(
                "Warning: Multi-pass analysis overhead ({:.1}%) exceeds 25% limit",
                overhead_percentage
            );
        }

        Ok(MultiPassResult {
            raw_complexity: raw_result,
            normalized_complexity: normalized_result,
            attribution,
            insights,
            recommendations,
            performance_metrics: Some(performance_metrics),
        })
    }

    /// Get current memory usage in MB (simplified implementation)
    fn get_memory_usage_mb() -> f32 {
        // This is a simplified implementation
        // In a production system, you might use a more sophisticated approach
        // like tracking heap allocations or using system memory APIs
        std::thread::available_parallelism()
            .map(|p| p.get() as f32 * 0.5) // Rough estimate
            .unwrap_or(2.0)
    }

    fn analyze_raw(&self, source: &str) -> Result<ComplexityResult> {
        let ast = self.raw_analyzer.parse(source, PathBuf::from("temp.rs"))?;
        let metrics = self.raw_analyzer.analyze(&ast);

        Ok(ComplexityResult {
            total_complexity: metrics.complexity.cyclomatic_complexity,
            cognitive_complexity: metrics.complexity.cognitive_complexity,
            functions: metrics.complexity.functions,
            analysis_type: AnalysisType::Raw,
        })
    }

    fn analyze_normalized(&self, source: &str) -> Result<ComplexityResult> {
        let ast = self
            .normalized_analyzer
            .parse(source, PathBuf::from("temp.rs"))?;
        let metrics = self.normalized_analyzer.analyze(&ast);

        Ok(ComplexityResult {
            total_complexity: metrics.complexity.cyclomatic_complexity,
            cognitive_complexity: metrics.complexity.cognitive_complexity,
            functions: metrics.complexity.functions,
            analysis_type: AnalysisType::Normalized,
        })
    }

    fn generate_insights(&self, attribution: &ComplexityAttribution) -> Vec<ComplexityInsight> {
        let mut insights = Vec::new();

        // Check for formatting impact
        let formatting_impact = attribution.formatting_artifacts.total as f32
            / (attribution.logical_complexity.total as f32 + 0.001);

        if formatting_impact > 0.2 {
            insights.push(ComplexityInsight {
                insight_type: InsightType::FormattingImpact,
                description: format!(
                    "Formatting contributes {:.0}% of measured complexity",
                    formatting_impact * 100.0
                ),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![
                    "Consider using automated formatting tools".to_string(),
                    "Standardize code formatting across the team".to_string(),
                ],
            });
        }

        // Check for pattern opportunities
        if attribution.pattern_complexity.confidence < 0.5 {
            insights.push(ComplexityInsight {
                insight_type: InsightType::PatternOpportunity,
                description: "Low pattern recognition suggests unique code structure".to_string(),
                impact_level: ImpactLevel::Low,
                actionable_steps: vec![
                    "Consider extracting common patterns".to_string(),
                    "Review for code duplication opportunities".to_string(),
                ],
            });
        }

        // Check for refactoring candidates
        if attribution.logical_complexity.total > 20 {
            insights.push(ComplexityInsight {
                insight_type: InsightType::RefactoringCandidate,
                description: format!(
                    "High logical complexity ({}) indicates refactoring opportunity",
                    attribution.logical_complexity.total
                ),
                impact_level: ImpactLevel::High,
                actionable_steps: vec![
                    "Break down into smaller functions".to_string(),
                    "Extract complex conditions into named variables".to_string(),
                    "Consider using early returns to reduce nesting".to_string(),
                ],
            });
        }

        insights
    }

    fn generate_recommendations(
        &self,
        attribution: &ComplexityAttribution,
        insights: &[ComplexityInsight],
    ) -> Vec<ComplexityRecommendation> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on attribution
        for component in &attribution.logical_complexity.breakdown {
            if component.contribution > 5 {
                recommendations.push(ComplexityRecommendation {
                    priority: RecommendationPriority::High,
                    category: RecommendationCategory::Refactoring,
                    title: format!("Simplify {}", component.description),
                    description: format!(
                        "This {} contributes {} complexity points",
                        component.description, component.contribution
                    ),
                    estimated_impact: component.contribution,
                    suggested_actions: component.suggestions.clone(),
                });
            }
        }

        // Generate recommendations based on insights
        for insight in insights {
            if insight.impact_level == ImpactLevel::High {
                recommendations.push(ComplexityRecommendation {
                    priority: RecommendationPriority::High,
                    category: match insight.insight_type {
                        InsightType::RefactoringCandidate => RecommendationCategory::Refactoring,
                        InsightType::PatternOpportunity => RecommendationCategory::Pattern,
                        InsightType::FormattingImpact => RecommendationCategory::Formatting,
                        _ => RecommendationCategory::General,
                    },
                    title: insight.description.clone(),
                    description: "Based on multi-pass analysis".to_string(),
                    estimated_impact: 0,
                    suggested_actions: insight.actionable_steps.clone(),
                });
            }
        }

        recommendations
    }

    pub fn generate_report(&self, result: &MultiPassResult) -> DiagnosticReport {
        self.diagnostic_reporter.generate_report(result)
    }
}

/// Analysis unit containing both raw and normalized source
pub struct AnalysisUnit {
    pub raw_source: String,
    pub normalized_source: String,
    pub language: Language,
    pub file_path: PathBuf,
}

impl AnalysisUnit {
    pub fn new(source: &str, language: Language, file_path: PathBuf) -> Self {
        let normalized_source = normalize_source(source, language);
        Self {
            raw_source: source.to_string(),
            normalized_source,
            language,
            file_path,
        }
    }
}

/// Result of complexity analysis (either raw or normalized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityResult {
    pub total_complexity: u32,
    pub cognitive_complexity: u32,
    pub functions: Vec<FunctionMetrics>,
    pub analysis_type: AnalysisType,
}

/// Type of analysis performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisType {
    Raw,
    Normalized,
}

/// Complete multi-pass analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPassResult {
    pub raw_complexity: ComplexityResult,
    pub normalized_complexity: ComplexityResult,
    pub attribution: ComplexityAttribution,
    pub insights: Vec<ComplexityInsight>,
    pub recommendations: Vec<ComplexityRecommendation>,
    pub performance_metrics: Option<AnalysisPerformanceMetrics>,
}

/// Configuration options for multi-pass analysis
#[derive(Debug, Clone)]
pub struct MultiPassOptions {
    pub language: Language,
    pub detail_level: DetailLevel,
    pub enable_recommendations: bool,
    pub track_source_locations: bool,
    pub generate_insights: bool,
    pub output_format: OutputFormat,
    pub performance_tracking: bool,
}

impl Default for MultiPassOptions {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            detail_level: DetailLevel::Standard,
            enable_recommendations: true,
            track_source_locations: true,
            generate_insights: true,
            output_format: OutputFormat::Json,
            performance_tracking: false,
        }
    }
}

/// Complexity insight generated from analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityInsight {
    pub insight_type: InsightType,
    pub description: String,
    pub impact_level: ImpactLevel,
    pub actionable_steps: Vec<String>,
}

/// Type of insight
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightType {
    FormattingImpact,
    PatternOpportunity,
    RefactoringCandidate,
    ComplexityHotspot,
    ImprovementSuggestion,
}

/// Impact level of an insight or issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Recommendation for complexity reduction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityRecommendation {
    pub priority: RecommendationPriority,
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub estimated_impact: u32,
    pub suggested_actions: Vec<String>,
}

/// Priority of a recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
}

/// Category of recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    Refactoring,
    Pattern,
    Formatting,
    General,
}

/// Performance metrics for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPerformanceMetrics {
    pub raw_analysis_time_ms: u64,
    pub normalized_analysis_time_ms: u64,
    pub attribution_time_ms: u64,
    pub total_time_ms: u64,
    pub memory_used_mb: f32,
}

// Normalized analyzer wrapper that applies semantic normalization
struct NormalizedAnalyzerWrapper {
    base_analyzer: Box<dyn Analyzer>,
}

impl NormalizedAnalyzerWrapper {
    fn new(
        base_analyzer: Box<dyn Analyzer>,
        _normalizer: Box<dyn SemanticNormalizer<Input = String, Output = String> + Send + Sync>,
    ) -> Self {
        Self { base_analyzer }
    }
}

impl Analyzer for NormalizedAnalyzerWrapper {
    fn parse(&self, content: &str, path: PathBuf) -> Result<crate::core::ast::Ast> {
        // Apply semantic normalization before parsing
        let normalized_source = normalize_source(content, self.language());
        self.base_analyzer.parse(&normalized_source, path)
    }

    fn analyze(&self, ast: &crate::core::ast::Ast) -> crate::core::FileMetrics {
        self.base_analyzer.analyze(ast)
    }

    fn language(&self) -> crate::core::Language {
        self.base_analyzer.language()
    }
}

// String-based semantic normalizer adapter for Rust
struct StringNormalizer;

impl StringNormalizer {
    fn new() -> Self {
        Self
    }
}

unsafe impl Send for StringNormalizer {}
unsafe impl Sync for StringNormalizer {}

impl SemanticNormalizer for StringNormalizer {
    type Input = String;
    type Output = String;

    fn normalize(&self, source: String) -> String {
        // Apply our existing string-based normalization for now
        // In a full implementation, this would parse to AST and use the semantic normalizer
        normalize_rust_source(&source)
    }
}

// Helper functions

fn create_raw_analyzer(language: Language) -> Box<dyn Analyzer> {
    crate::analyzers::get_analyzer(language)
}

fn create_normalized_analyzer(language: Language) -> Box<dyn Analyzer> {
    // Create a wrapper analyzer that applies semantic normalization
    match language {
        Language::Rust => Box::new(NormalizedAnalyzerWrapper::new(
            crate::analyzers::get_analyzer(language),
            Box::new(StringNormalizer::new()),
        )),
        _ => {
            // For other languages, use basic normalization for now
            crate::analyzers::get_analyzer(language)
        }
    }
}

fn normalize_source(source: &str, language: Language) -> String {
    // This is a simplified normalization
    // In a full implementation, this would use the semantic normalizer
    match language {
        Language::Rust => normalize_rust_source(source),
        Language::Python => normalize_python_source(source),
        _ => source.to_string(),
    }
}

fn normalize_rust_source(source: &str) -> String {
    // Remove excessive whitespace and normalize formatting while preserving structure
    let mut result = String::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim_end();
        let is_empty = trimmed.is_empty();

        // Skip multiple consecutive empty lines
        if is_empty && prev_empty {
            continue;
        }

        // Normalize internal whitespace (replace multiple spaces with single spaces)
        let normalized = if !is_empty {
            trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            String::new()
        };

        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&normalized);

        prev_empty = is_empty;
    }

    result
}

fn normalize_python_source(source: &str) -> String {
    // Similar normalization for Python
    source
        .lines()
        .map(|line| line.trim_end())
        .filter(|line| !line.trim().is_empty() || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// Public API functions

/// Main entry point for multi-pass analysis
pub fn analyze_with_attribution(
    source: &str,
    language: Language,
    options: MultiPassOptions,
) -> Result<MultiPassResult> {
    let analyzer = MultiPassAnalyzer::new(options);
    let unit = AnalysisUnit::new(source, language, PathBuf::from("source.rs"));
    analyzer.analyze(&unit)
}

/// Comparative analysis between two code versions
pub fn compare_complexity(
    before: &str,
    after: &str,
    language: Language,
) -> Result<ComparativeAnalysis> {
    let before_result = analyze_with_attribution(before, language, Default::default())?;
    let after_result = analyze_with_attribution(after, language, Default::default())?;

    Ok(generate_comparative_analysis(&before_result, &after_result))
}

/// Comparative analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeAnalysis {
    pub complexity_change: i32,
    pub cognitive_change: i32,
    pub formatting_impact_change: f32,
    pub improvements: Vec<String>,
    pub regressions: Vec<String>,
}

fn generate_comparative_analysis(
    before: &MultiPassResult,
    after: &MultiPassResult,
) -> ComparativeAnalysis {
    let complexity_change = after.raw_complexity.total_complexity as i32
        - before.raw_complexity.total_complexity as i32;

    let cognitive_change = after.raw_complexity.cognitive_complexity as i32
        - before.raw_complexity.cognitive_complexity as i32;

    let before_formatting = before.attribution.formatting_artifacts.total as f32
        / (before.attribution.logical_complexity.total as f32 + 0.001);
    let after_formatting = after.attribution.formatting_artifacts.total as f32
        / (after.attribution.logical_complexity.total as f32 + 0.001);
    let formatting_impact_change = after_formatting - before_formatting;

    let mut improvements = Vec::new();
    let mut regressions = Vec::new();

    if complexity_change < 0 {
        improvements.push(format!("Reduced complexity by {}", -complexity_change));
    } else if complexity_change > 0 {
        regressions.push(format!("Increased complexity by {}", complexity_change));
    }

    if formatting_impact_change < -0.1 {
        improvements.push("Reduced formatting-related complexity".to_string());
    } else if formatting_impact_change > 0.1 {
        regressions.push("Increased formatting-related complexity".to_string());
    }

    ComparativeAnalysis {
        complexity_change,
        cognitive_change,
        formatting_impact_change,
        improvements,
        regressions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_unit_creation() {
        let source = "fn main() { println!(\"Hello\"); }";
        let unit = AnalysisUnit::new(source, Language::Rust, PathBuf::from("test.rs"));

        assert_eq!(unit.raw_source, source);
        assert_eq!(unit.language, Language::Rust);
    }

    #[test]
    fn test_normalize_rust_source() {
        let source = "fn main()   {\n    println!(\"Hello\");\n\n\n}";
        let normalized = normalize_rust_source(source);

        assert!(!normalized.contains("   "));
        assert!(!normalized.contains("\n\n\n"));
    }

    #[test]
    fn test_multi_pass_options_default() {
        let options = MultiPassOptions::default();

        assert_eq!(options.language, Language::Rust);
        assert_eq!(options.detail_level, DetailLevel::Standard);
        assert!(options.enable_recommendations);
    }

    #[test]
    fn test_comparative_analysis_improvement() {
        let before = create_test_result(20, 15);
        let after = create_test_result(15, 12);

        let comparison = generate_comparative_analysis(&before, &after);

        assert_eq!(comparison.complexity_change, -5);
        assert_eq!(comparison.cognitive_change, -3);
        assert!(!comparison.improvements.is_empty());
    }

    fn create_test_result(complexity: u32, cognitive: u32) -> MultiPassResult {
        MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: complexity,
                cognitive_complexity: cognitive,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: complexity - 2,
                cognitive_complexity: cognitive - 1,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: crate::analysis::attribution::AttributedComplexity {
                    total: complexity - 5,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                formatting_artifacts: crate::analysis::attribution::AttributedComplexity {
                    total: 5,
                    breakdown: vec![],
                    confidence: 0.7,
                },
                pattern_complexity: crate::analysis::attribution::AttributedComplexity {
                    total: 0,
                    breakdown: vec![],
                    confidence: 0.5,
                },
                source_mappings: vec![],
            },
            insights: vec![],
            recommendations: vec![],
            performance_metrics: None,
        }
    }
}

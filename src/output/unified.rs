//! Unified output format that provides consistent structure for File and Function debt items
//!
//! This module implements spec 108, providing a normalized JSON output format where:
//! - All items have consistent top-level fields (type, score, category, priority, location)
//! - Score is at the same path for both File and Function items
//! - Location structure is unified (file, line, function)
//! - Simplifies filtering and sorting across item types

use crate::core::LanguageSpecificData;
use crate::io::writers::pattern_display::PATTERN_CONFIDENCE_THRESHOLD;
use crate::organization::anti_pattern_detector::{
    AntiPattern, AntiPatternSeverity, AntiPatternType,
};
use crate::priority::{
    DebtItem, DebtType, FileDebtItem, FunctionRole, UnifiedAnalysisQueries, UnifiedDebtItem,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
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

/// Codebase-wide cohesion statistics (spec 198)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohesionSummary {
    /// Average cohesion score across all analyzed files
    pub average: f64,
    /// Number of files with high cohesion (>= 0.7)
    pub high_cohesion_files: usize,
    /// Number of files with medium cohesion (0.4 - 0.7)
    pub medium_cohesion_files: usize,
    /// Number of files with low cohesion (< 0.4)
    pub low_cohesion_files: usize,
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

/// File-level cohesion metrics (spec 198)
///
/// Measures how tightly related the functions within a file are by analyzing
/// function call patterns. High cohesion indicates functions work together frequently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohesionOutput {
    /// Cohesion score between 0.0 (no cohesion) and 1.0 (perfect cohesion)
    pub score: f64,
    /// Number of internal function calls (within the same file)
    pub internal_calls: usize,
    /// Number of external function calls (to other files)
    pub external_calls: usize,
    /// Classification based on cohesion thresholds
    pub classification: CohesionClassification,
    /// Number of functions analyzed
    pub functions_analyzed: usize,
}

/// Cohesion classification based on score thresholds (spec 198)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CohesionClassification {
    /// Cohesion >= 0.7
    High,
    /// Cohesion 0.4 - 0.7
    Medium,
    /// Cohesion < 0.4
    Low,
}

impl CohesionClassification {
    /// Classify cohesion score into high/medium/low
    pub fn from_score(score: f64) -> Self {
        if score >= 0.7 {
            CohesionClassification::High
        } else if score >= 0.4 {
            CohesionClassification::Medium
        } else {
            CohesionClassification::Low
        }
    }
}

impl std::fmt::Display for CohesionClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CohesionClassification::High => write!(f, "High"),
            CohesionClassification::Medium => write!(f, "Medium"),
            CohesionClassification::Low => write!(f, "Low"),
        }
    }
}

/// File-level dependency metrics (spec 201)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependencies {
    /// Number of files that depend on this file
    pub afferent_coupling: usize,
    /// Number of files this file depends on
    pub efferent_coupling: usize,
    /// Instability metric (0.0 = stable, 1.0 = unstable)
    pub instability: f64,
    /// Total coupling (Ca + Ce)
    pub total_coupling: usize,
    /// Files that depend on this file (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependents: Vec<String>,
    /// Files this file depends on (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependencies: Vec<String>,
    /// Classification based on coupling characteristics
    pub coupling_classification: CouplingClassification,
}

/// Classification of file-level coupling characteristics (spec 201)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CouplingClassification {
    /// Low instability, high afferent coupling - core module others depend on
    StableCore,
    /// Balanced coupling - typical utility module
    UtilityModule,
    /// High instability, low afferent coupling - peripheral module
    LeafModule,
    /// Very low total coupling - may be dead code or standalone
    Isolated,
    /// High total coupling - may need refactoring
    HighlyCoupled,
}

impl std::fmt::Display for CouplingClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CouplingClassification::StableCore => write!(f, "Stable Core"),
            CouplingClassification::UtilityModule => write!(f, "Utility Module"),
            CouplingClassification::LeafModule => write!(f, "Leaf Module"),
            CouplingClassification::Isolated => write!(f, "Isolated"),
            CouplingClassification::HighlyCoupled => write!(f, "Highly Coupled"),
        }
    }
}

/// Classify coupling based on metrics (spec 201)
pub fn classify_coupling(afferent: usize, efferent: usize) -> CouplingClassification {
    let total = afferent + efferent;
    let instability = if total > 0 {
        efferent as f64 / total as f64
    } else {
        0.0
    };

    // Highly coupled threshold
    if total > 15 {
        return CouplingClassification::HighlyCoupled;
    }

    // Isolated module
    if total < 3 {
        return CouplingClassification::Isolated;
    }

    // Stable core: low instability, reasonable afferent coupling
    if instability < 0.3 && afferent >= 3 {
        return CouplingClassification::StableCore;
    }

    // Leaf module: high instability, low afferent coupling
    if instability > 0.7 && afferent <= 2 {
        return CouplingClassification::LeafModule;
    }

    // Default: utility module with balanced coupling
    CouplingClassification::UtilityModule
}

/// Calculate instability metric from coupling values
pub fn calculate_instability(afferent: usize, efferent: usize) -> f64 {
    let total = afferent + efferent;
    if total > 0 {
        efferent as f64 / total as f64
    } else {
        0.0
    }
}

/// Anti-pattern output for JSON serialization (spec 197)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternOutput {
    /// Quality score from anti-pattern analysis (0-100, higher = better)
    pub quality_score: f64,
    /// List of detected anti-patterns
    pub patterns: Vec<AntiPatternItem>,
    /// Summary counts by severity
    pub summary: AntiPatternSummary,
}

/// Individual anti-pattern item for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternItem {
    /// Type of anti-pattern detected
    pub pattern_type: AntiPatternType,
    /// Severity level
    pub severity: AntiPatternSeverity,
    /// Location where the anti-pattern was detected
    pub location: String,
    /// Human-readable description of the issue
    pub description: String,
    /// Recommended correction action
    pub recommendation: String,
    /// Affected methods (if applicable)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_methods: Vec<String>,
}

/// Summary counts of anti-patterns by severity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AntiPatternSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl From<&AntiPattern> for AntiPatternItem {
    fn from(pattern: &AntiPattern) -> Self {
        Self {
            pattern_type: pattern.pattern_type.clone(),
            severity: pattern.severity.clone(),
            location: pattern.location.clone(),
            description: pattern.description.clone(),
            recommendation: pattern.correction.clone(),
            affected_methods: pattern.affected_methods.clone(),
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<String>, // "state_machine" | "coordinator"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_details: Option<serde_json::Value>, // Pattern-specific metrics
}

/// Adjusted complexity based on entropy analysis
///
/// When high entropy is detected (repetitive patterns, similar branches),
/// complexity is dampened because the code is easier to understand than
/// raw complexity numbers suggest.
///
/// Formula: `dampened_cyclomatic = cyclomatic_complexity * dampening_factor`
///
/// Invariants:
/// - When `dampening_factor = 1.0`, `dampened_cyclomatic` equals original cyclomatic
/// - When `dampening_factor < 1.0`, `dampened_cyclomatic < cyclomatic_complexity`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedComplexity {
    /// Cyclomatic complexity adjusted by entropy dampening factor.
    /// Formula: `cyclomatic_complexity * dampening_factor`
    pub dampened_cyclomatic: f64,
    /// Factor applied to dampen complexity (0.0 - 1.0).
    /// 1.0 = no dampening, <1.0 = reduced complexity weight due to repetitive patterns.
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
    // Data flow factors (spec 218)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactorability_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_factor: Option<f64>,
}

/// Convert legacy DebtItem to unified format
impl UnifiedDebtItemOutput {
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

impl FileDebtItemOutput {
    /// Convert from FileDebtItem without cohesion data
    #[allow(dead_code)]
    fn from_file_item(item: &FileDebtItem, include_scoring_details: bool) -> Self {
        Self::from_file_item_with_cohesion(item, include_scoring_details, None)
    }

    fn from_file_item_with_cohesion(
        item: &FileDebtItem,
        include_scoring_details: bool,
        cohesion: Option<CohesionOutput>,
    ) -> Self {
        let score = item.score;

        // Build file dependencies if coupling data is present (spec 201)
        let dependencies = build_file_dependencies(&item.metrics);

        // Build anti-pattern output if present in god object analysis (spec 197)
        let anti_patterns = build_anti_patterns(&item.metrics);

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
            god_object_indicators: item.metrics.god_object_analysis.clone().map(|a| a.into()),
            dependencies,
            anti_patterns,
            cohesion,
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

/// Build AntiPatternOutput from FileDebtMetrics (spec 197)
fn build_anti_patterns(metrics: &crate::priority::FileDebtMetrics) -> Option<AntiPatternOutput> {
    // Get anti-pattern report from god object analysis
    let report = metrics
        .god_object_analysis
        .as_ref()
        .and_then(|a| a.anti_pattern_report.as_ref())?;

    // Convert patterns to output format
    let patterns: Vec<AntiPatternItem> = report.anti_patterns.iter().map(|p| p.into()).collect();

    // Build summary counts
    let mut summary = AntiPatternSummary::default();
    for pattern in &report.anti_patterns {
        match pattern.severity {
            AntiPatternSeverity::Critical => summary.critical += 1,
            AntiPatternSeverity::High => summary.high += 1,
            AntiPatternSeverity::Medium => summary.medium += 1,
            AntiPatternSeverity::Low => summary.low += 1,
        }
    }

    // Only return if there are patterns detected
    if patterns.is_empty() && summary.critical == 0 && summary.high == 0 {
        return None;
    }

    Some(AntiPatternOutput {
        quality_score: report.quality_score,
        patterns,
        summary,
    })
}

/// Build CohesionOutput from FileCohesionResult (spec 198)
fn build_cohesion_output(result: &crate::organization::FileCohesionResult) -> CohesionOutput {
    CohesionOutput {
        score: result.score,
        internal_calls: result.internal_calls,
        external_calls: result.external_calls,
        classification: CohesionClassification::from_score(result.score),
        functions_analyzed: result.functions_analyzed,
    }
}

/// Build FileDependencies from FileDebtMetrics (spec 201)
fn build_file_dependencies(metrics: &crate::priority::FileDebtMetrics) -> Option<FileDependencies> {
    // Only include if there's some coupling data
    let has_coupling_data = metrics.afferent_coupling > 0
        || metrics.efferent_coupling > 0
        || !metrics.dependents.is_empty()
        || !metrics.dependencies_list.is_empty();

    if !has_coupling_data {
        return None;
    }

    let afferent = metrics.afferent_coupling;
    let efferent = metrics.efferent_coupling;

    Some(FileDependencies {
        afferent_coupling: afferent,
        efferent_coupling: efferent,
        instability: metrics.instability,
        total_coupling: afferent + efferent,
        top_dependents: metrics.dependents.iter().take(5).cloned().collect(),
        top_dependencies: metrics.dependencies_list.iter().take(5).cloned().collect(),
        coupling_classification: classify_coupling(afferent, efferent),
    })
}

impl FunctionDebtItemOutput {
    fn from_function_item(item: &UnifiedDebtItem, include_scoring_details: bool) -> Self {
        let score = item.unified_score.final_score.value();
        let complexity_pattern = extract_complexity_pattern(
            &item.recommendation.rationale,
            &item.recommendation.primary_action,
        );
        let (pattern_type, pattern_confidence, pattern_details) =
            extract_pattern_data(&item.language_specific);
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
                    final_score: item.unified_score.final_score.value(),
                    purity_factor: item.unified_score.purity_factor,
                    refactorability_factor: item.unified_score.refactorability_factor,
                    pattern_factor: item.unified_score.pattern_factor,
                })
            } else {
                None
            },
            adjusted_complexity: item.entropy_details.as_ref().map(|e| AdjustedComplexity {
                // Dampened cyclomatic = cyclomatic * dampening_factor
                // When dampening_factor = 1.0, dampened_cyclomatic equals original cyclomatic
                dampened_cyclomatic: item.cyclomatic_complexity as f64 * e.dampening_factor,
                dampening_factor: e.dampening_factor,
            }),
            complexity_pattern,
            pattern_type,
            pattern_confidence,
            pattern_details,
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

/// Extract pattern data from language-specific information
///
/// Returns (pattern_type, confidence, details) if a pattern is detected with sufficient confidence
fn extract_pattern_data(
    language_specific: &Option<LanguageSpecificData>,
) -> (Option<String>, Option<f64>, Option<serde_json::Value>) {
    if let Some(LanguageSpecificData::Rust(rust_data)) = language_specific {
        // Check state machine first (higher priority)
        if let Some(sm_signals) = &rust_data.state_machine_signals {
            if sm_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                let details = serde_json::json!({
                    "transition_count": sm_signals.transition_count,
                    "match_expression_count": sm_signals.match_expression_count,
                    "action_dispatch_count": sm_signals.action_dispatch_count,
                });
                return (
                    Some("state_machine".to_string()),
                    Some(sm_signals.confidence),
                    Some(details),
                );
            }
        }

        // Check coordinator second
        if let Some(coord_signals) = &rust_data.coordinator_signals {
            if coord_signals.confidence >= PATTERN_CONFIDENCE_THRESHOLD {
                let details = serde_json::json!({
                    "actions": coord_signals.actions,
                    "comparisons": coord_signals.comparisons,
                });
                return (
                    Some("coordinator".to_string()),
                    Some(coord_signals.confidence),
                    Some(details),
                );
            }
        }
    }
    (None, None, None)
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

/// Unique key for debt item deduplication (spec 231)
///
/// Items are considered duplicates if they have the same file path, line number,
/// and function name (for function items) or just file path (for file items).
#[derive(Debug, Clone)]
struct DebtItemKey {
    file: String,
    line: Option<usize>,
    function: Option<String>,
}

impl PartialEq for DebtItemKey {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file && self.line == other.line && self.function == other.function
    }
}

impl Eq for DebtItemKey {}

impl Hash for DebtItemKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file.hash(state);
        self.line.hash(state);
        self.function.hash(state);
    }
}

impl From<&UnifiedDebtItemOutput> for DebtItemKey {
    fn from(item: &UnifiedDebtItemOutput) -> Self {
        match item {
            UnifiedDebtItemOutput::File(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: None,
                function: None,
            },
            UnifiedDebtItemOutput::Function(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: f.location.line,
                function: f.location.function.clone(),
            },
        }
    }
}

/// Deduplicate debt items by (file, line, function) key (spec 231)
///
/// Removes duplicate items that have the same location. Keeps the first occurrence
/// of each unique item. Logs when duplicates are removed for debugging.
fn deduplicate_items(items: Vec<UnifiedDebtItemOutput>) -> Vec<UnifiedDebtItemOutput> {
    let mut seen: HashSet<DebtItemKey> = HashSet::new();
    let mut result = Vec::with_capacity(items.len());
    let mut duplicate_count = 0;

    for item in items {
        let key = DebtItemKey::from(&item);

        if seen.insert(key.clone()) {
            result.push(item);
        } else {
            duplicate_count += 1;
            // Log duplicate removal for debugging (only in debug builds or when RUST_LOG is set)
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: Removed duplicate debt item: file={}, line={:?}, function={:?}",
                key.file, key.line, key.function
            );
        }
    }

    if duplicate_count > 0 {
        // Always log summary when duplicates are found
        eprintln!(
            "Warning: Removed {} duplicate debt items from output",
            duplicate_count
        );
    }

    result
}

/// Convert analysis results to unified output format
pub fn convert_to_unified_format(
    analysis: &crate::priority::UnifiedAnalysis,
    include_scoring_details: bool,
) -> UnifiedOutput {
    #[allow(unused_imports)]
    use crate::priority::score_types::Score0To100;
    use std::collections::HashMap;

    // Get all debt items sorted by score
    let all_items = analysis.get_top_mixed_priorities(usize::MAX);

    // Convert to unified format with call graph for cohesion calculation (spec 198)
    let unified_items: Vec<UnifiedDebtItemOutput> = all_items
        .iter()
        .map(|item| {
            UnifiedDebtItemOutput::from_debt_item_with_call_graph(
                item,
                include_scoring_details,
                Some(&analysis.call_graph),
            )
        })
        .collect();

    // Deduplicate items before calculating summary statistics (spec 231)
    let unified_items = deduplicate_items(unified_items);

    // Calculate summary statistics from deduplicated items
    let mut file_count = 0;
    let mut function_count = 0;
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut score_dist = ScoreDistribution {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };

    // Calculate total debt score from deduplicated items (spec 231)
    let total_debt_score: f64 = unified_items.iter().map(|item| item.score()).sum();

    // Cohesion summary statistics (spec 198)
    let mut cohesion_scores: Vec<f64> = Vec::new();
    let mut high_cohesion_count = 0;
    let mut medium_cohesion_count = 0;
    let mut low_cohesion_count = 0;

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
                // Collect cohesion stats (spec 198)
                if let Some(ref cohesion) = f.cohesion {
                    cohesion_scores.push(cohesion.score);
                    match cohesion.classification {
                        CohesionClassification::High => high_cohesion_count += 1,
                        CohesionClassification::Medium => medium_cohesion_count += 1,
                        CohesionClassification::Low => low_cohesion_count += 1,
                    }
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

    // Build cohesion summary if any cohesion data was collected (spec 198)
    let cohesion_summary = if !cohesion_scores.is_empty() {
        let average = cohesion_scores.iter().sum::<f64>() / cohesion_scores.len() as f64;
        Some(CohesionSummary {
            average,
            high_cohesion_files: high_cohesion_count,
            medium_cohesion_files: medium_cohesion_count,
            low_cohesion_files: low_cohesion_count,
        })
    } else {
        None
    };

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
            cohesion: cohesion_summary,
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

    // Tests for spec 201: File-level dependency metrics

    #[test]
    fn test_calculate_instability_balanced() {
        // Equal afferent and efferent should give 0.5 instability
        let instability = calculate_instability(5, 5);
        assert!((instability - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_stable() {
        // High afferent, no efferent should give 0.0 instability
        let instability = calculate_instability(10, 0);
        assert!((instability - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_unstable() {
        // No afferent, high efferent should give 1.0 instability
        let instability = calculate_instability(0, 10);
        assert!((instability - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_zero_coupling() {
        // Zero coupling should give 0.0 instability
        let instability = calculate_instability(0, 0);
        assert!((instability - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_classify_coupling_stable_core() {
        // Low instability, high afferent = stable core
        let classification = classify_coupling(8, 2); // instability = 0.2
        assert_eq!(classification, CouplingClassification::StableCore);
    }

    #[test]
    fn test_classify_coupling_leaf_module() {
        // High instability, low afferent = leaf module
        let classification = classify_coupling(1, 8); // instability = 0.89
        assert_eq!(classification, CouplingClassification::LeafModule);
    }

    #[test]
    fn test_classify_coupling_utility_module() {
        // Balanced coupling = utility module
        let classification = classify_coupling(5, 5);
        assert_eq!(classification, CouplingClassification::UtilityModule);
    }

    #[test]
    fn test_classify_coupling_isolated() {
        // Very low total coupling = isolated
        let classification = classify_coupling(1, 1);
        assert_eq!(classification, CouplingClassification::Isolated);
    }

    #[test]
    fn test_classify_coupling_highly_coupled() {
        // High total coupling > 15 = highly coupled
        let classification = classify_coupling(10, 10);
        assert_eq!(classification, CouplingClassification::HighlyCoupled);
    }

    #[test]
    fn test_coupling_classification_display() {
        assert_eq!(
            format!("{}", CouplingClassification::StableCore),
            "Stable Core"
        );
        assert_eq!(
            format!("{}", CouplingClassification::LeafModule),
            "Leaf Module"
        );
        assert_eq!(format!("{}", CouplingClassification::Isolated), "Isolated");
        assert_eq!(
            format!("{}", CouplingClassification::HighlyCoupled),
            "Highly Coupled"
        );
    }

    #[test]
    fn test_file_dependencies_serialization() {
        let deps = FileDependencies {
            afferent_coupling: 5,
            efferent_coupling: 3,
            instability: 0.375,
            total_coupling: 8,
            top_dependents: vec!["main.rs".to_string(), "lib.rs".to_string()],
            top_dependencies: vec!["std".to_string()],
            coupling_classification: CouplingClassification::UtilityModule,
        };

        let json = serde_json::to_string(&deps).unwrap();
        assert!(json.contains("\"afferent_coupling\":5"));
        assert!(json.contains("\"efferent_coupling\":3"));
        assert!(json.contains("\"instability\":0.375"));
        assert!(json.contains("\"total_coupling\":8"));
        assert!(json.contains("\"top_dependents\":[\"main.rs\",\"lib.rs\"]"));
        assert!(json.contains("\"top_dependencies\":[\"std\"]"));
        assert!(json.contains("\"coupling_classification\":\"utility_module\""));
    }

    #[test]
    fn test_file_dependencies_empty_lists_not_serialized() {
        let deps = FileDependencies {
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            total_coupling: 0,
            top_dependents: vec![],
            top_dependencies: vec![],
            coupling_classification: CouplingClassification::Isolated,
        };

        let json = serde_json::to_string(&deps).unwrap();
        // Empty vectors should be skipped
        assert!(!json.contains("\"top_dependents\""));
        assert!(!json.contains("\"top_dependencies\""));
    }

    // Tests for spec 231: Fix Duplicate Debt Items

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
    fn test_deduplication_removes_duplicate_functions() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_function_item("a.rs", 10, "foo", 45.0), // Duplicate
            create_test_function_item("b.rs", 20, "bar", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
        // Should keep first occurrence (score 50.0)
        assert_eq!(result[0].score(), 50.0);
        assert_eq!(result[1].score(), 30.0);
    }

    #[test]
    fn test_deduplication_preserves_unique_items() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_function_item("a.rs", 20, "bar", 45.0), // Different line
            create_test_function_item("b.rs", 10, "foo", 30.0), // Different file
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_deduplication_handles_file_items() {
        let items = vec![
            create_test_file_item("a.rs", 50.0),
            create_test_file_item("a.rs", 45.0), // Duplicate
            create_test_file_item("b.rs", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplication_mixed_item_types() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_file_item("a.rs", 45.0), // Different type, should not be duplicate
            create_test_function_item("a.rs", 10, "foo", 30.0), // Duplicate function
        ];

        let result = deduplicate_items(items);

        // Function and file items have different keys (file item has no line/function)
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplication_empty_input() {
        let items: Vec<UnifiedDebtItemOutput> = vec![];
        let result = deduplicate_items(items);
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplication_single_item() {
        let items = vec![create_test_function_item("a.rs", 10, "foo", 50.0)];
        let result = deduplicate_items(items);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_debt_item_key_equality() {
        let key1 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("foo".to_string()),
        };
        let key2 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("foo".to_string()),
        };
        let key3 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("bar".to_string()), // Different function
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_unified_debt_item_output_score() {
        let func_item = create_test_function_item("a.rs", 10, "foo", 75.5);
        let file_item = create_test_file_item("b.rs", 42.0);

        assert_eq!(func_item.score(), 75.5);
        assert_eq!(file_item.score(), 42.0);
    }

    // Tests for spec 232: Dampened cyclomatic calculation fix
    use crate::priority::unified_scorer::EntropyDetails;

    fn create_test_item_with_complexity(
        cyclomatic: u32,
        cognitive: u32,
        dampening_factor: f64,
    ) -> UnifiedDebtItem {
        use crate::priority::{
            ActionableRecommendation, FunctionRole, ImpactMetrics, Location, Score0To100,
            UnifiedDebtItem, UnifiedScore,
        };
        use std::path::PathBuf;

        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_func".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic,
                cognitive,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(50.0),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 20,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            entropy_details: Some(EntropyDetails {
                entropy_score: 0.5,
                pattern_repetition: 0.3,
                original_complexity: cognitive,
                adjusted_complexity: (cognitive as f64 * dampening_factor) as u32,
                dampening_factor,
                adjusted_cognitive: (cognitive as f64 * dampening_factor) as u32,
            }),
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: Some(dampening_factor),
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    #[test]
    fn test_dampening_factor_one_preserves_cyclomatic() {
        // Spec 232: When dampening_factor = 1.0, dampened_cyclomatic = cyclomatic
        let item = create_test_item_with_complexity(11, 23, 1.0);
        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        assert_eq!(adjusted.dampening_factor, 1.0);
        // Critical assertion: dampened_cyclomatic should equal cyclomatic, not cognitive
        assert_eq!(
            adjusted.dampened_cyclomatic, 11.0,
            "dampened_cyclomatic should equal cyclomatic_complexity when factor is 1.0"
        );
    }

    #[test]
    fn test_dampening_reduces_cyclomatic() {
        // Spec 232: dampened_cyclomatic = cyclomatic * dampening_factor
        let item = create_test_item_with_complexity(20, 40, 0.5);
        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        assert_eq!(adjusted.dampening_factor, 0.5);
        assert_eq!(
            adjusted.dampened_cyclomatic, 10.0,
            "dampened_cyclomatic should be cyclomatic * factor"
        );
    }

    #[test]
    fn test_dampened_cyclomatic_independent_of_cognitive() {
        // Spec 232: dampened_cyclomatic should only depend on cyclomatic, not cognitive
        // Two items with same cyclomatic but different cognitive
        let item1 = create_test_item_with_complexity(15, 10, 0.8);
        let item2 = create_test_item_with_complexity(15, 50, 0.8);

        let output1 = FunctionDebtItemOutput::from_function_item(&item1, false);
        let output2 = FunctionDebtItemOutput::from_function_item(&item2, false);

        let adjusted1 = output1
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        let adjusted2 = output2
            .adjusted_complexity
            .expect("should have adjusted_complexity");

        // Same dampened cyclomatic regardless of cognitive complexity
        assert_eq!(
            adjusted1.dampened_cyclomatic, adjusted2.dampened_cyclomatic,
            "dampened_cyclomatic should be the same for items with same cyclomatic complexity"
        );
        assert_eq!(
            adjusted1.dampened_cyclomatic,
            12.0, // 15 * 0.8
            "dampened_cyclomatic should be cyclomatic * dampening_factor"
        );
    }

    #[test]
    fn test_adjusted_complexity_serialization() {
        let adjusted = AdjustedComplexity {
            dampened_cyclomatic: 11.0,
            dampening_factor: 1.0,
        };
        let json = serde_json::to_string(&adjusted).unwrap();
        assert!(json.contains("\"dampened_cyclomatic\":11.0"));
        assert!(json.contains("\"dampening_factor\":1.0"));
    }
}

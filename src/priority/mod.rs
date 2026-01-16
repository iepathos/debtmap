pub mod architecture_recognition;
pub mod call_graph;
pub mod caller_classification;
pub mod classification;
pub mod complexity_patterns;
pub mod context;
pub mod coverage_propagation;
pub mod debt_aggregator;
pub mod debt_types;
pub mod detected_pattern;
pub mod external_api_detector;
pub mod file_metrics;
pub mod filter_config;
pub mod filter_predicates;
pub mod filtering;
pub mod formatted_output;
pub mod formatter;
pub mod formatter_markdown;
pub mod formatter_verbosity;
pub mod god_object_aggregation;
pub mod impact_calculation;
pub mod parallel_call_graph;
pub mod pipeline;
pub mod refactoring_impact;
pub mod score_formatter;
pub mod score_types;
pub mod scoring;
pub mod semantic_classifier;
pub mod tiers;
pub mod unified_analysis_queries;
pub mod unified_analysis_utils;
pub mod unified_scorer;
pub mod view;
pub mod view_pipeline;

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
pub use file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
pub use filter_predicates::FilterStatistics;
pub use filtering::{
    filter_with_metrics, ClassifiedItem, FilterConfig, FilterMetrics, FilterResult,
};
pub use formatter::{format_priorities, OutputFormat};
pub use formatter_markdown::{
    format_priorities_categorical_markdown, format_priorities_markdown,
    format_priorities_tiered_markdown,
};
pub use god_object_aggregation::{aggregate_god_object_metrics, GodObjectAggregatedMetrics};
pub use pipeline::{analyze_and_filter, filter_sort_limit, sort_by_score, take_top};
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use tiers::{classify_tier, RecommendationTier, TierConfig};
pub use unified_analysis_queries::UnifiedAnalysisQueries;
pub use unified_analysis_utils::UnifiedAnalysisUtils;
pub use unified_scorer::{calculate_unified_priority, Location, UnifiedDebtItem, UnifiedScore};
pub use view::{
    CategoryCounts, ItemLocation, LocationGroup, PreparedDebtView, ScoreDistribution, SortCriteria,
    ViewConfig, ViewItem, ViewSummary,
};
pub use view_pipeline::{
    prepare_view, prepare_view_default, prepare_view_for_terminal, prepare_view_for_tui,
};

// Re-export debt types from dedicated module (refactored for reduced complexity)
pub use debt_types::{DebtType, FunctionVisibility};

use im::Vector;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub file_items: Vector<FileDebtItem>,
    pub total_impact: ImpactMetrics,
    pub total_debt_score: f64,
    pub debt_density: f64,
    pub total_lines_of_code: usize,
    pub call_graph: CallGraph,
    pub data_flow_graph: crate::data_flow::DataFlowGraph,
    pub overall_coverage: Option<f64>,
    #[serde(default)]
    pub has_coverage_data: bool,
    /// Timing information for analysis phases (spec 130)
    #[serde(skip)]
    pub timings: Option<crate::builders::parallel_unified_analysis::AnalysisPhaseTimings>,
    /// Filter statistics for debugging (spec 242)
    #[serde(skip)]
    pub stats: FilterStatistics,
    /// All analyzed files with their line counts (Spec 201)
    /// Used for accurate total LOC calculation including files without debt items
    #[serde(skip)]
    pub analyzed_files: std::collections::HashMap<PathBuf, usize>,
}

// Single function analysis for evidence-based risk calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnalysis {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
    pub function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub is_pure: Option<bool>,
    pub purity_confidence: Option<f32>,
    pub nesting_depth: u32,
    pub is_test: bool,
    pub visibility: FunctionVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactMetrics {
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

/// Placeholder for recommendations (spec 262: recommendations removed)
/// This struct is kept for backward compatibility with JSON output.
/// All fields are now empty/default - debtmap focuses on identification and severity,
/// not on providing refactoring recommendations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub primary_action: String,
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementation_steps: Vec<String>,
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_items: Vec<String>,
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<ActionStep>>,
    /// Deprecated: recommendations removed in spec 262
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_effort_hours: Option<f32>,
}

/// Single actionable step with clear impact (spec 138a)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    /// What to do (concise, <80 chars)
    pub description: String,
    /// Expected impact (e.g., "-10 complexity", "+5 tests")
    pub impact: String,
    /// Difficulty level
    pub difficulty: Difficulty,
    /// Commands to execute this step
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}

/// Difficulty classification for actionable steps (spec 138a)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Difficulty {
    /// <30 min, straightforward
    Easy,
    /// 30min-2hr, requires some design
    Medium,
    /// >2hr, requires significant refactoring
    Hard,
}

impl Difficulty {
    /// Determine difficulty based on testing requirements
    pub fn for_testing(tests_needed: u32, cyclomatic: u32) -> Self {
        if tests_needed <= 3 && cyclomatic <= 10 {
            Difficulty::Easy
        } else if tests_needed <= 7 || cyclomatic <= 20 {
            Difficulty::Medium
        } else {
            Difficulty::Hard
        }
    }

    /// Determine difficulty for refactoring tasks
    pub fn for_refactoring(cyclomatic: u32, cognitive: u32) -> Self {
        if cyclomatic <= 15 && cognitive <= 20 {
            Difficulty::Easy
        } else if cyclomatic <= 25 || cognitive <= 35 {
            Difficulty::Medium
        } else {
            Difficulty::Hard
        }
    }
}

// DebtType moved to debt_types module for reduced complexity

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DebtCategory {
    Architecture,
    Testing,
    Performance,
    CodeQuality,
}

impl std::fmt::Display for DebtCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebtCategory::Architecture => write!(f, "Architecture"),
            DebtCategory::Testing => write!(f, "Testing"),
            DebtCategory::Performance => write!(f, "Performance"),
            DebtCategory::CodeQuality => write!(f, "CodeQuality"),
        }
    }
}

impl DebtCategory {
    pub fn from_debt_type(debt_type: &DebtType) -> Self {
        match debt_type {
            // Architecture Issues
            DebtType::GodObject { .. } => DebtCategory::Architecture,
            DebtType::FeatureEnvy { .. } => DebtCategory::Architecture,
            DebtType::PrimitiveObsession { .. } => DebtCategory::Architecture,
            DebtType::ScatteredType { .. } => DebtCategory::Architecture,
            DebtType::OrphanedFunctions { .. } => DebtCategory::Architecture,
            DebtType::UtilitiesSprawl { .. } => DebtCategory::Architecture,

            // Testing Gaps
            DebtType::TestingGap { .. } => DebtCategory::Testing,
            DebtType::TestComplexityHotspot { .. } => DebtCategory::Testing,
            DebtType::TestTodo { .. } => DebtCategory::Testing,
            DebtType::TestDuplication { .. } => DebtCategory::Testing,
            DebtType::AssertionComplexity { .. } => DebtCategory::Testing,
            DebtType::FlakyTestPattern { .. } => DebtCategory::Testing,

            // Performance Issues
            DebtType::AsyncMisuse { .. } => DebtCategory::Performance,
            DebtType::CollectionInefficiency { .. } => DebtCategory::Performance,
            DebtType::NestedLoops { .. } => DebtCategory::Performance,
            DebtType::BlockingIO { .. } => DebtCategory::Performance,
            DebtType::AllocationInefficiency { .. } => DebtCategory::Performance,
            DebtType::StringConcatenation { .. } => DebtCategory::Performance,
            DebtType::SuboptimalDataStructure { .. } => DebtCategory::Performance,
            DebtType::ResourceLeak { .. } => DebtCategory::Performance,

            // Code Quality (default category)
            DebtType::ComplexityHotspot { .. } => DebtCategory::CodeQuality,
            DebtType::DeadCode { .. } => DebtCategory::CodeQuality,
            DebtType::Duplication { .. } => DebtCategory::CodeQuality,
            DebtType::Risk { .. } => DebtCategory::CodeQuality,
            DebtType::ErrorSwallowing { .. } => DebtCategory::CodeQuality,
            DebtType::MagicValues { .. } => DebtCategory::CodeQuality,
            // Default for legacy variants
            _ => DebtCategory::CodeQuality,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DebtCategory::Architecture => "Architecture Issues",
            DebtCategory::Testing => "Testing Gaps",
            DebtCategory::Performance => "Performance Issues",
            DebtCategory::CodeQuality => "Code Quality",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DebtCategory::Architecture => "[ARCH]",
            DebtCategory::Testing => "[TEST]",
            DebtCategory::Performance => "[PERF]",
            DebtCategory::CodeQuality => "",
        }
    }

    /// Parse category from string (case-insensitive)
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "architecture" => Some(DebtCategory::Architecture),
            "testing" => Some(DebtCategory::Testing),
            "performance" => Some(DebtCategory::Performance),
            "codequality" | "code_quality" | "quality" => Some(DebtCategory::CodeQuality),
            _ => None,
        }
    }

    pub fn strategic_guidance(&self, item_count: usize, estimated_effort_hours: u32) -> String {
        match self {
            DebtCategory::Architecture => {
                format!(
                    "Focus on breaking down {} complex components. Consider design patterns and dependency injection. Estimated effort: {} hours.",
                    item_count, estimated_effort_hours
                )
            }
            DebtCategory::Testing => {
                format!(
                    "Implement {} missing tests. Target coverage improvement with focus on critical paths. Estimated effort: {} hours.",
                    item_count, estimated_effort_hours
                )
            }
            DebtCategory::Performance => {
                format!(
                    "Optimize {} performance bottlenecks. Profile and measure improvements. Estimated effort: {} hours.",
                    item_count, estimated_effort_hours
                )
            }
            DebtCategory::CodeQuality => {
                format!(
                    "Address {} code quality issues for better maintainability. Estimated effort: {} hours.",
                    item_count, estimated_effort_hours
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub category: DebtCategory,
    pub total_score: f64,
    pub item_count: usize,
    pub estimated_effort_hours: u32,
    pub average_severity: f64,
    pub top_items: Vec<DebtItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizedDebt {
    pub categories: BTreeMap<DebtCategory, CategorySummary>,
    pub cross_category_dependencies: Vec<CrossCategoryDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCategoryDependency {
    pub source_category: DebtCategory,
    pub target_category: DebtCategory,
    pub description: String,
    pub impact_level: ImpactLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Critical, // Blocks progress in target category
    High,     // Significantly affects target category
    Medium,   // Some effect on target category
    Low,      // Minor interaction
}

// FunctionVisibility moved to debt_types module

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DebtItem {
    File(Box<FileDebtItem>),
    Function(Box<UnifiedDebtItem>),
}

impl DebtItem {
    pub fn score(&self) -> f64 {
        match self {
            DebtItem::File(item) => item.score,
            DebtItem::Function(item) => item.unified_score.final_score,
        }
    }

    pub fn display_type(&self) -> &str {
        match self {
            DebtItem::File(_) => "FILE",
            DebtItem::Function(_) => "FUNCTION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    Critical, // Score ≥ 90.0
    High,     // Score 70.0-89.9
    Moderate, // Score 50.0-69.9
    Low,      // Score < 50.0
}

impl Tier {
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 90.0 => Tier::Critical,
            s if s >= 70.0 => Tier::High,
            s if s >= 50.0 => Tier::Moderate,
            _ => Tier::Low,
        }
    }

    pub fn header(&self) -> &'static str {
        match self {
            Tier::Critical => "[CRITICAL] CRITICAL - Immediate Action Required",
            Tier::High => "[WARN] HIGH - Current Sprint Priority",
            Tier::Moderate => "MODERATE - Next Sprint Planning",
            Tier::Low => "[INFO] LOW - Backlog Consideration",
        }
    }

    pub fn effort_estimate(&self) -> &'static str {
        match self {
            Tier::Critical => "1-2 days per item",
            Tier::High => "2-4 hours per item",
            Tier::Moderate => "1-2 hours per item",
            Tier::Low => "30 minutes per item",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplayGroup {
    pub tier: Tier,
    pub debt_type: String,
    pub items: Vec<DebtItem>,
    pub batch_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TieredDisplay {
    pub critical: Vec<DisplayGroup>,
    pub high: Vec<DisplayGroup>,
    pub moderate: Vec<DisplayGroup>,
    pub low: Vec<DisplayGroup>,
}

impl UnifiedAnalysis {
    pub fn new(call_graph: CallGraph) -> Self {
        // Create DataFlowGraph from the CallGraph
        let data_flow_graph = crate::data_flow::DataFlowGraph::from_call_graph(call_graph.clone());

        Self {
            items: Vector::new(),
            file_items: Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 0,
            call_graph,
            data_flow_graph,
            overall_coverage: None,
            has_coverage_data: false,
            timings: None,
            stats: FilterStatistics::new(),
            analyzed_files: std::collections::HashMap::new(),
        }
    }

    /// Register an analyzed file and its line count (Spec 201)
    ///
    /// This ensures accurate total LOC calculation by including files
    /// even if they have no debt items.
    pub fn register_analyzed_file(&mut self, path: PathBuf, line_count: usize) {
        self.analyzed_files.insert(path, line_count);
    }

    /// Register multiple analyzed files at once (Spec 201)
    pub fn register_analyzed_files(&mut self, files: impl IntoIterator<Item = (PathBuf, usize)>) {
        for (path, line_count) in files {
            self.analyzed_files.insert(path, line_count);
        }
    }

    pub fn calculate_total_impact(&mut self) {
        use impact_calculation::*;

        // Step 1: Accumulate metrics from function-level items
        let item_metrics = accumulate_item_metrics(&self.items, &self.analyzed_files);

        // Step 2: Collect god object files to avoid double-counting
        let god_object_files = collect_god_object_files(&self.items);

        // Step 3: Accumulate metrics from file-level items
        let file_metrics = accumulate_file_metrics(&self.file_items, &god_object_files);

        // Step 4: Merge unique files from both sources
        let unique_files = merge_unique_files(item_metrics.unique_files, file_metrics.unique_files);

        // Step 5: Calculate totals using pure functions
        let total_lines_of_code = calculate_total_loc(&unique_files);
        let total_debt_score = item_metrics.total_debt_score + file_metrics.additional_debt_score;
        let raw_coverage = item_metrics.coverage_improvement + file_metrics.coverage_improvement;
        let coverage_improvement = scale_coverage_improvement(raw_coverage);
        let debt_density = calculate_debt_density(total_debt_score, total_lines_of_code);

        // Step 6: Debug logging (side effect, kept separate)
        self.log_scoring_debug(total_debt_score, &god_object_files);

        // Step 7: Update self with computed values
        self.total_debt_score = total_debt_score;
        self.total_lines_of_code = total_lines_of_code;
        self.debt_density = debt_density;
        self.total_impact = ImpactMetrics {
            coverage_improvement,
            lines_reduction: item_metrics.lines_reduction + file_metrics.lines_reduction,
            complexity_reduction: item_metrics.complexity_reduction
                + file_metrics.complexity_reduction,
            risk_reduction: item_metrics.risk_reduction,
        };
    }

    /// Log scoring debug information when DEBTMAP_DEBUG_SCORING is set.
    fn log_scoring_debug(
        &self,
        total_debt_score: f64,
        god_object_files: &std::collections::HashSet<PathBuf>,
    ) {
        if std::env::var("DEBTMAP_DEBUG_SCORING").is_err() {
            return;
        }

        eprintln!("\n=== Score Calculation Debug ===");
        eprintln!("Function-level items count: {}", self.items.len());
        eprintln!("File-level items count: {}", self.file_items.len());
        eprintln!("God object files: {}", god_object_files.len());
        eprintln!("Total debt score: {:.0}", total_debt_score);
        eprintln!(
            "Average per function item: {:.1}",
            if self.items.is_empty() {
                0.0
            } else {
                total_debt_score / self.items.len() as f64
            }
        );

        // Show top 10 scores
        let mut sorted_items: Vec<_> = self.items.iter().collect();
        sorted_items.sort_by(|a, b| {
            b.unified_score
                .final_score
                .partial_cmp(&a.unified_score.final_score)
                .unwrap()
        });
        eprintln!("\nTop 10 scores:");
        for (i, item) in sorted_items.iter().take(10).enumerate() {
            eprintln!(
                "  {}: {:.1} - {:?} at {}::{}",
                i + 1,
                item.unified_score.final_score,
                item.debt_type,
                item.location.file.display(),
                item.location.function
            );
        }
        eprintln!("===============================\n");
    }

    /// Filter analysis results by debt categories
    pub fn filter_by_categories(&self, categories: &[DebtCategory]) -> Self {
        let filtered_items: Vector<UnifiedDebtItem> = self
            .items
            .iter()
            .filter(|item| {
                let item_category = DebtCategory::from_debt_type(&item.debt_type);
                categories.contains(&item_category)
            })
            .cloned()
            .collect();

        let filtered_file_items: Vector<FileDebtItem> = self
            .file_items
            .iter()
            .filter(|_item| {
                // File items (god objects) are architectural
                categories.contains(&DebtCategory::Architecture)
            })
            .cloned()
            .collect();

        // Recalculate totals for filtered set
        let total_debt_score: f64 = filtered_items
            .iter()
            .map(|item| item.unified_score.final_score)
            .sum();

        Self {
            items: filtered_items,
            file_items: filtered_file_items,
            total_debt_score,
            total_impact: self.total_impact.clone(),
            debt_density: self.debt_density,
            total_lines_of_code: self.total_lines_of_code,
            call_graph: self.call_graph.clone(),
            data_flow_graph: self.data_flow_graph.clone(),
            overall_coverage: self.overall_coverage,
            has_coverage_data: self.has_coverage_data,
            timings: self.timings.clone(),
            stats: self.stats.clone(),
            analyzed_files: self.analyzed_files.clone(),
        }
    }

    /// Get filtering statistics for debugging (spec 242).
    pub fn filter_statistics(&self) -> &FilterStatistics {
        &self.stats
    }

    /// Log filter summary if DEBTMAP_SHOW_FILTER_STATS is set (spec 242).
    pub fn log_filter_summary(&self) {
        if std::env::var("DEBTMAP_SHOW_FILTER_STATS").is_ok() {
            let stats = self.filter_statistics();
            eprintln!("\n=== Filter Statistics ===");
            eprintln!("Total processed: {}", stats.total_items_processed);
            eprintln!("Items added: {}", stats.items_added);
            eprintln!("Acceptance rate: {:.1}%", stats.acceptance_rate());
            eprintln!("\nRejection reasons:");
            eprintln!("  Score threshold: {}", stats.filtered_by_score);
            eprintln!("  Risk threshold: {}", stats.filtered_by_risk);
            eprintln!("  Complexity threshold: {}", stats.filtered_by_complexity);
            eprintln!("  Duplicates: {}", stats.filtered_as_duplicate);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod category_tests;

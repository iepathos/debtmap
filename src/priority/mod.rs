pub mod call_graph;
pub mod classification;
pub mod complexity_patterns;
pub mod coverage_propagation;
pub mod debt_aggregator;
pub mod detected_pattern;
pub mod external_api_detector;
pub mod file_metrics;
pub mod filtering;
pub mod formatted_output;
pub mod formatter;
pub mod formatter_markdown;
pub mod formatter_verbosity;
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

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
pub use file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
pub use filtering::{
    filter_with_metrics, ClassifiedItem, FilterConfig, FilterMetrics, FilterResult,
};
pub use formatter::{format_priorities, OutputFormat};
pub use formatter_markdown::{
    format_priorities_categorical_markdown, format_priorities_markdown,
    format_priorities_tiered_markdown,
};
pub use pipeline::{analyze_and_filter, filter_sort_limit, sort_by_score, take_top};
pub use score_types::{Score0To1, Score0To100};
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use tiers::{classify_tier, RecommendationTier, TierConfig};
pub use unified_analysis_queries::UnifiedAnalysisQueries;
pub use unified_analysis_utils::UnifiedAnalysisUtils;
pub use unified_scorer::{calculate_unified_priority, Location, UnifiedDebtItem, UnifiedScore};

use im::Vector;
use std::collections::BTreeMap;
use std::hash::Hash;
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

/// Concise recommendation with clear action steps (spec 138a)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    /// One-line primary action
    pub primary_action: String,
    /// Why this matters
    pub rationale: String,
    /// Legacy steps (for backward compatibility)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementation_steps: Vec<String>,
    /// Related items
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_items: Vec<String>,
    /// New structured steps with impact and difficulty (spec 138a)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<ActionStep>>,
    /// Estimated total effort in hours (spec 138a)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebtType {
    // Legacy variants from core::DebtType (spec 203)
    Todo {
        reason: Option<String>,
    },
    Fixme {
        reason: Option<String>,
    },
    CodeSmell {
        smell_type: Option<String>,
    },
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
    },
    Dependency {
        dependency_type: Option<String>,
    },
    ResourceManagement {
        issue_type: Option<String>,
    },
    CodeOrganization {
        issue_type: Option<String>,
    },
    TestComplexity {
        cyclomatic: u32,
        cognitive: u32,
    },
    TestQuality {
        issue_type: Option<String>,
    },
    // Priority-specific variants
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        /// Entropy-adjusted cyclomatic complexity (spec 182)
        /// None if entropy analysis was not performed
        #[serde(default)]
        adjusted_cyclomatic: Option<u32>,
    },
    DeadCode {
        visibility: FunctionVisibility,
        cyclomatic: u32,
        cognitive: u32,
        usage_hints: Vec<String>,
    },
    Duplication {
        instances: u32,
        total_lines: u32,
    },
    Risk {
        risk_score: f64,
        factors: Vec<String>,
    },
    // Test-specific debt types
    TestComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        threshold: u32,
    },
    TestTodo {
        priority: crate::core::Priority,
        reason: Option<String>,
    },
    TestDuplication {
        instances: u32,
        total_lines: u32,
        similarity: f64,
    },
    ErrorSwallowing {
        pattern: String,
        context: Option<String>,
    },
    // Resource Management debt types
    AllocationInefficiency {
        pattern: String,
        impact: String,
    },
    StringConcatenation {
        loop_type: String,
        iterations: Option<u32>,
    },
    NestedLoops {
        depth: u32,
        complexity_estimate: String,
    },
    BlockingIO {
        operation: String,
        context: String,
    },
    SuboptimalDataStructure {
        current_type: String,
        recommended_type: String,
    },
    // Organization debt types
    GodObject {
        methods: u32,
        fields: u32,
        responsibilities: u32,
        god_object_score: Score0To100,
    },
    GodModule {
        functions: u32,
        lines: u32,
        responsibilities: u32,
    },
    FeatureEnvy {
        external_class: String,
        usage_ratio: f64,
    },
    PrimitiveObsession {
        primitive_type: String,
        domain_concept: String,
    },
    MagicValues {
        value: String,
        occurrences: u32,
    },
    // Testing quality debt types
    AssertionComplexity {
        assertion_count: u32,
        complexity_score: f64,
    },
    FlakyTestPattern {
        pattern_type: String,
        reliability_impact: String,
    },
    // Resource management debt types
    AsyncMisuse {
        pattern: String,
        performance_impact: String,
    },
    ResourceLeak {
        resource_type: String,
        cleanup_missing: String,
    },
    CollectionInefficiency {
        collection_type: String,
        inefficiency_type: String,
    },
    // Type organization debt types (Spec 187)
    ScatteredType {
        type_name: String,
        total_methods: usize,
        file_count: usize,
        severity: String,
    },
    OrphanedFunctions {
        target_type: String,
        function_count: usize,
        file_count: usize,
    },
    UtilitiesSprawl {
        function_count: usize,
        distinct_types: usize,
    },
}

// Custom Eq implementation that handles f64 fields by comparing their bit representations
impl Eq for DebtType {}

// Custom Hash implementation that handles f64 fields by hashing their bit representations
impl std::hash::Hash for DebtType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            DebtType::Todo { reason } => reason.hash(state),
            DebtType::Fixme { reason } => reason.hash(state),
            DebtType::CodeSmell { smell_type } => smell_type.hash(state),
            DebtType::Complexity {
                cyclomatic,
                cognitive,
            } => {
                cyclomatic.hash(state);
                cognitive.hash(state);
            }
            DebtType::Dependency { dependency_type } => dependency_type.hash(state),
            DebtType::ResourceManagement { issue_type } => issue_type.hash(state),
            DebtType::CodeOrganization { issue_type } => issue_type.hash(state),
            DebtType::TestComplexity {
                cyclomatic,
                cognitive,
            } => {
                cyclomatic.hash(state);
                cognitive.hash(state);
            }
            DebtType::TestQuality { issue_type } => issue_type.hash(state),
            DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive,
            } => {
                coverage.to_bits().hash(state);
                cyclomatic.hash(state);
                cognitive.hash(state);
            }
            DebtType::ComplexityHotspot {
                cyclomatic,
                cognitive,
                adjusted_cyclomatic,
            } => {
                cyclomatic.hash(state);
                cognitive.hash(state);
                adjusted_cyclomatic.hash(state);
            }
            DebtType::DeadCode {
                visibility,
                cyclomatic,
                cognitive,
                usage_hints,
            } => {
                visibility.hash(state);
                cyclomatic.hash(state);
                cognitive.hash(state);
                usage_hints.hash(state);
            }
            DebtType::Duplication {
                instances,
                total_lines,
            } => {
                instances.hash(state);
                total_lines.hash(state);
            }
            DebtType::Risk {
                risk_score,
                factors,
            } => {
                risk_score.to_bits().hash(state);
                factors.hash(state);
            }
            DebtType::TestComplexityHotspot {
                cyclomatic,
                cognitive,
                threshold,
            } => {
                cyclomatic.hash(state);
                cognitive.hash(state);
                threshold.hash(state);
            }
            DebtType::TestTodo { priority, reason } => {
                priority.hash(state);
                reason.hash(state);
            }
            DebtType::TestDuplication {
                instances,
                total_lines,
                similarity,
            } => {
                instances.hash(state);
                total_lines.hash(state);
                similarity.to_bits().hash(state);
            }
            DebtType::ErrorSwallowing { pattern, context } => {
                pattern.hash(state);
                context.hash(state);
            }
            DebtType::AllocationInefficiency { pattern, impact } => {
                pattern.hash(state);
                impact.hash(state);
            }
            DebtType::StringConcatenation {
                loop_type,
                iterations,
            } => {
                loop_type.hash(state);
                iterations.hash(state);
            }
            DebtType::NestedLoops {
                depth,
                complexity_estimate,
            } => {
                depth.hash(state);
                complexity_estimate.hash(state);
            }
            DebtType::BlockingIO { operation, context } => {
                operation.hash(state);
                context.hash(state);
            }
            DebtType::SuboptimalDataStructure {
                current_type,
                recommended_type,
            } => {
                current_type.hash(state);
                recommended_type.hash(state);
            }
            DebtType::MagicValues { value, occurrences } => {
                value.hash(state);
                occurrences.hash(state);
            }
            DebtType::AssertionComplexity {
                assertion_count,
                complexity_score,
            } => {
                assertion_count.hash(state);
                complexity_score.to_bits().hash(state);
            }
            DebtType::GodObject {
                methods,
                fields,
                responsibilities,
                god_object_score,
            } => {
                methods.hash(state);
                fields.hash(state);
                responsibilities.hash(state);
                god_object_score.value().to_bits().hash(state);
            }
            DebtType::GodModule {
                functions,
                lines,
                responsibilities,
            } => {
                functions.hash(state);
                lines.hash(state);
                responsibilities.hash(state);
            }
            DebtType::FeatureEnvy {
                external_class,
                usage_ratio,
            } => {
                external_class.hash(state);
                usage_ratio.to_bits().hash(state);
            }
            DebtType::PrimitiveObsession {
                primitive_type,
                domain_concept,
            } => {
                primitive_type.hash(state);
                domain_concept.hash(state);
            }
            DebtType::ResourceLeak {
                resource_type,
                cleanup_missing,
            } => {
                resource_type.hash(state);
                cleanup_missing.hash(state);
            }
            DebtType::CollectionInefficiency {
                collection_type,
                inefficiency_type,
            } => {
                collection_type.hash(state);
                inefficiency_type.hash(state);
            }
            DebtType::ScatteredType {
                type_name,
                total_methods,
                file_count,
                severity,
            } => {
                type_name.hash(state);
                total_methods.hash(state);
                file_count.hash(state);
                severity.hash(state);
            }
            DebtType::OrphanedFunctions {
                target_type,
                function_count,
                file_count,
            } => {
                target_type.hash(state);
                function_count.hash(state);
                file_count.hash(state);
            }
            DebtType::UtilitiesSprawl {
                function_count,
                distinct_types,
            } => {
                function_count.hash(state);
                distinct_types.hash(state);
            }
            DebtType::FlakyTestPattern {
                pattern_type,
                reliability_impact,
            } => {
                pattern_type.hash(state);
                reliability_impact.hash(state);
            }
            DebtType::AsyncMisuse {
                pattern,
                performance_impact,
            } => {
                pattern.hash(state);
                performance_impact.hash(state);
            }
        }
    }
}

impl std::fmt::Display for DebtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Legacy variants
            DebtType::Todo { .. } => write!(f, "TODO"),
            DebtType::Fixme { .. } => write!(f, "FIXME"),
            DebtType::CodeSmell { .. } => write!(f, "Code Smell"),
            DebtType::Complexity { .. } => write!(f, "Complexity"),
            DebtType::Dependency { .. } => write!(f, "Dependency"),
            DebtType::ResourceManagement { .. } => write!(f, "Resource Management"),
            DebtType::CodeOrganization { .. } => write!(f, "Code Organization"),
            DebtType::TestComplexity { .. } => write!(f, "Test Complexity"),
            DebtType::TestQuality { .. } => write!(f, "Test Quality"),
            // Priority-specific variants
            DebtType::TestingGap { .. } => write!(f, "Testing Gap"),
            DebtType::ComplexityHotspot { .. } => write!(f, "Complexity Hotspot"),
            DebtType::DeadCode { .. } => write!(f, "Dead Code"),
            DebtType::Duplication { .. } => write!(f, "Duplication"),
            DebtType::Risk { .. } => write!(f, "Risk"),
            DebtType::TestComplexityHotspot { .. } => write!(f, "Test Complexity Hotspot"),
            DebtType::TestTodo { .. } => write!(f, "Test TODO"),
            DebtType::TestDuplication { .. } => write!(f, "Test Duplication"),
            DebtType::ErrorSwallowing { pattern, .. } => write!(f, "Error Swallowing: {}", pattern),
            DebtType::AllocationInefficiency { .. } => write!(f, "Allocation Inefficiency"),
            DebtType::StringConcatenation { .. } => write!(f, "String Concatenation"),
            DebtType::NestedLoops { .. } => write!(f, "Nested Loops"),
            DebtType::BlockingIO { .. } => write!(f, "Blocking I/O"),
            DebtType::SuboptimalDataStructure { .. } => write!(f, "Suboptimal Data Structure"),
            DebtType::GodObject { .. } => write!(f, "God Object"),
            DebtType::GodModule { .. } => write!(f, "God Module"),
            DebtType::FeatureEnvy { .. } => write!(f, "Feature Envy"),
            DebtType::PrimitiveObsession { .. } => write!(f, "Primitive Obsession"),
            DebtType::MagicValues { .. } => write!(f, "Magic Values"),
            DebtType::AssertionComplexity { .. } => write!(f, "Assertion Complexity"),
            DebtType::FlakyTestPattern { .. } => write!(f, "Flaky Test Pattern"),
            DebtType::AsyncMisuse { .. } => write!(f, "Async Misuse"),
            DebtType::ResourceLeak { .. } => write!(f, "Resource Leak"),
            DebtType::CollectionInefficiency { .. } => write!(f, "Collection Inefficiency"),
            DebtType::ScatteredType { .. } => write!(f, "Scattered Type"),
            DebtType::OrphanedFunctions { .. } => write!(f, "Orphaned Functions"),
            DebtType::UtilitiesSprawl { .. } => write!(f, "Utilities Sprawl"),
        }
    }
}

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
            DebtType::GodModule { .. } => DebtCategory::Architecture,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtItem {
    File(Box<FileDebtItem>),
    Function(Box<UnifiedDebtItem>),
}

impl DebtItem {
    pub fn score(&self) -> f64 {
        match self {
            DebtItem::File(item) => item.score,
            DebtItem::Function(item) => item.unified_score.final_score.value(),
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
        }
    }

    pub fn calculate_total_impact(&mut self) {
        let mut coverage_improvement = 0.0;
        let mut lines_reduction = 0;
        let mut complexity_reduction = 0.0;
        let mut risk_reduction = 0.0;
        let mut _functions_to_test = 0;
        let mut total_debt_score = 0.0;

        // Track unique files to avoid double-counting LOC (spec 204 optimization)
        // Use cached line counts from items instead of file I/O
        let mut unique_files: std::collections::HashMap<std::path::PathBuf, usize> =
            std::collections::HashMap::new();

        for item in &self.items {
            // Sum up all final scores as the total debt score
            total_debt_score += item.unified_score.final_score.value();

            // Use cached file line count from item if available (spec 204)
            if let Some(line_count) = item.file_line_count {
                unique_files
                    .entry(item.location.file.clone())
                    .or_insert(line_count);
            }

            // Only count functions that actually need testing
            if item.expected_impact.coverage_improvement > 0.0 {
                _functions_to_test += 1;
                // Each function contributes a small amount to overall coverage
                // Estimate based on function count (rough approximation)
                coverage_improvement += item.expected_impact.coverage_improvement / 100.0;
            }
            lines_reduction += item.expected_impact.lines_reduction;
            complexity_reduction += item.expected_impact.complexity_reduction;
            risk_reduction += item.expected_impact.risk_reduction;
        }

        // Add file-level impacts
        for file_item in &self.file_items {
            total_debt_score += file_item.score;

            // Track file and its actual total lines
            unique_files.insert(
                file_item.metrics.path.clone(),
                file_item.metrics.total_lines,
            );

            // File-level impacts are typically larger
            complexity_reduction += file_item.impact.complexity_reduction;
            lines_reduction += (file_item.metrics.total_lines / 10) as u32; // Rough estimate of reduction

            // Coverage improvement from fixing file-level issues
            if file_item.metrics.coverage_percent < 0.8 {
                coverage_improvement += (0.8 - file_item.metrics.coverage_percent) * 10.0;
            }
        }

        // Sum up unique file line counts (no file I/O needed - spec 204)
        let total_lines_of_code: usize = unique_files.values().sum();

        // Coverage improvement is the estimated overall project coverage gain
        // Assuming tested functions represent a portion of the codebase
        coverage_improvement = (coverage_improvement * 5.0).min(100.0); // Scale factor for visibility

        // Total complexity reduction (sum of all reductions)
        let total_complexity_reduction = complexity_reduction;

        // Calculate debt density (per 1000 LOC)
        let debt_density = if total_lines_of_code > 0 {
            (total_debt_score / total_lines_of_code as f64) * 1000.0
        } else {
            0.0
        };

        self.total_debt_score = total_debt_score;
        self.total_lines_of_code = total_lines_of_code;
        self.debt_density = debt_density;
        self.total_impact = ImpactMetrics {
            coverage_improvement,
            lines_reduction,
            complexity_reduction: total_complexity_reduction,
            risk_reduction,
        };
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
            .map(|item| item.unified_score.final_score.value())
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
        }
    }

    /// Filter analysis results by minimum score threshold (spec 193)
    ///
    /// Removes items below the specified score threshold and recalculates
    /// total debt score and debt density based on remaining items.
    /// Also filters out T4 items if show_t4_in_main_report is false (default).
    pub fn filter_by_score_threshold(&self, min_score: f64) -> Self {
        use crate::priority::tiers::{classify_tier, RecommendationTier, TierConfig};

        let tier_config = TierConfig::default();

        let filtered_items: Vector<UnifiedDebtItem> = self
            .items
            .iter()
            .filter(|item| {
                // Filter by score
                if item.unified_score.final_score.value() < min_score {
                    return false;
                }

                // Filter T4 items unless explicitly requested (consistent with display behavior)
                if !tier_config.show_t4_in_main_report {
                    // Use pre-assigned tier if available (e.g., for god objects), otherwise classify
                    let tier = item
                        .tier
                        .unwrap_or_else(|| classify_tier(item, &tier_config));
                    if tier == RecommendationTier::T4Maintenance {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        let filtered_file_items: Vector<FileDebtItem> = self
            .file_items
            .iter()
            .filter(|item| item.score >= min_score)
            .cloned()
            .collect();

        // Recalculate total debt score for filtered items
        let function_debt_score: f64 = filtered_items
            .iter()
            .map(|item| item.unified_score.final_score.value())
            .sum();

        let file_debt_score: f64 = filtered_file_items.iter().map(|item| item.score).sum();

        let total_debt_score = function_debt_score + file_debt_score;

        // Recalculate debt density based on filtered items
        let debt_density = if self.total_lines_of_code > 0 {
            (total_debt_score / self.total_lines_of_code as f64) * 1000.0
        } else {
            0.0
        };

        Self {
            items: filtered_items,
            file_items: filtered_file_items,
            total_debt_score,
            total_impact: self.total_impact.clone(),
            debt_density,
            total_lines_of_code: self.total_lines_of_code,
            call_graph: self.call_graph.clone(),
            data_flow_graph: self.data_flow_graph.clone(),
            overall_coverage: self.overall_coverage,
            has_coverage_data: self.has_coverage_data,
            timings: self.timings.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debtitem_file_roundtrip() {
        use file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact};
        use std::path::PathBuf;

        // Create a File debt item
        let file_item = DebtItem::File(Box::new(FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("./test.rs"),
                total_lines: 100,
                function_count: 5,
                class_count: 1,
                avg_complexity: 3.0,
                max_complexity: 10,
                total_complexity: 50,
                coverage_percent: 0.5,
                uncovered_lines: 50,
                god_object_analysis: None,
                function_scores: vec![],
                god_object_type: None,
                file_type: None,
            },
            score: 50.0,
            priority_rank: 1,
            recommendation: "Test".to_string(),
            impact: FileImpact {
                complexity_reduction: 10.0,
                maintainability_improvement: 5.0,
                test_effort: 2.0,
            },
        }));

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&file_item).unwrap();
        eprintln!("Serialized JSON:\n{}", json);

        // Try to deserialize it back
        let result: Result<DebtItem, _> = serde_json::from_str(&json);
        if let Err(e) = &result {
            eprintln!("Deserialization error: {}", e);
        }
        assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

        match result.unwrap() {
            DebtItem::File(_) => {} // Success
            DebtItem::Function(_) => panic!("Deserialized as wrong variant!"),
        }
    }

    #[test]
    fn test_debtitem_from_real_json() {
        // This is the actual format from debtmap analyze output
        let json = r#"{
          "File": {
            "metrics": {
              "path": "./test.rs",
              "total_lines": 100,
              "function_count": 5,
              "class_count": 1,
              "avg_complexity": 3.0,
              "max_complexity": 10,
              "total_complexity": 50,
              "coverage_percent": 0.5,
              "uncovered_lines": 50,
              "function_scores": [],
              "god_object_analysis": null,
              "god_object_type": null,
              "file_type": null
            },
            "score": 50.0,
            "priority_rank": 1,
            "recommendation": "Test",
            "impact": {
              "complexity_reduction": 10.0,
              "maintainability_improvement": 5.0,
              "test_effort": 2.0
            }
          }
        }"#;

        let result: Result<DebtItem, _> = serde_json::from_str(json);
        if let Err(e) = &result {
            eprintln!("Deserialization error: {}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to deserialize real JSON: {:?}",
            result.err()
        );

        match result.unwrap() {
            DebtItem::File(f) => {
                assert_eq!(f.score, 50.0);
                assert_eq!(f.metrics.total_lines, 100);
            }
            DebtItem::Function(_) => panic!("Deserialized as wrong variant!"),
        }
    }

    #[test]
    fn test_debt_density_calculation_formula() {
        // Test the formula: (total_debt_score / total_lines_of_code) * 1000

        // Case 1: 100 debt score, 1000 LOC = 100.0 density
        let density1 = (100.0 / 1000.0) * 1000.0;
        assert_eq!(density1, 100.0);

        // Case 2: 80 debt score, 250 LOC = 320.0 density
        let density2 = (80.0 / 250.0) * 1000.0;
        assert_eq!(density2, 320.0);

        // Case 3: 5000 debt score, 50000 LOC = 100.0 density
        let density3 = (5000.0 / 50000.0) * 1000.0;
        assert_eq!(density3, 100.0);
    }

    #[test]
    fn test_debt_density_zero_lines() {
        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);
        analysis.calculate_total_impact();

        // No items, should have 0 density
        assert_eq!(analysis.total_debt_score, 0.0);
        assert_eq!(analysis.total_lines_of_code, 0);
        assert_eq!(analysis.debt_density, 0.0);
    }

    #[test]
    fn test_debt_density_scale_independence() {
        // Verify that projects with proportional debt/LOC have same density

        // Small project
        let density_small = (50.0 / 500.0) * 1000.0;

        // Large project (10x larger, 10x more debt)
        let density_large = (500.0 / 5000.0) * 1000.0;

        // Should have same density
        assert_eq!(density_small, 100.0);
        assert_eq!(density_large, 100.0);
        assert_eq!(density_small, density_large);
    }

    #[test]
    fn test_debt_density_example_values() {
        // Test real-world example values

        // Clean small project
        let clean_small = (250.0 / 5000.0) * 1000.0;
        assert_eq!(clean_small, 50.0);

        // Messy small project
        let messy_small = (750.0 / 5000.0) * 1000.0;
        assert_eq!(messy_small, 150.0);

        // Clean large project
        let clean_large = (5000.0 / 100000.0) * 1000.0;
        assert_eq!(clean_large, 50.0);

        // Messy large project
        let messy_large = (15000.0 / 100000.0) * 1000.0;
        assert_eq!(messy_large, 150.0);
    }

    #[test]
    fn test_unified_analysis_initializes_density_fields() {
        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        // Check fields are initialized
        assert_eq!(analysis.debt_density, 0.0);
        assert_eq!(analysis.total_lines_of_code, 0);
    }

    // Helper function to create test items
    fn create_test_item(
        debt_type: DebtType,
        cyclomatic: u32,
        cognitive: u32,
        score: f64,
    ) -> UnifiedDebtItem {
        use semantic_classifier::FunctionRole;

        UnifiedDebtItem {
            location: unified_scorer::Location {
                file: "test.rs".into(),
                function: "test_fn".into(),
                line: 1,
            },
            debt_type,
            unified_score: unified_scorer::UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 10.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
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
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
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
            function_length: 10,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
        }
    }

    #[test]
    fn test_filter_below_cyclomatic_threshold() {
        // Set minimum cyclomatic complexity threshold to 3
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create item with cyclomatic=2 (below threshold of 3)
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 2,
                cognitive: 10,
            },
            2,    // cyclomatic
            10,   // cognitive
            15.0, // score
        );

        analysis.add_item(item);

        // Should be filtered (2 < 3)
        assert_eq!(analysis.items.len(), 0);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_filter_below_cognitive_threshold() {
        // Set minimum cognitive complexity threshold to 5
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "0");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create item with cognitive=4 (below threshold of 5)
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 4,
            },
            10,   // cyclomatic
            4,    // cognitive - below threshold
            15.0, // score
        );

        analysis.add_item(item);

        // Should be filtered (4 < 5)
        assert_eq!(analysis.items.len(), 0);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_keep_at_threshold() {
        // Set thresholds
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create item with complexities exactly at thresholds
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 3,
                cognitive: 5,
            },
            3,    // cyclomatic - at threshold
            5,    // cognitive - at threshold
            15.0, // score
        );

        analysis.add_item(item);

        // Should be kept (3 >= 3 and 5 >= 5)
        assert_eq!(analysis.items.len(), 1);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_untested_trivial_function_filtered() {
        // Set minimum cyclomatic complexity threshold to 3
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "0");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create trivial function with 0% coverage (high coverage_factor)
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 1,
                cognitive: 0,
            },
            1,    // cyclomatic - trivial
            0,    // cognitive - trivial
            17.5, // high score due to coverage gap
        );

        analysis.add_item(item);

        // Should be filtered despite 0% coverage and high score
        // The bug was that this was NOT filtered
        assert_eq!(analysis.items.len(), 0);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_test_items_exempt_from_filtering() {
        // Set high thresholds
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "10");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "20");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create test-related item with low complexity
        let item = create_test_item(
            DebtType::TestComplexityHotspot {
                cyclomatic: 1,
                cognitive: 0,
                threshold: 5,
            },
            1,    // cyclomatic - below threshold
            0,    // cognitive - below threshold
            15.0, // score
        );

        analysis.add_item(item);

        // Should NOT be filtered (test items exempt)
        assert_eq!(analysis.items.len(), 1);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_both_thresholds_must_be_satisfied() {
        // Set both thresholds
        std::env::set_var("DEBTMAP_MIN_CYCLOMATIC", "3");
        std::env::set_var("DEBTMAP_MIN_COGNITIVE", "5");

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create item that meets cyclomatic but not cognitive
        let item1 = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 5,
                cognitive: 3,
            },
            5,    // cyclomatic - above threshold
            3,    // cognitive - below threshold
            15.0, // score
        );

        analysis.add_item(item1);
        // Should be filtered (cognitive 3 < 5)
        assert_eq!(analysis.items.len(), 0);

        // Create item that meets cognitive but not cyclomatic
        let item2 = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 2,
                cognitive: 10,
            },
            2,    // cyclomatic - below threshold
            10,   // cognitive - above threshold
            15.0, // score
        );

        analysis.add_item(item2);
        // Should be filtered (cyclomatic 2 < 3)
        assert_eq!(analysis.items.len(), 0);

        // Create item that meets both thresholds
        let item3 = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 3,
                cognitive: 5,
            },
            3,    // cyclomatic - at threshold
            5,    // cognitive - at threshold
            15.0, // score
        );

        analysis.add_item(item3);
        // Should be kept (both thresholds satisfied)
        assert_eq!(analysis.items.len(), 1);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_CYCLOMATIC");
        std::env::remove_var("DEBTMAP_MIN_COGNITIVE");
    }

    #[test]
    fn test_filter_by_score_threshold() {
        use crate::priority::call_graph::CallGraph;

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Create items with different scores
        // Use higher complexity to avoid T4 classification (T4 threshold is complexity < 10)
        let high_score_item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 15, // High complexity - won't be T4
                cognitive: 20,
            },
            15,
            20,
            10.0, // High score - should be kept
        );

        let low_score_item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 12, // Moderate complexity - won't be T4
                cognitive: 15,
            },
            12,
            15,
            2.0, // Low score - should be filtered by score
        );

        let mid_score_item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.3,
                cyclomatic: 11, // Moderate complexity - won't be T4
                cognitive: 12,
            },
            11,
            12,
            5.0, // Mid score
        );

        analysis.items.push_back(high_score_item);
        analysis.items.push_back(low_score_item);
        analysis.items.push_back(mid_score_item);

        // Calculate totals before filtering
        analysis.calculate_total_impact();
        let original_score = analysis.total_debt_score;
        assert_eq!(original_score, 17.0); // 10.0 + 2.0 + 5.0

        // Filter by threshold of 3.0
        let filtered = analysis.filter_by_score_threshold(3.0);

        // Should keep items with score >= 3.0 (high_score and mid_score)
        // T4 items are also filtered out by default, but our items have high complexity so they won't be T4
        assert_eq!(filtered.items.len(), 2);
        assert_eq!(filtered.total_debt_score, 15.0); // 10.0 + 5.0

        // Filter by higher threshold of 6.0
        let filtered_high = analysis.filter_by_score_threshold(6.0);

        // Should only keep high_score item
        assert_eq!(filtered_high.items.len(), 1);
        assert_eq!(filtered_high.total_debt_score, 10.0);
    }

    #[test]
    fn test_filter_by_score_threshold_recalculates_density() {
        use crate::priority::call_graph::CallGraph;

        let call_graph = CallGraph::new();
        let mut analysis = UnifiedAnalysis::new(call_graph);

        // Add items with higher complexity to avoid T4 classification
        let item1 = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 15, // High complexity - won't be T4
                cognitive: 20,
            },
            15,
            20,
            10.0,
        );

        let item2 = create_test_item(
            DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 12, // Moderate complexity - won't be T4
                cognitive: 15,
            },
            12,
            15,
            2.0,
        );

        analysis.items.push_back(item1);
        analysis.items.push_back(item2);

        // Calculate totals and manually set LOC for density calculation
        analysis.calculate_total_impact();
        analysis.total_lines_of_code = 1000;

        // Manually calculate density since calculate_total_impact sets LOC to 0 in test env
        analysis.debt_density = (analysis.total_debt_score / 1000.0) * 1000.0;

        // Original debt score should be 12.0 (10.0 + 2.0)
        assert_eq!(analysis.total_debt_score, 12.0);
        // Original density: (12.0 / 1000) * 1000 = 12.0 per 1K LOC
        assert_eq!(analysis.debt_density, 12.0);

        // Filter to keep only items with score >= 5.0
        let filtered = analysis.filter_by_score_threshold(5.0);

        // Should only keep item1 with score 10.0
        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.total_debt_score, 10.0);
        // New density: (10.0 / 1000) * 1000 = 10.0 per 1K LOC
        assert_eq!(filtered.debt_density, 10.0);
    }
}

#[cfg(test)]
mod category_tests;

pub mod call_graph;
pub mod coverage_propagation;
pub mod debt_aggregator;
pub mod external_api_detector;
pub mod file_metrics;
pub mod formatter;
pub mod formatter_markdown;
pub mod parallel_call_graph;
pub mod score_formatter;
pub mod scoring;
pub mod semantic_classifier;
pub mod unified_scorer;

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
pub use file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
pub use formatter::{format_priorities, OutputFormat};
pub use formatter_markdown::{
    format_priorities_categorical_markdown, format_priorities_markdown,
    format_priorities_tiered_markdown,
};
pub use semantic_classifier::{classify_function_role, FunctionRole};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub related_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtType {
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
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
        responsibility_count: u32,
        complexity_score: f64,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DebtCategory {
    Architecture,
    Testing,
    Performance,
    CodeQuality,
}

impl DebtCategory {
    pub fn from_debt_type(debt_type: &DebtType) -> Self {
        match debt_type {
            // Architecture Issues
            DebtType::GodObject { .. } => DebtCategory::Architecture,
            DebtType::FeatureEnvy { .. } => DebtCategory::Architecture,
            DebtType::PrimitiveObsession { .. } => DebtCategory::Architecture,

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
            DebtCategory::Architecture => "🏗️",
            DebtCategory::Testing => "🧪",
            DebtCategory::Performance => "⚡",
            DebtCategory::CodeQuality => "📊",
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtItem {
    Function(Box<UnifiedDebtItem>),
    File(Box<FileDebtItem>),
}

impl DebtItem {
    pub fn score(&self) -> f64 {
        match self {
            DebtItem::Function(item) => item.unified_score.final_score,
            DebtItem::File(item) => item.score,
        }
    }

    pub fn display_type(&self) -> &str {
        match self {
            DebtItem::Function(_) => "FUNCTION",
            DebtItem::File(_) => "FILE",
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
            Tier::Critical => "🚨 CRITICAL - Immediate Action Required",
            Tier::High => "⚠️ HIGH - Current Sprint Priority",
            Tier::Moderate => "📊 MODERATE - Next Sprint Planning",
            Tier::Low => "📝 LOW - Backlog Consideration",
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
        }
    }

    pub fn add_file_item(&mut self, item: FileDebtItem) {
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();

        // Filter out items below minimum thresholds
        if item.score < min_score {
            return;
        }

        // Check for duplicates before adding
        let is_duplicate = self
            .file_items
            .iter()
            .any(|existing| existing.metrics.path == item.metrics.path);

        if !is_duplicate {
            self.file_items.push_back(item);
        }
    }

    pub fn add_item(&mut self, item: UnifiedDebtItem) {
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();
        let min_cyclomatic = crate::config::get_minimum_cyclomatic_complexity();
        let min_cognitive = crate::config::get_minimum_cognitive_complexity();
        let min_risk = crate::config::get_minimum_risk_score();

        // Filter out items below minimum thresholds
        if item.unified_score.final_score < min_score {
            return;
        }

        // Check risk score threshold for Risk debt types
        if let DebtType::Risk { risk_score, .. } = &item.debt_type {
            if *risk_score < min_risk {
                return;
            }
        }

        // For non-test items, also check complexity thresholds
        // This helps filter out trivial functions that aren't really debt
        if !matches!(
            item.debt_type,
            DebtType::TestComplexityHotspot { .. }
                | DebtType::TestTodo { .. }
                | DebtType::TestDuplication { .. }
        ) && item.cyclomatic_complexity <= min_cyclomatic
            && item.cognitive_complexity <= min_cognitive
        {
            // Skip trivial functions unless they have other significant issues
            // (like being completely untested critical paths)
            if item.unified_score.coverage_factor < 8.0 {
                return;
            }
        }

        // Check for duplicates before adding
        // Two items are considered duplicates if they have the same location and debt type
        let is_duplicate = self.items.iter().any(|existing| {
            existing.location.file == item.location.file
                && existing.location.line == item.location.line
                && std::mem::discriminant(&existing.debt_type)
                    == std::mem::discriminant(&item.debt_type)
        });

        if !is_duplicate {
            self.items.push_back(item);
        }
    }

    pub fn sort_by_priority(&mut self) {
        // Sort function items
        let mut items_vec: Vec<UnifiedDebtItem> = self.items.iter().cloned().collect();
        items_vec.sort_by(|a, b| {
            b.unified_score
                .final_score
                .partial_cmp(&a.unified_score.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.items = items_vec.into_iter().collect();

        // Sort file items
        let mut file_items_vec: Vec<FileDebtItem> = self.file_items.iter().cloned().collect();
        file_items_vec.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.file_items = file_items_vec.into_iter().collect();
    }

    pub fn calculate_total_impact(&mut self) {
        let mut coverage_improvement = 0.0;
        let mut lines_reduction = 0;
        let mut complexity_reduction = 0.0;
        let mut risk_reduction = 0.0;
        let mut _functions_to_test = 0;
        let mut total_debt_score = 0.0;
        let mut total_lines_of_code = 0;

        for item in &self.items {
            // Sum up all final scores as the total debt score
            total_debt_score += item.unified_score.final_score;

            // Track lines of code from function-level items
            total_lines_of_code += item.function_length;

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
            total_lines_of_code += file_item.metrics.total_lines;

            // File-level impacts are typically larger
            complexity_reduction += file_item.impact.complexity_reduction;
            lines_reduction += (file_item.metrics.total_lines / 10) as u32; // Rough estimate of reduction

            // Coverage improvement from fixing file-level issues
            if file_item.metrics.coverage_percent < 0.8 {
                coverage_improvement += (0.8 - file_item.metrics.coverage_percent) * 10.0;
            }
        }

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

    pub fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        self.items.iter().take(n).cloned().collect()
    }

    pub fn get_top_mixed_priorities(&self, n: usize) -> Vector<DebtItem> {
        // Combine function and file items, sorted by score
        let mut all_items: Vec<DebtItem> = Vec::new();

        // Add function items
        for item in &self.items {
            all_items.push(DebtItem::Function(Box::new(item.clone())));
        }

        // Add file items
        for item in &self.file_items {
            all_items.push(DebtItem::File(Box::new(item.clone())));
        }

        // Sort by score descending
        all_items.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top n items
        all_items.into_iter().take(n).collect()
    }

    pub fn get_bottom_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        let total_items = self.items.len();
        if total_items <= n {
            self.items.clone()
        } else {
            self.items.iter().skip(total_items - n).cloned().collect()
        }
    }

    /// Generate a tiered display of debt items grouped by priority tier
    pub fn get_tiered_display(&self, limit: usize) -> TieredDisplay {
        let all_items = self.get_top_mixed_priorities(limit);

        let mut critical_groups: Vec<DisplayGroup> = Vec::new();
        let mut high_groups: Vec<DisplayGroup> = Vec::new();
        let mut moderate_groups: Vec<DisplayGroup> = Vec::new();
        let mut low_groups: Vec<DisplayGroup> = Vec::new();

        // Group items by tier and debt type
        use std::collections::HashMap;
        let mut tier_groups: HashMap<(Tier, String), Vec<DebtItem>> = HashMap::new();

        for item in all_items {
            let tier = Tier::from_score(item.score());
            let debt_type = self.get_debt_type_key(&item);

            // Never group god objects or architectural issues
            if self.is_critical_item(&item) {
                // Add as individual group
                let group = DisplayGroup {
                    tier: tier.clone(),
                    debt_type: debt_type.clone(),
                    items: vec![item],
                    batch_action: None,
                };

                match tier {
                    Tier::Critical => critical_groups.push(group),
                    Tier::High => high_groups.push(group),
                    Tier::Moderate => moderate_groups.push(group),
                    Tier::Low => low_groups.push(group),
                }
            } else {
                // Group similar items
                tier_groups.entry((tier, debt_type)).or_default().push(item);
            }
        }

        // Create display groups for grouped items
        for ((tier, debt_type), items) in tier_groups {
            if items.is_empty() {
                continue;
            }

            let batch_action = if items.len() > 1 {
                Some(self.generate_batch_action(&debt_type, items.len()))
            } else {
                None
            };

            let group = DisplayGroup {
                tier: tier.clone(),
                debt_type,
                items,
                batch_action,
            };

            match tier {
                Tier::Critical => critical_groups.push(group),
                Tier::High => high_groups.push(group),
                Tier::Moderate => moderate_groups.push(group),
                Tier::Low => low_groups.push(group),
            }
        }

        // Sort groups within each tier by total score
        let sort_groups = |groups: &mut Vec<DisplayGroup>| {
            groups.sort_by(|a, b| {
                let a_score: f64 = a.items.iter().map(|i| i.score()).sum();
                let b_score: f64 = b.items.iter().map(|i| i.score()).sum();
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        };

        sort_groups(&mut critical_groups);
        sort_groups(&mut high_groups);
        sort_groups(&mut moderate_groups);
        sort_groups(&mut low_groups);

        TieredDisplay {
            critical: critical_groups,
            high: high_groups,
            moderate: moderate_groups,
            low: low_groups,
        }
    }

    fn get_debt_type_key(&self, item: &DebtItem) -> String {
        match item {
            DebtItem::Function(func) => match &func.debt_type {
                DebtType::TestingGap { .. } => "Untested Complex Functions".to_string(),
                DebtType::ComplexityHotspot { .. } => "High Complexity Functions".to_string(),
                DebtType::DeadCode { .. } => "Dead Code".to_string(),
                DebtType::Duplication { .. } => "Code Duplication".to_string(),
                DebtType::Risk { .. } => "High Risk Functions".to_string(),
                DebtType::GodObject { .. } => "God Object".to_string(),
                DebtType::FeatureEnvy { .. } => "Feature Envy".to_string(),
                DebtType::TestComplexityHotspot { .. } => "Complex Test Functions".to_string(),
                _ => "Technical Debt".to_string(),
            },
            DebtItem::File(file) => {
                if file.metrics.god_object_indicators.is_god_object {
                    "God Object File".to_string()
                } else if file.metrics.total_lines > 1000 {
                    "Large File".to_string()
                } else if file.metrics.avg_complexity > 10.0 {
                    "Complex File".to_string()
                } else {
                    "File-Level Debt".to_string()
                }
            }
        }
    }

    fn is_critical_item(&self, item: &DebtItem) -> bool {
        match item {
            DebtItem::Function(func) => {
                matches!(func.debt_type, DebtType::GodObject { .. })
                    || func.unified_score.final_score >= 95.0
            }
            DebtItem::File(file) => {
                file.metrics.god_object_indicators.is_god_object
                    || file.metrics.total_lines > 2000
                    || file.score >= 95.0
            }
        }
    }

    fn generate_batch_action(&self, debt_type: &str, count: usize) -> String {
        match debt_type {
            "Untested Complex Functions" => {
                format!("Add test coverage for {} complex functions", count)
            }
            "High Complexity Functions" => {
                format!("Refactor {} complex functions into smaller units", count)
            }
            "Dead Code" => format!("Remove {} unused functions", count),
            "Code Duplication" => format!(
                "Extract {} duplicated code blocks into shared utilities",
                count
            ),
            "Complex Test Functions" => format!("Simplify {} complex test functions", count),
            _ => format!("Address {} {} items", count, debt_type.to_lowercase()),
        }
    }

    /// Get a reference to the data flow graph
    pub fn data_flow_graph(&self) -> &crate::data_flow::DataFlowGraph {
        &self.data_flow_graph
    }

    /// Get a mutable reference to the data flow graph
    pub fn data_flow_graph_mut(&mut self) -> &mut crate::data_flow::DataFlowGraph {
        &mut self.data_flow_graph
    }

    /// Populate the data flow graph with purity analysis data from function metrics
    pub fn populate_purity_analysis(&mut self, metrics: &[crate::core::FunctionMetrics]) {
        use crate::data_flow::PurityInfo;
        use crate::priority::call_graph::FunctionId;

        for metric in metrics {
            let func_id = FunctionId {
                file: metric.file.clone(),
                name: metric.name.clone(),
                line: metric.line,
            };

            let purity_info = PurityInfo {
                is_pure: metric.is_pure.unwrap_or(false),
                confidence: metric.purity_confidence.unwrap_or(0.0),
                impurity_reasons: if !metric.is_pure.unwrap_or(false) {
                    vec!["Function may have side effects".to_string()]
                } else {
                    vec![]
                },
            };

            self.data_flow_graph.set_purity_info(func_id, purity_info);
        }
    }

    /// Add I/O operation detected during analysis
    pub fn add_io_operation(
        &mut self,
        func_id: call_graph::FunctionId,
        operation: crate::data_flow::IoOperation,
    ) {
        self.data_flow_graph.add_io_operation(func_id, operation);
    }

    /// Add variable dependencies for a function
    pub fn add_variable_dependencies(
        &mut self,
        func_id: call_graph::FunctionId,
        variables: std::collections::HashSet<String>,
    ) {
        self.data_flow_graph
            .add_variable_dependencies(func_id, variables);
    }

    /// Generate a categorized view of debt items
    pub fn get_categorized_debt(&self, limit: usize) -> CategorizedDebt {
        let all_items = self.get_top_mixed_priorities(limit);
        let mut categories: BTreeMap<DebtCategory, Vec<DebtItem>> = BTreeMap::new();

        // Categorize all items
        for item in all_items {
            let category = match &item {
                DebtItem::Function(func) => DebtCategory::from_debt_type(&func.debt_type),
                DebtItem::File(file) => {
                    // File-level items typically indicate architectural issues
                    if file.metrics.god_object_indicators.is_god_object {
                        DebtCategory::Architecture
                    } else if file.metrics.coverage_percent < 0.5 {
                        DebtCategory::Testing
                    } else {
                        DebtCategory::CodeQuality
                    }
                }
            };

            categories.entry(category).or_default().push(item);
        }

        // Create category summaries
        let mut category_summaries = BTreeMap::new();
        for (category, items) in categories {
            if items.is_empty() {
                continue;
            }

            let total_score: f64 = items.iter().map(|item| item.score()).sum();
            let item_count = items.len();
            let average_severity = total_score / item_count as f64;

            // Estimate effort based on category and average severity
            let effort_per_item = match category {
                DebtCategory::Architecture => {
                    if average_severity >= 90.0 {
                        16
                    }
                    // 2 days
                    else if average_severity >= 70.0 {
                        8
                    }
                    // 1 day
                    else {
                        4
                    } // Half day
                }
                DebtCategory::Testing => {
                    if average_severity >= 70.0 {
                        4
                    } else {
                        2
                    }
                }
                DebtCategory::Performance => {
                    if average_severity >= 70.0 {
                        8
                    } else {
                        4
                    }
                }
                DebtCategory::CodeQuality => {
                    if average_severity >= 70.0 {
                        4
                    } else {
                        2
                    }
                }
            };

            let estimated_effort_hours = (item_count as u32) * effort_per_item;

            // Take top 5 items per category
            let top_items = items.into_iter().take(5).collect();

            let summary = CategorySummary {
                category: category.clone(),
                total_score,
                item_count,
                estimated_effort_hours,
                average_severity,
                top_items,
            };

            category_summaries.insert(category, summary);
        }

        // Identify cross-category dependencies
        let cross_dependencies = self.identify_cross_category_dependencies(&category_summaries);

        CategorizedDebt {
            categories: category_summaries,
            cross_category_dependencies: cross_dependencies,
        }
    }

    fn identify_cross_category_dependencies(
        &self,
        categories: &BTreeMap<DebtCategory, CategorySummary>,
    ) -> Vec<CrossCategoryDependency> {
        let mut dependencies = Vec::new();

        // Architecture issues often block effective testing
        if categories.contains_key(&DebtCategory::Architecture)
            && categories.contains_key(&DebtCategory::Testing)
        {
            if let Some(arch) = categories.get(&DebtCategory::Architecture) {
                // Check for god objects which are hard to test
                let has_god_objects = arch.top_items.iter().any(|item| match item {
                    DebtItem::Function(func) => {
                        matches!(func.debt_type, DebtType::GodObject { .. })
                    }
                    DebtItem::File(file) => file.metrics.god_object_indicators.is_god_object,
                });

                if has_god_objects {
                    dependencies.push(CrossCategoryDependency {
                        source_category: DebtCategory::Architecture,
                        target_category: DebtCategory::Testing,
                        description: "God objects and complex architectures make testing difficult. Refactor architecture first to enable effective testing.".to_string(),
                        impact_level: ImpactLevel::High,
                    });
                }
            }
        }

        // Performance issues may require architectural changes
        if categories.contains_key(&DebtCategory::Performance)
            && categories.contains_key(&DebtCategory::Architecture)
        {
            if let Some(perf) = categories.get(&DebtCategory::Performance) {
                // Check for async misuse which often requires architectural changes
                let has_async_issues = perf.top_items.iter().any(|item| match item {
                    DebtItem::Function(func) => {
                        matches!(func.debt_type, DebtType::AsyncMisuse { .. })
                    }
                    _ => false,
                });

                if has_async_issues {
                    dependencies.push(CrossCategoryDependency {
                        source_category: DebtCategory::Performance,
                        target_category: DebtCategory::Architecture,
                        description: "Async performance issues may require architectural refactoring for proper async/await patterns.".to_string(),
                        impact_level: ImpactLevel::Medium,
                    });
                }
            }
        }

        // Complex code affects testability
        if categories.contains_key(&DebtCategory::CodeQuality)
            && categories.contains_key(&DebtCategory::Testing)
        {
            if let Some(quality) = categories.get(&DebtCategory::CodeQuality) {
                if quality.average_severity >= 70.0 {
                    dependencies.push(CrossCategoryDependency {
                        source_category: DebtCategory::CodeQuality,
                        target_category: DebtCategory::Testing,
                        description: "High complexity code is harder to test effectively. Simplify code first for better test coverage.".to_string(),
                        impact_level: ImpactLevel::Medium,
                    });
                }
            }
        }

        dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

#[cfg(test)]
mod category_tests;

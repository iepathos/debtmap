//! Unified view data model for debt display.
//!
//! This module provides the canonical data model that all output formats
//! consume. Following Stillwater's "still pond" principle, this is pure
//! data with no I/O operations.
//!
//! # Architecture
//!
//! All output formats (TUI, terminal, JSON, markdown) consume `PreparedDebtView`,
//! ensuring consistent results across all display modes.
//!
//! ```text
//! UnifiedAnalysis → prepare_view() → PreparedDebtView → OutputFormatter
//!                        ↑                                    ↓
//!                   ViewConfig                          Formatted Output
//! ```
//!
//! # Key Types
//!
//! - [`ViewItem`] - Unified wrapper for function/file items
//! - [`LocationGroup`] - Pre-computed groups for display
//! - [`ViewSummary`] - Statistics and filter metrics
//! - [`PreparedDebtView`] - The canonical view model

use crate::priority::{
    classification::Severity,
    file_metrics::FileDebtItem,
    tiers::RecommendationTier,
    unified_scorer::{Location, UnifiedDebtItem},
    DebtCategory,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unified item type wrapping both function and file debt items.
///
/// This enables heterogeneous collections and consistent interfaces
/// across all output formats.
///
/// # Examples
///
/// ```
/// use debtmap::priority::view::ViewItem;
///
/// // Access common properties regardless of item type
/// fn print_item(item: &ViewItem) {
///     println!("Score: {:.1}, Severity: {:?}", item.score(), item.severity());
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewItem {
    /// Function-level debt item
    Function(Box<UnifiedDebtItem>),
    /// File-level debt item (god objects, large files)
    File(Box<FileDebtItem>),
}

impl ViewItem {
    /// Returns the debt score for this item.
    pub fn score(&self) -> f64 {
        match self {
            ViewItem::Function(item) => item.unified_score.final_score.value(),
            ViewItem::File(item) => item.score,
        }
    }

    /// Returns the location of this item.
    pub fn location(&self) -> ItemLocation {
        match self {
            ViewItem::Function(item) => ItemLocation {
                file: item.location.file.clone(),
                function: Some(item.location.function.clone()),
                line: Some(item.location.line),
            },
            ViewItem::File(item) => ItemLocation {
                file: item.metrics.path.clone(),
                function: None,
                line: None,
            },
        }
    }

    /// Returns the severity classification for this item.
    pub fn severity(&self) -> Severity {
        Severity::from_score_100(self.score())
    }

    /// Returns the recommendation tier for this item.
    pub fn tier(&self) -> Option<RecommendationTier> {
        match self {
            ViewItem::Function(item) => item.tier,
            ViewItem::File(_) => Some(RecommendationTier::T1CriticalArchitecture), // File items are critical
        }
    }

    /// Returns the debt category for this item.
    pub fn category(&self) -> DebtCategory {
        match self {
            ViewItem::Function(item) => DebtCategory::from_debt_type(&item.debt_type),
            ViewItem::File(_) => DebtCategory::Architecture, // File items are architectural
        }
    }

    /// Returns display type label.
    pub fn display_type(&self) -> &'static str {
        match self {
            ViewItem::Function(_) => "FUNCTION",
            ViewItem::File(_) => "FILE",
        }
    }

    /// Returns the inner function item if this is a function variant.
    pub fn as_function(&self) -> Option<&UnifiedDebtItem> {
        match self {
            ViewItem::Function(item) => Some(item),
            ViewItem::File(_) => None,
        }
    }

    /// Returns the inner file item if this is a file variant.
    pub fn as_file(&self) -> Option<&FileDebtItem> {
        match self {
            ViewItem::Function(_) => None,
            ViewItem::File(item) => Some(item),
        }
    }
}

/// Location information for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ItemLocation {
    pub file: PathBuf,
    pub function: Option<String>,
    pub line: Option<usize>,
}

impl ItemLocation {
    /// Returns grouping key for location-based grouping.
    pub fn group_key(&self) -> (PathBuf, String, usize) {
        (
            self.file.clone(),
            self.function.clone().unwrap_or_default(),
            self.line.unwrap_or(0),
        )
    }

    /// Creates a location from function Location type.
    pub fn from_function_location(loc: &Location) -> Self {
        Self {
            file: loc.file.clone(),
            function: Some(loc.function.clone()),
            line: Some(loc.line),
        }
    }
}

/// Pre-computed group of items at the same location.
///
/// Used for TUI grouped view and potential grouped display in other formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationGroup {
    /// Representative location for this group
    pub location: ItemLocation,
    /// All items at this location
    pub items: Vec<ViewItem>,
    /// Combined score (sum of all item scores)
    pub combined_score: f64,
    /// Highest severity among items
    pub max_severity: Severity,
    /// Number of items in group
    pub item_count: usize,
}

impl LocationGroup {
    /// Creates a new group from items at the same location.
    pub fn new(location: ItemLocation, items: Vec<ViewItem>) -> Self {
        let combined_score = items.iter().map(|i| i.score()).sum();
        let max_severity = items
            .iter()
            .map(|i| i.severity())
            .max()
            .unwrap_or(Severity::Low);
        let item_count = items.len();

        Self {
            location,
            items,
            combined_score,
            max_severity,
            item_count,
        }
    }

    /// Returns true if this group contains any items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Summary statistics for the view.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViewSummary {
    /// Total items before filtering
    pub total_items_before_filter: usize,
    /// Total items after filtering
    pub total_items_after_filter: usize,
    /// Items filtered by T4 tier
    pub filtered_by_tier: usize,
    /// Items filtered by score threshold
    pub filtered_by_score: usize,
    /// Total debt score (sum of all item scores)
    pub total_debt_score: f64,
    /// Score distribution by severity
    pub score_distribution: ScoreDistribution,
    /// Items by category
    pub category_counts: CategoryCounts,
    /// Total lines of code analyzed
    pub total_lines_of_code: usize,
    /// Debt density per 1K LOC
    pub debt_density: f64,
    /// Overall coverage if available
    pub overall_coverage: Option<f64>,
}

/// Distribution of items by severity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreDistribution {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl ScoreDistribution {
    /// Total count across all severity levels.
    pub fn total(&self) -> usize {
        self.critical + self.high + self.medium + self.low
    }
}

/// Count of items by category.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryCounts {
    pub architecture: usize,
    pub testing: usize,
    pub performance: usize,
    pub code_quality: usize,
}

impl CategoryCounts {
    /// Total count across all categories.
    pub fn total(&self) -> usize {
        self.architecture + self.testing + self.performance + self.code_quality
    }
}

/// The canonical view model for all output formats.
///
/// This is the **single source of truth** for displaying debt items.
/// All output formats (TUI, terminal, JSON, markdown) consume this
/// same data structure, ensuring consistent results.
///
/// # Stillwater Pattern
///
/// This follows the "still pond" model - a pure data structure with
/// no I/O operations. The "flowing water" (I/O) happens in the output
/// formatters that consume this model.
///
/// # Examples
///
/// ```
/// use debtmap::priority::view::{PreparedDebtView, ViewConfig};
///
/// // All output formats use the same view
/// fn format_output(view: &PreparedDebtView) {
///     println!("Total items: {}", view.len());
///     println!("Total groups: {}", view.group_count());
///     for item in view.ungrouped_items() {
///         println!("  {:.1}: {:?}", item.score(), item.location());
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedDebtView {
    /// All items sorted by score (highest first)
    pub items: Vec<ViewItem>,
    /// Pre-computed groups by location
    pub groups: Vec<LocationGroup>,
    /// Summary statistics
    pub summary: ViewSummary,
    /// Configuration used to create this view
    pub config: ViewConfig,
}

/// Configuration for view preparation.
///
/// This captures all the parameters that affect how items are
/// filtered, sorted, and grouped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    /// Minimum score threshold (items below are filtered)
    pub min_score_threshold: f64,
    /// Whether to exclude T4 maintenance tier items
    pub exclude_t4_maintenance: bool,
    /// Maximum number of items (None = unlimited)
    pub limit: Option<usize>,
    /// Sort criteria
    pub sort_by: SortCriteria,
    /// Whether to compute groups
    pub compute_groups: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 3.0,
            exclude_t4_maintenance: true,
            limit: None,
            sort_by: SortCriteria::Score,
            compute_groups: true,
        }
    }
}

impl ViewConfig {
    /// Creates a config that shows all items (no filtering).
    pub fn show_all() -> Self {
        Self {
            min_score_threshold: 0.0,
            exclude_t4_maintenance: false,
            limit: None,
            sort_by: SortCriteria::Score,
            compute_groups: true,
        }
    }

    /// Creates a config for TUI display with grouping.
    pub fn for_tui() -> Self {
        Self {
            min_score_threshold: 3.0,
            exclude_t4_maintenance: true,
            limit: None,
            sort_by: SortCriteria::Score,
            compute_groups: true,
        }
    }

    /// Creates a config for terminal/CLI display.
    pub fn for_terminal(limit: Option<usize>) -> Self {
        Self {
            min_score_threshold: 3.0,
            exclude_t4_maintenance: true,
            limit,
            sort_by: SortCriteria::Score,
            compute_groups: false,
        }
    }

    /// Creates a config for JSON export.
    pub fn for_json() -> Self {
        Self {
            min_score_threshold: 0.0,
            exclude_t4_maintenance: false,
            limit: None,
            sort_by: SortCriteria::Score,
            compute_groups: false,
        }
    }
}

/// Sort criteria for items.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SortCriteria {
    #[default]
    Score,
    Coverage,
    Complexity,
    FilePath,
    FunctionName,
}

impl PreparedDebtView {
    /// Returns items suitable for ungrouped display.
    pub fn ungrouped_items(&self) -> &[ViewItem] {
        &self.items
    }

    /// Returns groups suitable for grouped display.
    pub fn grouped_items(&self) -> &[LocationGroup] {
        &self.groups
    }

    /// Returns whether this view has any items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns the number of groups.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Creates an empty view with default config.
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            groups: Vec::new(),
            summary: ViewSummary::default(),
            config: ViewConfig::default(),
        }
    }

    /// Creates an empty view with specified config.
    pub fn empty_with_config(config: ViewConfig) -> Self {
        Self {
            items: Vec::new(),
            groups: Vec::new(),
            summary: ViewSummary::default(),
            config,
        }
    }

    /// Returns items filtered to a specific severity level.
    pub fn items_by_severity(&self, severity: Severity) -> Vec<&ViewItem> {
        self.items
            .iter()
            .filter(|item| item.severity() == severity)
            .collect()
    }

    /// Returns items filtered to a specific category.
    pub fn items_by_category(&self, category: DebtCategory) -> Vec<&ViewItem> {
        self.items
            .iter()
            .filter(|item| item.category() == category)
            .collect()
    }

    /// Returns the top N items by score.
    pub fn top_items(&self, n: usize) -> &[ViewItem] {
        let end = n.min(self.items.len());
        &self.items[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact},
        score_types::Score0To100,
        semantic_classifier::FunctionRole,
        ActionableRecommendation, DebtType, ImpactMetrics, UnifiedScore,
    };

    fn create_test_function_item(score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_fn".into(),
                line: 10,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 15,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 2.0,
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
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add tests".into(),
                rationale: "Improve coverage".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 20.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 3,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 15,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
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
            entropy_analysis: None,
        }
    }

    fn create_test_file_item(score: f64) -> FileDebtItem {
        FileDebtItem {
            metrics: FileDebtMetrics {
                path: "test_file.rs".into(),
                total_lines: 500,
                function_count: 20,
                class_count: 1,
                avg_complexity: 8.0,
                max_complexity: 25,
                total_complexity: 160,
                coverage_percent: 0.3,
                uncovered_lines: 350,
                god_object_analysis: None,
                function_scores: vec![5.0; 20],
                god_object_type: None,
                file_type: None,
                ..Default::default()
            },
            score,
            priority_rank: 1,
            recommendation: "Split into multiple modules".into(),
            impact: FileImpact {
                complexity_reduction: 50.0,
                maintainability_improvement: 30.0,
                test_effort: 20.0,
            },
        }
    }

    #[test]
    fn test_view_item_score() {
        let func_item = create_test_function_item(50.0);
        let file_item = create_test_file_item(75.0);

        assert_eq!(ViewItem::Function(Box::new(func_item)).score(), 50.0);
        assert_eq!(ViewItem::File(Box::new(file_item)).score(), 75.0);
    }

    #[test]
    fn test_view_item_severity() {
        let critical = create_test_function_item(95.0);
        let high = create_test_function_item(60.0);
        let medium = create_test_function_item(40.0);
        let low = create_test_function_item(20.0);

        assert_eq!(
            ViewItem::Function(Box::new(critical)).severity(),
            Severity::Critical
        );
        assert_eq!(
            ViewItem::Function(Box::new(high)).severity(),
            Severity::High
        );
        assert_eq!(
            ViewItem::Function(Box::new(medium)).severity(),
            Severity::Medium
        );
        assert_eq!(ViewItem::Function(Box::new(low)).severity(), Severity::Low);
    }

    #[test]
    fn test_view_item_location() {
        let func_item = create_test_function_item(50.0);
        let file_item = create_test_file_item(75.0);

        let func_loc = ViewItem::Function(Box::new(func_item)).location();
        assert_eq!(func_loc.file, PathBuf::from("test.rs"));
        assert_eq!(func_loc.function, Some("test_fn".to_string()));
        assert_eq!(func_loc.line, Some(10));

        let file_loc = ViewItem::File(Box::new(file_item)).location();
        assert_eq!(file_loc.file, PathBuf::from("test_file.rs"));
        assert_eq!(file_loc.function, None);
        assert_eq!(file_loc.line, None);
    }

    #[test]
    fn test_location_group_combined_score() {
        let items = vec![
            ViewItem::Function(Box::new(create_test_function_item(30.0))),
            ViewItem::Function(Box::new(create_test_function_item(20.0))),
            ViewItem::Function(Box::new(create_test_function_item(10.0))),
        ];
        let location = ItemLocation {
            file: PathBuf::from("test.rs"),
            function: Some("test_fn".to_string()),
            line: Some(10),
        };

        let group = LocationGroup::new(location, items);

        assert_eq!(group.combined_score, 60.0);
        assert_eq!(group.item_count, 3);
    }

    #[test]
    fn test_location_group_max_severity() {
        let items = vec![
            ViewItem::Function(Box::new(create_test_function_item(25.0))), // Low
            ViewItem::Function(Box::new(create_test_function_item(85.0))), // Critical
            ViewItem::Function(Box::new(create_test_function_item(45.0))), // Medium
        ];
        let location = ItemLocation {
            file: PathBuf::from("test.rs"),
            function: Some("test_fn".to_string()),
            line: Some(10),
        };

        let group = LocationGroup::new(location, items);

        assert_eq!(group.max_severity, Severity::Critical);
    }

    #[test]
    fn test_view_config_default() {
        let config = ViewConfig::default();

        assert_eq!(config.min_score_threshold, 3.0);
        assert!(config.exclude_t4_maintenance);
        assert!(config.limit.is_none());
        assert_eq!(config.sort_by, SortCriteria::Score);
        assert!(config.compute_groups);
    }

    #[test]
    fn test_view_config_variants() {
        let tui = ViewConfig::for_tui();
        assert!(tui.compute_groups);
        assert!(tui.exclude_t4_maintenance);

        let terminal = ViewConfig::for_terminal(Some(10));
        assert!(!terminal.compute_groups);
        assert_eq!(terminal.limit, Some(10));

        let json = ViewConfig::for_json();
        assert!(!json.exclude_t4_maintenance);
        assert_eq!(json.min_score_threshold, 0.0);

        let all = ViewConfig::show_all();
        assert_eq!(all.min_score_threshold, 0.0);
        assert!(!all.exclude_t4_maintenance);
    }

    #[test]
    fn test_prepared_debt_view_accessors() {
        let view = PreparedDebtView {
            items: vec![ViewItem::Function(Box::new(create_test_function_item(
                50.0,
            )))],
            groups: vec![],
            summary: ViewSummary::default(),
            config: ViewConfig::default(),
        };

        assert!(!view.is_empty());
        assert_eq!(view.len(), 1);
        assert_eq!(view.group_count(), 0);
    }

    #[test]
    fn test_prepared_debt_view_empty() {
        let view = PreparedDebtView::empty();

        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
        assert_eq!(view.group_count(), 0);
    }

    #[test]
    fn test_prepared_debt_view_top_items() {
        let view = PreparedDebtView {
            items: vec![
                ViewItem::Function(Box::new(create_test_function_item(90.0))),
                ViewItem::Function(Box::new(create_test_function_item(70.0))),
                ViewItem::Function(Box::new(create_test_function_item(50.0))),
                ViewItem::Function(Box::new(create_test_function_item(30.0))),
            ],
            groups: vec![],
            summary: ViewSummary::default(),
            config: ViewConfig::default(),
        };

        let top_2 = view.top_items(2);
        assert_eq!(top_2.len(), 2);
        assert_eq!(top_2[0].score(), 90.0);
        assert_eq!(top_2[1].score(), 70.0);

        // Request more than available
        let top_10 = view.top_items(10);
        assert_eq!(top_10.len(), 4);
    }

    #[test]
    fn test_view_item_display_type() {
        let func_item = ViewItem::Function(Box::new(create_test_function_item(50.0)));
        let file_item = ViewItem::File(Box::new(create_test_file_item(75.0)));

        assert_eq!(func_item.display_type(), "FUNCTION");
        assert_eq!(file_item.display_type(), "FILE");
    }

    #[test]
    fn test_view_item_category() {
        let testing_item = ViewItem::Function(Box::new(create_test_function_item(50.0)));
        let file_item = ViewItem::File(Box::new(create_test_file_item(75.0)));

        assert_eq!(testing_item.category(), DebtCategory::Testing);
        assert_eq!(file_item.category(), DebtCategory::Architecture);
    }

    #[test]
    fn test_view_item_as_accessors() {
        let func_item = ViewItem::Function(Box::new(create_test_function_item(50.0)));
        let file_item = ViewItem::File(Box::new(create_test_file_item(75.0)));

        assert!(func_item.as_function().is_some());
        assert!(func_item.as_file().is_none());

        assert!(file_item.as_function().is_none());
        assert!(file_item.as_file().is_some());
    }

    #[test]
    fn test_score_distribution_total() {
        let dist = ScoreDistribution {
            critical: 5,
            high: 10,
            medium: 15,
            low: 20,
        };

        assert_eq!(dist.total(), 50);
    }

    #[test]
    fn test_category_counts_total() {
        let counts = CategoryCounts {
            architecture: 3,
            testing: 12,
            performance: 5,
            code_quality: 8,
        };

        assert_eq!(counts.total(), 28);
    }

    #[test]
    fn test_prepared_debt_view_items_by_severity() {
        let view = PreparedDebtView {
            items: vec![
                ViewItem::Function(Box::new(create_test_function_item(85.0))), // Critical
                ViewItem::Function(Box::new(create_test_function_item(60.0))), // High
                ViewItem::Function(Box::new(create_test_function_item(55.0))), // High
                ViewItem::Function(Box::new(create_test_function_item(25.0))), // Low
            ],
            groups: vec![],
            summary: ViewSummary::default(),
            config: ViewConfig::default(),
        };

        let critical = view.items_by_severity(Severity::Critical);
        assert_eq!(critical.len(), 1);

        let high = view.items_by_severity(Severity::High);
        assert_eq!(high.len(), 2);

        let low = view.items_by_severity(Severity::Low);
        assert_eq!(low.len(), 1);
    }

    #[test]
    fn test_prepared_debt_view_json_roundtrip() {
        let view = PreparedDebtView {
            items: vec![ViewItem::Function(Box::new(create_test_function_item(
                50.0,
            )))],
            groups: vec![],
            summary: ViewSummary {
                total_items_before_filter: 10,
                total_items_after_filter: 1,
                filtered_by_tier: 5,
                filtered_by_score: 4,
                total_debt_score: 50.0,
                score_distribution: ScoreDistribution {
                    critical: 0,
                    high: 1,
                    medium: 0,
                    low: 0,
                },
                category_counts: CategoryCounts {
                    architecture: 0,
                    testing: 1,
                    performance: 0,
                    code_quality: 0,
                },
                total_lines_of_code: 1000,
                debt_density: 50.0,
                overall_coverage: Some(0.75),
            },
            config: ViewConfig::default(),
        };

        let json = serde_json::to_string(&view).unwrap();
        let deserialized: PreparedDebtView = serde_json::from_str(&json).unwrap();

        assert_eq!(view.items.len(), deserialized.items.len());
        assert_eq!(
            view.summary.total_debt_score,
            deserialized.summary.total_debt_score
        );
        assert_eq!(
            view.summary.total_items_before_filter,
            deserialized.summary.total_items_before_filter
        );
    }

    #[test]
    fn test_location_group_key() {
        let loc = ItemLocation {
            file: PathBuf::from("src/lib.rs"),
            function: Some("process".to_string()),
            line: Some(42),
        };

        let (file, func, line) = loc.group_key();
        assert_eq!(file, PathBuf::from("src/lib.rs"));
        assert_eq!(func, "process");
        assert_eq!(line, 42);
    }

    #[test]
    fn test_location_group_key_defaults() {
        let loc = ItemLocation {
            file: PathBuf::from("src/lib.rs"),
            function: None,
            line: None,
        };

        let (file, func, line) = loc.group_key();
        assert_eq!(file, PathBuf::from("src/lib.rs"));
        assert_eq!(func, "");
        assert_eq!(line, 0);
    }
}

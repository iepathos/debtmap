//! Query and data access operations for UnifiedAnalysis.
//!
//! This module provides methods for retrieving, filtering, and displaying
//! technical debt items from a UnifiedAnalysis. Operations are pure and
//! functional, creating new data structures rather than mutating.

use super::{
    CategorizedDebt, CategorySummary, CrossCategoryDependency, DebtCategory, DebtItem, DebtType,
    DisplayGroup, ImpactLevel, Tier, TieredDisplay, UnifiedAnalysis, UnifiedDebtItem,
};
use crate::priority::filtering::{ClassifiedItem, FilterConfig, FilterResult};
use crate::priority::tiers::{classify_tier, TierConfig};
use im::Vector;
use std::collections::{BTreeMap, HashMap};

/// Extension trait providing query operations for UnifiedAnalysis
pub trait UnifiedAnalysisQueries {
    /// Get the top N debt items sorted by priority
    fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem>;

    /// Get top N mixed priorities (both function and file items) using default tier config
    fn get_top_mixed_priorities(&self, n: usize) -> Vector<DebtItem>;

    /// Get top N mixed priorities with custom tier configuration
    fn get_top_mixed_priorities_tiered(
        &self,
        n: usize,
        tier_config: &TierConfig,
    ) -> Vector<DebtItem>;

    /// Get top N mixed priorities with filtering transparency (metrics exposed)
    fn get_top_mixed_priorities_with_metrics(
        &self,
        n: usize,
        tier_config: &TierConfig,
    ) -> crate::priority::filtering::FilterResult;

    /// Get the bottom N debt items (lowest priority)
    fn get_bottom_priorities(&self, n: usize) -> Vector<UnifiedDebtItem>;

    /// Generate a tiered display of debt items grouped by priority tier
    fn get_tiered_display(&self, limit: usize) -> TieredDisplay;

    /// Generate a categorized view of debt items
    fn get_categorized_debt(&self, limit: usize) -> CategorizedDebt;
}

impl UnifiedAnalysisQueries for UnifiedAnalysis {
    fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        self.items.iter().take(n).cloned().collect()
    }

    fn get_top_mixed_priorities(&self, n: usize) -> Vector<DebtItem> {
        self.get_top_mixed_priorities_tiered(n, &TierConfig::default())
    }

    fn get_top_mixed_priorities_tiered(
        &self,
        n: usize,
        tier_config: &TierConfig,
    ) -> Vector<DebtItem> {
        use crate::priority::tiers::RecommendationTier;

        // Combine function and file items with tier classification
        let mut all_items: Vec<DebtItem> = Vec::new();

        // Get configurable score threshold (spec 193)
        let min_score = crate::config::get_minimum_score_threshold();

        // Add function items with tier classification
        for item in &self.items {
            let mut item_with_tier = item.clone();
            let tier = classify_tier(item, tier_config);
            item_with_tier.tier = Some(tier);

            // Filter out Tier 4 items unless explicitly requested (spec: reduce spam)
            if tier == RecommendationTier::T4Maintenance && !tier_config.show_t4_in_main_report {
                continue;
            }

            // Filter out items below score threshold (spec 193)
            // Items with score 0.0 are "non-debt" and should always be excluded
            if item.unified_score.final_score <= 0.0 || item.unified_score.final_score < min_score {
                continue;
            }

            all_items.push(DebtItem::Function(Box::new(item_with_tier)));
        }

        // Add file items (files are always T1 if they're god objects)
        for item in &self.file_items {
            // Apply score filtering to file items as well (spec 193)
            // Items with score 0.0 are "non-debt" and should always be excluded
            if item.score <= 0.0 || item.score < min_score {
                continue;
            }

            all_items.push(DebtItem::File(Box::new(item.clone())));
        }

        // Sort by score (highest first) - spec 171: pure score-based ranking
        // Exponential scaling and risk boosting ensure architectural issues naturally rank higher
        all_items.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top n items
        all_items.into_iter().take(n).collect()
    }

    fn get_top_mixed_priorities_with_metrics(
        &self,
        limit: usize,
        tier_config: &TierConfig,
    ) -> FilterResult {
        use crate::priority::pipeline::filter_sort_limit;

        // Get configurable score threshold (spec 193)
        let min_score = crate::config::get_minimum_score_threshold();

        // Stage 1: Classify all function items (pure)
        let function_items: Vec<ClassifiedItem> = self
            .items
            .iter()
            .map(|item| {
                let tier = classify_tier(item, tier_config);
                let score = item.unified_score.final_score;
                let debt_item = DebtItem::Function(Box::new(item.clone()));
                ClassifiedItem {
                    item: debt_item,
                    tier,
                    score,
                }
            })
            .collect();

        // Stage 2: Classify file items (pure)
        let file_items: Vec<ClassifiedItem> = self
            .file_items
            .iter()
            .map(|item| {
                let tier = crate::priority::tiers::RecommendationTier::T1CriticalArchitecture;
                let score = item.score;
                let debt_item = DebtItem::File(Box::new(item.clone()));
                ClassifiedItem {
                    item: debt_item,
                    tier,
                    score,
                }
            })
            .collect();

        // Stage 3: Combine (pure - iterator chain, no mutation)
        let classified: Vec<ClassifiedItem> =
            function_items.into_iter().chain(file_items).collect();

        // Stage 4-6: Filter, sort, limit (pure pipeline)
        let config = FilterConfig {
            min_score,
            show_t4: tier_config.show_t4_in_main_report,
        };

        filter_sort_limit(classified, &config, limit)
    }

    fn get_bottom_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        let total_items = self.items.len();
        if total_items <= n {
            self.items.clone()
        } else {
            self.items.iter().skip(total_items - n).cloned().collect()
        }
    }

    fn get_tiered_display(&self, limit: usize) -> TieredDisplay {
        let all_items = self.get_top_mixed_priorities(limit);
        let (critical_items, groupable_items) = partition_by_criticality(all_items);

        let mut tier_groups = TierGroups::default();

        // Add critical items as individual groups
        for item in critical_items {
            tier_groups.add_group(create_critical_item_group(item));
        }

        // Group similar items by tier and debt type
        let grouped = group_items_by_tier_and_type(groupable_items);
        for ((tier, debt_type), items) in grouped {
            tier_groups.add_group(create_grouped_display_group(tier, debt_type, items));
        }

        tier_groups.into_tiered_display()
    }

    fn get_categorized_debt(&self, limit: usize) -> CategorizedDebt {
        let all_items = self.get_top_mixed_priorities(limit);

        // Collect and categorize items
        let categories = collect_categorized_items(all_items);

        // Build category summaries
        let category_summaries: BTreeMap<DebtCategory, CategorySummary> = categories
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(category, items)| {
                let summary = build_category_summary(category.clone(), items);
                (category, summary)
            })
            .collect();

        // Identify cross-category dependencies
        let cross_dependencies = identify_cross_category_dependencies(&category_summaries);

        CategorizedDebt {
            categories: category_summaries,
            cross_category_dependencies: cross_dependencies,
        }
    }
}

// Private helper functions

/// Categorize a debt item into one of the standard categories.
///
/// Function-level items are categorized by their debt type.
/// File-level items are categorized based on god object indicators,
/// coverage, or default to code quality.
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::{DebtItem, DebtCategory};
///
/// # fn categorize_debt_item(item: &DebtItem) -> DebtCategory { DebtCategory::CodeQuality }
/// # let item: DebtItem = unimplemented!();
/// let category = categorize_debt_item(&item);
/// ```
fn categorize_debt_item(item: &DebtItem) -> DebtCategory {
    match item {
        DebtItem::Function(func) => DebtCategory::from_debt_type(&func.debt_type),
        DebtItem::File(file) => {
            // File-level items typically indicate architectural issues
            if file
                .metrics
                .god_object_analysis
                .as_ref()
                .is_some_and(|a| a.is_god_object)
            {
                DebtCategory::Architecture
            } else if file.metrics.coverage_percent < 0.5 {
                DebtCategory::Testing
            } else {
                DebtCategory::CodeQuality
            }
        }
    }
}

/// Estimate effort per item in hours based on category and average severity.
///
/// Returns the estimated number of hours needed to address a single debt item
/// of the given category with the specified average severity score.
///
/// # Arguments
///
/// * `category` - The debt category (Architecture, Testing, Performance, CodeQuality)
/// * `average_severity` - The average severity score (0-100)
///
/// # Returns
///
/// Estimated hours per item. Architecture issues take longest (4-16 hours),
/// followed by Performance (4-8 hours), and Testing/CodeQuality (2-4 hours).
fn estimate_effort_per_item(category: &DebtCategory, average_severity: f64) -> u32 {
    match category {
        DebtCategory::Architecture => {
            if average_severity >= 90.0 {
                16 // 2 days
            } else if average_severity >= 70.0 {
                8 // 1 day
            } else {
                4 // Half day
            }
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
    }
}

/// Collect and categorize debt items into a map of categories to items.
///
/// Takes a collection of debt items and groups them by their assigned
/// debt category using the categorize_debt_item function.
///
/// # Arguments
///
/// * `items` - Vector of debt items to categorize
///
/// # Returns
///
/// BTreeMap with debt categories as keys and vectors of items as values
fn collect_categorized_items(items: Vector<DebtItem>) -> BTreeMap<DebtCategory, Vec<DebtItem>> {
    items
        .into_iter()
        .fold(BTreeMap::new(), |mut categories, item| {
            let category = categorize_debt_item(&item);
            categories.entry(category).or_default().push(item);
            categories
        })
}

/// Build a CategorySummary from a collection of debt items.
///
/// Computes aggregate metrics for the given category including total score,
/// item count, estimated effort, and selects the top 5 highest-priority items.
///
/// # Arguments
///
/// * `category` - The debt category for this summary
/// * `items` - Vector of debt items in this category
///
/// # Returns
///
/// CategorySummary with computed metrics and top items
fn build_category_summary(category: DebtCategory, items: Vec<DebtItem>) -> CategorySummary {
    let total_score: f64 = items.iter().map(|item| item.score()).sum();
    let item_count = items.len();
    let average_severity = total_score / item_count as f64;

    // Estimate effort based on category and average severity
    let effort_per_item = estimate_effort_per_item(&category, average_severity);
    let estimated_effort_hours = (item_count as u32) * effort_per_item;

    // Take top 5 items per category
    let top_items = items.into_iter().take(5).collect();

    CategorySummary {
        category,
        total_score,
        item_count,
        estimated_effort_hours,
        average_severity,
        top_items,
    }
}

/// Mutable container for collecting display groups by tier.
/// Used internally during tiered display construction.
#[derive(Default)]
struct TierGroups {
    critical: Vec<DisplayGroup>,
    high: Vec<DisplayGroup>,
    moderate: Vec<DisplayGroup>,
    low: Vec<DisplayGroup>,
}

impl TierGroups {
    /// Add a group to the appropriate tier bucket.
    fn add_group(&mut self, group: DisplayGroup) {
        match group.tier {
            Tier::Critical => self.critical.push(group),
            Tier::High => self.high.push(group),
            Tier::Moderate => self.moderate.push(group),
            Tier::Low => self.low.push(group),
        }
    }

    /// Convert into a TieredDisplay after sorting groups within each tier.
    fn into_tiered_display(mut self) -> TieredDisplay {
        sort_groups_by_score(&mut self.critical);
        sort_groups_by_score(&mut self.high);
        sort_groups_by_score(&mut self.moderate);
        sort_groups_by_score(&mut self.low);

        TieredDisplay {
            critical: self.critical,
            high: self.high,
            moderate: self.moderate,
            low: self.low,
        }
    }
}

/// Sort groups by total score (descending).
fn sort_groups_by_score(groups: &mut [DisplayGroup]) {
    groups.sort_by(|a, b| {
        let a_score: f64 = a.items.iter().map(|i| i.score()).sum();
        let b_score: f64 = b.items.iter().map(|i| i.score()).sum();
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Create a DisplayGroup for a single critical item.
fn create_critical_item_group(item: DebtItem) -> DisplayGroup {
    let tier = Tier::from_score(item.score());
    let debt_type = get_debt_type_key(&item);
    DisplayGroup {
        tier,
        debt_type,
        items: vec![item],
        batch_action: None,
    }
}

/// Create a DisplayGroup from a collection of similar items.
fn create_grouped_display_group(
    tier: Tier,
    debt_type: String,
    items: Vec<DebtItem>,
) -> DisplayGroup {
    let batch_action = if items.len() > 1 {
        Some(generate_batch_action(&debt_type, items.len()))
    } else {
        None
    };
    DisplayGroup {
        tier,
        debt_type,
        items,
        batch_action,
    }
}

/// Partition items into critical (ungroupable) and groupable items.
fn partition_by_criticality(items: im::Vector<DebtItem>) -> (Vec<DebtItem>, Vec<DebtItem>) {
    items.into_iter().partition(is_critical_item)
}

/// Group items by their tier and debt type key.
fn group_items_by_tier_and_type(items: Vec<DebtItem>) -> HashMap<(Tier, String), Vec<DebtItem>> {
    let mut groups: HashMap<(Tier, String), Vec<DebtItem>> = HashMap::new();
    for item in items {
        let tier = Tier::from_score(item.score());
        let debt_type = get_debt_type_key(&item);
        groups.entry((tier, debt_type)).or_default().push(item);
    }
    groups
}

/// Get a descriptive key for the debt type
fn get_debt_type_key(item: &DebtItem) -> String {
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
            if file
                .metrics
                .god_object_analysis
                .as_ref()
                .is_some_and(|a| a.is_god_object)
            {
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

/// Check if an item is considered critical (should not be grouped)
fn is_critical_item(item: &DebtItem) -> bool {
    match item {
        DebtItem::Function(func) => {
            matches!(func.debt_type, DebtType::GodObject { .. })
                || func.unified_score.final_score >= 95.0
        }
        DebtItem::File(file) => {
            file.metrics
                .god_object_analysis
                .as_ref()
                .is_some_and(|a| a.is_god_object)
                || file.metrics.total_lines > 2000
                || file.score >= 95.0
        }
    }
}

/// Generate a batch action description for a group of similar debt items
fn generate_batch_action(debt_type: &str, count: usize) -> String {
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

/// Identify dependencies between debt categories
fn identify_cross_category_dependencies(
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
                DebtItem::File(file) => file
                    .metrics
                    .god_object_analysis
                    .as_ref()
                    .is_some_and(|a| a.is_god_object),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{ActionableRecommendation, ImpactMetrics};
    use std::path::PathBuf;

    /// Test that sorting logic works correctly (higher scores rank higher)
    #[test]
    fn test_score_based_sorting() {
        // Test the sorting logic directly with DebtItems

        use crate::priority::semantic_classifier::FunctionRole;

        let item1 = DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "high_score".to_string(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 40,
                cognitive: 50,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: 50.0,
                base_score: Some(50.0),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
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
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 10,
            cognitive_complexity: 10,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }));

        let item2 = DebtItem::Function(Box::new(UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "low_score".to_string(),
                line: 10,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 5,
                cognitive: 5,
            },
            unified_score: UnifiedScore {
                complexity_factor: 2.0,
                coverage_factor: 2.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 10.0,
                base_score: Some(10.0),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
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
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 5,
            cyclomatic_complexity: 5,
            cognitive_complexity: 5,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }));

        let mut items = [item2.clone(), item1.clone()]; // Start with low score first

        // Sort using the same logic as get_top_mixed_priorities
        items.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Verify higher score is first
        assert!(items[0].score() > items[1].score());
        assert_eq!(items[0].score(), 50.0);
        assert_eq!(items[1].score(), 10.0);
    }

    /// Test get_minimum_score_threshold with environment variable override (spec 193)
    #[test]
    fn test_get_minimum_score_threshold_with_env_override() {
        // Test default threshold
        std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
        let default_threshold = crate::config::get_minimum_score_threshold();
        assert_eq!(default_threshold, 3.0);

        // Test environment variable override
        std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "5.0");
        let env_threshold = crate::config::get_minimum_score_threshold();
        assert_eq!(env_threshold, 5.0);

        // Test zero threshold (disable filtering)
        std::env::set_var("DEBTMAP_MIN_SCORE_THRESHOLD", "0.0");
        let zero_threshold = crate::config::get_minimum_score_threshold();
        assert_eq!(zero_threshold, 0.0);

        // Clean up
        std::env::remove_var("DEBTMAP_MIN_SCORE_THRESHOLD");
    }

    // ============================================================
    // Tests for get_tiered_display
    // ============================================================

    use crate::priority::debt_types::FunctionVisibility;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::CallGraph;

    /// Create a test UnifiedDebtItem with specified score and debt type
    fn create_test_unified_item(name: &str, score: f64, debt_type: DebtType) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: name.to_string(),
                line: 1,
            },
            debt_type,
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: score,
                base_score: Some(score),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
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
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 10,
            cognitive_complexity: 10,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
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
            context_suggestion: None,
        }
    }

    /// Create an empty UnifiedAnalysis for testing
    fn create_empty_analysis() -> UnifiedAnalysis {
        UnifiedAnalysis::new(CallGraph::new())
    }

    /// Create a UnifiedAnalysis with the given items
    fn create_analysis_with_items(items: Vec<UnifiedDebtItem>) -> UnifiedAnalysis {
        let mut analysis = create_empty_analysis();
        for item in items {
            analysis.items.push_back(item);
        }
        analysis
    }

    #[test]
    fn test_get_tiered_display_empty_analysis() {
        let analysis = create_empty_analysis();
        let display = analysis.get_tiered_display(10);

        assert!(display.critical.is_empty());
        assert!(display.high.is_empty());
        assert!(display.moderate.is_empty());
        assert!(display.low.is_empty());
    }

    #[test]
    fn test_get_tiered_display_single_critical_item() {
        // Score >= 90.0 goes to Critical tier
        let item = create_test_unified_item(
            "critical_func",
            92.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 60,
            },
        );

        let analysis = create_analysis_with_items(vec![item]);
        let display = analysis.get_tiered_display(10);

        assert_eq!(display.critical.len(), 1);
        assert!(display.high.is_empty());
        assert!(display.moderate.is_empty());
        assert!(display.low.is_empty());

        assert_eq!(display.critical[0].items.len(), 1);
        assert_eq!(display.critical[0].debt_type, "High Complexity Functions");
    }

    #[test]
    fn test_get_tiered_display_single_high_item() {
        // Score 70.0-89.9 goes to High tier
        let item = create_test_unified_item(
            "high_func",
            75.0,
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 20,
                cognitive: 25,
            },
        );

        let analysis = create_analysis_with_items(vec![item]);
        let display = analysis.get_tiered_display(10);

        assert!(display.critical.is_empty());
        assert_eq!(display.high.len(), 1);
        assert!(display.moderate.is_empty());
        assert!(display.low.is_empty());

        assert_eq!(display.high[0].debt_type, "Untested Complex Functions");
    }

    #[test]
    fn test_get_tiered_display_single_moderate_item() {
        // Score 50.0-69.9 goes to Moderate tier
        let item = create_test_unified_item(
            "moderate_func",
            55.0,
            DebtType::Duplication {
                instances: 2,
                total_lines: 50,
            },
        );

        let analysis = create_analysis_with_items(vec![item]);
        let display = analysis.get_tiered_display(10);

        assert!(display.critical.is_empty());
        assert!(display.high.is_empty());
        assert_eq!(display.moderate.len(), 1);
        assert!(display.low.is_empty());

        assert_eq!(display.moderate[0].debt_type, "Code Duplication");
    }

    #[test]
    fn test_get_tiered_display_single_low_item() {
        // Score < 50.0 goes to Low tier
        let item = create_test_unified_item(
            "low_func",
            30.0,
            DebtType::DeadCode {
                visibility: FunctionVisibility::Public,
                cyclomatic: 5,
                cognitive: 5,
                usage_hints: vec![],
            },
        );

        let analysis = create_analysis_with_items(vec![item]);
        let display = analysis.get_tiered_display(10);

        assert!(display.critical.is_empty());
        assert!(display.high.is_empty());
        assert!(display.moderate.is_empty());
        assert_eq!(display.low.len(), 1);

        assert_eq!(display.low[0].debt_type, "Dead Code");
    }

    #[test]
    fn test_get_tiered_display_groups_similar_items() {
        // Two items with same debt type should be grouped
        let item1 = create_test_unified_item(
            "func1",
            55.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
        );
        let item2 = create_test_unified_item(
            "func2",
            52.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 18,
                cognitive: 22,
            },
        );

        let analysis = create_analysis_with_items(vec![item1, item2]);
        let display = analysis.get_tiered_display(10);

        // Both should be in Moderate tier (scores 50-69)
        assert_eq!(display.moderate.len(), 1); // Grouped together
        assert_eq!(display.moderate[0].items.len(), 2);
        assert_eq!(display.moderate[0].debt_type, "High Complexity Functions");

        // Should have batch action for multiple items
        assert!(display.moderate[0].batch_action.is_some());
        assert!(display.moderate[0]
            .batch_action
            .as_ref()
            .unwrap()
            .contains("2"));
    }

    #[test]
    fn test_get_tiered_display_critical_items_not_grouped() {
        // Items with score >= 95.0 should NOT be grouped (is_critical_item)
        let item1 = create_test_unified_item(
            "critical1",
            96.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 100,
                cognitive: 120,
            },
        );
        let item2 = create_test_unified_item(
            "critical2",
            97.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 110,
                cognitive: 130,
            },
        );

        let analysis = create_analysis_with_items(vec![item1, item2]);
        let display = analysis.get_tiered_display(10);

        // Both should be separate groups in Critical tier
        assert_eq!(display.critical.len(), 2);
        assert_eq!(display.critical[0].items.len(), 1);
        assert_eq!(display.critical[1].items.len(), 1);
    }

    #[test]
    fn test_get_tiered_display_god_objects_not_grouped() {
        // God objects should NOT be grouped regardless of score
        let item1 = create_test_unified_item(
            "god1",
            75.0,
            DebtType::GodObject {
                methods: 50,
                fields: Some(30),
                responsibilities: 5,
                god_object_score: 85.0,
                lines: 2000,
            },
        );
        let item2 = create_test_unified_item(
            "god2",
            72.0,
            DebtType::GodObject {
                methods: 45,
                fields: Some(25),
                responsibilities: 4,
                god_object_score: 80.0,
                lines: 1800,
            },
        );

        let analysis = create_analysis_with_items(vec![item1, item2]);
        let display = analysis.get_tiered_display(10);

        // Both should be separate groups in High tier
        assert_eq!(display.high.len(), 2);
        assert_eq!(display.high[0].items.len(), 1);
        assert_eq!(display.high[1].items.len(), 1);
    }

    #[test]
    fn test_get_tiered_display_sorting_within_tier() {
        // Groups should be sorted by total score within each tier
        let item1 = create_test_unified_item(
            "low_score",
            52.0,
            DebtType::DeadCode {
                visibility: FunctionVisibility::Public,
                cyclomatic: 5,
                cognitive: 5,
                usage_hints: vec![],
            },
        );
        let item2 = create_test_unified_item(
            "high_score",
            68.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 35,
            },
        );

        let analysis = create_analysis_with_items(vec![item1, item2]);
        let display = analysis.get_tiered_display(10);

        // Both in Moderate tier, but sorted by score
        assert_eq!(display.moderate.len(), 2);

        // Higher score group should come first
        let first_total: f64 = display.moderate[0].items.iter().map(|i| i.score()).sum();
        let second_total: f64 = display.moderate[1].items.iter().map(|i| i.score()).sum();
        assert!(first_total >= second_total);
    }

    #[test]
    fn test_get_tiered_display_items_across_all_tiers() {
        // Create items for each tier
        let critical = create_test_unified_item(
            "critical",
            92.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 60,
            },
        );
        let high = create_test_unified_item(
            "high",
            75.0,
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 20,
                cognitive: 25,
            },
        );
        let moderate = create_test_unified_item(
            "moderate",
            55.0,
            DebtType::Duplication {
                instances: 2,
                total_lines: 50,
            },
        );
        let low = create_test_unified_item(
            "low",
            30.0,
            DebtType::DeadCode {
                visibility: FunctionVisibility::Public,
                cyclomatic: 5,
                cognitive: 5,
                usage_hints: vec![],
            },
        );

        let analysis = create_analysis_with_items(vec![critical, high, moderate, low]);
        let display = analysis.get_tiered_display(10);

        assert_eq!(display.critical.len(), 1);
        assert_eq!(display.high.len(), 1);
        assert_eq!(display.moderate.len(), 1);
        assert_eq!(display.low.len(), 1);
    }

    #[test]
    fn test_get_tiered_display_respects_limit() {
        // Create more items than the limit
        let items: Vec<UnifiedDebtItem> = (0..10)
            .map(|i| {
                create_test_unified_item(
                    &format!("func_{}", i),
                    90.0 + (i as f64 * 0.5), // All critical tier
                    DebtType::ComplexityHotspot {
                        cyclomatic: 50 + i,
                        cognitive: 60 + i,
                    },
                )
            })
            .collect();

        let analysis = create_analysis_with_items(items);
        let display = analysis.get_tiered_display(5);

        // Should only show 5 items total
        let total_items: usize = display
            .critical
            .iter()
            .chain(display.high.iter())
            .chain(display.moderate.iter())
            .chain(display.low.iter())
            .map(|g| g.items.len())
            .sum();

        assert_eq!(total_items, 5);
    }

    #[test]
    fn test_get_tiered_display_batch_actions() {
        // Create multiple items of each type to test batch action generation
        let items: Vec<UnifiedDebtItem> = (0..3)
            .map(|i| {
                create_test_unified_item(
                    &format!("untested_{}", i),
                    55.0 + (i as f64),
                    DebtType::TestingGap {
                        coverage: 0.0,
                        cyclomatic: 10 + i,
                        cognitive: 15 + i,
                    },
                )
            })
            .collect();

        let analysis = create_analysis_with_items(items);
        let display = analysis.get_tiered_display(10);

        // Find the "Untested Complex Functions" group
        let untested_group = display
            .moderate
            .iter()
            .find(|g| g.debt_type == "Untested Complex Functions");

        assert!(untested_group.is_some());
        let group = untested_group.unwrap();
        assert_eq!(group.items.len(), 3);
        assert!(group.batch_action.is_some());
        assert!(group.batch_action.as_ref().unwrap().contains("3"));
        assert!(group
            .batch_action
            .as_ref()
            .unwrap()
            .contains("test coverage"));
    }

    #[test]
    fn test_get_tiered_display_no_batch_action_for_single_item() {
        let item = create_test_unified_item(
            "single",
            55.0,
            DebtType::Duplication {
                instances: 2,
                total_lines: 50,
            },
        );

        let analysis = create_analysis_with_items(vec![item]);
        let display = analysis.get_tiered_display(10);

        // Single item should not have batch action
        assert_eq!(display.moderate.len(), 1);
        assert!(display.moderate[0].batch_action.is_none());
    }

    // Tests for extracted helper functions

    #[test]
    fn test_partition_by_criticality_separates_critical_items() {
        let critical = create_test_unified_item(
            "critical",
            96.0, // >= 95.0 makes it critical
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 60,
            },
        );
        let normal = create_test_unified_item(
            "normal",
            60.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
        );

        let items: im::Vector<DebtItem> = vec![
            DebtItem::Function(Box::new(critical)),
            DebtItem::Function(Box::new(normal)),
        ]
        .into_iter()
        .collect();

        let (critical_items, groupable_items) = super::partition_by_criticality(items);

        assert_eq!(critical_items.len(), 1);
        assert_eq!(groupable_items.len(), 1);
        assert_eq!(critical_items[0].score(), 96.0);
        assert_eq!(groupable_items[0].score(), 60.0);
    }

    #[test]
    fn test_partition_by_criticality_god_objects_always_critical() {
        let god_object = create_test_unified_item(
            "god_object",
            50.0, // Low score, but god object type makes it critical
            DebtType::GodObject {
                methods: 100,
                fields: Some(50),
                responsibilities: 10,
                god_object_score: 85.0,
                lines: 2000,
            },
        );

        let items: im::Vector<DebtItem> = vec![DebtItem::Function(Box::new(god_object))]
            .into_iter()
            .collect();

        let (critical_items, groupable_items) = super::partition_by_criticality(items);

        assert_eq!(critical_items.len(), 1);
        assert_eq!(groupable_items.len(), 0);
    }

    #[test]
    fn test_group_items_by_tier_and_type_groups_correctly() {
        let item1 = create_test_unified_item(
            "func1",
            75.0, // High tier
            DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 35,
            },
        );
        let item2 = create_test_unified_item(
            "func2",
            78.0, // High tier
            DebtType::ComplexityHotspot {
                cyclomatic: 32,
                cognitive: 38,
            },
        );
        let item3 = create_test_unified_item(
            "func3",
            55.0, // Moderate tier
            DebtType::TestingGap {
                coverage: 0.2,
                cyclomatic: 15,
                cognitive: 18,
            },
        );

        let items = vec![
            DebtItem::Function(Box::new(item1)),
            DebtItem::Function(Box::new(item2)),
            DebtItem::Function(Box::new(item3)),
        ];

        let groups = super::group_items_by_tier_and_type(items);

        // Should have 2 groups: High/ComplexityHotspot and Moderate/TestingGap
        assert_eq!(groups.len(), 2);

        let high_complexity_key = (Tier::High, "High Complexity Functions".to_string());
        assert!(groups.contains_key(&high_complexity_key));
        assert_eq!(groups[&high_complexity_key].len(), 2);

        let moderate_testing_key = (Tier::Moderate, "Untested Complex Functions".to_string());
        assert!(groups.contains_key(&moderate_testing_key));
        assert_eq!(groups[&moderate_testing_key].len(), 1);
    }

    #[test]
    fn test_sort_groups_by_score_descending() {
        let item1 = create_test_unified_item(
            "low",
            30.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 12,
            },
        );
        let item2 = create_test_unified_item(
            "high",
            80.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 40,
                cognitive: 50,
            },
        );

        let mut groups = vec![
            DisplayGroup {
                tier: Tier::Low,
                debt_type: "Low Score".to_string(),
                items: vec![DebtItem::Function(Box::new(item1))],
                batch_action: None,
            },
            DisplayGroup {
                tier: Tier::High,
                debt_type: "High Score".to_string(),
                items: vec![DebtItem::Function(Box::new(item2))],
                batch_action: None,
            },
        ];

        super::sort_groups_by_score(&mut groups);

        // Higher score should be first
        assert_eq!(groups[0].debt_type, "High Score");
        assert_eq!(groups[1].debt_type, "Low Score");
    }

    #[test]
    fn test_create_critical_item_group_single_item_no_batch() {
        let item = create_test_unified_item(
            "critical",
            96.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 60,
            },
        );

        let group = super::create_critical_item_group(DebtItem::Function(Box::new(item)));

        assert_eq!(group.items.len(), 1);
        assert!(group.batch_action.is_none());
        assert_eq!(group.tier, Tier::Critical);
    }

    #[test]
    fn test_create_grouped_display_group_with_batch_action() {
        let item1 = create_test_unified_item(
            "func1",
            75.0,
            DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 20,
                cognitive: 25,
            },
        );
        let item2 = create_test_unified_item(
            "func2",
            72.0,
            DebtType::TestingGap {
                coverage: 0.2,
                cyclomatic: 18,
                cognitive: 22,
            },
        );

        let items = vec![
            DebtItem::Function(Box::new(item1)),
            DebtItem::Function(Box::new(item2)),
        ];

        let group = super::create_grouped_display_group(
            Tier::High,
            "Untested Complex Functions".to_string(),
            items,
        );

        assert_eq!(group.items.len(), 2);
        assert!(group.batch_action.is_some());
        assert_eq!(
            group.batch_action.unwrap(),
            "Add test coverage for 2 complex functions"
        );
    }

    #[test]
    fn test_create_grouped_display_group_single_item_no_batch() {
        let item = create_test_unified_item(
            "func1",
            75.0,
            DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 20,
                cognitive: 25,
            },
        );

        let items = vec![DebtItem::Function(Box::new(item))];

        let group = super::create_grouped_display_group(
            Tier::High,
            "Untested Complex Functions".to_string(),
            items,
        );

        assert_eq!(group.items.len(), 1);
        assert!(group.batch_action.is_none());
    }

    #[test]
    fn test_tier_groups_into_tiered_display_sorts_all_tiers() {
        let critical_item = create_test_unified_item(
            "critical",
            92.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 45,
                cognitive: 55,
            },
        );
        let high_item = create_test_unified_item(
            "high",
            75.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 35,
            },
        );

        let mut tier_groups = super::TierGroups::default();
        tier_groups.add_group(DisplayGroup {
            tier: Tier::Critical,
            debt_type: "Test".to_string(),
            items: vec![DebtItem::Function(Box::new(critical_item))],
            batch_action: None,
        });
        tier_groups.add_group(DisplayGroup {
            tier: Tier::High,
            debt_type: "Test".to_string(),
            items: vec![DebtItem::Function(Box::new(high_item))],
            batch_action: None,
        });

        let display = tier_groups.into_tiered_display();

        assert_eq!(display.critical.len(), 1);
        assert_eq!(display.high.len(), 1);
        assert_eq!(display.moderate.len(), 0);
        assert_eq!(display.low.len(), 0);
    }
}

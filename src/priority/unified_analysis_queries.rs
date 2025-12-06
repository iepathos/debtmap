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
            if item.unified_score.final_score < min_score {
                continue;
            }

            all_items.push(DebtItem::Function(Box::new(item_with_tier)));
        }

        // Add file items (files are always T1 if they're god objects)
        for item in &self.file_items {
            // Apply score filtering to file items as well (spec 193)
            if item.score < min_score {
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
        let classified: Vec<ClassifiedItem> = function_items
            .into_iter()
            .chain(file_items)
            .collect();

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

        let mut critical_groups: Vec<DisplayGroup> = Vec::new();
        let mut high_groups: Vec<DisplayGroup> = Vec::new();
        let mut moderate_groups: Vec<DisplayGroup> = Vec::new();
        let mut low_groups: Vec<DisplayGroup> = Vec::new();

        // Group items by tier and debt type
        let mut tier_groups: HashMap<(Tier, String), Vec<DebtItem>> = HashMap::new();

        for item in all_items {
            let tier = Tier::from_score(item.score());
            let debt_type = get_debt_type_key(&item);

            // Never group god objects or architectural issues
            if is_critical_item(&item) {
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
                Some(generate_batch_action(&debt_type, items.len()))
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
            if file.metrics.god_object_indicators.is_god_object {
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

/// Check if an item is considered critical (should not be grouped)
fn is_critical_item(item: &DebtItem) -> bool {
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
                adjusted_cyclomatic: None,
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
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 10,
            cognitive_complexity: 10,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
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
            nesting_depth: 1,
            function_length: 5,
            cyclomatic_complexity: 5,
            cognitive_complexity: 5,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
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
}

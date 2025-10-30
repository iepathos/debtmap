//! Query and data access operations for UnifiedAnalysis.
//!
//! This module provides methods for retrieving, filtering, and displaying
//! technical debt items from a UnifiedAnalysis. Operations are pure and
//! functional, creating new data structures rather than mutating.

use super::{
    CategorizedDebt, CategorySummary, CrossCategoryDependency, DebtCategory, DebtItem, DebtType,
    DisplayGroup, ImpactLevel, Tier, TieredDisplay, UnifiedAnalysis, UnifiedDebtItem,
};
use crate::priority::tiers::{classify_tier, RecommendationTier, TierConfig};
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
        // Combine function and file items with tier classification
        let mut all_items: Vec<DebtItem> = Vec::new();

        // Add function items with tier classification
        for item in &self.items {
            let mut item_with_tier = item.clone();
            item_with_tier.tier = Some(classify_tier(item, tier_config));
            all_items.push(DebtItem::Function(Box::new(item_with_tier)));
        }

        // Add file items (files are always T1 if they're god objects)
        for item in &self.file_items {
            all_items.push(DebtItem::File(Box::new(item.clone())));
        }

        // Sort by tier first (T1 > T2 > T3 > T4), then by score within tier
        all_items.sort_by(|a, b| {
            // Get tier for comparison
            let tier_a = match a {
                DebtItem::Function(f) => f.tier.unwrap_or(RecommendationTier::T4Maintenance),
                DebtItem::File(_) => RecommendationTier::T1CriticalArchitecture, // Files are architectural
            };
            let tier_b = match b {
                DebtItem::Function(f) => f.tier.unwrap_or(RecommendationTier::T4Maintenance),
                DebtItem::File(_) => RecommendationTier::T1CriticalArchitecture,
            };

            // Primary sort: by tier (lower enum value = higher priority)
            match tier_a.cmp(&tier_b) {
                std::cmp::Ordering::Equal => {
                    // Secondary sort: by score within tier (higher score = higher priority)
                    b.score()
                        .partial_cmp(&a.score())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                other => other,
            }
        });

        // Return top n items
        all_items.into_iter().take(n).collect()
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
        let cross_dependencies = identify_cross_category_dependencies(&category_summaries);

        CategorizedDebt {
            categories: category_summaries,
            cross_category_dependencies: cross_dependencies,
        }
    }
}

// Private helper functions

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

//! Pure transformation pipeline for view preparation.
//!
//! This module implements the "still water" - pure transformations
//! with no I/O. All functions are deterministic and testable.
//!
//! # Architecture
//!
//! The pipeline transforms `UnifiedAnalysis` into `PreparedDebtView` through
//! composable, pure stages:
//!
//! ```text
//! UnifiedAnalysis
//!        │
//!        ▼
//! ┌──────────────────┐
//! │  prepare_view()  │ ← ViewConfig, TierConfig (params, not env vars)
//! └──────────────────┘
//!        │
//!        ├─→ combine_items()       ← Merge function + file items
//!        ├─→ classify_all_tiers()  ← Assign recommendation tiers
//!        ├─→ filter_items()        ← Apply score/tier filters
//!        ├─→ sort_items()          ← Sort by criteria
//!        ├─→ limit_items()         ← Apply optional limit
//!        ├─→ compute_groups()      ← Create location groups
//!        └─→ calculate_summary()   ← Aggregate statistics
//!        │
//!        ▼
//! PreparedDebtView
//! ```
//!
//! # Module Organization
//!
//! - **filter.rs**: Stage 3 - Pure filter predicates and item filtering
//! - **sort.rs**: Stage 4 - Pure sorting functions
//! - **group.rs**: Stage 6 - Pure grouping functions
//! - **summary.rs**: Stage 7 - Pure summary calculation
//!
//! # Purity Guarantee
//!
//! All stages are pure functions:
//! - No environment variable access
//! - No file I/O
//! - Deterministic results
//! - No side effects
//!
//! Configuration is passed as parameters, not read from environment.

pub mod filter;
pub mod group;
pub mod sort;
pub mod summary;

#[cfg(test)]
mod tests;

use crate::priority::{
    file_metrics::FileDebtItem,
    tiers::{classify_tier, TierConfig},
    unified_scorer::UnifiedDebtItem,
    view::{PreparedDebtView, ViewConfig, ViewItem},
    UnifiedAnalysis,
};

// Re-export key types for convenience
pub use filter::FilterStats;

/// Prepares a canonical view from analysis results.
///
/// This is the **single entry point** for transforming `UnifiedAnalysis`
/// into `PreparedDebtView`. All output formats should use this function.
///
/// # Pure Function
///
/// This function has no side effects:
/// - No environment variable access
/// - No file I/O
/// - No logging or printing
/// - Deterministic: same inputs always produce same outputs
///
/// # Arguments
///
/// * `analysis` - The analysis results to transform
/// * `config` - View configuration (thresholds, limits, sorting)
/// * `tier_config` - Tier classification configuration
///
/// # Returns
///
/// A `PreparedDebtView` ready for consumption by any output format.
///
/// # Examples
///
/// ```ignore
/// let config = ViewConfig::default();
/// let tier_config = TierConfig::default();
/// let view = prepare_view(&analysis, &config, &tier_config);
///
/// // All output formats use the same view
/// render_tui(&view);
/// render_json(&view);
/// render_markdown(&view);
/// ```
pub fn prepare_view(
    analysis: &UnifiedAnalysis,
    config: &ViewConfig,
    tier_config: &TierConfig,
) -> PreparedDebtView {
    // Stage 1: Combine function and file items (pure)
    let combined = combine_items(&analysis.items, &analysis.file_items);
    let total_before_filter = combined.len();

    // Stage 2: Classify tiers (pure)
    let classified = classify_all_tiers(combined, tier_config);

    // Stage 3: Filter (pure)
    let (filtered, filter_stats) = filter::filter_items(classified, config);

    // Stage 4: Sort (pure)
    let sorted = sort::sort_items(filtered, config.sort_by);

    // Stage 5: Limit (pure)
    let limited = limit_items(sorted, config.limit);

    // Stage 6: Compute groups (pure)
    let groups = if config.compute_groups {
        group::compute_groups(&limited, config.sort_by)
    } else {
        vec![]
    };

    // Stage 7: Calculate summary (pure)
    let sum = summary::calculate_summary(
        &limited,
        total_before_filter,
        filter_stats,
        analysis.total_lines_of_code,
        analysis.overall_coverage,
    );

    PreparedDebtView {
        items: limited,
        groups,
        summary: sum,
        config: config.clone(),
    }
}

// ============================================================================
// STAGE 1: COMBINE ITEMS
// ============================================================================

/// Combines function and file items into unified ViewItems.
///
/// Pure function - operates on input slices, returns new Vec.
fn combine_items(
    function_items: &im::Vector<UnifiedDebtItem>,
    file_items: &im::Vector<FileDebtItem>,
) -> Vec<ViewItem> {
    let mut combined = Vec::with_capacity(function_items.len() + file_items.len());

    for item in function_items.iter() {
        combined.push(ViewItem::Function(Box::new(item.clone())));
    }

    for item in file_items.iter() {
        combined.push(ViewItem::File(Box::new(item.clone())));
    }

    combined
}

// ============================================================================
// STAGE 2: TIER CLASSIFICATION
// ============================================================================

/// Classifies tiers for all items.
///
/// For function items, uses the tier classification logic.
/// For file items, assigns T1CriticalArchitecture (god objects are always critical).
fn classify_all_tiers(items: Vec<ViewItem>, tier_config: &TierConfig) -> Vec<ViewItem> {
    items
        .into_iter()
        .map(|item| classify_item_tier(item, tier_config))
        .collect()
}

/// Classifies tier for a single item.
fn classify_item_tier(mut item: ViewItem, tier_config: &TierConfig) -> ViewItem {
    if let ViewItem::Function(ref mut func) = item {
        let tier = classify_tier(func, tier_config);
        func.tier = Some(tier);
    }
    // File items don't need tier classification (always T1)
    item
}

// ============================================================================
// STAGE 5: LIMITING
// ============================================================================

/// Limits items to specified count.
///
/// Pure function - returns new Vec with at most `limit` items.
fn limit_items(items: Vec<ViewItem>, limit: Option<usize>) -> Vec<ViewItem> {
    match limit {
        Some(n) => items.into_iter().take(n).collect(),
        None => items,
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Creates a view with default configuration.
///
/// Useful for quick usage without custom configuration.
pub fn prepare_view_default(analysis: &UnifiedAnalysis) -> PreparedDebtView {
    prepare_view(analysis, &ViewConfig::default(), &TierConfig::default())
}

/// Creates a view with TUI-optimized configuration.
///
/// Mirrors current TUI behavior:
/// - No score threshold (show all)
/// - No T4 filtering
/// - Grouping enabled
pub fn prepare_view_for_tui(analysis: &UnifiedAnalysis) -> PreparedDebtView {
    let config = ViewConfig {
        min_score_threshold: 0.0,
        exclude_t4_maintenance: false,
        compute_groups: true,
        ..Default::default()
    };
    prepare_view(analysis, &config, &TierConfig::default())
}

/// Creates a view with terminal-optimized configuration.
///
/// Mirrors current --no-tui behavior:
/// - Score threshold 3.0
/// - T4 filtering enabled
/// - No grouping
pub fn prepare_view_for_terminal(
    analysis: &UnifiedAnalysis,
    limit: Option<usize>,
) -> PreparedDebtView {
    let config = ViewConfig {
        min_score_threshold: 3.0,
        exclude_t4_maintenance: true,
        limit,
        compute_groups: false,
        ..Default::default()
    };
    prepare_view(analysis, &config, &TierConfig::default())
}

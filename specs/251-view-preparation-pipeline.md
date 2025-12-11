---
number: 251
title: View Preparation Pipeline (Pure Transformations)
category: foundation
priority: critical
status: draft
dependencies: [250]
created: 2025-12-10
---

# Specification 251: View Preparation Pipeline (Pure Transformations)

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 250 (Unified View Data Model)

## Context

With the `PreparedDebtView` type defined in Spec 250, we need a **pure transformation pipeline** to convert `UnifiedAnalysis` into this canonical view model.

Currently, transformation logic is scattered across multiple locations:
- `src/priority/unified_analysis_queries.rs` - `get_top_mixed_priorities()`
- `src/tui/results/grouping.rs` - `group_by_location()`
- `src/output/markdown.rs` - `apply_filters()`
- Environment variable reading mixed with transformation

Following Stillwater's "Pure Core, Imperative Shell" pattern:
- **Pure Core**: `prepare_view()` - takes data + config, returns view (no I/O)
- **Imperative Shell**: Config reading happens at the boundary

## Objective

Create a **single pure pipeline function** `prepare_view()` that transforms `UnifiedAnalysis` into `PreparedDebtView`. This function:

1. Combines function and file items
2. Classifies tiers
3. Filters by score threshold and tier
4. Sorts by criteria
5. Limits if specified
6. Computes groups
7. Calculates summary statistics

All operations are **pure** - no environment variable access, no I/O.

## Requirements

### Functional Requirements

1. **Main Pipeline Function**
   - `prepare_view(analysis: &UnifiedAnalysis, config: &ViewConfig, tier_config: &TierConfig) -> PreparedDebtView`
   - Pure function - deterministic, no side effects
   - Combines all transformation steps
   - Returns complete `PreparedDebtView`

2. **Composable Stage Functions**
   - `combine_items()` - Merge function and file items into `Vec<ViewItem>`
   - `classify_tiers()` - Add tier classification to items
   - `filter_items()` - Apply score and tier filters
   - `sort_items()` - Sort by criteria
   - `limit_items()` - Apply optional limit
   - `compute_groups()` - Create location groups
   - `calculate_summary()` - Compute statistics
   - Each function pure, under 20 lines

3. **Filter Predicates**
   - `passes_score_threshold()` - Score >= threshold
   - `passes_tier_filter()` - Not T4 (if excluding)
   - Composable with `&&` for combined filters

4. **Sort Functions**
   - `sort_by_score()` - Highest score first
   - `sort_by_coverage()` - Lowest coverage first
   - `sort_by_complexity()` - Highest complexity first
   - `sort_by_file_path()` - Alphabetical by path
   - `sort_by_function_name()` - Alphabetical by function

5. **Statistics Calculation**
   - Count items by severity
   - Count items by category
   - Sum total debt score
   - Track filter statistics

### Non-Functional Requirements

1. **Purity**
   - No `std::env::var()` calls
   - No `eprintln!` or logging
   - No file I/O
   - Deterministic results
   - No mutation of inputs

2. **Performance**
   - O(n log n) for sorting
   - Single pass for filtering where possible
   - Efficient grouping using HashMap

3. **Testability**
   - All functions unit testable
   - No mocks needed
   - Fast tests (pure computation)

## Acceptance Criteria

- [ ] `prepare_view()` function created as pure pipeline
- [ ] Each stage function is pure and under 20 lines
- [ ] No environment variable access in any function
- [ ] Unit tests for each stage function
- [ ] Integration test for full pipeline
- [ ] Property-based tests for invariants
- [ ] Determinism verified (same input → same output)
- [ ] Performance benchmarks added
- [ ] All existing tests continue to pass

## Technical Details

### Implementation Approach

**File Location**: `src/priority/view_pipeline.rs`

**Main Pipeline**:

```rust
//! Pure transformation pipeline for view preparation.
//!
//! This module implements the "still water" - pure transformations
//! with no I/O. All functions are deterministic and testable.

use crate::priority::{
    file_metrics::FileDebtItem,
    tiers::{classify_tier, RecommendationTier, TierConfig},
    unified_scorer::UnifiedDebtItem,
    view::{
        CategoryCounts, ItemLocation, LocationGroup, PreparedDebtView,
        ScoreDistribution, SortCriteria, ViewConfig, ViewItem, ViewSummary,
    },
    DebtCategory, UnifiedAnalysis,
};
use std::collections::HashMap;

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
/// ```
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
    let (filtered, filter_stats) = filter_items(classified, config);

    // Stage 4: Sort (pure)
    let sorted = sort_items(filtered, config.sort_by);

    // Stage 5: Limit (pure)
    let limited = limit_items(sorted, config.limit);

    // Stage 6: Compute groups (pure)
    let groups = if config.compute_groups {
        compute_groups(&limited, config.sort_by)
    } else {
        vec![]
    };

    // Stage 7: Calculate summary (pure)
    let summary = calculate_summary(
        &limited,
        total_before_filter,
        filter_stats,
        analysis.total_lines_of_code,
        analysis.overall_coverage,
    );

    PreparedDebtView {
        items: limited,
        groups,
        summary,
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
/// For file items, assigns T1Critical (god objects are always critical).
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
// STAGE 3: FILTERING
// ============================================================================

/// Statistics about filtered items.
#[derive(Debug, Default)]
struct FilterStats {
    filtered_by_score: usize,
    filtered_by_tier: usize,
}

/// Filters items based on configuration.
///
/// Pure function - returns new Vec and filter statistics.
fn filter_items(items: Vec<ViewItem>, config: &ViewConfig) -> (Vec<ViewItem>, FilterStats) {
    let mut stats = FilterStats::default();

    let filtered = items
        .into_iter()
        .filter(|item| {
            if !passes_score_threshold(item, config.min_score_threshold) {
                stats.filtered_by_score += 1;
                return false;
            }
            if !passes_tier_filter(item, config.exclude_t4_maintenance) {
                stats.filtered_by_tier += 1;
                return false;
            }
            true
        })
        .collect();

    (filtered, stats)
}

/// Checks if item passes score threshold.
fn passes_score_threshold(item: &ViewItem, threshold: f64) -> bool {
    item.score() >= threshold
}

/// Checks if item passes tier filter.
fn passes_tier_filter(item: &ViewItem, exclude_t4: bool) -> bool {
    if !exclude_t4 {
        return true;
    }

    match item.tier() {
        Some(RecommendationTier::T4Maintenance) => false,
        _ => true,
    }
}

// ============================================================================
// STAGE 4: SORTING
// ============================================================================

/// Sorts items by the specified criteria.
///
/// Pure function - returns new sorted Vec.
fn sort_items(mut items: Vec<ViewItem>, criteria: SortCriteria) -> Vec<ViewItem> {
    match criteria {
        SortCriteria::Score => sort_by_score(&mut items),
        SortCriteria::Coverage => sort_by_coverage(&mut items),
        SortCriteria::Complexity => sort_by_complexity(&mut items),
        SortCriteria::FilePath => sort_by_file_path(&mut items),
        SortCriteria::FunctionName => sort_by_function_name(&mut items),
    }
    items
}

/// Sorts by score descending (highest first).
fn sort_by_score(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Sorts by coverage ascending (lowest first).
fn sort_by_coverage(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        let cov_a = get_coverage(a);
        let cov_b = get_coverage(b);
        match (cov_a, cov_b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less, // No coverage is worst
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        }
    });
}

/// Sorts by complexity descending (highest first).
fn sort_by_complexity(items: &mut [ViewItem]) {
    items.sort_by(|a, b| get_complexity(b).cmp(&get_complexity(a)));
}

/// Sorts by file path alphabetically.
fn sort_by_file_path(items: &mut [ViewItem]) {
    items.sort_by(|a, b| a.location().file.cmp(&b.location().file));
}

/// Sorts by function name alphabetically.
fn sort_by_function_name(items: &mut [ViewItem]) {
    items.sort_by(|a, b| {
        let name_a = a.location().function.as_deref().unwrap_or("");
        let name_b = b.location().function.as_deref().unwrap_or("");
        name_a.cmp(name_b)
    });
}

/// Extracts coverage from item (if available).
fn get_coverage(item: &ViewItem) -> Option<f64> {
    match item {
        ViewItem::Function(f) => f.transitive_coverage.as_ref().map(|c| c.direct),
        ViewItem::File(f) => Some(f.metrics.coverage_percent),
    }
}

/// Extracts complexity from item.
fn get_complexity(item: &ViewItem) -> u32 {
    match item {
        ViewItem::Function(f) => f.cognitive_complexity,
        ViewItem::File(f) => f.metrics.max_complexity,
    }
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
// STAGE 6: GROUPING
// ============================================================================

/// Computes location groups from items.
///
/// Groups items by (file, function, line) and calculates combined scores.
fn compute_groups(items: &[ViewItem], sort_by: SortCriteria) -> Vec<LocationGroup> {
    let mut groups_map: HashMap<(PathBuf, String, usize), Vec<ViewItem>> = HashMap::new();

    for item in items {
        let loc = item.location();
        let key = loc.group_key();
        groups_map
            .entry(key)
            .or_default()
            .push(item.clone());
    }

    let mut groups: Vec<LocationGroup> = groups_map
        .into_iter()
        .map(|(_, items)| {
            let location = items[0].location();
            LocationGroup::new(location, items)
        })
        .collect();

    // Sort groups by same criteria as items
    sort_groups(&mut groups, sort_by);

    groups
}

/// Sorts groups by criteria.
fn sort_groups(groups: &mut [LocationGroup], criteria: SortCriteria) {
    match criteria {
        SortCriteria::Score => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortCriteria::FilePath => {
            groups.sort_by(|a, b| a.location.file.cmp(&b.location.file));
        }
        SortCriteria::FunctionName => {
            groups.sort_by(|a, b| {
                let name_a = a.location.function.as_deref().unwrap_or("");
                let name_b = b.location.function.as_deref().unwrap_or("");
                name_a.cmp(name_b)
            });
        }
        // For coverage/complexity, use combined score as fallback
        _ => {
            groups.sort_by(|a, b| {
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

// ============================================================================
// STAGE 7: SUMMARY CALCULATION
// ============================================================================

/// Calculates summary statistics from items.
///
/// Pure function - aggregates data from items list.
fn calculate_summary(
    items: &[ViewItem],
    total_before_filter: usize,
    filter_stats: FilterStats,
    total_loc: usize,
    overall_coverage: Option<f64>,
) -> ViewSummary {
    let total_debt_score: f64 = items.iter().map(|i| i.score()).sum();

    let score_distribution = calculate_score_distribution(items);
    let category_counts = calculate_category_counts(items);

    let debt_density = if total_loc > 0 {
        (total_debt_score / total_loc as f64) * 1000.0
    } else {
        0.0
    };

    ViewSummary {
        total_items_before_filter: total_before_filter,
        total_items_after_filter: items.len(),
        filtered_by_tier: filter_stats.filtered_by_tier,
        filtered_by_score: filter_stats.filtered_by_score,
        total_debt_score,
        score_distribution,
        category_counts,
        total_lines_of_code: total_loc,
        debt_density,
        overall_coverage,
    }
}

/// Calculates score distribution by severity.
fn calculate_score_distribution(items: &[ViewItem]) -> ScoreDistribution {
    let mut dist = ScoreDistribution::default();

    for item in items {
        match item.severity() {
            crate::priority::classification::Severity::Critical => dist.critical += 1,
            crate::priority::classification::Severity::High => dist.high += 1,
            crate::priority::classification::Severity::Medium => dist.medium += 1,
            crate::priority::classification::Severity::Low => dist.low += 1,
        }
    }

    dist
}

/// Calculates category counts.
fn calculate_category_counts(items: &[ViewItem]) -> CategoryCounts {
    let mut counts = CategoryCounts::default();

    for item in items {
        match item.category() {
            DebtCategory::Architecture => counts.architecture += 1,
            DebtCategory::Testing => counts.testing += 1,
            DebtCategory::Performance => counts.performance += 1,
            DebtCategory::CodeQuality => counts.code_quality += 1,
        }
    }

    counts
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
```

### Architecture Changes

**Before** (scattered logic):
```
get_top_mixed_priorities()
    ├── reads DEBTMAP_MIN_SCORE_THRESHOLD (I/O)
    ├── combines items
    ├── filters
    └── sorts

group_by_location() (TUI only)
    ├── groups by key
    ├── calculates combined score
    └── sorts groups

apply_filters() (markdown only)
    ├── takes N from items
    └── takes N from file_items (separately!)
```

**After** (unified pipeline):
```
prepare_view()  ← Single pure function
    ├── combine_items()       ← Pure
    ├── classify_all_tiers()  ← Pure
    ├── filter_items()        ← Pure
    ├── sort_items()          ← Pure
    ├── limit_items()         ← Pure
    ├── compute_groups()      ← Pure
    └── calculate_summary()   ← Pure
```

### Data Flow

```
UnifiedAnalysis
       │
       ▼
┌──────────────────┐
│  prepare_view()  │ ← ViewConfig, TierConfig (params, not env vars)
└──────────────────┘
       │
       ▼
PreparedDebtView
       │
       ├──→ TUI Renderer
       ├──→ Terminal Renderer
       ├──→ JSON Renderer
       └──→ Markdown Renderer
```

## Dependencies

- **Prerequisites**: Spec 250 (types must exist)
- **Affected Components**:
  - `src/priority/mod.rs` - Re-exports
  - `src/priority/tiers.rs` - `classify_tier` function
  - `src/priority/classification.rs` - `Severity` type
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Per Stage)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Stage 1: Combine
    #[test]
    fn test_combine_items_preserves_all() {
        let functions = create_test_functions(3);
        let files = create_test_files(2);

        let combined = combine_items(&functions, &files);

        assert_eq!(combined.len(), 5);
    }

    #[test]
    fn test_combine_items_empty() {
        let combined = combine_items(&im::Vector::new(), &im::Vector::new());
        assert!(combined.is_empty());
    }

    // Stage 3: Filter
    #[test]
    fn test_filter_by_score_threshold() {
        let items = vec![
            create_view_item(10.0),  // Above threshold
            create_view_item(2.0),   // Below threshold
            create_view_item(5.0),   // Above threshold
        ];
        let config = ViewConfig {
            min_score_threshold: 3.0,
            ..Default::default()
        };

        let (filtered, stats) = filter_items(items, &config);

        assert_eq!(filtered.len(), 2);
        assert_eq!(stats.filtered_by_score, 1);
    }

    #[test]
    fn test_filter_by_tier() {
        let items = vec![
            create_view_item_with_tier(50.0, RecommendationTier::T1Critical),
            create_view_item_with_tier(30.0, RecommendationTier::T4Maintenance),
            create_view_item_with_tier(40.0, RecommendationTier::T2High),
        ];
        let config = ViewConfig {
            min_score_threshold: 0.0,
            exclude_t4_maintenance: true,
            ..Default::default()
        };

        let (filtered, stats) = filter_items(items, &config);

        assert_eq!(filtered.len(), 2);
        assert_eq!(stats.filtered_by_tier, 1);
    }

    // Stage 4: Sort
    #[test]
    fn test_sort_by_score_descending() {
        let items = vec![
            create_view_item(30.0),
            create_view_item(50.0),
            create_view_item(10.0),
        ];

        let sorted = sort_items(items, SortCriteria::Score);

        assert_eq!(sorted[0].score(), 50.0);
        assert_eq!(sorted[1].score(), 30.0);
        assert_eq!(sorted[2].score(), 10.0);
    }

    // Stage 5: Limit
    #[test]
    fn test_limit_items() {
        let items = vec![
            create_view_item(50.0),
            create_view_item(40.0),
            create_view_item(30.0),
            create_view_item(20.0),
        ];

        let limited = limit_items(items, Some(2));

        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_limit_none_returns_all() {
        let items = vec![
            create_view_item(50.0),
            create_view_item(40.0),
        ];

        let limited = limit_items(items, None);

        assert_eq!(limited.len(), 2);
    }

    // Stage 6: Grouping
    #[test]
    fn test_compute_groups_combines_same_location() {
        let items = vec![
            create_view_item_at("file.rs", "func", 10, 30.0),
            create_view_item_at("file.rs", "func", 10, 20.0),
            create_view_item_at("other.rs", "func", 10, 50.0),
        ];

        let groups = compute_groups(&items, SortCriteria::Score);

        assert_eq!(groups.len(), 2);
        // First group should have combined score 50 (other.rs)
        assert_eq!(groups[0].combined_score, 50.0);
        // Second group should have combined score 50 (file.rs: 30+20)
        assert_eq!(groups[1].combined_score, 50.0);
    }

    // Full pipeline
    #[test]
    fn test_prepare_view_deterministic() {
        let analysis = create_test_analysis();
        let config = ViewConfig::default();
        let tier_config = TierConfig::default();

        let view1 = prepare_view(&analysis, &config, &tier_config);
        let view2 = prepare_view(&analysis, &config, &tier_config);

        assert_eq!(view1.items.len(), view2.items.len());
        for (a, b) in view1.items.iter().zip(view2.items.iter()) {
            assert_eq!(a.score(), b.score());
        }
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_filter_never_increases_count(
        items in prop::collection::vec(any::<ViewItem>(), 0..100),
        threshold in 0.0..100.0f64,
    ) {
        let config = ViewConfig {
            min_score_threshold: threshold,
            ..Default::default()
        };
        let original_count = items.len();

        let (filtered, _) = filter_items(items, &config);

        prop_assert!(filtered.len() <= original_count);
    }

    #[test]
    fn test_sort_preserves_count(
        items in prop::collection::vec(any::<ViewItem>(), 0..100),
    ) {
        let original_count = items.len();
        let sorted = sort_items(items, SortCriteria::Score);
        prop_assert_eq!(sorted.len(), original_count);
    }

    #[test]
    fn test_limit_respects_bound(
        items in prop::collection::vec(any::<ViewItem>(), 0..100),
        limit in 0usize..50,
    ) {
        let limited = limit_items(items, Some(limit));
        prop_assert!(limited.len() <= limit);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_prepare_view_matches_tui_behavior() {
    let analysis = load_test_analysis();

    let view = prepare_view_for_tui(&analysis);

    // Should include all items (no filtering)
    assert!(view.summary.filtered_by_score == 0);
    assert!(view.summary.filtered_by_tier == 0);

    // Should have groups
    assert!(!view.groups.is_empty());
}

#[test]
fn test_prepare_view_matches_terminal_behavior() {
    let analysis = load_test_analysis();

    let view = prepare_view_for_terminal(&analysis, Some(10));

    // Should have at most 10 items
    assert!(view.items.len() <= 10);

    // Should not have groups
    assert!(view.groups.is_empty());
}
```

## Documentation Requirements

### Code Documentation

All functions have rustdoc with:
- Pure function properties
- Arguments and returns
- Examples
- Performance characteristics

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## View Preparation Pipeline

The `prepare_view()` function is the single entry point for transforming
`UnifiedAnalysis` into displayable `PreparedDebtView`.

### Pipeline Stages

1. **combine_items** - Merge function + file items
2. **classify_tiers** - Assign recommendation tiers
3. **filter_items** - Apply score/tier filters
4. **sort_items** - Sort by criteria
5. **limit_items** - Apply optional limit
6. **compute_groups** - Create location groups
7. **calculate_summary** - Aggregate statistics

### Purity Guarantee

All stages are pure functions:
- No environment variable access
- No file I/O
- Deterministic results
- No side effects

Configuration is passed as parameters, not read from environment.
```

## Implementation Notes

### Refactoring Steps

1. Create `src/priority/view_pipeline.rs`
2. Implement each stage function
3. Add unit tests for each stage
4. Add re-exports to `src/priority/mod.rs`
5. Verify compilation
6. Add property-based tests
7. Add integration tests

### Design Decisions

1. **Clone on filter** - Filter takes ownership and returns new Vec for simplicity
2. **Mutable sort** - Sort in place for efficiency, then return
3. **HashMap for grouping** - O(n) grouping instead of O(n^2)
4. **Separate filter stats** - Track what was filtered for debugging

### Common Pitfalls

1. **Float comparison in sort** - Use `partial_cmp().unwrap_or()`
2. **Empty input handling** - All stages handle empty input gracefully
3. **Group sort consistency** - Groups sorted same as items

## Migration and Compatibility

### Breaking Changes

**None** - New code alongside existing. Integration in Spec 252.

### Migration Path

1. Spec 250: Create types
2. **Spec 251: Create pipeline** (this spec)
3. Spec 252: Update all output formats to use pipeline

## Success Metrics

- All stage functions pure (verified by code review)
- Unit tests pass for all stages
- Property tests pass
- Full pipeline is deterministic
- Performance within 10% of current code

## Follow-up Work

- Spec 252: Update all output formats to use this pipeline
- Deprecate `get_top_mixed_priorities()` after migration

## References

- **Stillwater PHILOSOPHY.md** - "Composition Over Complexity"
- **Spec 250** - Types consumed by this pipeline
- **Spec 183** - Similar pure/impure separation pattern
- **src/priority/unified_analysis_queries.rs** - Current logic to replace
- **src/tui/results/grouping.rs** - Current grouping logic to unify

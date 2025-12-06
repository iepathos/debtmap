---
number: 226
title: Composable Filter Pipeline with Functional Composition
category: optimization
priority: medium
status: draft
dependencies: [224, 225]
created: 2025-12-05
---

# Specification 226: Composable Filter Pipeline with Functional Composition

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 224 (Pure Tier Classification), Spec 225 (Filter Transparency)

## Context

The current filtering implementation in `unified_analysis_queries.rs` uses imperative code with mutation (`.sort_by()`, `.truncate()`), which violates Stillwater's functional programming principles. After implementing pure predicates (Spec 224) and filter metrics (Spec 225), we should complete the transformation by making the entire filtering pipeline functional.

**Current Issues:**

1. **Imperative Mutations** - Uses `sort_by()` and `truncate()` which modify in place
2. **Mixed Concerns** - Filtering, sorting, and limiting all in one function
3. **Not Composable** - Can't easily extend or modify filter pipeline
4. **Violates Stillwater** - Not following "Composition Over Complexity" principle

**Current Code Pattern:**

```rust
// src/priority/unified_analysis_queries.rs (lines 49-101)
pub fn get_top_mixed_priorities_tiered(&self, ...) -> Vec<UnifiedDebtItem> {
    let mut items = Vec::new();

    // Imperative collection + filtering
    for item in &self.items {
        let tier = tiers::classify_tier(&item, tier_config);

        if tier == RecommendationTier::T4Maintenance && !tier_config.show_t4_in_main_report {
            continue;  // Imperative control flow
        }

        if item.unified_score.final_score < min_score {
            continue;  // Imperative control flow
        }

        items.push(item.clone());  // Mutation
    }

    // Imperative sorting (mutation)
    items.sort_by(|a, b| {
        b.unified_score.final_score
            .partial_cmp(&a.unified_score.final_score)
            .unwrap_or(Ordering::Equal)
    });

    // Imperative truncation (mutation)
    items.truncate(limit);

    items
}
```

**Stillwater Philosophy Violation:**

From "Composition Over Complexity": Build complex behavior from simple, composable pieces. The current implementation is a monolithic function that does everything imperatively.

## Objective

Refactor filtering to a pure, functional, composable pipeline:

1. **Immutable transformations** - No mutation, only pure transformations
2. **Composable pipeline** - Chain operations via iterators
3. **Reusable filters** - Extract filter predicates for reuse
4. **Clear data flow** - Easy to see: classify → filter → sort → limit
5. **Maintain performance** - No performance regression from functional style

Result: Clean, functional filter pipeline that's easy to test, extend, and understand.

## Requirements

### Functional Requirements

1. **Pure Filter Functions**
   - All filters as pure predicate functions
   - No mutation in any filter operation
   - Composable via iterator chains
   - Each filter independently testable

2. **Immutable Pipeline**
   - No `sort_by()` mutation - use `.sorted()` or collect + sort
   - No `truncate()` mutation - use `.take()`
   - Return new collections, don't modify existing
   - Functional composition throughout

3. **Preserve Functionality**
   - Same filtering results as current implementation
   - Same sorting behavior (by score, descending)
   - Same limiting behavior
   - Backward compatible API

4. **Pipeline Composability**
   - Easy to add new filter stages
   - Easy to modify filter criteria
   - Easy to change sort order
   - Clear separation of concerns

### Non-Functional Requirements

1. **Performance**
   - No performance regression (< 5% variance)
   - Lazy evaluation where possible
   - Efficient iterator chains
   - Zero-cost abstractions

2. **Maintainability**
   - Clear pipeline structure
   - Self-documenting code
   - Easy to extend with new filters
   - Consistent functional patterns

3. **Testability**
   - Each stage independently testable
   - Property-based tests for pipeline
   - Easy to verify composition

## Acceptance Criteria

- [ ] All filtering operations use immutable transformations
- [ ] No mutation in filter pipeline (no `sort_by`, no `truncate`)
- [ ] Pure filter functions extracted to `src/priority/filters.rs`
- [ ] Pipeline uses iterator chains for composition
- [ ] Backward compatible API maintained
- [ ] All existing tests pass
- [ ] Performance benchmarks show < 5% variance
- [ ] Unit tests for each filter function
- [ ] Property tests for pipeline composition
- [ ] No clippy warnings
- [ ] Documentation updated with examples

## Technical Details

### Implementation Approach

**Phase 1: Extract Pure Filter Functions**

```rust
// src/priority/filters.rs

/// Pure filter predicates for debt items.
/// Each filter is a pure function that can be composed.

/// Filters items by tier visibility (pure).
pub fn tier_visible(tier: RecommendationTier, show_t4: bool) -> bool {
    tier != RecommendationTier::T4Maintenance || show_t4
}

/// Filters items by score threshold (pure).
pub fn score_above_threshold(score: f64, threshold: f64) -> bool {
    score >= threshold
}

/// Filters items by debt type (pure).
pub fn debt_type_enabled(debt_type: &DebtType, enabled_types: &HashSet<DebtType>) -> bool {
    enabled_types.is_empty() || enabled_types.contains(debt_type)
}

/// Combines all filter predicates (pure composition).
pub fn passes_all_filters(
    item: &ClassifiedItem,
    config: &FilterConfig,
) -> bool {
    tier_visible(item.tier, config.show_t4)
        && score_above_threshold(item.score, config.min_score)
        && debt_type_enabled(&item.debt_type, &config.enabled_debt_types)
}
```

**Phase 2: Create Immutable Pipeline Functions**

```rust
// src/priority/pipeline.rs

use std::cmp::Ordering;

/// Sorts items by score (descending, pure).
///
/// Creates new sorted vector, doesn't mutate input.
pub fn sort_by_score(mut items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem> {
    items.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
    });
    items
}

/// Sorts items by score using functional approach (pure).
///
/// Alternative implementation using iterator collect for clarity.
pub fn sort_by_score_functional(items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem> {
    let mut sorted: Vec<_> = items.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
    });
    sorted
}

/// Limits items to top N (pure).
pub fn take_top(items: Vec<ClassifiedItem>, limit: usize) -> Vec<ClassifiedItem> {
    items.into_iter().take(limit).collect()
}

/// Complete filter pipeline (pure, composed).
///
/// classify → filter → sort → limit
pub fn analyze_and_filter(
    items: Vec<UnifiedDebtItem>,
    tier_config: &TierConfig,
    filter_config: &FilterConfig,
    limit: usize,
) -> FilterResult {
    // Stage 1: Classify (pure, from Spec 224)
    let classified: Vec<_> = items
        .into_iter()
        .map(|item| classify_item(item, tier_config))
        .collect();

    let total = classified.len();

    // Stage 2: Filter with metrics (pure, from Spec 225)
    let filtered = filter_with_metrics(classified, filter_config);

    // Stage 3: Sort (pure)
    let sorted = sort_by_score(filtered.included);

    // Stage 4: Limit (pure)
    let limited = take_top(sorted, limit);

    // Return result with metrics
    FilterResult {
        included: limited,
        metrics: filtered.metrics,
    }
}
```

**Phase 3: Iterator-Based Pipeline (Alternative)**

```rust
// src/priority/pipeline.rs

/// Complete pipeline using iterator chains (lazy, efficient).
pub fn analyze_and_filter_lazy(
    items: impl Iterator<Item = UnifiedDebtItem>,
    tier_config: &TierConfig,
    filter_config: &FilterConfig,
    limit: usize,
) -> Vec<ClassifiedItem> {
    items
        // Stage 1: Classify (lazy)
        .map(|item| classify_item(item, tier_config))
        // Stage 2: Filter by tier (lazy)
        .filter(|item| tier_visible(item.tier, filter_config.show_t4))
        // Stage 3: Filter by score (lazy)
        .filter(|item| score_above_threshold(item.score, filter_config.min_score))
        // Stage 4: Collect for sorting (materialization point)
        .collect::<Vec<_>>()
        // Stage 5: Sort (creates sorted vec)
        |> sort_by_score
        // Stage 6: Limit (lazy again)
        .into_iter()
        .take(limit)
        .collect()
}
```

**Phase 4: Update Queries to Use Pipeline**

```rust
// src/priority/unified_analysis_queries.rs

use crate::priority::pipeline::analyze_and_filter;

impl UnifiedAnalysisData {
    /// Gets top mixed priorities using functional pipeline.
    ///
    /// Pure, composable approach: classify → filter → sort → limit
    pub fn get_top_mixed_priorities_tiered(
        &self,
        limit: usize,
        tier_config: &TierConfig,
        min_score: f64,
    ) -> Vec<UnifiedDebtItem> {
        let filter_config = FilterConfig {
            min_score,
            show_t4: tier_config.show_t4_in_main_report,
            enabled_debt_types: HashSet::new(),
        };

        // Functional pipeline (pure, composable)
        analyze_and_filter(
            self.items.clone(),  // TODO: Consider borrowing
            tier_config,
            &filter_config,
            limit,
        )
        .included
        .into_iter()
        .map(|c| c.item)
        .collect()
    }

    /// Gets top priorities with metrics (from Spec 225).
    pub fn get_top_mixed_priorities_with_metrics(
        &self,
        limit: usize,
        tier_config: &TierConfig,
        min_score: f64,
    ) -> FilterResult {
        let filter_config = FilterConfig {
            min_score,
            show_t4: tier_config.show_t4_in_main_report,
            enabled_debt_types: HashSet::new(),
        };

        // Functional pipeline with metrics
        analyze_and_filter(
            self.items.clone(),
            tier_config,
            &filter_config,
            limit,
        )
    }
}
```

**Phase 5: Add Pipeline Extension Points**

```rust
// src/priority/pipeline.rs

/// Pipeline stage for custom transformations (functional).
pub trait PipelineStage {
    fn apply(&self, items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem>;
}

/// Custom filter stage (example).
pub struct CustomFilter<F>
where
    F: Fn(&ClassifiedItem) -> bool,
{
    predicate: F,
}

impl<F> PipelineStage for CustomFilter<F>
where
    F: Fn(&ClassifiedItem) -> bool,
{
    fn apply(&self, items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem> {
        items.into_iter().filter(&self.predicate).collect()
    }
}

/// Composable pipeline builder (functional).
pub struct Pipeline {
    stages: Vec<Box<dyn PipelineStage>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(mut self, stage: Box<dyn PipelineStage>) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn execute(self, items: Vec<ClassifiedItem>) -> Vec<ClassifiedItem> {
        self.stages
            .into_iter()
            .fold(items, |items, stage| stage.apply(items))
    }
}
```

### Comparison: Before vs After

**Before (Imperative, Mutation):**

```rust
pub fn get_top_mixed_priorities_tiered(&self, ...) -> Vec<UnifiedDebtItem> {
    let mut items = Vec::new();

    // Imperative loop + mutation
    for item in &self.items {
        let tier = tiers::classify_tier(&item, tier_config);
        if tier == RecommendationTier::T4Maintenance && !tier_config.show_t4_in_main_report {
            continue;
        }
        if item.unified_score.final_score < min_score {
            continue;
        }
        items.push(item.clone());
    }

    // Mutation
    items.sort_by(|a, b| b.unified_score.final_score.partial_cmp(&a.unified_score.final_score).unwrap_or(Ordering::Equal));
    items.truncate(limit);

    items
}
```

**After (Functional, Immutable):**

```rust
pub fn get_top_mixed_priorities_tiered(&self, ...) -> Vec<UnifiedDebtItem> {
    let filter_config = FilterConfig {
        min_score,
        show_t4: tier_config.show_t4_in_main_report,
        enabled_debt_types: HashSet::new(),
    };

    // Functional pipeline (pure, composable)
    analyze_and_filter(self.items.clone(), tier_config, &filter_config, limit)
        .included
        .into_iter()
        .map(|c| c.item)
        .collect()
}
```

**Benefits:**
- 18 lines → 9 lines
- Clear data flow (classify → filter → sort → limit)
- No mutation
- Each stage independently testable
- Easy to extend with new stages

## Dependencies

- **Prerequisites**:
  - Spec 224 (Extract Pure Tier Classification) - provides `classify_item()`
  - Spec 225 (Filter Transparency) - provides `filter_with_metrics()`
- **Affected Components**:
  - `src/priority/filters.rs` (new - pure filter functions)
  - `src/priority/pipeline.rs` (new - pipeline composition)
  - `src/priority/unified_analysis_queries.rs` (refactor to use pipeline)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Filter Functions)

```rust
#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_tier_visible_shows_t4_when_enabled() {
        assert!(tier_visible(RecommendationTier::T4Maintenance, true));
    }

    #[test]
    fn test_tier_visible_hides_t4_by_default() {
        assert!(!tier_visible(RecommendationTier::T4Maintenance, false));
    }

    #[test]
    fn test_tier_visible_always_shows_other_tiers() {
        assert!(tier_visible(RecommendationTier::T1CriticalArchitecture, false));
        assert!(tier_visible(RecommendationTier::T2ComplexUntested, false));
        assert!(tier_visible(RecommendationTier::T3TestingGaps, false));
    }

    #[test]
    fn test_score_above_threshold() {
        assert!(score_above_threshold(5.0, 3.0));
        assert!(score_above_threshold(3.0, 3.0));  // Inclusive
        assert!(!score_above_threshold(2.9, 3.0));
    }
}
```

### Unit Tests (Pipeline Functions)

```rust
#[cfg(test)]
mod pipeline_tests {
    use super::*;

    #[test]
    fn test_sort_by_score_descending() {
        let items = vec![
            create_item(score: 50.0),
            create_item(score: 95.0),
            create_item(score: 70.0),
        ];

        let sorted = sort_by_score(items);

        assert_eq!(sorted[0].score, 95.0);
        assert_eq!(sorted[1].score, 70.0);
        assert_eq!(sorted[2].score, 50.0);
    }

    #[test]
    fn test_take_top_limits_correctly() {
        let items = vec![
            create_item(1),
            create_item(2),
            create_item(3),
            create_item(4),
        ];

        let top = take_top(items, 2);

        assert_eq!(top.len(), 2);
    }

    #[test]
    fn test_sort_is_immutable() {
        let original = vec![
            create_item(score: 50.0),
            create_item(score: 95.0),
        ];

        let original_first_score = original[0].score;
        let _ = sort_by_score(original.clone());

        // Original unchanged
        assert_eq!(original[0].score, original_first_score);
    }
}
```

### Integration Tests (Full Pipeline)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_pipeline_produces_correct_results() {
        let items = vec![
            create_item(tier: T1, score: 95.0),  // Should be #1
            create_item(tier: T4, score: 90.0),  // Filtered (T4)
            create_item(tier: T2, score: 2.0),   // Filtered (score)
            create_item(tier: T2, score: 85.0),  // Should be #2
            create_item(tier: T1, score: 80.0),  // Should be #3
        ];

        let filter_config = FilterConfig {
            min_score: 3.0,
            show_t4: false,
            ..default()
        };

        let result = analyze_and_filter(
            items,
            &TierConfig::default(),
            &filter_config,
            10,
        );

        assert_eq!(result.included.len(), 3);
        assert_eq!(result.included[0].score, 95.0);
        assert_eq!(result.included[1].score, 85.0);
        assert_eq!(result.included[2].score, 80.0);
    }

    #[test]
    fn test_pipeline_respects_limit() {
        let items = create_many_items(100);

        let result = analyze_and_filter(
            items,
            &TierConfig::default(),
            &FilterConfig::default(),
            10,
        );

        assert_eq!(result.included.len(), 10);
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_pipeline_is_deterministic(
        items in prop::collection::vec(any::<UnifiedDebtItem>(), 0..100),
    ) {
        let result1 = analyze_and_filter(
            items.clone(),
            &TierConfig::default(),
            &FilterConfig::default(),
            10,
        );
        let result2 = analyze_and_filter(
            items,
            &TierConfig::default(),
            &FilterConfig::default(),
            10,
        );

        prop_assert_eq!(result1.included.len(), result2.included.len());
        for (item1, item2) in result1.included.iter().zip(result2.included.iter()) {
            prop_assert_eq!(item1.score, item2.score);
        }
    }

    #[test]
    fn test_sort_maintains_order_invariant(
        items in prop::collection::vec(any::<ClassifiedItem>(), 0..100),
    ) {
        let sorted = sort_by_score(items);

        // Verify descending order
        for window in sorted.windows(2) {
            prop_assert!(window[0].score >= window[1].score);
        }
    }

    #[test]
    fn test_filter_never_increases_count(
        items in prop::collection::vec(any::<ClassifiedItem>(), 0..100),
    ) {
        let original_count = items.len();
        let filtered = filter_with_metrics(items, &FilterConfig::default());

        prop_assert!(filtered.included.len() <= original_count);
    }
}
```

### Performance Tests

```rust
#[test]
fn test_pipeline_performance() {
    let items: Vec<_> = (0..10000)
        .map(|i| create_test_item(i))
        .collect();

    // Imperative approach (baseline)
    let start = Instant::now();
    let _ = imperative_filter(&items);
    let imperative_time = start.elapsed();

    // Functional pipeline
    let start = Instant::now();
    let _ = analyze_and_filter(items, &TierConfig::default(), &FilterConfig::default(), 100);
    let pipeline_time = start.elapsed();

    // Should be within 5% variance
    let variance = (pipeline_time.as_secs_f64() / imperative_time.as_secs_f64()) - 1.0;
    assert!(variance < 0.05, "Performance regression: {:.1}%", variance * 100.0);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Complete filter pipeline using functional composition.
///
/// Transforms debt items through these pure stages:
/// 1. Classify - Assign tier to each item
/// 2. Filter - Remove items by tier/score/type
/// 3. Sort - Order by score (descending)
/// 4. Limit - Take top N items
///
/// All stages are pure functions with no side effects.
///
/// # Arguments
///
/// * `items` - Raw debt items to process
/// * `tier_config` - Tier classification configuration
/// * `filter_config` - Filter criteria (thresholds, enabled types)
/// * `limit` - Maximum items to return
///
/// # Returns
///
/// FilterResult containing filtered items and metrics
///
/// # Examples
///
/// ```
/// let result = analyze_and_filter(
///     debt_items,
///     &TierConfig::default(),
///     &FilterConfig { min_score: 3.0, ..default() },
///     50,
/// );
/// println!("Included {} items", result.included.len());
/// ```
pub fn analyze_and_filter(
    items: Vec<UnifiedDebtItem>,
    tier_config: &TierConfig,
    filter_config: &FilterConfig,
    limit: usize,
) -> FilterResult {
    // ...
}
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Filter Pipeline Architecture

### Functional Composition

The filter pipeline uses pure functional composition:

```
Items → Classify → Filter → Sort → Limit → Result
         ↓          ↓        ↓      ↓
       Pure      Pure    Pure   Pure
```

Each stage:
- Pure function (no side effects)
- Immutable transformations
- Independently testable
- Composable with other stages

### Pipeline Stages

1. **Classify** (`classify_item`): Assigns tier based on metrics
2. **Filter** (`filter_with_metrics`): Removes items by criteria
3. **Sort** (`sort_by_score`): Orders by score (descending)
4. **Limit** (`take_top`): Takes top N items

### Extension

Add new pipeline stages:

```rust
let custom_pipeline = Pipeline::new()
    .add_stage(Box::new(ClassifyStage))
    .add_stage(Box::new(FilterStage))
    .add_stage(Box::new(CustomStage))  // Your stage
    .add_stage(Box::new(SortStage))
    .add_stage(Box::new(LimitStage));

let results = custom_pipeline.execute(items);
```
```

## Implementation Notes

### Design Decisions

**Why not lazy evaluation everywhere?**
- Sorting requires materialization (full collection)
- Metrics collection needs full pass
- Benchmark showed minimal benefit vs code clarity

**Why `sort_by()` instead of fully functional sort?**
- Rust's `sort_by()` is highly optimized
- Creating new vec is minimal cost
- Pragmatism over purity (Stillwater principle)

**Why separate `filters.rs` and `pipeline.rs`?**
- Clear module boundaries
- Filters are predicates (reusable)
- Pipeline is composition (orchestration)

### Common Pitfalls

1. **Forgetting to collect** - Iterator chains need materialization
2. **Cloning unnecessarily** - Consider borrowing where possible
3. **Over-abstraction** - Keep pipeline simple, don't over-engineer

## Migration and Compatibility

### Breaking Changes

**None** - Public API unchanged. Internal refactoring only.

### Migration Steps

No user or developer migration needed. Internal improvement only.

## Success Metrics

- ✅ No mutation in filter pipeline
- ✅ All stages use immutable transformations
- ✅ Iterator chains for composition
- ✅ All existing tests pass
- ✅ Performance < 5% variance from current implementation
- ✅ Unit tests for all filter functions
- ✅ Property tests verify pipeline invariants
- ✅ No clippy warnings
- ✅ Documentation shows clear pipeline structure

## References

- **Stillwater PHILOSOPHY.md** - Composition Over Complexity
- **Spec 224** - Extract Pure Tier Classification (dependency)
- **Spec 225** - Add Filter Transparency (dependency)
- **Rust Iterators** - Lazy evaluation patterns

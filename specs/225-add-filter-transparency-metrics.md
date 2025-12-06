---
number: 225
title: Add Filter Transparency with Metrics
category: foundation
priority: high
status: draft
dependencies: [224]
created: 2025-12-05
---

# Specification 225: Add Filter Transparency with Metrics

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 224 (Extract Pure Tier Classification)

## Context

The tiering system currently filters out T4 (Maintenance) items and low-score items silently, with no visibility into what was filtered or why. This creates a poor debugging experience and makes it difficult for users to understand why certain debt items don't appear in the output.

**Current Issues:**

1. **Silent Filtering** - Users don't know items were filtered (`unified_analysis_queries.rs:68-76`)
2. **No Metrics** - Can't see how many T4 items exist, how many below threshold, etc.
3. **Poor Debuggability** - Hard to determine if filtering is working correctly
4. **Violates "Errors Should Tell Stories"** - Filtering decisions are invisible

**Current Code Pattern:**

```rust
// src/priority/unified_analysis_queries.rs (lines 68-76)
// Filter out Tier 4 items unless explicitly requested (spec: reduce spam)
if tier == RecommendationTier::T4Maintenance && !tier_config.show_t4_in_main_report {
    continue;  // Silent drop, no record
}

// Filter out items below score threshold (spec 193)
if item.unified_score.final_score < min_score {
    continue;  // Silent drop, no record
}
```

**User Experience Problem:**

```
$ debtmap analyze .
Found 15 debt items

# User questions:
# - Were there more items that got filtered?
# - How many T4 items exist but aren't shown?
# - What's the score distribution?
# - Is filtering working correctly?
```

**Stillwater Philosophy Violation:**

From "Errors Should Tell Stories": Context should accumulate along the way. Users should understand what happened and why.

## Objective

Make filtering decisions transparent and debuggable by adding filter metrics:

1. **Track filtering decisions** - Record what was filtered and why
2. **Expose metrics** - Make filter stats available to formatters
3. **Display option** - Add `--show-filter-stats` CLI flag
4. **Maintain backward compatibility** - Metrics are optional, default behavior unchanged
5. **Pure implementation** - Use functional approach consistent with Spec 224

Result: Users can see "Filtered 47 T4 items, 12 below score threshold" and understand what's being hidden.

## Requirements

### Functional Requirements

1. **FilterMetrics Structure**
   - Track total items analyzed
   - Track items filtered by tier (T4)
   - Track items filtered by score threshold
   - Track items filtered by debt type (if applicable)
   - Track items included in output
   - Pure, immutable data structure

2. **Metrics Collection**
   - Collect metrics during filtering process
   - No mutation - use functional accumulation
   - No performance impact (< 5% overhead)
   - Works with existing filter logic

3. **CLI Integration**
   - Add `--show-filter-stats` flag
   - Optional display (off by default)
   - Backward compatible (no behavior change when off)
   - Clear, concise output format

4. **Formatter Integration**
   - Pass FilterMetrics to markdown formatter
   - Display stats in dedicated section
   - Show tier distribution
   - Show filtering summary

### Non-Functional Requirements

1. **Performance**
   - Metrics collection adds < 5% overhead
   - No additional memory allocations for common case
   - Efficient tracking during iteration

2. **Maintainability**
   - Clear data structures
   - Easy to extend with new filter types
   - Consistent with pure functional approach from Spec 224

3. **User Experience**
   - Clear, scannable output format
   - Helpful for debugging filtering issues
   - Not overwhelming (summary only, not every item)

## Acceptance Criteria

- [ ] `FilterMetrics` struct created in new `src/priority/filtering.rs` module
- [ ] `filter_with_metrics()` function implemented (pure, functional)
- [ ] `--show-filter-stats` CLI flag added
- [ ] Markdown formatter displays filter metrics when enabled
- [ ] Filter metrics show: total, filtered by tier, filtered by score, included
- [ ] Backward compatible (default behavior unchanged)
- [ ] Performance overhead < 5% (benchmark verification)
- [ ] Unit tests for FilterMetrics creation and display
- [ ] Integration tests verify metrics accuracy
- [ ] Documentation updated with examples
- [ ] All existing tests pass

## Technical Details

### Implementation Approach

**Phase 1: Create FilterMetrics Data Structure**

```rust
// src/priority/filtering.rs

/// Metrics tracking filtering decisions.
///
/// Pure, immutable data structure that records what was filtered and why.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterMetrics {
    /// Total items before filtering
    pub total_items: usize,

    /// Items filtered because they're T4 Maintenance tier
    pub filtered_t4_maintenance: usize,

    /// Items filtered because score below threshold
    pub filtered_below_score: usize,

    /// Items filtered because debt type disabled
    pub filtered_by_debt_type: usize,

    /// Items included in final output
    pub included: usize,

    /// Minimum score threshold used
    pub min_score_threshold: f64,

    /// Whether T4 items were shown
    pub show_t4: bool,
}

impl FilterMetrics {
    /// Creates empty metrics.
    pub fn empty() -> Self {
        Self {
            total_items: 0,
            filtered_t4_maintenance: 0,
            filtered_below_score: 0,
            filtered_by_debt_type: 0,
            included: 0,
            min_score_threshold: 0.0,
            show_t4: false,
        }
    }

    /// Creates metrics from configuration.
    pub fn new(total: usize, min_score: f64, show_t4: bool) -> Self {
        Self {
            total_items: total,
            min_score_threshold: min_score,
            show_t4,
            ..Self::empty()
        }
    }

    /// Total items filtered (all reasons).
    pub fn total_filtered(&self) -> usize {
        self.filtered_t4_maintenance
            + self.filtered_below_score
            + self.filtered_by_debt_type
    }

    /// Percentage of items included.
    pub fn inclusion_rate(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            (self.included as f64 / self.total_items as f64) * 100.0
        }
    }
}
```

**Phase 2: Create Filter Result Type**

```rust
// src/priority/filtering.rs

/// Result of filtering with transparency metrics.
///
/// Pure data structure containing filtered items and metrics about filtering.
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// Items that passed all filters
    pub included: Vec<ClassifiedItem>,

    /// Metrics about what was filtered
    pub metrics: FilterMetrics,
}

impl FilterResult {
    /// Creates new filter result.
    pub fn new(included: Vec<ClassifiedItem>, metrics: FilterMetrics) -> Self {
        Self { included, metrics }
    }

    /// Creates empty result.
    pub fn empty() -> Self {
        Self {
            included: Vec::new(),
            metrics: FilterMetrics::empty(),
        }
    }
}
```

**Phase 3: Implement Pure Filtering with Metrics**

```rust
// src/priority/filtering.rs

use super::tiers::predicates::*;

/// Filters items with metric collection (pure, functional).
///
/// This is a pure function that partitions items and tracks filtering decisions.
/// No side effects, fully deterministic.
pub fn filter_with_metrics(
    items: Vec<ClassifiedItem>,
    config: &FilterConfig,
) -> FilterResult {
    let total = items.len();
    let mut metrics = FilterMetrics::new(total, config.min_score, config.show_t4);

    // Partition items into included/excluded (pure)
    let included: Vec<_> = items
        .into_iter()
        .filter(|item| {
            // Track why items are filtered
            if !tier_passes(item.tier, config) {
                metrics.filtered_t4_maintenance += 1;
                return false;
            }

            if !score_passes(item.score, config.min_score) {
                metrics.filtered_below_score += 1;
                return false;
            }

            if !debt_type_enabled(&item.debt_type, config) {
                metrics.filtered_by_debt_type += 1;
                return false;
            }

            true
        })
        .collect();

    metrics.included = included.len();

    FilterResult::new(included, metrics)
}

/// Checks if tier should be included based on config (pure).
fn tier_passes(tier: RecommendationTier, config: &FilterConfig) -> bool {
    tier != RecommendationTier::T4Maintenance || config.show_t4
}

/// Checks if score passes threshold (pure).
fn score_passes(score: f64, threshold: f64) -> bool {
    score >= threshold
}

/// Checks if debt type is enabled (pure).
fn debt_type_enabled(debt_type: &DebtType, config: &FilterConfig) -> bool {
    config.enabled_debt_types.is_empty()
        || config.enabled_debt_types.contains(debt_type)
}
```

**Phase 4: Update Queries to Use FilterMetrics**

```rust
// src/priority/unified_analysis_queries.rs

use crate::priority::filtering::{filter_with_metrics, FilterResult};

impl UnifiedAnalysisData {
    /// Gets top mixed priorities with filtering transparency.
    pub fn get_top_mixed_priorities_with_metrics(
        &self,
        limit: usize,
        tier_config: &TierConfig,
        min_score: f64,
    ) -> FilterResult {
        // Classify all items (pure, from Spec 224)
        let classified: Vec<ClassifiedItem> = self.items
            .iter()
            .map(|item| classify_item(item, tier_config))
            .collect();

        // Filter with metrics (pure)
        let config = FilterConfig {
            min_score,
            show_t4: tier_config.show_t4_in_main_report,
            enabled_debt_types: HashSet::new(),  // All enabled
        };

        let mut result = filter_with_metrics(classified, &config);

        // Sort by score (creates new vec, pure)
        result.included.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)
        });

        // Limit results (pure)
        result.included.truncate(limit);

        result
    }

    /// Backward compatible method (no metrics exposed).
    pub fn get_top_mixed_priorities_tiered(
        &self,
        limit: usize,
        tier_config: &TierConfig,
        min_score: f64,
    ) -> Vec<UnifiedDebtItem> {
        self.get_top_mixed_priorities_with_metrics(limit, tier_config, min_score)
            .included
            .into_iter()
            .map(|c| c.item)
            .collect()
    }
}
```

**Phase 5: Add CLI Flag**

```rust
// src/cli.rs

#[derive(Parser)]
pub struct AnalyzeCommand {
    // ... existing fields ...

    /// Show filter statistics (how many items filtered and why)
    #[arg(long = "show-filter-stats")]
    pub show_filter_stats: bool,
}
```

**Phase 6: Update Formatter**

```rust
// src/priority/formatter_markdown/mod.rs

/// Formats filter metrics for display.
pub fn format_filter_metrics(metrics: &FilterMetrics) -> String {
    let mut output = String::new();

    output.push_str("## Filtering Summary\n\n");
    output.push_str(&format!("- **Total items analyzed**: {}\n", metrics.total_items));
    output.push_str(&format!("- **Items included**: {} ({:.1}%)\n",
        metrics.included,
        metrics.inclusion_rate()
    ));
    output.push_str(&format!("- **Items filtered**: {}\n\n", metrics.total_filtered()));

    if metrics.total_filtered() > 0 {
        output.push_str("### Filtering Breakdown\n\n");

        if metrics.filtered_t4_maintenance > 0 {
            output.push_str(&format!(
                "- **Low-priority (T4)**: {} items (use `--min-score 0` to see)\n",
                metrics.filtered_t4_maintenance
            ));
        }

        if metrics.filtered_below_score > 0 {
            output.push_str(&format!(
                "- **Below score threshold**: {} items (threshold: {:.1})\n",
                metrics.filtered_below_score,
                metrics.min_score_threshold
            ));
        }

        if metrics.filtered_by_debt_type > 0 {
            output.push_str(&format!(
                "- **Disabled debt types**: {} items\n",
                metrics.filtered_by_debt_type
            ));
        }
    }

    output.push('\n');
    output
}

/// Main formatting function (updated to accept metrics).
pub fn format_priorities_tiered_markdown(
    items: &[UnifiedDebtItem],
    metrics: Option<&FilterMetrics>,  // Optional for backward compat
    config: &FormatterConfig,
) -> String {
    let mut output = String::new();

    // ... existing formatting ...

    // Show filter metrics if provided and enabled
    if let Some(metrics) = metrics {
        if config.show_filter_stats {
            output.push_str(&format_filter_metrics(metrics));
        }
    }

    // ... rest of formatting ...

    output
}
```

### Example Output

**With `--show-filter-stats`:**

```markdown
# Debtmap Analysis Results

## Filtering Summary

- **Total items analyzed**: 127
- **Items included**: 23 (18.1%)
- **Items filtered**: 104

### Filtering Breakdown

- **Low-priority (T4)**: 89 items (use `--min-score 0` to see)
- **Below score threshold**: 15 items (threshold: 3.0)

## Top Priority Items

1. **GodObject**: UserManager (score: 95.2)
   ...
```

**Without `--show-filter-stats` (default):**

```markdown
# Debtmap Analysis Results

## Top Priority Items

1. **GodObject**: UserManager (score: 95.2)
   ...
```

## Dependencies

- **Prerequisites**: Spec 224 (Extract Pure Tier Classification)
- **Affected Components**:
  - `src/priority/filtering.rs` (new)
  - `src/priority/unified_analysis_queries.rs` (add metrics methods)
  - `src/priority/formatter_markdown/mod.rs` (display metrics)
  - `src/cli.rs` (add flag)
- **External Dependencies**: None
- **Enables**: Spec 226 (Composable Filter Pipeline)

## Testing Strategy

### Unit Tests (FilterMetrics)

```rust
#[cfg(test)]
mod filter_metrics_tests {
    use super::*;

    #[test]
    fn test_empty_metrics() {
        let m = FilterMetrics::empty();
        assert_eq!(m.total_items, 0);
        assert_eq!(m.total_filtered(), 0);
        assert_eq!(m.inclusion_rate(), 0.0);
    }

    #[test]
    fn test_inclusion_rate() {
        let m = FilterMetrics {
            total_items: 100,
            included: 25,
            ..FilterMetrics::empty()
        };
        assert_eq!(m.inclusion_rate(), 25.0);
    }

    #[test]
    fn test_total_filtered() {
        let m = FilterMetrics {
            filtered_t4_maintenance: 10,
            filtered_below_score: 5,
            filtered_by_debt_type: 3,
            ..FilterMetrics::empty()
        };
        assert_eq!(m.total_filtered(), 18);
    }
}
```

### Integration Tests (Filtering with Metrics)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_filter_with_metrics_accuracy() {
        let items = vec![
            create_t1_item(score: 95.0),  // Included
            create_t4_item(score: 85.0),  // Filtered (T4)
            create_t2_item(score: 2.0),   // Filtered (score)
            create_t1_item(score: 90.0),  // Included
        ];

        let config = FilterConfig {
            min_score: 3.0,
            show_t4: false,
            ..default()
        };

        let result = filter_with_metrics(items, &config);

        assert_eq!(result.included.len(), 2);
        assert_eq!(result.metrics.total_items, 4);
        assert_eq!(result.metrics.filtered_t4_maintenance, 1);
        assert_eq!(result.metrics.filtered_below_score, 1);
        assert_eq!(result.metrics.included, 2);
    }

    #[test]
    fn test_show_t4_includes_all_tiers() {
        let items = vec![
            create_t1_item(score: 95.0),
            create_t4_item(score: 85.0),
            create_t2_item(score: 70.0),
        ];

        let config = FilterConfig {
            min_score: 0.0,
            show_t4: true,
            ..default()
        };

        let result = filter_with_metrics(items, &config);

        assert_eq!(result.included.len(), 3);
        assert_eq!(result.metrics.filtered_t4_maintenance, 0);
    }
}
```

### Performance Tests

```rust
#[test]
fn test_metrics_overhead_acceptable() {
    let items: Vec<_> = (0..10000)
        .map(|i| create_test_item(i))
        .collect();

    // Without metrics
    let start = Instant::now();
    let _ = filter_without_metrics(&items, &config);
    let without_metrics = start.elapsed();

    // With metrics
    let start = Instant::now();
    let _ = filter_with_metrics(items, &config);
    let with_metrics = start.elapsed();

    // Overhead should be < 5%
    let overhead = (with_metrics.as_secs_f64() / without_metrics.as_secs_f64()) - 1.0;
    assert!(overhead < 0.05, "Overhead too high: {:.1}%", overhead * 100.0);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Filters items with metric collection.
///
/// Pure function that partitions items based on filtering criteria and
/// tracks metrics about filtering decisions. No side effects.
///
/// # Arguments
///
/// * `items` - Classified items to filter
/// * `config` - Filter configuration (thresholds, enabled tiers)
///
/// # Returns
///
/// FilterResult containing included items and filtering metrics
///
/// # Examples
///
/// ```
/// let items = classify_all_items(&debt_items);
/// let result = filter_with_metrics(items, &config);
/// println!("Included {} of {} items", result.metrics.included, result.metrics.total_items);
/// ```
pub fn filter_with_metrics(
    items: Vec<ClassifiedItem>,
    config: &FilterConfig,
) -> FilterResult {
    // ...
}
```

### User Documentation

Update README.md:

```markdown
## Filtering Transparency

By default, debtmap filters out low-priority items. To see what was filtered:

```bash
debtmap analyze . --show-filter-stats
```

This shows:
- Total items analyzed
- Items included in output
- Items filtered by tier (T4 maintenance)
- Items filtered by score threshold
- Inclusion percentage

To see ALL items (including T4):

```bash
debtmap analyze . --min-score 0
```
```

## Implementation Notes

### Design Decisions

**Why separate FilterResult type?**
- Cleanly separates filtered data from metadata
- Makes metrics optional for backward compatibility
- Follows functional composition pattern

**Why track metrics during filtering?**
- Single-pass collection (efficient)
- Accurate counts (no post-hoc calculation)
- Natural fit with functional filter chain

**Why default to hidden?**
- Backward compatibility
- Most users don't need metrics
- Opt-in discovery feature

### Common Pitfalls

1. **Double counting** - Ensure items only counted in ONE filter category
2. **Mutation** - Use functional accumulation, not mutation
3. **Performance** - Keep tracking lightweight (simple counters)

## Migration and Compatibility

### Breaking Changes

**None** - All changes are additive:
- New flag is optional
- New methods alongside existing ones
- Metrics display is opt-in

### Migration Steps

No user or developer migration needed. Purely additive feature.

## Success Metrics

- ✅ FilterMetrics tracks all filtering decisions accurately
- ✅ `--show-filter-stats` CLI flag works
- ✅ Markdown formatter displays clear metrics summary
- ✅ Backward compatible (default behavior unchanged)
- ✅ Performance overhead < 5%
- ✅ Unit tests for FilterMetrics (100% coverage)
- ✅ Integration tests verify metric accuracy
- ✅ All existing tests pass

## References

- **Stillwater PHILOSOPHY.md** - "Errors Should Tell Stories" principle
- **Spec 224** - Extract Pure Tier Classification (dependency)
- **User feedback** - Request for filtering visibility

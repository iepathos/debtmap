---
number: 205
title: Tier-Aware Filtering for Critical Architecture Items
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 205: Tier-Aware Filtering for Critical Architecture Items

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently filters technical debt items using two orthogonal classification systems:

1. **Tier System** (Architectural Importance):
   - T1 Critical Architecture: Error handling, god objects, extreme complexity
   - T2 Complex Untested: Complex code without test coverage, entry points
   - T3 Testing Gaps: Functions needing test coverage
   - T4 Maintenance: Low-priority refactoring and cleanup

2. **Severity System** (Score-Based):
   - Critical: score ≥ 8.0
   - High: score ≥ 6.0
   - Medium: score ≥ 4.0
   - Low: score < 4.0

The filtering logic (`src/priority/filtering.rs`) currently applies **score-based filtering uniformly** to all items, including T1 Critical Architecture items. This creates a semantic mismatch:

**Problem:** Error swallowing patterns are correctly classified as **T1 Critical Architecture** based on architectural importance, but 6 out of 7 patterns are **hidden by default** because their scores fall below the 3.0 threshold:

```rust
// Error swallowing scoring (src/priority/debt_aggregator.rs:172-195)
Priority::High → score 2.0 × Resource weight 2.5 = 5.0  ✅ Visible
Priority::Medium → score 1.0 × Resource weight 2.5 = 2.5  ❌ Hidden (below 3.0)
Priority::Low → score 0.5 × Resource weight 2.5 = 1.25  ❌ Hidden (below 3.0)
```

**Affected Patterns:**

| Pattern | Priority | Score | T1 Tier? | Filtered? | Visibility |
|---------|----------|-------|----------|-----------|------------|
| `let _ = result` | High | 5.0 | ✅ Yes | ❌ No | **VISIBLE** |
| `if let Ok(x) ...` (no else) | Medium | 2.5 | ✅ Yes | ⚠️ YES | **HIDDEN** |
| `if let Ok(x) ... else {}` | Medium | 2.5 | ✅ Yes | ⚠️ YES | **HIDDEN** |
| `.ok()` discard | Medium | 2.5 | ✅ Yes | ⚠️ YES | **HIDDEN** |
| `match Err(_) {}` | Medium | 2.5 | ✅ Yes | ⚠️ YES | **HIDDEN** |
| `.unwrap_or()` | Low | 1.25 | ✅ Yes | ⚠️ YES | **HIDDEN** |
| `.unwrap_or_default()` | Low | 1.25 | ✅ Yes | ⚠️ YES | **HIDDEN** |

This violates the **principle of least surprise**: when debtmap classifies something as "Critical Architecture", users expect to see it. Hiding T1 items based on score contradicts the tier classification and undermines the value proposition of architectural debt analysis.

**Root Cause:** The tier system exists to identify architectural importance, but filtering doesn't respect this classification.

## Objective

Implement tier-aware filtering that:

1. **Always shows T1 and T2 items** regardless of score (architecturally critical)
2. **Applies score threshold filtering to T3 items** (testing gaps with moderate importance)
3. **Applies tier-based filtering to T4 items** (maintenance, controlled by `--show-t4` flag)
4. **Tracks filtering metrics separately** for tier-based vs score-based filtering
5. **Maintains backwards compatibility** with existing CLI flags and configuration

**Success Metric:** All 7 error swallowing patterns visible with default settings, while maintaining signal-to-noise ratio for non-critical items.

## Requirements

### Functional Requirements

1. **Tier-Aware Filter Logic**
   - T1 Critical Architecture items bypass score threshold filtering
   - T2 Complex Untested items bypass score threshold filtering
   - T3 Testing Gaps items subject to score threshold filtering
   - T4 Maintenance items controlled by `show_t4` configuration flag

2. **Filter Priority Order**
   ```
   Step 1: Check if T1/T2 (critical tier) → INCLUDE (bypass score)
   Step 2: Check if T4 and show_t4=false → EXCLUDE (tier filter)
   Step 3: Check if score < threshold → EXCLUDE (score filter)
   Step 4: INCLUDE (passed all filters)
   ```

3. **Filter Metrics Transparency**
   - Track `filtered_by_tier_critical_bypass` (T1/T2 included despite low score)
   - Track `filtered_t4_maintenance` (existing - T4 filtered by tier flag)
   - Track `filtered_below_score` (existing - T3/T4 filtered by score)
   - Track `included` (existing - items in final output)

4. **Configuration Compatibility**
   - `--min-score` flag: Applies to T3/T4 only (not T1/T2)
   - `--show-t4` flag: Existing behavior unchanged
   - `DEBTMAP_MIN_SCORE_THRESHOLD`: Existing behavior for T3/T4
   - Default threshold: 3.0 (unchanged)

### Non-Functional Requirements

1. **Performance**
   - No performance degradation (filtering is pure functional, already O(n))
   - Tier check is O(1) enum comparison

2. **Maintainability**
   - Pure functional implementation (no side effects)
   - Clear predicate functions for each filter step
   - Comprehensive test coverage for all tier combinations

3. **User Experience**
   - Clear documentation explaining tier-aware filtering
   - Helpful filter metrics output (`--show-filter-stats`)
   - CLI help text clarifies score threshold applies to T3/T4

4. **Backwards Compatibility**
   - Existing configurations continue to work
   - No breaking changes to public API
   - Existing test suites pass (except where behavior intentionally changes)

## Acceptance Criteria

- [ ] **AC1: T1 items always visible**
  - Given: Error swallowing pattern with score 2.5 (Medium priority)
  - When: Running `debtmap analyze` with default settings (threshold 3.0)
  - Then: Item is included in output (tier T1 bypasses score filter)
  - Validation: `cargo test test_t1_bypasses_score_filter`

- [ ] **AC2: T2 items always visible**
  - Given: Complex untested function with score 2.0
  - When: Running with `--min-score 5.0`
  - Then: Item is included in output (tier T2 bypasses score filter)
  - Validation: `cargo test test_t2_bypasses_high_threshold`

- [ ] **AC3: T3 items filtered by score**
  - Given: Testing gap item with score 2.5
  - When: Running with default threshold 3.0
  - Then: Item is excluded from output (T3 uses score filter)
  - Validation: `cargo test test_t3_respects_score_threshold`

- [ ] **AC4: T4 items filtered by tier flag**
  - Given: T4 maintenance item with score 5.0
  - When: Running with default settings (`show_t4=false`)
  - Then: Item is excluded from output (T4 uses tier filter)
  - Validation: `cargo test test_t4_filtered_by_tier_flag`

- [ ] **AC5: Filter metrics track tier bypass**
  - Given: 5 T1 items with scores below threshold
  - When: Filtering with threshold 3.0
  - Then: FilterMetrics.filtered_by_tier_critical_bypass = 5
  - Validation: `cargo test test_metrics_track_tier_bypass`

- [ ] **AC6: Error swallowing patterns visible**
  - Given: All 7 error swallowing patterns in codebase
  - When: Running `debtmap analyze` on debtmap's own codebase
  - Then: All 7 patterns visible in output
  - Validation: Integration test with real error swallowing examples

- [ ] **AC7: Backwards compatibility maintained**
  - Given: Existing test suite
  - When: Running full test suite with new filtering logic
  - Then: All non-filtering tests pass, filtering tests updated
  - Validation: `cargo test --all-features`

- [ ] **AC8: CLI help text accurate**
  - Given: User runs `debtmap analyze --help`
  - When: Viewing `--min-score` flag documentation
  - Then: Help text clarifies threshold applies to T3/T4 items only
  - Validation: Manual verification of CLI help output

## Technical Details

### Implementation Approach

**File:** `src/priority/filtering.rs`

**Core Change:** Replace current uniform score filter with tier-aware logic:

```rust
// BEFORE (current implementation)
pub fn filter_with_metrics(items: Vec<ClassifiedItem>, config: &FilterConfig) -> FilterResult {
    let included: Vec<_> = items
        .into_iter()
        .filter_map(|item| {
            if !tier_passes(item.tier, config) {
                metrics.filtered_t4_maintenance += 1;
                return None;
            }
            if !score_passes(item.score, config.min_score) {
                metrics.filtered_below_score += 1;
                return None;
            }
            Some(item.item)
        })
        .collect();
    // ...
}

// AFTER (tier-aware implementation)
pub fn filter_with_metrics(items: Vec<ClassifiedItem>, config: &FilterConfig) -> FilterResult {
    let total = items.len();
    let mut metrics = FilterMetrics::new(total, config.min_score, config.show_t4);

    let included: Vec<_> = items
        .into_iter()
        .filter_map(|item| {
            // Step 1: Critical tiers (T1/T2) bypass score filter
            if is_critical_tier(item.tier) {
                if item.score < config.min_score {
                    metrics.tier_critical_bypass += 1;
                }
                return Some(item.item);
            }

            // Step 2: T4 maintenance filtered by tier flag
            if !tier_passes(item.tier, config) {
                metrics.filtered_t4_maintenance += 1;
                return None;
            }

            // Step 3: T3 (and T4 if shown) filtered by score
            if !score_passes(item.score, config.min_score) {
                metrics.filtered_below_score += 1;
                return None;
            }

            Some(item.item)
        })
        .collect();

    metrics.included = included.len();
    FilterResult::new(included, metrics)
}

// New predicate: Check if tier is architecturally critical
fn is_critical_tier(tier: RecommendationTier) -> bool {
    matches!(
        tier,
        RecommendationTier::T1CriticalArchitecture | RecommendationTier::T2ComplexUntested
    )
}
```

### Architecture Changes

**Modified Modules:**
- `src/priority/filtering.rs`: Core filtering logic with tier awareness
- `src/priority/filtering.rs`: FilterMetrics struct extended

**New Functions:**
- `is_critical_tier(tier: RecommendationTier) -> bool`: Pure predicate for T1/T2 check

**Modified Functions:**
- `filter_with_metrics()`: Implements tier-aware filtering
- `FilterMetrics` struct: Adds `tier_critical_bypass` field

**Unchanged:**
- Tier classification logic (`src/priority/tiers/pure.rs`)
- Score calculation (`src/priority/scoring/`)
- Public API and CLI flags

### Data Structures

**Extended FilterMetrics:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct FilterMetrics {
    pub total_items: usize,
    pub filtered_t4_maintenance: usize,
    pub filtered_below_score: usize,
    pub filtered_by_debt_type: usize,
    pub tier_critical_bypass: usize,  // NEW: T1/T2 included despite low score
    pub included: usize,
    pub min_score_threshold: f64,
    pub show_t4: bool,
}
```

**Updated total_filtered():**

```rust
impl FilterMetrics {
    pub fn total_filtered(&self) -> usize {
        // tier_critical_bypass is NOT counted as filtered (they're included)
        self.filtered_t4_maintenance + self.filtered_below_score + self.filtered_by_debt_type
    }
}
```

### APIs and Interfaces

**Public API (unchanged):**
```rust
pub fn filter_with_metrics(items: Vec<ClassifiedItem>, config: &FilterConfig) -> FilterResult
pub struct FilterConfig { pub min_score: f64, pub show_t4: bool }
pub struct FilterResult { pub included: Vec<DebtItem>, pub metrics: FilterMetrics }
```

**Internal API (new):**
```rust
fn is_critical_tier(tier: RecommendationTier) -> bool  // Pure predicate
```

## Dependencies

**Prerequisites:** None (builds on existing tier classification system)

**Affected Components:**
- `src/priority/filtering.rs`: Core implementation
- `src/priority/tiers/`: Uses existing tier classifications
- `src/commands/analyze.rs`: Uses filtering results (no changes needed)
- `src/output/markdown.rs`: Displays filter metrics (no changes needed)

**External Dependencies:** None (internal refactoring only)

## Testing Strategy

### Unit Tests

**File:** `src/priority/filtering.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_t1_bypasses_score_filter() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T1CriticalArchitecture,
            score: 2.5,  // Below default threshold of 3.0
        }];

        let config = FilterConfig::default();
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 1, "T1 item should be included");
        assert_eq!(result.metrics.tier_critical_bypass, 1, "Should track bypass");
        assert_eq!(result.metrics.filtered_below_score, 0, "Score filter not applied");
    }

    #[test]
    fn test_t2_bypasses_high_threshold() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T2ComplexUntested,
            score: 2.0,  // Far below threshold
        }];

        let config = FilterConfig {
            min_score: 5.0,  // High threshold
            show_t4: false,
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 1, "T2 item should bypass threshold");
        assert_eq!(result.metrics.tier_critical_bypass, 1);
    }

    #[test]
    fn test_t3_respects_score_threshold() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T3TestingGaps,
            score: 2.5,  // Below threshold
        }];

        let config = FilterConfig::default();  // threshold 3.0
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 0, "T3 should be filtered by score");
        assert_eq!(result.metrics.filtered_below_score, 1);
        assert_eq!(result.metrics.tier_critical_bypass, 0);
    }

    #[test]
    fn test_t4_filtered_by_tier_flag() {
        let items = vec![ClassifiedItem {
            item: create_test_debt_item(),
            tier: RecommendationTier::T4Maintenance,
            score: 5.0,  // High score, but T4
        }];

        let config = FilterConfig {
            min_score: 3.0,
            show_t4: false,  // T4 hidden
        };
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 0, "T4 should be filtered by tier");
        assert_eq!(result.metrics.filtered_t4_maintenance, 1);
        assert_eq!(result.metrics.filtered_below_score, 0);
    }

    #[test]
    fn test_metrics_track_tier_bypass() {
        let items = vec![
            ClassifiedItem {
                tier: RecommendationTier::T1CriticalArchitecture,
                score: 1.0,
                ..create_test_item()
            },
            ClassifiedItem {
                tier: RecommendationTier::T1CriticalArchitecture,
                score: 2.0,
                ..create_test_item()
            },
            ClassifiedItem {
                tier: RecommendationTier::T2ComplexUntested,
                score: 1.5,
                ..create_test_item()
            },
        ];

        let config = FilterConfig::default();
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 3, "All critical tier items included");
        assert_eq!(result.metrics.tier_critical_bypass, 3, "All 3 bypassed score");
        assert_eq!(result.metrics.filtered_below_score, 0);
    }

    #[test]
    fn test_is_critical_tier_predicate() {
        assert!(is_critical_tier(RecommendationTier::T1CriticalArchitecture));
        assert!(is_critical_tier(RecommendationTier::T2ComplexUntested));
        assert!(!is_critical_tier(RecommendationTier::T3TestingGaps));
        assert!(!is_critical_tier(RecommendationTier::T4Maintenance));
    }

    #[test]
    fn test_mixed_tiers_filtered_correctly() {
        let items = vec![
            // T1 with low score → included
            ClassifiedItem { tier: T1, score: 1.0, ..test_item() },
            // T2 with low score → included
            ClassifiedItem { tier: T2, score: 2.0, ..test_item() },
            // T3 with low score → excluded
            ClassifiedItem { tier: T3, score: 2.5, ..test_item() },
            // T3 with high score → included
            ClassifiedItem { tier: T3, score: 5.0, ..test_item() },
            // T4 with show_t4=false → excluded
            ClassifiedItem { tier: T4, score: 10.0, ..test_item() },
        ];

        let config = FilterConfig { min_score: 3.0, show_t4: false };
        let result = filter_with_metrics(items, &config);

        assert_eq!(result.metrics.included, 3, "T1, T2, and high-score T3");
        assert_eq!(result.metrics.tier_critical_bypass, 2, "T1 and T2 bypassed");
        assert_eq!(result.metrics.filtered_below_score, 1, "Low-score T3");
        assert_eq!(result.metrics.filtered_t4_maintenance, 1, "T4 by tier");
    }
}
```

### Integration Tests

**File:** `tests/error_swallowing_visibility_test.rs`

```rust
/// Integration test: Verify all error swallowing patterns visible
#[test]
fn test_all_error_swallowing_patterns_visible() {
    // Create test file with all 7 error swallowing patterns
    let test_code = r#"
        fn example() {
            // Pattern 1: let _ = result (High priority, score 5.0)
            let _ = operation_that_returns_result();

            // Pattern 2: if let Ok without else (Medium priority, score 2.5)
            if let Ok(val) = another_operation() {
                use_val(val);
            }

            // Pattern 3: if let Ok with empty else (Medium priority, score 2.5)
            if let Ok(val) = third_operation() {
                use_val(val);
            } else {}

            // Pattern 4: .ok() discard (Medium priority, score 2.5)
            some_result.ok();

            // Pattern 5: match with empty Err (Medium priority, score 2.5)
            match fourth_operation() {
                Ok(v) => process(v),
                Err(_) => {},
            }

            // Pattern 6: unwrap_or (Low priority, score 1.25)
            let val = fifth_operation().unwrap_or(default);

            // Pattern 7: unwrap_or_default (Low priority, score 1.25)
            let val = sixth_operation().unwrap_or_default();
        }
    "#;

    // Run analysis
    let result = analyze_code(test_code);

    // Verify all 7 patterns detected and included
    let error_items: Vec<_> = result.items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::ErrorSwallowing { .. }))
        .collect();

    assert_eq!(
        error_items.len(),
        7,
        "All 7 error swallowing patterns should be visible"
    );

    // Verify tiers are T1
    for item in &error_items {
        assert_eq!(
            item.tier,
            Some(RecommendationTier::T1CriticalArchitecture),
            "Error swallowing should be T1"
        );
    }
}
```

### Performance Tests

```rust
#[bench]
fn bench_tier_aware_filtering(b: &mut Bencher) {
    let items = create_mixed_tier_items(1000);
    let config = FilterConfig::default();

    b.iter(|| {
        filter_with_metrics(items.clone(), &config)
    });
}
```

**Expected:** No performance regression (tier check is O(1) enum comparison)

### User Acceptance Tests

1. **UAT1: Run debtmap on its own codebase**
   ```bash
   cargo run -- analyze
   # Verify error swallowing items visible in output
   # Verify T1/T2 items shown despite low scores
   ```

2. **UAT2: Verify filter metrics output**
   ```bash
   cargo run -- analyze --show-filter-stats
   # Check FilterMetrics shows tier_critical_bypass count
   ```

3. **UAT3: Test with various thresholds**
   ```bash
   cargo run -- analyze --min-score 5.0
   # Verify T1/T2 still shown despite high threshold
   ```

## Documentation Requirements

### Code Documentation

1. **Function Documentation:**
   ```rust
   /// Checks if a tier is architecturally critical (T1 or T2).
   ///
   /// Critical tier items bypass score threshold filtering because they
   /// represent architectural issues (error handling, god objects, extreme
   /// complexity) that should always be visible regardless of calculated score.
   ///
   /// # Arguments
   /// * `tier` - The recommendation tier to check
   ///
   /// # Returns
   /// `true` if tier is T1CriticalArchitecture or T2ComplexUntested
   ///
   /// # Examples
   /// ```
   /// assert!(is_critical_tier(RecommendationTier::T1CriticalArchitecture));
   /// assert!(!is_critical_tier(RecommendationTier::T3TestingGaps));
   /// ```
   fn is_critical_tier(tier: RecommendationTier) -> bool;
   ```

2. **Struct Documentation:**
   ```rust
   /// Metrics tracking filtering decisions with tier awareness.
   ///
   /// Tracks items filtered by tier (T4), score (T3/T4), and tier bypass (T1/T2).
   /// This enables transparency in filtering decisions and helps users understand
   /// why certain items are included or excluded.
   pub struct FilterMetrics {
       /// Items with T1/T2 tier included despite score below threshold
       pub tier_critical_bypass: usize,
       // ... existing fields
   }
   ```

### User Documentation

**Update:** `README.md`

Add section explaining tier-aware filtering:

```markdown
### Filtering Behavior

Debtmap uses tier-aware filtering to ensure architecturally critical items are always visible:

- **T1 Critical Architecture** (error handling, god objects): Always shown
- **T2 Complex Untested** (complex code without tests): Always shown
- **T3 Testing Gaps**: Filtered by `--min-score` threshold (default 3.0)
- **T4 Maintenance**: Controlled by `--show-t4` flag

The `--min-score` flag controls the threshold for T3/T4 items only. T1 and T2 items
bypass score filtering because they represent architecturally important issues.

Examples:
```bash
# Show all items (T1-T4, any score)
debtmap analyze --min-score 0.0 --show-t4

# Show only critical items (T1-T2 always, T3 if score ≥ 5.0)
debtmap analyze --min-score 5.0

# Default: T1-T2 always, T3 if score ≥ 3.0, T4 hidden
debtmap analyze
```

**Update:** CLI help text in `src/cli.rs`

```rust
/// Minimum score threshold for filtering recommendations (default: 3.0)
///
/// Items with scores below this threshold will be filtered from the output.
/// Note: T1 and T2 tier items bypass this filter as they represent
/// architecturally critical issues (error handling, god objects, extreme
/// complexity) that should always be visible.
///
/// T3 and T4 tier items are subject to score filtering.
#[arg(long, default_value = "3.0")]
pub min_score: f64,
```

### Architecture Documentation

**Update:** `ARCHITECTURE.md` or `ERROR_SWALLOWING_FILTER_ANALYSIS.md`

Add section:

```markdown
## Tier-Aware Filtering

Filtering respects the tier classification system (spec 205):

**Tier Priority:**
1. T1/T2 items bypass score filter (architecturally critical)
2. T4 items use tier flag filter (`show_t4`)
3. T3/T4 items (if T4 shown) use score filter

**Rationale:**
The tier system classifies architectural importance. Score-based filtering
is appropriate for testing gaps and maintenance items, but critical
architecture issues should always be visible to prevent hiding important
debt like error swallowing patterns.
```

## Implementation Notes

### Pure Functional Approach

All filtering functions remain **pure** (no side effects):
- `is_critical_tier()`: Pure predicate
- `filter_with_metrics()`: Pure transformation
- `tier_passes()`: Pure predicate (existing)
- `score_passes()`: Pure predicate (existing)

### Performance Considerations

- Tier check: O(1) enum match
- No additional iterations (single pass filtering)
- FilterMetrics fields: negligible memory overhead
- Expected performance: No measurable change

### Error Handling

No new error cases introduced:
- Tier is always present on ClassifiedItem
- Enum match is exhaustive
- Pure functions can't fail

### Edge Cases

1. **T1 item with score 0.0**: Included (tier bypass)
2. **T4 item with score 10.0 and show_t4=false**: Excluded (tier filter)
3. **Empty item list**: Works correctly (no special handling needed)
4. **All items T1**: All included, metrics track bypass count

## Migration and Compatibility

### Breaking Changes

**None.** This is a behavioral change but not a breaking API change:
- Public function signatures unchanged
- CLI flags unchanged
- Configuration format unchanged
- Existing code compiles without modification

### Behavioral Changes

**What Changes:**
- T1/T2 items now visible even if score < threshold
- More items may be visible by default (T1/T2 with low scores)

**What Stays the Same:**
- T3 items still filtered by score
- T4 items still filtered by `show_t4` flag
- Default threshold still 3.0
- CLI flags work identically

### Migration Path

**For Users:**
1. Update debtmap: `cargo install debtmap` or rebuild
2. No configuration changes required
3. Expect to see more T1/T2 items (this is intended)
4. Use `--min-score` for T3/T4 filtering as before

**For Developers:**
1. Pull latest code
2. Run `cargo test --all-features`
3. Update any tests expecting T1/T2 items to be filtered
4. No code changes required in downstream tools

### Rollback Plan

If issues arise:
1. Revert filtering logic changes in `src/priority/filtering.rs`
2. Remove `tier_critical_bypass` field from FilterMetrics
3. Revert documentation changes
4. Re-run test suite

**Risk:** Low (pure functional change, isolated to filtering module)

## Success Metrics

### Quantitative Metrics

1. **Error Swallowing Visibility:** 7/7 patterns visible (was 1/7)
2. **Test Coverage:** All new code paths covered (>95%)
3. **Performance:** No regression (< 1% overhead acceptable)
4. **Backwards Compatibility:** 100% of existing tests pass (except intentional changes)

### Qualitative Metrics

1. **User Surprise:** Reduced (T1 items always visible matches expectations)
2. **Semantic Correctness:** Improved (filtering respects tier classification)
3. **Code Quality:** Maintained (pure functional, well-tested)
4. **Documentation Clarity:** Improved (tier-aware behavior explained)

## Open Questions

1. **Should T2 bypass score filter?**
   - **Decision:** Yes. T2 represents "complex untested code" which is architecturally important.
   - **Rationale:** Untested complex code is a structural risk that shouldn't be hidden.

2. **Should FilterMetrics include tier bypass in total_filtered()?**
   - **Decision:** No. Bypassed items are included, not filtered.
   - **Rationale:** Helps users understand filtering is working as intended.

3. **Should CLI help text emphasize tier awareness?**
   - **Decision:** Yes. Add note to `--min-score` help text.
   - **Rationale:** Prevents user confusion about why some low-score items are shown.

## Related Work

- **Spec 193:** Score threshold filtering (this spec builds on it)
- **Tier Classification:** `src/priority/tiers/pure.rs` (used but not modified)
- **Error Swallowing Detection:** `src/debt/error_swallowing.rs` (benefits from fix)

## References

- Analysis document: `ERROR_SWALLOWING_FILTER_ANALYSIS.md`
- Severity thresholds: `src/priority/classification/severity.rs`
- Tier system: `src/priority/tiers/`
- Filtering implementation: `src/priority/filtering.rs`

# Severity Classification Inconsistency Analysis

## Problem Summary

There's a fundamental mismatch between the scoring scale (0-100) and the severity classification thresholds (designed for 0-10 scale), causing inconsistent severity labels between TUI and non-TUI output.

## Root Cause

### Scoring System
- **Actual Scale**: 0-100 (using `Score0To100` type throughout the codebase)
- **Your Scores**: 53.2, 30.1, 13.2, 7.35

### Severity Classification

**Non-TUI (Markdown/JSON) - INCORRECT**
- Location: `src/priority/classification/severity.rs:48-58`
- Uses: `Severity::from_score()` with 0-10 scale thresholds
- Thresholds: 8.0 / 6.0 / 4.0 / 0.0
- Result for your scores:
  - 53.2 >= 8.0 → **CRITICAL** ❌
  - 30.1 >= 8.0 → **CRITICAL** ❌
  - 13.2 >= 8.0 → **CRITICAL** ❌
  - 7.35 >= 6.0 → **HIGH** ❌

**TUI - PARTIALLY CORRECT**
- Locations (4 duplicated implementations):
  - `src/tui/results/filter.rs:532`
  - `src/tui/results/list_view.rs:532`
  - `src/tui/results/grouping.rs:164`
  - `src/tui/results/detail_pages/overview.rs:310`
- Uses: Local `calculate_severity()` functions
- Thresholds: 100.0 / 50.0 / 10.0 / 0.0
- Result for your scores:
  - 53.2 >= 50.0 → **high** ✓
  - 30.1 >= 10.0 → **medium** ✓
  - 13.2 >= 10.0 → **medium** ✓
  - 7.35 < 10.0 → **low** ⚠️ (questionable)

## Issues Identified

1. **Scale Mismatch**: `Severity::from_score()` is documented for 0-10 scale but receives 0-100 scores
2. **Code Duplication**: TUI has 4 identical `calculate_severity()` functions
3. **Inconsistent Output**: Same scores produce different severity labels
4. **Poor Threshold Design**: TUI thresholds (100/50/10) are unbalanced for 0-100 scale

## Proposed Solution

### Option 1: Scale-Aware Severity Classification (RECOMMENDED)

Add a new method to handle 0-100 scale scores with balanced thresholds:

```rust
impl Severity {
    /// Classify severity for 0-100 scale scores (unified scoring system)
    pub fn from_score_100(score: f64) -> Self {
        if score >= 70.0 {
            Self::Critical   // Top priority: 70-100
        } else if score >= 50.0 {
            Self::High       // High priority: 50-69
        } else if score >= 30.0 {
            Self::Medium     // Medium priority: 30-49
        } else {
            Self::Low        // Low priority: 0-29
        }
    }
}
```

**Rationale for 70/50/30 thresholds**:
- **70+**: Critical issues requiring immediate attention (top 30% of scale)
- **50-69**: High priority, address soon (20% of scale)
- **30-49**: Medium priority, plan for refactoring (20% of scale)
- **0-29**: Low priority, nice-to-have improvements (bottom 30%)

**Your scores would be classified as**:
- 53.2 → HIGH ✓
- 30.1 → MEDIUM ✓
- 13.2 → LOW ⚠️ (down from medium)
- 7.35 → LOW ✓

### Option 2: Use TUI Thresholds (100/50/10)

Keep existing TUI thresholds but centralize:

```rust
impl Severity {
    pub fn from_score_100(score: f64) -> Self {
        if score >= 100.0 {
            Self::Critical   // Only perfect scores
        } else if score >= 50.0 {
            Self::High       // 50-99
        } else if score >= 10.0 {
            Self::Medium     // 10-49
        } else {
            Self::Low        // 0-9
        }
    }
}
```

**Issues with this approach**:
- Very few items would ever be "critical" (requires score >= 100)
- Large "medium" range (10-49, 40 point spread)
- Doesn't align with best practices for priority distribution

### Option 3: Alternative Balanced Thresholds (80/60/40)

More conservative thresholds:

```rust
impl Severity {
    pub fn from_score_100(score: f64) -> Self {
        if score >= 80.0 {
            Self::Critical   // 80-100
        } else if score >= 60.0 {
            Self::High       // 60-79
        } else if score >= 40.0 {
            Self::Medium     // 40-59
        } else {
            Self::Low        // 0-39
        }
    }
}
```

**Your scores would be classified as**:
- 53.2 → MEDIUM ✓
- 30.1 → LOW ⚠️
- 13.2 → LOW ✓
- 7.35 → LOW ✓

## Implementation Plan

### 1. Update Severity Classification (src/priority/classification/severity.rs)

- Add `from_score_100()` method for 0-100 scale
- Keep existing `from_score()` for any legacy 0-10 scale usage
- Update documentation to clarify scale expectations

### 2. Update All Callers

**Non-TUI locations to fix**:
- `src/priority/formatter_markdown/utilities.rs:69` - Change to `from_score_100()`
- `src/priority/formatter/context.rs:53` - Change to `from_score_100()`
- `src/priority/formatter/mod.rs:109` - Change to `from_score_100()`
- `src/priority/formatter_verbosity/body.rs:340` - Change to `from_score_100()`

**TUI locations to fix** (replace local functions):
- `src/tui/results/filter.rs:532` - Use `Severity::from_score_100()`
- `src/tui/results/list_view.rs:532` - Use `Severity::from_score_100()`
- `src/tui/results/grouping.rs:164` - Use `Severity::from_score_100()`
- `src/tui/results/detail_pages/overview.rs:310` - Use `Severity::from_score_100()`

### 3. Update Tests

- Add tests for `from_score_100()` with 0-100 scale values
- Test threshold boundaries
- Ensure monotonicity property holds

### 4. Update Documentation

- Update module docs to explain both scale functions
- Add migration guide if needed
- Update CHANGELOG.md

## Recommendation

**Use Option 1 (70/50/30 thresholds)** because:

1. **Balanced distribution**: Roughly equal ranges for each severity level
2. **Practical prioritization**: Aligns with agile/technical debt management practices
3. **Future-proof**: Works well as scores naturally distribute across the range
4. **Industry standard**: Similar to many other code quality tools

## Questions for Discussion

1. **Threshold preferences**: Do you prefer 70/50/30 or 80/60/40 thresholds?
2. **Current TUI thresholds**: Should we preserve the 100/50/10 behavior for backward compatibility?
3. **Migration strategy**: Should we update all at once or deprecate old thresholds gradually?

## Example Output After Fix

With 70/50/30 thresholds:

```
#1 SCORE: 53.2 [HIGH]
├─ LOCATION: ./etl_bad.rs:42 process_user_data()
...

#2 SCORE: 30.1 [MEDIUM]
├─ LOCATION: ./events_bad.rs:95 on_user_registered()
...

#3 SCORE: 13.2 [LOW]
├─ LOCATION: ./events_good.rs:323 publish_domain_events()
...

#4 SCORE: 7.35 [LOW]
├─ LOCATION: ./etl_good.rs:261 process_user_data()
...
```

Both TUI and non-TUI would show identical severity labels.

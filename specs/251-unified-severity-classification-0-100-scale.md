---
number: 251
title: Unified Severity Classification for 0-100 Scale
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-07
---

# Specification 251: Unified Severity Classification for 0-100 Scale

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently has a **critical inconsistency** in severity classification between output formats. The unified scoring system produces scores on a 0-100 scale (`Score0To100` type), but severity classification uses thresholds designed for a 0-10 scale (8.0/6.0/4.0). This causes:

1. **Incorrect severity labels in non-TUI output**: All scores above 8.0 are classified as CRITICAL
2. **Inconsistent classification across formats**: Same debt item shows different severity in TUI vs markdown/JSON
3. **Code duplication**: TUI has 4 identical `calculate_severity()` functions with different thresholds
4. **User confusion**: Output inconsistency undermines trust in the analysis

### Example of Current Problem

Given scores: 53.2, 30.1, 13.2, 7.35

**Non-TUI (Markdown/JSON)** - Uses `Severity::from_score()` with 8/6/4 thresholds:
- 53.2 → CRITICAL ❌
- 30.1 → CRITICAL ❌
- 13.2 → CRITICAL ❌
- 7.35 → HIGH ❌

**TUI** - Uses local `calculate_severity()` with 100/50/10 thresholds:
- 53.2 → high ✓
- 30.1 → medium ✓
- 13.2 → medium ✓
- 7.35 → low ⚠️

### Root Cause

The `Severity::from_score()` function was designed for a 0-10 scale but is being called with 0-100 scale scores throughout the codebase. The TUI independently implemented its own thresholds (100/50/10), creating divergent behavior.

## Objective

Unify severity classification across all output formats by:

1. Adding a scale-aware `Severity::from_score_100()` method with balanced thresholds for 0-100 scale
2. Replacing all calls to `Severity::from_score()` with the new method
3. Removing duplicated `calculate_severity()` functions from TUI modules
4. Ensuring consistent severity labels across TUI, markdown, JSON, and terminal output

**Success Metric**: All output formats show identical severity labels for the same debt score.

## Requirements

### Functional Requirements

1. **New Classification Method**
   - Add `Severity::from_score_100(score: f64) -> Self` method
   - Use balanced thresholds appropriate for 0-100 scale: **70/50/30**
   - Maintain existing `from_score()` for any legacy 0-10 scale usage
   - Document scale expectations clearly

2. **Threshold Design**
   - **Critical** (≥70.0): Top 30% of scale - immediate action required
   - **High** (≥50.0): Next 20% - high priority, address soon
   - **Medium** (≥30.0): Next 20% - moderate priority, plan refactoring
   - **Low** (<30.0): Bottom 30% - nice-to-have improvements

3. **Unified Usage**
   - All output formatters use `from_score_100()`
   - Remove duplicated TUI `calculate_severity()` functions
   - Single source of truth for severity classification

### Non-Functional Requirements

1. **Consistency**: Identical classification across all output formats
2. **Maintainability**: Single implementation, no duplication
3. **Type Safety**: Leverages existing `Score0To100` type
4. **Backward Compatibility**: Preserve `from_score()` for potential 0-10 scale usage

## Acceptance Criteria

- [ ] `Severity::from_score_100()` method added with 70/50/30 thresholds
- [ ] All non-TUI formatters updated to use `from_score_100()`
- [ ] All TUI modules updated to use `from_score_100()`
- [ ] Four duplicated `calculate_severity()` functions removed from TUI
- [ ] Tests added for `from_score_100()` boundary conditions
- [ ] Tests verify monotonicity property (higher score → same or higher severity)
- [ ] Documentation updated to explain both scale functions
- [ ] All output formats show identical severity for same score
- [ ] No clippy warnings or compilation errors
- [ ] CHANGELOG.md updated

## Technical Details

### Implementation Approach

#### 1. Add New Method to Severity Enum

**File**: `src/priority/classification/severity.rs`

```rust
impl Severity {
    /// Pure function: score (0-100 scale) → severity
    ///
    /// Classifies a debt score from the unified scoring system (0-100 scale)
    /// into a severity level based on these thresholds:
    /// - score >= 70.0: Critical
    /// - score >= 50.0: High
    /// - score >= 30.0: Medium
    /// - score <  30.0: Low
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::Severity;
    ///
    /// assert_eq!(Severity::from_score_100(85.0), Severity::Critical);
    /// assert_eq!(Severity::from_score_100(60.0), Severity::High);
    /// assert_eq!(Severity::from_score_100(40.0), Severity::Medium);
    /// assert_eq!(Severity::from_score_100(15.0), Severity::Low);
    /// ```
    #[inline]
    pub fn from_score_100(score: f64) -> Self {
        if score >= 70.0 {
            Self::Critical
        } else if score >= 50.0 {
            Self::High
        } else if score >= 30.0 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}
```

#### 2. Update Non-TUI Callers

**Files to modify**:
- `src/priority/formatter_markdown/utilities.rs:69`
- `src/priority/formatter/context.rs:53`
- `src/priority/formatter/mod.rs:109`
- `src/priority/formatter_verbosity/body.rs:340`

**Change**:
```rust
// Before
let severity = Severity::from_score(score);

// After
let severity = Severity::from_score_100(score);
```

#### 3. Update TUI Modules

**Files to modify**:
- `src/tui/results/filter.rs:532`
- `src/tui/results/list_view.rs:532`
- `src/tui/results/grouping.rs:164`
- `src/tui/results/detail_pages/overview.rs:310`

**Changes**:

1. Add import:
```rust
use crate::priority::classification::Severity;
```

2. Replace local `calculate_severity()` function with:
```rust
// Remove this:
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "critical"
    } else if score >= 50.0 {
        "high"
    } else if score >= 10.0 {
        "medium"
    } else {
        "low"
    }
}

// Use this instead:
let severity = Severity::from_score_100(score).as_str().to_lowercase();
```

3. Replace call sites:
```rust
// Before
let severity_level = calculate_severity(item.score);

// After
let severity_level = Severity::from_score_100(item.score).as_str().to_lowercase();
```

**Note**: TUI uses lowercase severity labels ("critical", "high", etc.) while non-TUI uses uppercase ("CRITICAL", "HIGH"). Use `.to_lowercase()` in TUI to maintain current display style.

### Architecture Changes

**Before**:
```
Non-TUI Formatters → Severity::from_score() → 8/6/4 thresholds (WRONG SCALE)
TUI Modules        → calculate_severity()  → 100/50/10 thresholds (DUPLICATED)
```

**After**:
```
All Formatters → Severity::from_score_100() → 70/50/30 thresholds (UNIFIED)
```

### Data Structures

No new data structures. Uses existing:
- `Severity` enum: `Critical | High | Medium | Low`
- `Score0To100` newtype wrapper for 0-100 scale scores

### APIs and Interfaces

**New Public API**:
```rust
impl Severity {
    pub fn from_score_100(score: f64) -> Self
}
```

**Existing APIs** (unchanged):
- `Severity::from_score(score: f64) -> Self` - Keep for legacy 0-10 scale
- `Severity::as_str(self) -> &'static str` - Returns "CRITICAL", "HIGH", etc.
- `Severity::color(self) -> Color` - Returns terminal color

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/classification/severity.rs` (core module)
  - `src/priority/formatter_markdown/utilities.rs` (markdown output)
  - `src/priority/formatter/context.rs` (terminal formatter)
  - `src/priority/formatter/mod.rs` (terminal formatter)
  - `src/priority/formatter_verbosity/body.rs` (verbose formatter)
  - `src/tui/results/filter.rs` (TUI filtering)
  - `src/tui/results/list_view.rs` (TUI list display)
  - `src/tui/results/grouping.rs` (TUI grouping)
  - `src/tui/results/detail_pages/overview.rs` (TUI detail pages)

## Testing Strategy

### Unit Tests

Add to `src/priority/classification/severity.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_thresholds_0_100_scale() {
        // Critical boundary
        assert_eq!(Severity::from_score_100(100.0), Severity::Critical);
        assert_eq!(Severity::from_score_100(70.0), Severity::Critical);
        assert_eq!(Severity::from_score_100(69.9), Severity::High);

        // High boundary
        assert_eq!(Severity::from_score_100(50.0), Severity::High);
        assert_eq!(Severity::from_score_100(49.9), Severity::Medium);

        // Medium boundary
        assert_eq!(Severity::from_score_100(30.0), Severity::Medium);
        assert_eq!(Severity::from_score_100(29.9), Severity::Low);

        // Low boundary
        assert_eq!(Severity::from_score_100(0.0), Severity::Low);
    }

    #[test]
    fn severity_100_scale_is_monotonic() {
        // Higher scores produce same or higher severity
        let test_cases = [
            (0.0, 29.9),
            (30.0, 49.9),
            (50.0, 69.9),
            (70.0, 100.0),
        ];

        for (lower, higher) in test_cases {
            let sev_lower = Severity::from_score_100(lower);
            let sev_higher = Severity::from_score_100(higher);
            assert!(
                sev_higher >= sev_lower,
                "Higher score ({}) should have same or higher severity than lower ({})",
                higher,
                lower
            );
        }
    }

    #[test]
    fn severity_100_scale_practical_examples() {
        // Real-world example scores
        assert_eq!(Severity::from_score_100(53.2), Severity::High);
        assert_eq!(Severity::from_score_100(30.1), Severity::Medium);
        assert_eq!(Severity::from_score_100(13.2), Severity::Low);
        assert_eq!(Severity::from_score_100(7.35), Severity::Low);
    }
}
```

### Integration Tests

Add to `tests/severity_classification_integration_test.rs`:

```rust
#[test]
fn test_consistent_severity_across_formats() {
    let analysis = run_analysis_on_test_project();

    // Extract severity from each format
    let markdown_severities = parse_markdown_severities(&analysis);
    let json_severities = parse_json_severities(&analysis);
    let tui_severities = parse_tui_severities(&analysis);

    // All formats must agree
    assert_eq!(markdown_severities, json_severities);
    assert_eq!(json_severities, tui_severities);
}

#[test]
fn test_severity_distribution_makes_sense() {
    let analysis = run_analysis_on_large_codebase();

    // Should have some items in each severity level
    let severities = collect_all_severities(&analysis);
    assert!(severities.iter().any(|s| s == &Severity::Critical));
    assert!(severities.iter().any(|s| s == &Severity::High));
    assert!(severities.iter().any(|s| s == &Severity::Medium));
    assert!(severities.iter().any(|s| s == &Severity::Low));
}
```

### Manual Testing

Test with real codebase:

```bash
# Run analysis and check consistency
cargo run -- analyze . --no-tui > output_notui.txt
cargo run -- analyze .              # View TUI

# Compare severity labels manually
# Should see identical classifications in both outputs
```

## Documentation Requirements

### Code Documentation

1. **Update `severity.rs` module docs**:
   - Explain both `from_score()` (0-10 scale) and `from_score_100()` (0-100 scale)
   - Document when to use each
   - Provide examples for both scales

2. **Inline comments**:
   - Add rationale for 70/50/30 threshold choices
   - Explain relationship to unified scoring system

### User Documentation

1. **CHANGELOG.md**:
```markdown
### Fixed

- **Severity Classification Consistency** (Spec 251)
  - Fixed critical bug where severity labels were inconsistent between output formats
  - Unified severity classification to use 70/50/30 thresholds for 0-100 scale
  - TUI, markdown, JSON, and terminal output now show identical severity labels
  - Removed duplicated severity calculation logic from TUI modules
  - Improved clarity: scores 70+ are CRITICAL, 50+ are HIGH, 30+ are MEDIUM, <30 are LOW
```

2. **Architecture Documentation**:
   - Update `ARCHITECTURE.md` section on severity classification
   - Document threshold rationale and design decisions

## Implementation Notes

### Threshold Rationale (70/50/30)

These thresholds were chosen to:

1. **Balance distribution**: Each severity level covers a reasonable portion of the 0-100 range
2. **Align with practices**: Similar to industry-standard code quality tools
3. **Provide clear priorities**:
   - Critical (70-100): ~30% range for urgent issues
   - High (50-69): ~20% range for important issues
   - Medium (30-49): ~20% range for moderate issues
   - Low (0-29): ~30% range for minor issues

### Alternative Thresholds Considered

1. **100/50/10** (current TUI):
   - Pro: More conservative critical threshold
   - Con: Very unbalanced ranges, few items ever "critical"
   - Rejected: Poor distribution

2. **80/60/40**:
   - Pro: More conservative overall
   - Con: Most items would be "low" severity
   - Rejected: Doesn't reflect actual priority distribution

### Code Cleanup Opportunities

While implementing, consider:
- Could `severity_color()` helper in TUI be consolidated?
- Are there other duplicated severity-related functions?

## Migration and Compatibility

### Breaking Changes

**None**. This is a bug fix that corrects incorrect behavior. Users will see:
- More accurate severity classifications
- Consistent labels across output formats

### Output Changes

Users will notice severity labels change in non-TUI output:

**Before** (incorrect):
```
#1 SCORE: 53.2 [CRITICAL]  ← Wrong!
#2 SCORE: 30.1 [CRITICAL]  ← Wrong!
```

**After** (correct):
```
#1 SCORE: 53.2 [HIGH]      ← Correct
#2 SCORE: 30.1 [MEDIUM]    ← Correct
```

### Migration Path

1. Implement and test changes
2. Update all tests to use new expected values
3. Run on real codebases to verify sensible distributions
4. Document change in CHANGELOG as bug fix
5. No user migration required (output-only change)

## Success Metrics

After implementation:

- ✅ All output formats show identical severity for same score
- ✅ No duplicated severity classification code
- ✅ Severity distribution makes practical sense
- ✅ Tests verify boundary conditions and monotonicity
- ✅ Documentation clarifies scale expectations
- ✅ Zero clippy warnings or compilation errors

## Example Output After Fix

Given scores: 53.2, 30.1, 13.2, 7.35

**All formats** (TUI, markdown, JSON, terminal):
```
#1 SCORE: 53.2 [HIGH]
#2 SCORE: 30.1 [MEDIUM]
#3 SCORE: 13.2 [LOW]
#4 SCORE: 7.35 [LOW]
```

Consistent classification across all output formats.

## References

- Unified scoring system: `src/priority/score_types.rs`
- Current severity classification: `src/priority/classification/severity.rs`
- TUI duplicated functions: `src/tui/results/*.rs`
- Analysis document: `SEVERITY_CLASSIFICATION_ISSUE.md`

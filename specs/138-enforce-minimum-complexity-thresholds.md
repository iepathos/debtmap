---
number: 138
title: Enforce Minimum Complexity Thresholds
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-10-23
---

# Specification 138: Enforce Minimum Complexity Thresholds

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap provides configuration options to filter out trivial functions that don't represent meaningful technical debt:

```toml
[thresholds]
minimum_cyclomatic_complexity = 3     # Skip functions with complexity ≤ 3
minimum_cognitive_complexity = 5      # Skip functions with low cognitive load
minimum_debt_score = 2.0              # Filter out minor issues
```

**Current Behavior**: These thresholds are **not being properly enforced**, causing trivial functions to appear in the top 10 recommendations despite configuration.

**Example from debtmap v0.2.9 output**:

```
#2 SCORE: 17.5 - RefactoringPattern::name()
├─ Cyclomatic: 1, Cognitive: 1
├─ What it is: Simple 6-arm match expression returning static strings
└─ Config says: minimum_cyclomatic_complexity = 3

#7 SCORE: 12.5 - UnifiedAnalysisCache::stats()
├─ Cyclomatic: 1, Cognitive: 0
├─ What it is: Debug string formatter
└─ Config says: minimum_cognitive_complexity = 5
```

Both functions have complexity far below the configured minimums but still appear in output.

### Root Cause: Broken Filtering Logic

Located in `src/priority/mod.rs:476-491`:

```rust
// For non-test items, also check complexity thresholds
if !matches!(
    item.debt_type,
    DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. }
) && item.cyclomatic_complexity <= min_cyclomatic
    && item.cognitive_complexity <= min_cognitive
{
    // Skip trivial functions unless they have other significant issues
    // (like being completely untested critical paths)
    if item.unified_score.coverage_factor < 8.0 {  // ← BUG HERE
        return;
    }
}
```

**The Bug**:
- `coverage_factor = (1.0 - coverage_pct) × 10.0`
- For 0% coverage: `coverage_factor = 10.0`
- Check: `if coverage_factor < 8.0` → `if 10.0 < 8.0` → **FALSE**
- Result: Trivial untested functions **bypass** the complexity threshold filter!

**Intent vs Reality**:
- **Intent**: "Filter trivial functions UNLESS they're critical untested paths"
- **Reality**: "Filter trivial functions ONLY IF they have >20% coverage"
- **Effect**: The exact opposite - trivial untested functions are kept, trivial tested functions are filtered

### Impact

**False Positives Introduced**:
1. Trivial getters with 0% coverage rank #2 and #7 in top 10
2. Users lose trust in recommendations
3. Configuration thresholds are ignored
4. Time wasted investigating non-issues

**User Confusion**:
- User sets `minimum_cyclomatic_complexity = 3`
- Expects functions with complexity ≤ 3 to be filtered
- Sees complexity=1 functions in top 10
- Loses confidence in tool

## Objective

Fix the filtering logic to **unconditionally enforce** minimum complexity thresholds as configured, eliminating the coverage-based exception that creates false positives.

## Requirements

### Functional Requirements

1. **Strict Threshold Enforcement**
   - Functions with `cyclomatic_complexity < minimum_cyclomatic_complexity` must be filtered
   - Functions with `cognitive_complexity < minimum_cognitive_complexity` must be filtered
   - No exceptions based on coverage level or other factors

2. **Independent Threshold Checks**
   - Cyclomatic threshold checked independently
   - Cognitive threshold checked independently
   - Both must be satisfied for the function to be reported

3. **Test Exclusion Maintained**
   - Test-related debt types (TestComplexityHotspot, TestTodo, TestDuplication) exempt from filtering
   - Test functions have different complexity characteristics and should be handled separately

4. **Backward Compatibility**
   - Existing configurations work without modification
   - Default thresholds remain unchanged (cyclomatic=5, cognitive=10)
   - No breaking changes to public API

### Non-Functional Requirements

1. **Performance**: Filtering remains O(1) per item
2. **Clarity**: Logic is simple and matches user expectations
3. **Testability**: Filtering behavior is easily unit tested
4. **Documentation**: Clear explanation of filtering behavior

## Acceptance Criteria

- [ ] Functions with `cyclomatic_complexity < minimum_cyclomatic_complexity` do not appear in output
- [ ] Functions with `cognitive_complexity < minimum_cognitive_complexity` do not appear in output
- [ ] RefactoringPattern::name() (cyclomatic=1) no longer appears when `minimum_cyclomatic_complexity = 3`
- [ ] UnifiedAnalysisCache::stats() (cyclomatic=1, cognitive=0) no longer appears when thresholds are set
- [ ] Test-related debt types are still reported regardless of complexity
- [ ] Coverage-based exception is removed from filtering logic
- [ ] Unit tests verify filtering for all threshold combinations
- [ ] Integration test confirms trivial functions excluded from top 10
- [ ] Existing tests continue to pass (no regressions)
- [ ] Configuration documentation updated with clear filtering semantics

## Technical Details

### Current Implementation (Broken)

`src/priority/mod.rs:476-491`:

```rust
if !matches!(
    item.debt_type,
    DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. }
) && item.cyclomatic_complexity <= min_cyclomatic
    && item.cognitive_complexity <= min_cognitive
{
    // ❌ BROKEN: Exception for high coverage_factor allows trivial functions through
    if item.unified_score.coverage_factor < 8.0 {
        return;
    }
}
```

### Fixed Implementation

`src/priority/mod.rs:476-491`:

```rust
// Filter out trivial functions that don't represent meaningful technical debt
// Test-related items are exempt as they have different complexity characteristics
if !matches!(
    item.debt_type,
    DebtType::TestComplexityHotspot { .. }
        | DebtType::TestTodo { .. }
        | DebtType::TestDuplication { .. }
) {
    // Enforce cyclomatic complexity threshold
    if item.cyclomatic_complexity < min_cyclomatic {
        return;
    }

    // Enforce cognitive complexity threshold
    if item.cognitive_complexity < min_cognitive {
        return;
    }
}
```

**Key Changes**:
1. ✅ Remove coverage_factor exception
2. ✅ Use `<` instead of `<=` (threshold is exclusive)
3. ✅ Check thresholds independently
4. ✅ Clear, simple logic matching user expectations

### Alternative Considered: Configurable Exception

An alternative approach would be to make the coverage exception configurable:

```toml
[thresholds]
minimum_cyclomatic_complexity = 3
allow_trivial_if_untested = false  # Default: false (strict enforcement)
```

**Rejected because**:
- Adds complexity for minimal benefit
- Users already have `minimum_debt_score` for fine-tuning
- Exceptions create confusion and false positives
- Simpler is better

### Why Use `<` Instead of `<=`?

**Before**: `if complexity <= min_cyclomatic`
- `minimum_cyclomatic_complexity = 3` filters complexity ≤ 3 (keeps 4+)
- Confusing: "minimum" suggests inclusive lower bound

**After**: `if complexity < min_cyclomatic`
- `minimum_cyclomatic_complexity = 3` filters complexity < 3 (keeps 3+)
- Clear: "minimum of 3" means "at least 3"

This matches user intuition: "I want at least complexity 3" → "filter anything less than 3"

## Dependencies

- **Prerequisites**: None (bug fix in existing code)
- **Affected Components**:
  - `src/priority/mod.rs` - Filtering logic in `add_item()`
  - Tests in `src/priority/` modules

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_below_cyclomatic_threshold() {
        let mut analysis = UnifiedAnalysis::new();

        // Create item with cyclomatic=2, config minimum=3
        let item = create_test_item(
            cyclomatic: 2,
            cognitive: 10,
            score: 15.0,
        );

        // With minimum_cyclomatic_complexity = 3
        set_config_threshold("minimum_cyclomatic_complexity", 3);

        analysis.add_item(item);

        // Should be filtered (2 < 3)
        assert_eq!(analysis.items.len(), 0);
    }

    #[test]
    fn test_filter_below_cognitive_threshold() {
        let mut analysis = UnifiedAnalysis::new();

        // Create item with cognitive=4, config minimum=5
        let item = create_test_item(
            cyclomatic: 10,
            cognitive: 4,
            score: 15.0,
        );

        // With minimum_cognitive_complexity = 5
        set_config_threshold("minimum_cognitive_complexity", 5);

        analysis.add_item(item);

        // Should be filtered (4 < 5)
        assert_eq!(analysis.items.len(), 0);
    }

    #[test]
    fn test_keep_at_threshold() {
        let mut analysis = UnifiedAnalysis::new();

        // Create item with cyclomatic=3, config minimum=3
        let item = create_test_item(
            cyclomatic: 3,
            cognitive: 5,
            score: 15.0,
        );

        // With minimum thresholds = 3, 5
        set_config_threshold("minimum_cyclomatic_complexity", 3);
        set_config_threshold("minimum_cognitive_complexity", 5);

        analysis.add_item(item);

        // Should be kept (3 >= 3, 5 >= 5)
        assert_eq!(analysis.items.len(), 1);
    }

    #[test]
    fn test_untested_trivial_function_filtered() {
        let mut analysis = UnifiedAnalysis::new();

        // Create trivial function with 0% coverage
        let item = create_test_item(
            cyclomatic: 1,
            cognitive: 0,
            coverage_pct: 0.0,  // 0% coverage
            score: 17.5,        // High score due to coverage gap
        );

        // With minimum_cyclomatic_complexity = 3
        set_config_threshold("minimum_cyclomatic_complexity", 3);

        analysis.add_item(item);

        // Should be filtered despite 0% coverage and high score
        assert_eq!(analysis.items.len(), 0);
    }

    #[test]
    fn test_test_items_exempt_from_filtering() {
        let mut analysis = UnifiedAnalysis::new();

        // Create test-related item with low complexity
        let item = create_test_item(
            cyclomatic: 1,
            cognitive: 0,
            debt_type: DebtType::TestComplexityHotspot { .. },
        );

        // With minimum_cyclomatic_complexity = 3
        set_config_threshold("minimum_cyclomatic_complexity", 3);

        analysis.add_item(item);

        // Should NOT be filtered (test items exempt)
        assert_eq!(analysis.items.len(), 1);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_top_10_respects_complexity_thresholds() {
    // Configure thresholds
    let config = r#"
        [thresholds]
        minimum_cyclomatic_complexity = 3
        minimum_cognitive_complexity = 5
    "#;
    write_config(".debtmap.toml", config);

    // Run debtmap analysis
    let results = run_debtmap_analysis(".");

    // Get top 10
    let top_10 = results.top_n(10);

    // Verify all items meet thresholds
    for item in top_10 {
        assert!(
            item.cyclomatic_complexity >= 3,
            "Item {} has cyclomatic={} < 3",
            item.function_name,
            item.cyclomatic_complexity
        );
        assert!(
            item.cognitive_complexity >= 5,
            "Item {} has cognitive={} < 5",
            item.function_name,
            item.cognitive_complexity
        );
    }
}
```

### Regression Tests

```rust
#[test]
fn test_existing_behavior_preserved() {
    // Ensure complex functions still appear
    // Ensure test items still appear
    // Ensure score filtering still works
    // Ensure duplicate detection still works
}
```

## Documentation Requirements

### Code Documentation

Add comprehensive comment explaining the filtering logic:

```rust
/// Filter out trivial functions based on configured complexity thresholds.
///
/// This enforces the `minimum_cyclomatic_complexity` and `minimum_cognitive_complexity`
/// thresholds from configuration, preventing simple getters, formatters, and utility
/// functions from appearing in technical debt reports.
///
/// # Rationale
///
/// Trivial functions (low complexity) rarely represent meaningful technical debt:
/// - Simple getters/setters are not worth testing or refactoring
/// - Debug formatters don't need extensive test coverage
/// - Utility functions with complexity=1 are low-risk
///
/// Users configure thresholds to focus on complex, high-value targets.
///
/// # Threshold Semantics
///
/// - `minimum_cyclomatic_complexity = N`: Keep functions with complexity >= N
/// - `minimum_cognitive_complexity = N`: Keep functions with complexity >= N
/// - Both thresholds must be satisfied
///
/// # Exemptions
///
/// Test-related debt types (TestComplexityHotspot, TestTodo, TestDuplication) are
/// exempt from filtering as they represent different concerns and have different
/// complexity characteristics.
///
/// # Examples
///
/// ```rust
/// // With minimum_cyclomatic_complexity = 3
/// let item = UnifiedDebtItem {
///     cyclomatic_complexity: 2,  // Below threshold
///     cognitive_complexity: 10,
///     // ...
/// };
/// analysis.add_item(item);  // Filtered out (2 < 3)
///
/// let item2 = UnifiedDebtItem {
///     cyclomatic_complexity: 3,  // At threshold
///     cognitive_complexity: 10,
///     // ...
/// };
/// analysis.add_item(item2);  // Kept (3 >= 3)
/// ```
```

### User Documentation

Update `docs/configuration.md`:

```markdown
## Complexity Thresholds

Debtmap filters out trivial functions to focus recommendations on meaningful technical debt.

### Configuration

```toml
[thresholds]
minimum_cyclomatic_complexity = 3     # Filter functions with complexity < 3
minimum_cognitive_complexity = 5      # Filter functions with cognitive < 5
minimum_debt_score = 2.0              # Filter items with score < 2.0
```

### Threshold Semantics

- **minimum_cyclomatic_complexity**: Functions must have **at least** this cyclomatic complexity
- **minimum_cognitive_complexity**: Functions must have **at least** this cognitive complexity
- Both thresholds must be satisfied for a function to be reported

### Default Values

| Setting | Default | Rationale |
|---------|---------|-----------|
| `minimum_cyclomatic_complexity` | 5 | Skip simple functions (1-4 branches) |
| `minimum_cognitive_complexity` | 10 | Skip low mental overhead |
| `minimum_debt_score` | 2.0 | Skip minor issues |

### Examples

**Filter trivial getters** (complexity = 1):
```toml
minimum_cyclomatic_complexity = 2  # Keep complexity >= 2
```

**Focus on complex functions only**:
```toml
minimum_cyclomatic_complexity = 10  # Keep complexity >= 10
minimum_cognitive_complexity = 20   # Keep cognitive >= 20
```

**Report everything** (disable filtering):
```toml
minimum_cyclomatic_complexity = 0  # No filtering
minimum_cognitive_complexity = 0   # No filtering
minimum_debt_score = 0.0           # No filtering
```

### Test Functions

Test-related items (TestComplexityHotspot, TestTodo, TestDuplication) are **exempt**
from complexity filtering as they represent different concerns.
```

## Implementation Notes

### Why This Fix is Critical

This is a **critical priority** bug fix because:

1. **User trust**: Configuration is ignored, users lose confidence
2. **False positives**: Top 10 dominated by trivial functions (20% false positive rate)
3. **Wasted time**: Users investigate non-issues
4. **Product quality**: Recommendations appear broken

### Historical Context

The coverage_factor exception was likely added with good intent:
- "Keep untested critical paths even if trivial"

But it created the opposite effect:
- Kept untested trivial functions
- Filtered partially-tested trivial functions
- Confused users about threshold behavior

### Design Principle

**Simple, predictable behavior > Smart exceptions**

Users can already fine-tune with:
- `minimum_debt_score` - Overall severity threshold
- `minimum_cyclomatic_complexity` - Structural complexity
- `minimum_cognitive_complexity` - Mental overhead

No need for additional coverage-based exceptions.

## Migration and Compatibility

### Breaking Changes

**None**. This is a bug fix that makes the tool behave as documented and expected.

### Impact on Existing Users

**Positive impact**:
- Fewer false positives in output
- Configuration works as expected
- Top 10 recommendations more actionable

**Potential concern**:
- Some users may have worked around the bug by setting thresholds lower
- Solution: Document the fix in release notes, explain new behavior

### Migration Path

1. **No action required**: Users automatically get fixed behavior
2. **Adjust thresholds if needed**: If output changes significantly, users can lower thresholds
3. **Validate**: Run debtmap and confirm top 10 looks reasonable

### Release Notes Template

```markdown
## Fixed: Minimum Complexity Thresholds Now Properly Enforced

**Bug**: Functions with complexity below `minimum_cyclomatic_complexity` and
`minimum_cognitive_complexity` were appearing in output despite configuration.

**Fix**: Thresholds are now strictly enforced. Trivial functions are filtered
regardless of coverage level.

**Impact**: Top 10 recommendations will no longer include simple getters, formatters,
and utility functions when thresholds are configured.

**Action**: If you were working around this bug by setting lower thresholds, you may
want to adjust your `.debtmap.toml` configuration.

**Example**:
- Before: RefactoringPattern::name() (complexity=1) appeared in output despite `minimum_cyclomatic_complexity = 3`
- After: Correctly filtered (1 < 3)
```

## Success Metrics

- [ ] RefactoringPattern::name() no longer in top 10 (currently #2)
- [ ] UnifiedAnalysisCache::stats() no longer in top 10 (currently #7)
- [ ] No functions with complexity < configured minimum in output
- [ ] Integration tests pass with strict threshold enforcement
- [ ] User feedback confirms improved recommendation quality
- [ ] False positive rate in top 10 decreases from ~20% to <5%

## Future Enhancements

1. **Per-language thresholds**: Different minimums for Rust vs Python vs JavaScript
2. **Per-pattern thresholds**: Different minimums for getters vs business logic
3. **Threshold presets**: "strict", "moderate", "permissive" preset configurations
4. **Threshold recommendations**: Analyze codebase and suggest optimal thresholds

---
number: 137
title: Complexity-Based Coverage Weight Scaling
category: optimization
priority: high
status: draft
dependencies: [135, 136]
created: 2025-10-23
---

# Specification 137: Complexity-Based Coverage Weight Scaling

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Specs 135, 136

## Context

After implementing specs 135 and 136 to fix cross-module call graph resolution, debtmap still produces significant false positives where trivial functions (cyclomatic complexity = 1) with 0% coverage rank higher than complex business logic functions.

**Current False Positives in Top 10**:
```
#2 SCORE: 17.5 - RefactoringPattern::name()
   - Cyclomatic: 1, Cognitive: 1, Adjusted: 0
   - What it is: Simple 6-arm match expression returning static strings
   - Why flagged: 0% unit test coverage
   - Why wrong: Trivial getter called by 14 functions (integration tested)

#7 SCORE: 12.5 - UnifiedAnalysisCache::stats()
   - Cyclomatic: 1, Cognitive: 0, Adjusted: 0
   - What it is: Debug string formatter
   - Why flagged: 0% unit test coverage
   - Why wrong: Trivial formatting function with no complexity
```

**Root Cause**: The current scoring model weights coverage at 50% uniformly across all functions, causing trivial untested getters to score higher than complex partially-tested business logic.

**Current Scoring Formula**:
```
Coverage Score: 11.0 × 40% = 4.40 (for 0% coverage)
Complexity Score: 0.0 × 40% = 0.00 (entropy-adjusted from 1)
Dependency Score: 10.0 × 20% = 2.00 (14 callers)
Final Score: 17.50 (after role adjustment)
```

**Problem**: A trivial function scores 17.5, appearing as #2 priority, while complex functions with partial coverage rank lower.

**Why Current Approach Fails**:
- Coverage weight (50%) is too high for trivial functions that don't need extensive testing
- No consideration of whether the function is worth testing in the first place
- Simple getters/formatters are flagged as critical gaps when they're integration tested
- Users lose trust in recommendations when trivial functions dominate the top 10

## Objective

Implement complexity-based coverage weight scaling that dynamically adjusts the coverage weight based on entropy-adjusted cyclomatic complexity, ensuring trivial functions cannot rank highly even with 0% coverage while maintaining high coverage priority for complex functions.

## Requirements

### Functional Requirements

1. **Complexity-Based Coverage Scaling**
   - Scale coverage weight based on entropy-adjusted cyclomatic complexity
   - Use adjusted complexity (not raw cyclomatic) to respect existing entropy dampening
   - Apply scaling before computing coverage score in the unified scorer
   - Preserve full coverage weight (1.0x) for complex functions (adjusted ≥ 8)

2. **Configuration Support**
   - Add `[scoring.complexity_scaling]` section to `.debtmap.toml`
   - Support enabling/disabling the feature via `enabled` flag (default: true)
   - Allow customization of scaling thresholds per complexity level
   - Provide sensible defaults that eliminate false positives

3. **Backward Compatibility**
   - When `complexity_scaling.enabled = false`, use original scoring behavior
   - Existing configurations without `[scoring.complexity_scaling]` use defaults
   - No breaking changes to scoring API or output format

4. **Scoring Integration**
   - Integrate scaling into existing unified scorer calculation
   - Apply scaling in `calculate_coverage_score()` function
   - Preserve existing entropy dampening and role-based adjustments
   - Document scaling factor in score calculation output

### Non-Functional Requirements

1. **Performance**: No measurable performance degradation (< 1% overhead)
2. **Maintainability**: Scaling logic isolated in dedicated function for testability
3. **Observability**: Include scaling factor in verbose scoring output
4. **Documentation**: Clear explanation in user docs and code comments

## Acceptance Criteria

- [ ] Configuration section `[scoring.complexity_scaling]` is supported in `.debtmap.toml`
- [ ] Default scaling thresholds are defined:
  - `adjusted_0_1 = 0.05` (trivial functions get 5% of coverage weight)
  - `adjusted_2 = 0.20` (very simple functions get 20%)
  - `adjusted_3_4 = 0.50` (simple functions get 50%)
  - `adjusted_5_7 = 0.75` (moderate functions get 75%)
  - `adjusted_8_plus = 1.00` (complex functions get full weight)
- [ ] RefactoringPattern::name() (adjusted complexity 0) no longer appears in top 10
- [ ] UnifiedAnalysisCache::stats() (adjusted complexity 0) no longer appears in top 10
- [ ] Complex functions (adjusted ≥ 8) maintain same or higher priority
- [ ] Scaling can be disabled via `enabled = false` in config
- [ ] Score calculation output includes coverage weight scaling factor
- [ ] Unit tests verify scaling calculation for all complexity levels
- [ ] Integration test confirms top 10 no longer contains trivial functions
- [ ] Documentation updated with configuration examples and rationale

## Technical Details

### Implementation Approach

1. **Add Configuration Schema**

```rust
// In src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct ComplexityCoverageScaling {
    #[serde(default = "default_complexity_scaling_enabled")]
    pub enabled: bool,

    #[serde(default = "default_scaling_adjusted_0_1")]
    pub adjusted_0_1: f64,

    #[serde(default = "default_scaling_adjusted_2")]
    pub adjusted_2: f64,

    #[serde(default = "default_scaling_adjusted_3_4")]
    pub adjusted_3_4: f64,

    #[serde(default = "default_scaling_adjusted_5_7")]
    pub adjusted_5_7: f64,

    #[serde(default = "default_scaling_adjusted_8_plus")]
    pub adjusted_8_plus: f64,
}

fn default_complexity_scaling_enabled() -> bool { true }
fn default_scaling_adjusted_0_1() -> f64 { 0.05 }
fn default_scaling_adjusted_2() -> f64 { 0.20 }
fn default_scaling_adjusted_3_4() -> f64 { 0.50 }
fn default_scaling_adjusted_5_7() -> f64 { 0.75 }
fn default_scaling_adjusted_8_plus() -> f64 { 1.00 }

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringWeights {
    // ... existing fields

    #[serde(default)]
    pub complexity_scaling: ComplexityCoverageScaling,
}
```

2. **Implement Scaling Function**

```rust
// In src/priority/unified_scorer.rs

/// Get coverage weight multiplier based on entropy-adjusted cyclomatic complexity
fn get_coverage_weight_scaling(adjusted_cyclomatic: u32, config: &ComplexityCoverageScaling) -> f64 {
    if !config.enabled {
        return 1.0; // No scaling when disabled
    }

    match adjusted_cyclomatic {
        0..=1 => config.adjusted_0_1,
        2 => config.adjusted_2,
        3..=4 => config.adjusted_3_4,
        5..=7 => config.adjusted_5_7,
        _ => config.adjusted_8_plus,
    }
}
```

3. **Integrate into Coverage Score Calculation**

```rust
// Modify calculate_coverage_score() in src/priority/scoring/calculation.rs

pub fn calculate_coverage_score(
    coverage_gap: f64,
    coverage_pct: f64,
    adjusted_cyclomatic: u32,
    role: &FunctionRole,
) -> f64 {
    let base_coverage_weight = 0.40; // 40% weight on coverage gaps

    // Get complexity-based scaling
    let config = crate::config::get_complexity_coverage_scaling();
    let complexity_scale = get_coverage_weight_scaling(adjusted_cyclomatic, &config);

    // Apply both complexity scaling and role-based adjustment
    let role_coverage_weights = crate::config::get_role_coverage_weights();
    let role_multiplier = get_role_coverage_multiplier(role, &role_coverage_weights);

    // Calculate coverage score with scaling
    let coverage_score = calculate_base_coverage_score(coverage_gap, coverage_pct);

    (coverage_score * base_coverage_weight * complexity_scale * role_multiplier)
}
```

4. **Update Score Calculation Output**

```rust
// Add scaling factor to score breakdown output
├─ SCORE CALCULATION:
│  ├─ Coverage Score: 11.0 × 40% × 0.05 = 0.22 (gap: 100%, complexity scale: 0.05)
│  ├─ Complexity Score: 0.0 × 40% = 0.00 (entropy-adjusted from 1)
│  ├─ Dependency Score: 10.0 × 20% = 2.00 (14 callers)
│  └─ Final Score: 2.22 (was 17.50)
```

### Architecture Changes

**Modified Components**:
- `src/config.rs`: Add `ComplexityCoverageScaling` configuration
- `src/priority/unified_scorer.rs`: Add `get_coverage_weight_scaling()` function
- `src/priority/scoring/calculation.rs`: Modify `calculate_coverage_score()` to apply scaling
- `src/priority/scoring/output.rs`: Update score breakdown formatting

**No New Components**: This feature integrates into existing scoring infrastructure

### Data Structures

```rust
pub struct ComplexityCoverageScaling {
    pub enabled: bool,
    pub adjusted_0_1: f64,     // Scale for complexity 0-1
    pub adjusted_2: f64,       // Scale for complexity 2
    pub adjusted_3_4: f64,     // Scale for complexity 3-4
    pub adjusted_5_7: f64,     // Scale for complexity 5-7
    pub adjusted_8_plus: f64,  // Scale for complexity 8+
}
```

### Configuration Example

```toml
[scoring]
coverage = 0.50      # Base coverage weight (50%)
complexity = 0.35
dependency = 0.15

[scoring.complexity_scaling]
enabled = true
adjusted_0_1 = 0.05      # Trivial functions: 5% coverage weight
adjusted_2 = 0.20        # Very simple: 20%
adjusted_3_4 = 0.50      # Simple: 50%
adjusted_5_7 = 0.75      # Moderate: 75%
adjusted_8_plus = 1.00   # Complex: full weight
```

## Dependencies

- **Prerequisites**:
  - Spec 135: Fix cross-module call graph resolution
  - Spec 136: Improve function ID cross-module resolution
  - Entropy-adjusted cyclomatic complexity calculation (already implemented)

- **Affected Components**:
  - Unified scoring system
  - Configuration parsing
  - Score calculation output formatting

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_scaling_trivial_function() {
        let config = ComplexityCoverageScaling::default();
        assert_eq!(get_coverage_weight_scaling(0, &config), 0.05);
        assert_eq!(get_coverage_weight_scaling(1, &config), 0.05);
    }

    #[test]
    fn test_coverage_scaling_complex_function() {
        let config = ComplexityCoverageScaling::default();
        assert_eq!(get_coverage_weight_scaling(8, &config), 1.00);
        assert_eq!(get_coverage_weight_scaling(20, &config), 1.00);
    }

    #[test]
    fn test_coverage_scaling_disabled() {
        let mut config = ComplexityCoverageScaling::default();
        config.enabled = false;
        assert_eq!(get_coverage_weight_scaling(0, &config), 1.00);
        assert_eq!(get_coverage_weight_scaling(1, &config), 1.00);
    }

    #[test]
    fn test_scoring_with_complexity_scaling() {
        // Test that trivial function (complexity 0) scores low
        let score_trivial = calculate_coverage_score(
            100.0, // 100% coverage gap
            0.0,   // 0% coverage
            0,     // adjusted cyclomatic = 0
            &FunctionRole::PureLogic
        );
        assert!(score_trivial < 1.0); // Should be very low

        // Test that complex function (complexity 16) scores high
        let score_complex = calculate_coverage_score(
            100.0, // 100% coverage gap
            0.0,   // 0% coverage
            16,    // adjusted cyclomatic = 16
            &FunctionRole::PureLogic
        );
        assert!(score_complex > score_trivial * 10.0); // Should be much higher
    }
}
```

### Integration Tests

```rust
#[test]
fn test_top_10_excludes_trivial_functions() {
    // Run debtmap on the codebase
    let results = run_debtmap_analysis();

    // Get top 10 recommendations
    let top_10 = results.top_n(10);

    // Verify no trivial functions (adjusted complexity 0-1) in top 10
    for item in top_10 {
        assert!(item.metrics.adjusted_cyclomatic >= 2,
            "Trivial function {} should not be in top 10",
            item.function_name);
    }
}
```

### Validation Tests

1. **Before/After Comparison**:
   - Run debtmap on the current codebase before implementing
   - Save top 10 results
   - Implement feature
   - Run debtmap again
   - Verify RefactoringPattern::name() and UnifiedAnalysisCache::stats() are no longer in top 10

2. **Regression Testing**:
   - Ensure complex functions (adjusted ≥ 8) maintain or improve their rankings
   - Verify total debt score doesn't increase significantly
   - Check that legitimate high-priority items remain visible

## Documentation Requirements

### Code Documentation

```rust
/// Calculates coverage weight scaling factor based on entropy-adjusted cyclomatic complexity.
///
/// This prevents trivial functions (low complexity) from ranking highly solely due to
/// missing test coverage. Complex functions maintain full coverage priority, while
/// simple getters, formatters, and pattern matchers are deprioritized.
///
/// # Rationale
///
/// A trivial getter with 0% coverage should not rank higher than complex business
/// logic with 50% coverage. This scaling ensures coverage priority aligns with
/// the value of testing the function.
///
/// # Examples
///
/// ```
/// // Trivial getter (adjusted complexity 0)
/// let scale = get_coverage_weight_scaling(0, &config);
/// assert_eq!(scale, 0.05); // Only 5% of coverage weight
///
/// // Complex business logic (adjusted complexity 16)
/// let scale = get_coverage_weight_scaling(16, &config);
/// assert_eq!(scale, 1.00); // Full coverage weight
/// ```
pub fn get_coverage_weight_scaling(
    adjusted_cyclomatic: u32,
    config: &ComplexityCoverageScaling
) -> f64
```

### User Documentation

Add section to `docs/configuration.md`:

```markdown
## Complexity-Based Coverage Weight Scaling

Debtmap scales the coverage weight based on function complexity to prevent false
positives where trivial functions rank highly solely due to missing tests.

### How It Works

Simple functions (like getters, formatters, and trivial pattern matchers) receive
reduced coverage priority because:
- They are often integration tested rather than unit tested
- Writing tests for them provides minimal value
- They have low risk of defects

Complex functions maintain full coverage priority because:
- Test coverage is critical for complex logic
- Untested complex code represents high risk
- Testing complex code provides high value

### Configuration

```toml
[scoring.complexity_scaling]
enabled = true              # Enable complexity-based scaling (default: true)
adjusted_0_1 = 0.05         # Trivial functions: 5% coverage weight
adjusted_2 = 0.20           # Very simple: 20%
adjusted_3_4 = 0.50         # Simple: 50%
adjusted_5_7 = 0.75         # Moderate: 75%
adjusted_8_plus = 1.00      # Complex: full weight
```

### Examples

**Before**: Trivial getter ranks #2
```
#2 SCORE: 17.5 - RefactoringPattern::name()
   Cyclomatic: 1, Coverage: 0%
```

**After**: Filtered from top 10
```
(No longer appears in top 10)
```

Complex functions maintain priority:
```
#3 SCORE: 14.0 - get_debt_type_key()
   Cyclomatic: 16, Coverage: 36.8%
   (Priority maintained or improved)
```
```

## Implementation Notes

### Why Use Adjusted Cyclomatic (Not Raw or Cognitive)?

1. **Consistency**: Scoring already uses entropy-adjusted complexity
2. **Handles Patterns**: Match expressions with repetitive arms correctly dampened
3. **No Double-Dampening**: Avoid applying entropy reduction twice
4. **Semantic Correctness**: "How complex is this really?" is the right question

### Threshold Tuning

The default thresholds are conservative:
- `adjusted_0_1 = 0.05`: Very aggressive dampening for trivial functions
- `adjusted_8_plus = 1.00`: Full weight above complexity 8

These can be adjusted based on user feedback:
- Increase `adjusted_0_1` to 0.10 if too aggressive
- Lower `adjusted_8_plus` threshold to 6 if complex functions need more priority

### Performance Considerations

The scaling function is O(1) and adds negligible overhead:
- Simple match expression on integer
- No additional data structures or allocations
- Called once per function during scoring

## Migration and Compatibility

### Breaking Changes

**None**. This is a backward-compatible enhancement:
- Existing configurations work unchanged
- Default behavior improves false positive rate
- Can be disabled with `enabled = false`

### Migration Path

1. **No action required**: Users automatically get improved scoring
2. **Opt-out**: Add `enabled = false` to preserve old behavior
3. **Customize**: Tune thresholds based on codebase characteristics

### Rollback Plan

If issues arise:
1. Set `enabled = false` in `.debtmap.toml`
2. Original scoring behavior is restored
3. No data loss or compatibility issues

## Success Metrics

- [ ] RefactoringPattern::name() no longer in top 10 (currently #2)
- [ ] UnifiedAnalysisCache::stats() no longer in top 10 (currently #7)
- [ ] No complex functions (adjusted ≥ 8) drop out of top 10 inappropriately
- [ ] User feedback confirms improved recommendation quality
- [ ] False positive rate in top 10 decreases from ~20% to <5%

## Future Enhancements

1. **Machine Learning Tuning**: Use historical data to optimize thresholds
2. **Per-Language Scaling**: Different thresholds for Rust vs Python vs JavaScript
3. **Custom Scaling Curves**: Allow polynomial or exponential scaling functions
4. **Integration Testing Detection**: Auto-detect integration tested functions and lower priority

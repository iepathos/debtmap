---
number: 109
title: Rebalance Coverage and Complexity Weights in Scoring Model
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-06
---

# Specification 109: Rebalance Coverage and Complexity Weights in Scoring Model

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current scoring model uses a weighted sum approach with the following weights:

```rust
// src/priority/scoring/calculation.rs:93-95
let coverage_weight = 0.50;    // 50% weight on coverage gaps
let complexity_weight = 0.35;  // 35% weight on complexity
let dependency_weight = 0.15;  // 15% weight on dependencies
```

This creates a coverage-dominated prioritization that produces counterintuitive results:

**Example from debtmap self-analysis with coverage:**
- **#4**: `CallGraphCache::put()` - cc=3, 0% coverage → Score **27.5**
- **#1**: `shared_cache.rs` - 2529 lines, 129 functions → Score **97.9**

**Problem**: The scoring gap (27.5 vs 97.9) is too small given the severity difference:
- A simple 3-line function with 0% coverage ranks only **3.6x** lower than a god object with 2500+ lines
- **Coverage weight (50%)** overwhelms structural issues (complexity + dependencies = 50%)
- Simple untested functions consistently outrank massive architectural problems

**Real-World Impact:**
- God objects (#1-#3) drop in priority despite being critical refactoring targets
- Simple helper functions with 0% coverage dominate recommendations (#4-#10)
- Developers focus on easy test additions instead of structural improvements
- Coverage metric becomes a "test count" game rather than quality indicator

## Objective

Rebalance the scoring weights to give **equal importance to complexity and coverage** while maintaining dependency influence. This ensures that structural technical debt (god objects, high complexity) maintains higher priority than simple untested functions.

## Requirements

### Functional Requirements

1. **Weight Rebalancing**
   - Reduce coverage weight from 50% to 40%
   - Increase complexity weight from 35% to 40%
   - Increase dependency weight from 15% to 20%
   - Maintain total weight sum of 100%

2. **Scoring Model Update**
   ```rust
   // New weights
   const COVERAGE_WEIGHT: f64 = 0.40;    // 40% (was 50%)
   const COMPLEXITY_WEIGHT: f64 = 0.40;  // 40% (was 35%)
   const DEPENDENCY_WEIGHT: f64 = 0.20;  // 20% (was 15%)
   ```

3. **Consistency Across Codebase**
   - Update `calculate_base_score()` in `scoring/calculation.rs`
   - Update display formatting in `formatter_verbosity.rs` (lines 419, 429, 434-436)
   - Update tests to reflect new expected scores
   - Ensure no-coverage mode remains unchanged (50% complexity, 25% deps, 25% debt)

4. **Documentation Updates**
   - Update scoring algorithm documentation
   - Add rationale for weight choices
   - Document expected score distribution changes

### Non-Functional Requirements

1. **Backward Compatibility**
   - Scores will change for all items (not breaking, expected behavior)
   - JSON output structure remains unchanged
   - CLI flags and options remain unchanged

2. **Performance**
   - No performance impact (same calculation, different constants)
   - All existing optimizations remain valid

3. **Testability**
   - Update test fixtures with new expected scores
   - Add regression tests for scoring balance
   - Verify god objects rank higher than simple untested functions

## Acceptance Criteria

- [ ] Coverage weight reduced to 40% in all scoring calculations
- [ ] Complexity weight increased to 40% in all scoring calculations
- [ ] Dependency weight increased to 20% in all scoring calculations
- [ ] Display formatting updated to show correct percentages (40%, 40%, 20%)
- [ ] God object scores increase relative to simple untested functions
- [ ] Test suite updated with new expected score values
- [ ] Real-world validation: debtmap self-analysis shows god objects in top 3
- [ ] Documentation updated with new weight rationale
- [ ] All existing tests pass with updated expected values

## Technical Details

### Implementation Approach

1. **Update Scoring Constants**
   ```rust
   // File: src/priority/scoring/calculation.rs
   // Lines: 93-95

   // BEFORE:
   let coverage_weight = 0.50;
   let complexity_weight = 0.35;
   let dependency_weight = 0.15;

   // AFTER:
   let coverage_weight = 0.40;
   let complexity_weight = 0.40;
   let dependency_weight = 0.20;
   ```

2. **Update Display Formatting**
   ```rust
   // File: src/priority/formatter_verbosity.rs
   // Lines: 419, 429, 434-436

   // BEFORE:
   factors.complexity_factor * 10.0 * 0.35
   factors.dependency_factor * 10.0 * 0.15
   let coverage_contribution = factors.coverage_factor * 10.0 * 0.5;
   let complexity_contribution = factors.complexity_factor * 10.0 * 0.35;
   let dependency_contribution = factors.dependency_factor * 10.0 * 0.15;

   // AFTER:
   factors.complexity_factor * 10.0 * 0.40
   factors.dependency_factor * 10.0 * 0.20
   let coverage_contribution = factors.coverage_factor * 10.0 * 0.4;
   let complexity_contribution = factors.complexity_factor * 10.0 * 0.4;
   let dependency_contribution = factors.dependency_factor * 10.0 * 0.2;
   ```

3. **Verification Points**
   - Check all grep results for `0.50`, `0.35`, `0.15` in scoring context
   - Ensure no-coverage scoring remains unchanged (different weight distribution)
   - Update test assertions that check exact score values

### Expected Score Changes

**Before rebalancing:**
```
Simple untested function (cc=3, 0% cov, 19 callers):
  Coverage:    11.0 × 50% = 5.50
  Complexity:   7.5 × 35% = 2.62
  Dependency:  10.0 × 15% = 1.50
  Base Score: 9.62 → Final: 27.5

God object (2529 lines, 129 functions):
  Coverage:    5.0 × 50% = 2.50
  Complexity:  10.0 × 35% = 3.50
  Dependency:  10.0 × 15% = 1.50
  Base Score: 7.50 → Final: 97.9

Ratio: 97.9 / 27.5 = 3.6x
```

**After rebalancing:**
```
Simple untested function (cc=3, 0% cov, 19 callers):
  Coverage:    11.0 × 40% = 4.40
  Complexity:   7.5 × 40% = 3.00
  Dependency:  10.0 × 20% = 2.00
  Base Score: 9.40 → Final: ~26.5

God object (2529 lines, 129 functions):
  Coverage:    5.0 × 40% = 2.00
  Complexity:  10.0 × 40% = 4.00
  Dependency:  10.0 × 20% = 2.00
  Base Score: 8.00 → Final: ~105

Ratio: 105 / 26.5 = 4.0x (better differentiation)
```

### Architecture Changes

None - this is a parameter tuning exercise within the existing architecture.

### Data Structures

No data structure changes. The `UnifiedScore` struct remains unchanged:
```rust
pub struct UnifiedScore {
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub dependency_factor: f64,
    pub role_multiplier: f64,
    pub final_score: f64,
}
```

### APIs and Interfaces

No API changes. The scoring functions maintain the same signatures:
```rust
pub fn calculate_base_score(
    coverage_factor: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/scoring/calculation.rs` (weight constants)
  - `src/priority/formatter_verbosity.rs` (display percentages)
  - Test files with expected score assertions
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Update `test_calculate_base_score()` with new expected values
  - Add test comparing simple vs complex function scores
  - Verify weights sum to 1.0

- **Integration Tests**:
  - Run debtmap self-analysis and verify god objects in top 5
  - Compare before/after rankings for representative codebase
  - Ensure simple untested functions don't dominate top 10

- **Regression Tests**:
  - Verify all test files compile and pass with updated assertions
  - Check that score distributions look reasonable
  - Validate no-coverage mode unchanged

- **Real-World Validation**:
  - Run on debtmap itself and verify:
    - `shared_cache.rs` ranks in top 3
    - `debt_item.rs` ranks in top 3
    - Simple `CallGraphCache::put()` ranks below #10

## Test Cases

```rust
#[test]
fn test_rebalanced_weights() {
    // Simple untested function
    let simple_score = calculate_base_score(
        11.0,  // High coverage gap (0% coverage)
        7.5,   // Low complexity (cc=3)
        10.0   // Many callers
    );

    // God object
    let god_score = calculate_base_score(
        5.0,   // Some coverage
        10.0,  // Max complexity
        10.0   // Max dependencies
    );

    // God object should score higher
    assert!(god_score > simple_score);

    // Ratio should be reasonable (3.5x - 5x range)
    let ratio = god_score / simple_score;
    assert!(ratio >= 3.5 && ratio <= 5.0);
}

#[test]
fn test_weights_sum_to_one() {
    const COVERAGE_WEIGHT: f64 = 0.40;
    const COMPLEXITY_WEIGHT: f64 = 0.40;
    const DEPENDENCY_WEIGHT: f64 = 0.20;

    let sum = COVERAGE_WEIGHT + COMPLEXITY_WEIGHT + DEPENDENCY_WEIGHT;
    assert!((sum - 1.0).abs() < 0.001);
}

#[test]
fn test_complexity_coverage_balance() {
    // Coverage and complexity should have equal influence
    assert_eq!(COVERAGE_WEIGHT, COMPLEXITY_WEIGHT);
}
```

## Documentation Requirements

- **Code Documentation**:
  - Add comment explaining weight rationale
  - Document the balance between structural vs testing debt
  - Explain why equal weights for coverage and complexity

- **User Documentation**:
  - Update README.md scoring section
  - Add to CHANGELOG.md
  - Update any scoring diagrams or examples

- **Architecture Updates**:
  - Update ARCHITECTURE.md if it documents scoring weights
  - Add design decision record for weight rebalancing

## Implementation Notes

### Rationale for 40/40/20 Split

1. **Equal Coverage & Complexity (40% each)**
   - Coverage indicates testing debt
   - Complexity indicates structural debt
   - Both are equally important for code quality
   - Neither should dominate prioritization

2. **Dependency Weight Increase (15% → 20%)**
   - High caller count indicates ripple effect risk
   - Dependency information is high-signal, low-noise
   - Deserves more influence than minimal 15%

3. **Why Not 33/33/33?**
   - Dependency factor is binary/discrete (caller count)
   - Coverage & complexity are continuous measures
   - Slightly higher weight on continuous metrics provides better differentiation

### Alternative Weights Considered

| Weights | Pros | Cons | Decision |
|---------|------|------|----------|
| 50/35/15 | Current | Coverage dominates | ❌ Reject |
| 40/40/20 | Balanced, emphasizes deps | Good differentiation | ✅ **Chosen** |
| 33/33/33 | Perfectly balanced | Less differentiation | ❌ Too uniform |
| 45/35/20 | Slight coverage emphasis | Still coverage-heavy | ❌ Insufficient change |
| 35/45/20 | Complexity-first | May undervalue coverage | ❌ Overcorrection |

### Migration Strategy

**Phase 1: Update Constants**
- Change weight values in `calculation.rs`
- Update display formatting

**Phase 2: Test Updates**
- Run tests, identify failures
- Update expected score values
- Add new validation tests

**Phase 3: Validation**
- Run on debtmap self-analysis
- Verify god objects rank appropriately
- Check edge cases (0% coverage, max complexity, etc.)

**Phase 4: Documentation**
- Update all docs with new weights
- Add migration note to CHANGELOG
- Update examples and tutorials

### Monitoring After Deployment

After merging, monitor:
1. **Score distribution** - Should see better spread
2. **Top 10 recommendations** - Should include god objects
3. **User feedback** - Are prioritizations more intuitive?
4. **False positive rate** - Should decrease

## Migration and Compatibility

**Breaking Changes:**
- Scores will change for all items (expected, not a breaking change in API sense)
- Rankings will shift (god objects rise, simple untested functions fall)
- Existing score thresholds may need adjustment

**User Impact:**
- **Positive**: More intuitive prioritization
- **Positive**: Structural debt surfaces properly
- **Neutral**: Scores change (users don't rely on absolute values)
- **Minimal**: No CLI or API changes

**Rollout Strategy:**
- Include in next minor version (e.g., 0.2.6)
- Highlight in release notes
- Provide before/after examples in changelog
- No opt-out needed (this is a bug fix/improvement, not new behavior)

## Future Enhancements

After this rebalancing, consider:

1. **Configurable Weights**
   - Allow users to customize weights via config file
   - Preset profiles: "coverage-first", "balanced", "complexity-first"
   - Validation to ensure weights sum to 1.0

2. **Adaptive Weights**
   - Adjust weights based on project maturity
   - More coverage weight for mature projects with good architecture
   - More complexity weight for new projects establishing patterns

3. **Context-Aware Weighting**
   - Different weights for different debt types
   - God objects use different weights than individual functions
   - Entry points vs internal functions

4. **A/B Testing Framework**
   - Compare different weight configurations
   - Collect metrics on recommendation quality
   - Data-driven weight optimization

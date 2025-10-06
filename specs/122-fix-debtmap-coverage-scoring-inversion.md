---
number: 122
title: Fix Debtmap Coverage Scoring Inversion
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-10-05
---

# Specification 122: Fix Debtmap Coverage Scoring Inversion

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap exhibits a critical scoring bug where adding coverage data **increases** total debt scores by 10x instead of reducing them. Without coverage, total debt score is 5,947; with coverage, it jumps to 57,985. This is backwards - coverage data should help identify well-tested code and reduce debt scores, not inflate them.

### Current Behavior

**Without Coverage:**
```
📊 TOTAL DEBT SCORE: 5947
📏 DEBT DENSITY: 200.1 per 1K LOC (29724 total LOC)
```

**With Coverage:**
```
📊 TOTAL DEBT SCORE: 57985
📏 DEBT DENSITY: 1640.9 per 1K LOC (35336 total LOC)
```

### Root Cause Analysis

The weighted sum model appears to be **adding** coverage gaps as standalone debt items instead of using coverage to **modulate** existing complexity/dependency scores. Each untested function creates a new high-scoring debt item (76-77 range), flooding the top 10 recommendations and obscuring real architectural problems.

### Impact

1. **Misleading Priorities**: Coverage-driven noise drowns out critical God Object files
2. **Trust Erosion**: 10x score increase contradicts user expectations
3. **Actionability Loss**: Top 10 becomes wall of similar-scored untested functions
4. **Strategic Confusion**: Users can't identify architectural debt vs testing gaps

## Objective

Fix the coverage scoring model so that:
1. Coverage data **reduces** total debt scores for well-tested code
2. Coverage gaps **modulate** existing complexity/dependency scores, not create standalone items
3. Top priorities surface architectural issues first, testing gaps second
4. Total debt score decreases (or stays similar) when coverage improves

## Requirements

### Functional Requirements

1. **Coverage as Score Modifier**: Coverage percentage should multiply/dampen existing debt scores, not add new standalone scores
2. **Threshold-Based Surfacing**: Untested functions should only appear in top recommendations if they also have:
   - Cyclomatic complexity ≥ 15, OR
   - High dependency count (≥ 10 callers or callees), OR
   - Critical role (entry points, public APIs)
3. **Architectural Debt Priority**: God Objects and God Modules should maintain top priority even with coverage data
4. **Coverage Gap Contextualization**: Coverage gaps should be shown as **part of** existing debt items, not as separate items

### Non-Functional Requirements

1. **Score Monotonicity**: Adding coverage data should never increase total debt score
2. **Backward Compatibility**: No-coverage analysis should produce identical results to current behavior
3. **Performance**: Coverage integration should not add more than 2x overhead to analysis time
4. **Transparency**: Score calculation should be explainable and auditable

## Acceptance Criteria

- [ ] Total debt score with coverage ≤ total debt score without coverage
- [ ] Top 3 recommendations with coverage include at least 2 architectural issues (God Objects/Modules)
- [ ] Untested functions only appear in top 10 if complexity ≥ 15 OR dependencies ≥ 10
- [ ] Coverage percentage acts as score dampener: 80% coverage → 20% of base score
- [ ] Score calculation documentation explains coverage impact clearly
- [ ] All existing tests pass without modification
- [ ] New integration test validates score monotonicity property

## Technical Details

### Current Scoring Model (Broken)

```rust
// This creates standalone items for coverage gaps:
let coverage_score = coverage_gap * coverage_weight;  // 100% gap * 50% = 50 points!
let base_score = coverage_score + complexity_score + dependency_score;
```

### Proposed Scoring Model

```rust
// Coverage should modulate, not add:
let base_score = complexity_score + dependency_score;
let coverage_multiplier = if has_coverage {
    1.0 - (coverage_percent / 100.0)  // 80% coverage → 0.2 multiplier
} else {
    1.0  // No coverage data → no penalty
};
let final_score = base_score * coverage_multiplier * role_adjustment;
```

### Implementation Strategy

1. **Refactor Score Calculation**:
   - Extract coverage logic from weighted sum model
   - Apply coverage as multiplier after base score calculation
   - Preserve role adjustments and entropy dampening

2. **Filter Untested Functions**:
   - Add complexity/dependency threshold checks
   - Only surface low-complexity untested functions in separate report section
   - Keep architectural issues in main top 10

3. **Update Score Display**:
   - Show coverage impact as multiplier in breakdown
   - Display base score and coverage-adjusted score separately
   - Add visual indicator for coverage-improved scores

### File Changes Required

**Primary Changes:**
- `src/priority/scoring/debt_item.rs`: Refactor score calculation to use coverage as multiplier
- `src/priority/scoring/classification.rs`: Add threshold filters for untested functions
- `src/priority/formatter.rs`: Update score breakdown display

**Supporting Changes:**
- `src/analysis/coverage/mod.rs`: Ensure coverage data is properly normalized (0-100%)
- `tests/integration/coverage_scoring.rs`: New integration tests

### Data Structures

```rust
pub struct ScoreBreakdown {
    pub base_score: f64,           // Complexity + Dependencies
    pub coverage_multiplier: f64,  // 0.0 (100% coverage) to 1.0 (0% coverage)
    pub coverage_adjusted: f64,    // base_score * coverage_multiplier
    pub role_adjustment: f64,      // Entry point, public API, etc.
    pub final_score: f64,          // coverage_adjusted * role_adjustment
}
```

## Dependencies

**Prerequisites**: None - this is a bug fix for existing functionality

**Affected Components**:
- Score calculation engine
- Priority ranking logic
- Markdown/JSON output formatters
- Coverage integration module

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_coverage_reduces_score() {
    let item_no_coverage = create_debt_item(complexity: 10, coverage: None);
    let item_with_coverage = create_debt_item(complexity: 10, coverage: Some(80.0));

    assert!(item_with_coverage.score < item_no_coverage.score);
    assert_approx_eq!(item_with_coverage.score, item_no_coverage.score * 0.2);
}

#[test]
fn test_perfect_coverage_near_zero_score() {
    let item = create_debt_item(complexity: 10, coverage: Some(100.0));
    assert!(item.score < 5.0);  // Should be very low
}

#[test]
fn test_untested_complex_function_surfaces() {
    let item = create_debt_item(complexity: 20, coverage: Some(0.0));
    assert!(item.should_surface_in_top_10());  // High complexity
}

#[test]
fn test_untested_simple_function_filtered() {
    let item = create_debt_item(complexity: 5, coverage: Some(0.0), dependencies: 2);
    assert!(!item.should_surface_in_top_10());  // Too simple, low deps
}
```

### Integration Tests

```rust
#[test]
fn test_score_monotonicity_property() {
    let analysis_no_cov = analyze_codebase(coverage: None);
    let analysis_with_cov = analyze_codebase(coverage: Some(lcov_data));

    assert!(analysis_with_cov.total_debt_score <= analysis_no_cov.total_debt_score);
}

#[test]
fn test_architectural_issues_maintain_priority() {
    let analysis = analyze_codebase(coverage: Some(lcov_data));
    let top_3 = &analysis.recommendations[0..3];

    let god_objects = top_3.iter().filter(|r| r.is_god_object()).count();
    assert!(god_objects >= 2, "Top 3 should include architectural issues");
}
```

### Regression Tests

- Run all existing debtmap tests with coverage data
- Verify no-coverage mode produces identical results
- Test edge cases: 0% coverage, 100% coverage, partial coverage

## Documentation Requirements

### Code Documentation

- Add inline comments explaining coverage multiplier calculation
- Document threshold values for surfacing untested functions
- Include examples in rustdoc for `ScoreBreakdown`

### User Documentation

Update CLI help and README to explain:
- How coverage affects debt scores
- Why total score may decrease with coverage data
- Thresholds for untested function surfacing
- How to interpret coverage-adjusted scores

### Architecture Updates

Add section to debtmap design docs:
- Coverage integration philosophy
- Score calculation model
- Filtering and ranking logic

## Implementation Notes

### Gotchas

1. **Zero Division**: Handle 0% coverage carefully in multiplier calculation
2. **Missing Coverage Data**: Distinguish "not covered" from "no coverage data available"
3. **Partial Coverage**: Some files may have coverage, others not - handle gracefully
4. **Legacy Compatibility**: Ensure old reports without coverage still work

### Best Practices

1. Use `Option<f64>` for coverage to distinguish presence vs absence
2. Add debug logging for score calculations during development
3. Create property-based tests for score monotonicity
4. Preserve original scores in `ScoreBreakdown` for debugging

### Migration Path

1. Ship fix in minor version (backward compatible)
2. Add deprecation warning if users rely on old scoring behavior
3. Provide migration guide for CI/CD pipelines expecting specific scores

## Migration and Compatibility

### Breaking Changes

**None** - this is a bug fix that corrects incorrect behavior. Users should expect:
- Lower total debt scores with coverage (correct behavior)
- Different top 10 recommendations (more actionable)
- Same results in no-coverage mode (backward compatible)

### Configuration

Add optional configuration for threshold tuning:

```toml
[debtmap.coverage]
# Minimum complexity to surface untested functions
untested_complexity_threshold = 15

# Minimum dependencies to surface untested functions
untested_dependency_threshold = 10

# Weight of coverage in score calculation (0.0 - 1.0)
coverage_weight = 0.5
```

### Version Compatibility

- Debtmap 0.2.5 → 0.2.6: Bug fix release
- All existing CLI flags and options preserved
- JSON output format unchanged (only score values differ)

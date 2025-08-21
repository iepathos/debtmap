---
number: 57
title: Fix Coverage Factor Gradation
category: optimization
priority: high
status: draft
dependencies: [19, 55]
created: 2025-01-21
---

# Specification 57: Fix Coverage Factor Gradation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [19, 55]

## Context

The current coverage factor calculation in debtmap's scoring system produces a binary result:
- Fully covered functions: 0.0 (no debt)
- Any uncovered code: High urgency score based on complexity

This binary approach loses important nuance. A function with 90% coverage is treated the same as one with 100% coverage (both get 0.0), while a function with 10% coverage gets the same high urgency as one with 0% coverage.

The current implementation:
```rust
let coverage_factor = if func.is_test {
    0.0  // Test functions don't need coverage
} else if let Some(cov) = coverage {
    calculate_coverage_urgency(&func_id, call_graph, cov, func.cyclomatic)
} else {
    10.0  // No coverage data - assume worst case
};
```

The `calculate_coverage_urgency` function uses transitive coverage (coverage through callers) but still produces sharp transitions rather than smooth gradients.

## Objective

Implement smooth coverage factor gradation that:
- Provides continuous scoring from 0% to 100% coverage
- Weights partial coverage appropriately
- Considers both direct and transitive coverage
- Maintains higher urgency for complex uncovered code
- Produces meaningful differentiation across coverage levels

## Requirements

### Functional Requirements

1. **Smooth Coverage Gradient**
   - Replace binary scoring with continuous function
   - Scale linearly or using smooth curve from 0% to 100%
   - Account for partial coverage at line and branch level
   - Consider both statement and branch coverage

2. **Complexity Weighting**
   - Higher complexity uncovered code gets higher urgency
   - Low complexity uncovered code gets moderate urgency
   - Use logarithmic scaling for complexity influence

3. **Coverage Types**
   - Direct coverage: Lines directly covered by tests
   - Transitive coverage: Coverage through tested callers
   - Branch coverage: Decision points covered
   - Combined score using weighted average

4. **Score Calculation**
```
coverage_factor = (1.0 - coverage_percentage) * complexity_weight * 10.0
where:
  coverage_percentage = weighted average of coverage types
  complexity_weight = log(complexity + 1) / log(max_complexity)
```

### Non-Functional Requirements

1. **Continuity**: No discontinuous jumps in scores
2. **Interpretability**: Clear relationship between coverage and score
3. **Configurability**: Adjustable weights for coverage types
4. **Performance**: Minimal overhead for coverage calculations

## Acceptance Criteria

- [ ] Coverage factor provides smooth gradient from 0% to 100%
- [ ] 50% covered function scores ~5.0 (with average complexity)
- [ ] 90% covered function scores ~1.0 (with average complexity)
- [ ] 10% covered function scores ~9.0 (with average complexity)
- [ ] Complexity appropriately weights the coverage urgency
- [ ] Both direct and transitive coverage considered
- [ ] No binary jumps at coverage boundaries
- [ ] Tests verify gradual score changes
- [ ] Documentation explains coverage calculation
- [ ] Configuration options for coverage weights

## Technical Details

### Implementation Approach

1. **Update Coverage Urgency Calculation**
```rust
pub fn calculate_coverage_urgency(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
    complexity: u32,
) -> f64 {
    let transitive_cov = calculate_transitive_coverage(func_id, call_graph, coverage);
    
    // Use weighted average of direct and transitive coverage
    let coverage_weight = 0.7;  // Direct coverage weight
    let effective_coverage = 
        transitive_cov.direct * coverage_weight + 
        transitive_cov.transitive * (1.0 - coverage_weight);
    
    // Calculate coverage gap (0.0 = fully covered, 1.0 = no coverage)
    let coverage_gap = 1.0 - effective_coverage.min(1.0).max(0.0);
    
    // Apply complexity weighting with logarithmic scaling
    // Complexity 1-5 = 0.5-0.8x, 6-10 = 0.8-1.2x, 11-20 = 1.2-1.5x, 20+ = 1.5-2.0x
    let complexity_weight = (((complexity as f64 + 1.0).ln() / 3.0) + 0.5).min(2.0);
    
    // Calculate urgency score with smooth gradient
    (coverage_gap * complexity_weight * 10.0).min(10.0)
}
```

2. **Add Branch Coverage Support**
```rust
pub struct CoverageMetrics {
    pub line_coverage: f64,      // 0.0 to 1.0
    pub branch_coverage: f64,    // 0.0 to 1.0
    pub function_coverage: f64,  // 0.0 to 1.0
}

fn calculate_effective_coverage(metrics: &CoverageMetrics) -> f64 {
    // Weighted average favoring line coverage
    metrics.line_coverage * 0.6 + 
    metrics.branch_coverage * 0.3 + 
    metrics.function_coverage * 0.1
}
```

3. **Configuration Options**
```rust
pub struct CoverageWeights {
    pub direct_weight: f64,       // Default: 0.7
    pub transitive_weight: f64,   // Default: 0.3
    pub line_weight: f64,         // Default: 0.6
    pub branch_weight: f64,       // Default: 0.3
    pub function_weight: f64,     // Default: 0.1
}
```

### Architecture Changes

- Modify `coverage_propagation.rs` to implement smooth gradients
- Update `LcovData` to track branch coverage if available
- Add configuration for coverage weights

### Data Structures

Update coverage tracking to include more granular metrics:
```rust
pub struct DetailedCoverage {
    pub lines_hit: usize,
    pub lines_total: usize,
    pub branches_hit: usize,
    pub branches_total: usize,
    pub functions_hit: usize,
    pub functions_total: usize,
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 19: Unified Debt Prioritization
  - Spec 55: Remove ROI from Scoring
- **Affected Components**:
  - Coverage propagation module
  - LCOV data parser
  - Unified scorer
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test gradient calculation for various coverage levels
  - Verify complexity weighting formula
  - Test edge cases (0%, 100%, missing data)
- **Integration Tests**:
  - Verify smooth score transitions on real code
  - Test with various coverage data formats
  - Ensure no score clustering
- **Regression Tests**:
  - Ensure test functions still get 0.0
  - Verify uncovered complex code gets high scores
- **Performance Tests**:
  - Measure overhead of detailed coverage calculations

## Documentation Requirements

- **Code Documentation**:
  - Document coverage gradient formula
  - Explain weighting rationale
  - Provide examples of score calculations
- **User Documentation**:
  - Update README with new coverage behavior
  - Explain how partial coverage affects scores
  - Provide configuration examples
- **Architecture Updates**:
  - Document coverage calculation flow

## Implementation Notes

1. **Coverage Score Examples**:
   - 0% coverage, complexity 10: ~12.0 → 10.0 (capped)
   - 25% coverage, complexity 10: ~9.0
   - 50% coverage, complexity 10: ~6.0
   - 75% coverage, complexity 10: ~3.0
   - 90% coverage, complexity 10: ~1.2
   - 100% coverage: 0.0

2. **Complexity Weight Examples**:
   - Complexity 1: 0.5x multiplier
   - Complexity 5: 0.8x multiplier
   - Complexity 10: 1.0x multiplier
   - Complexity 20: 1.3x multiplier
   - Complexity 50: 1.7x multiplier

3. **Transitive Coverage**:
   - Functions called only by tested code get partial credit
   - Helps reduce false positives for utility functions
   - Weight can be configured based on testing philosophy

## Migration and Compatibility

### Breaking Changes
- Coverage factor scores will change for partially covered functions
- Functions with 90%+ coverage will now have non-zero scores
- Priority categorization may shift

### Migration Path
1. Scores will automatically adjust with new calculation
2. Users may need to adjust coverage thresholds
3. Existing LCOV files will work without changes

### Compatibility
- LCOV format compatibility maintained
- JSON output structure unchanged
- Command-line interface unchanged

## Expected Outcomes

1. **Better Differentiation**: Partial coverage reflected in scores
2. **Smoother Prioritization**: No artificial gaps in priority
3. **More Actionable**: Can see progress as coverage improves
4. **Reduced Clustering**: Scores spread across full range
5. **Intuitive Scoring**: Higher coverage = lower debt score

## Risks and Mitigation

1. **Risk**: Users expect binary covered/uncovered distinction
   - **Mitigation**: Document gradient approach clearly

2. **Risk**: Partial coverage may reduce urgency too much
   - **Mitigation**: Configurable weights allow tuning

3. **Risk**: Branch coverage not available in all LCOV files
   - **Mitigation**: Gracefully fall back to line coverage only
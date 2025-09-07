---
number: 98
title: Prioritize Zero Coverage Functions
category: optimization
priority: high
status: draft
dependencies: [96]
created: 2025-09-07
---

# Specification 98: Prioritize Zero Coverage Functions

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [96 - Remove Score Capping]

## Context

Functions with 0% test coverage represent the highest risk in a codebase, yet the current scoring algorithm doesn't sufficiently prioritize them. While the coverage factor calculation is mathematically correct (higher gap = higher factor), functions with 100% coverage are still appearing in top recommendations due to other factors. Untested code should receive a significant scoring boost to ensure it appears at the top of the priority list.

## Objective

Dramatically increase the priority of functions with zero test coverage by applying a significant multiplier, ensuring untested code is addressed before well-tested code regardless of other factors.

## Requirements

### Functional Requirements
- Functions with 0% coverage receive 10x base score multiplier
- Functions with <20% coverage receive 5x multiplier
- Gradual scaling for coverage between 20-50%
- Maintain existing scoring for >50% coverage
- Clear indication in output when zero coverage is the primary factor

### Non-Functional Requirements
- No false positives (ensure coverage data is accurate)
- Deterministic scoring behavior
- Clear explanation in recommendations
- No performance impact

## Acceptance Criteria

- [ ] Zero coverage functions score minimum 50.0 (after spec 96)
- [ ] Zero coverage functions appear before any >50% coverage items
- [ ] Coverage factor calculation updated with special zero-coverage handling
- [ ] Output clearly indicates "UNTESTED" for zero coverage items
- [ ] Test files and test functions excluded from zero coverage boost
- [ ] Documentation explains coverage prioritization
- [ ] All existing tests updated for new scoring

## Technical Details

### Updated Coverage Factor Calculation

```rust
// src/priority/scoring/calculation.rs
pub fn calculate_coverage_factor(coverage_pct: f64, is_test_code: bool) -> f64 {
    // Don't penalize test code for coverage
    if is_test_code {
        return 0.1;
    }
    
    let coverage_gap = 1.0 - coverage_pct;
    
    match coverage_pct {
        // Zero coverage: maximum priority
        0.0 => 10.0,
        
        // Very low coverage: high priority
        c if c < 0.2 => 5.0 + (coverage_gap * 3.0),
        
        // Low coverage: elevated priority
        c if c < 0.5 => 2.0 + (coverage_gap * 2.0),
        
        // Moderate to high coverage: standard calculation
        _ => (coverage_gap.powf(1.5) + 0.1).max(0.1)
    }
}
```

### Recommendation Generation Updates

```rust
// src/priority/scoring/recommendation.rs
pub fn generate_coverage_recommendation(
    coverage_pct: f64,
    func: &FunctionMetrics,
) -> Recommendation {
    match coverage_pct {
        0.0 => Recommendation {
            action: format!(
                "⚠️ URGENT: Add tests for completely untested function (0% coverage)"
            ),
            reason: format!(
                "UNTESTED CODE: This function has never been tested. \
                 With {} branches and complexity {}, this represents high risk. \
                 Minimum {} test cases needed.",
                func.branches,
                func.cyclomatic_complexity,
                func.cyclomatic_complexity.max(3)
            ),
            priority: RecommendationPriority::Critical,
            ..Default::default()
        },
        
        c if c < 0.2 => Recommendation {
            action: format!(
                "Add comprehensive tests (currently {:.0}% coverage)",
                c * 100.0
            ),
            reason: format!(
                "SEVERELY UNDERTESTED: Only {:.0}% of code is tested. \
                 Need {} more test cases for adequate coverage.",
                c * 100.0,
                calculate_needed_tests(func, c)
            ),
            priority: RecommendationPriority::High,
            ..Default::default()
        },
        
        _ => generate_standard_coverage_recommendation(coverage_pct, func)
    }
}
```

### Display Format Updates

```rust
// src/priority/formatter.rs
fn format_priority_item(item: &DebtItem) -> String {
    let coverage_indicator = match item.coverage_percent {
        Some(0.0) => " [🔴 UNTESTED]",
        Some(c) if c < 0.2 => " [🟠 LOW COVERAGE]",
        Some(c) if c < 0.5 => " [🟡 PARTIAL COVERAGE]",
        _ => ""
    };
    
    let urgency = if item.coverage_percent == Some(0.0) {
        "URGENT - "
    } else {
        ""
    };
    
    format!(
        "#{} SCORE: {:.1}{}\n   ↳ {}{}\n",
        item.rank,
        item.score,
        coverage_indicator,
        urgency,
        item.primary_factor
    )
}
```

### Test Detection

```rust
// src/analyzers/test_detector.rs
pub fn is_test_code(path: &Path, function_name: &str) -> bool {
    // Check if in test module
    if path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }
    
    // Check if test file
    if path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with("_test") || s.starts_with("test_"))
        .unwrap_or(false) 
    {
        return true;
    }
    
    // Check if test function
    function_name.starts_with("test_") 
        || function_name.contains("_test")
        || function_name == "test"
}
```

### Integration with Base Score

```rust
// src/priority/unified_scorer.rs
pub fn calculate_unified_score(
    func: &FunctionMetrics,
    coverage_pct: Option<f64>,
    call_graph: &CallGraph,
) -> UnifiedScore {
    let is_test = is_test_code(&func.path, &func.name);
    let coverage = coverage_pct.unwrap_or(0.0);
    
    // Calculate base factors
    let coverage_factor = calculate_coverage_factor(coverage, is_test);
    let complexity_factor = calculate_complexity_factor(func);
    let dependency_factor = calculate_dependency_factor(func, call_graph);
    
    // Apply zero-coverage boost directly to base score
    let zero_coverage_boost = if coverage == 0.0 && !is_test {
        5.0  // Minimum score of 50 when combined with 10x coverage factor
    } else {
        1.0
    };
    
    let base_score = calculate_base_score(
        coverage_factor,
        complexity_factor,
        dependency_factor
    ) * zero_coverage_boost;
    
    // ... rest of calculation
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 96 (Remove Score Capping) - needed for scores >10
- **Affected Components**: 
  - Coverage factor calculation
  - Recommendation generation
  - Display formatting
  - Test detection logic
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test coverage factor calculation with various inputs
- **Integration Tests**: Verify zero coverage items rank first
- **Edge Cases**: Test with missing coverage data
- **Test Detection**: Verify test code isn't boosted
- **Regression Tests**: Ensure relative ordering preserved for covered code

## Documentation Requirements

- **Scoring Guide**: Explain coverage prioritization levels
- **Risk Documentation**: Why untested code is highest priority
- **User Guide**: How to interpret coverage indicators
- **Best Practices**: Testing strategies for zero coverage code

## Implementation Notes

1. **Coverage Data Validation**:
   ```rust
   // Ensure coverage data is reliable
   fn validate_coverage_data(cov: &Coverage) -> bool {
       cov.line_coverage.is_some() 
           || cov.branch_coverage.is_some()
           || cov.function_coverage.is_some()
   }
   ```

2. **Gradual Rollout**:
   - Start with 5x multiplier
   - Monitor impact on recommendations
   - Adjust to 10x if needed

3. **Special Cases**:
   - Generated code: exclude from zero coverage boost
   - Deprecated functions: reduced boost
   - Dead code: no boost (already identified separately)

## Migration and Compatibility

- Significant change in scoring priorities
- Existing zero coverage items will jump to top
- May need to phase in over multiple releases
- Consider configuration option for boost level
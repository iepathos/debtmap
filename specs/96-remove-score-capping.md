---
number: 96
title: Remove Score Capping at 10.0
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-09-07
---

# Specification 96: Remove Score Capping at 10.0

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The current debtmap scoring system caps all debt scores at 10.0, causing all high-priority items to have identical scores and defeating the purpose of prioritization. Analysis shows multiple locations using `.min(10.0)`, `.clamp(0.0, 10.0)`, and explicit capping in `normalize_final_score()`. This results in the top 10 recommendations all showing "SCORE: 10.00", making it impossible to distinguish between a minor issue and a critical 3,860-line god object file.

## Objective

Remove all artificial score capping to allow debt scores to reflect the true magnitude of technical debt, enabling proper prioritization of high-impact items over minor issues.

## Requirements

### Functional Requirements
- Remove all instances of score capping at 10.0
- Allow scores to reflect true magnitude without upper bounds
- Maintain score calculation accuracy and consistency
- Preserve relative ordering of items below 10.0
- Update display formatting to handle larger scores

### Non-Functional Requirements
- No performance degradation from larger score values
- Maintain backward compatibility with score interpretation
- Ensure scores remain deterministic and reproducible
- Keep scoring algorithm transparent and explainable

## Acceptance Criteria

- [ ] All instances of `.min(10.0)` removed from scoring code
- [ ] All instances of `.clamp(0.0, 10.0)` changed to `.max(0.0)` (keep lower bound only)
- [ ] `normalize_final_score()` function no longer caps at 10.0
- [ ] God object files (like rust_call_graph.rs) score >100
- [ ] Display formatting handles scores up to 999.99
- [ ] Top 10 items show distinct, meaningful score differences
- [ ] Existing tests updated to expect uncapped scores
- [ ] Documentation updated to explain new score ranges

## Technical Details

### Files to Modify

1. **src/risk/priority/scoring.rs:41**
   ```rust
   // Before:
   (base_score * dependency_factor * size_factor * debt_factor).clamp(0.0, 10.0)
   // After:
   (base_score * dependency_factor * size_factor * debt_factor).max(0.0)
   ```

2. **src/risk/evidence_calculator.rs:198**
   ```rust
   // Before:
   (base_score * role_multiplier).min(10.0)
   // After:
   base_score * role_multiplier
   ```

3. **src/priority/debt_aggregator.rs:183**
   ```rust
   // Before:
   score.min(10.0) // Cap at 10.0
   // After:
   score // No cap
   ```

4. **src/priority/scoring/calculation.rs:77**
   ```rust
   // Before:
   (9.5 + (raw_score - 2.0) * 0.25).min(10.0)
   // After:
   if raw_score <= 10.0 {
       raw_score
   } else {
       10.0 + (raw_score - 10.0).sqrt() // Gradual scaling for very high scores
   }
   ```

5. **src/scoring/score_normalizer.rs:107**
   ```rust
   // Before:
   (score + jitter).clamp(0.0, 10.0)
   // After:
   (score + jitter).max(0.0)
   ```

### Display Format Updates

```rust
// Update score display to handle larger values
fn format_score(score: f64) -> String {
    if score < 10.0 {
        format!("{:.2}", score)
    } else if score < 100.0 {
        format!("{:.1}", score)
    } else {
        format!("{:.0}", score)
    }
}
```

### Expected Score Ranges After Change

- **Minor issues**: 0.1 - 5.0 (unchanged)
- **Medium issues**: 5.0 - 15.0 (was capped at 10)
- **Major issues**: 15.0 - 50.0 (was capped at 10)
- **Critical issues**: 50.0 - 150.0 (was capped at 10)
- **God objects**: 100.0 - 500.0+ (was capped at 10)

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - All scoring modules
  - Display/formatting modules
  - Test suites expecting capped scores
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Update all tests expecting scores ≤10
- **Integration Tests**: Verify end-to-end scoring without caps
- **Regression Tests**: Ensure relative ordering preserved
- **Boundary Tests**: Test with extreme complexity values
- **Display Tests**: Verify formatting handles large scores

## Documentation Requirements

- **Score Interpretation Guide**: Document new score ranges
- **Migration Notes**: Explain change to users
- **API Documentation**: Update score field descriptions
- **README Updates**: Update examples with new score ranges

## Implementation Notes

1. **Phased Rollout**:
   - Phase 1: Remove caps in calculation functions
   - Phase 2: Update display formatting
   - Phase 3: Update tests
   - Phase 4: Update documentation

2. **Validation Approach**:
   ```bash
   # Before change - capture current scores
   debtmap analyze src --output before.json
   
   # After change - verify meaningful differentiation
   debtmap analyze src --output after.json
   
   # Compare top items have different scores
   jq '.items[0:10] | map(.unified_score.final_score)' after.json
   ```

3. **Backward Compatibility**:
   - Scores below 10 remain unchanged
   - Relative ordering preserved
   - Only high-debt items affected

## Migration and Compatibility

- No breaking changes for scores under 10.0
- Tools parsing scores should handle larger values
- Consider adding `--legacy-scoring` flag for transition period
- Update any dashboards or reports expecting max score of 10
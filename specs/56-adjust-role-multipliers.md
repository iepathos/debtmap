---
number: 56
title: Adjust Role Multipliers for Better Differentiation
category: optimization
priority: critical
status: draft
dependencies: [19, 55]
created: 2025-01-21
---

# Specification 56: Adjust Role Multipliers for Better Differentiation

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [19, 55]

## Context

The current role multiplier system in debtmap's unified scoring uses extremely aggressive reduction factors that essentially zero out scores for certain function types:

- IOWrapper functions: 0.1x multiplier (90% reduction)
- Orchestrator functions: 0.2x multiplier (80% reduction)
- PatternMatch functions: 0.1x multiplier (90% reduction)

These extreme multipliers were intended to reduce false positives by de-prioritizing certain function types. However, the current implementation is too aggressive, causing:

1. **Score Collapse**: Functions with these roles get scores near zero, losing all differentiation
2. **Hidden Debt**: Real issues in IO wrappers and orchestrators become invisible
3. **Prioritization Failure**: Cannot distinguish between trivial vs complex orchestrators
4. **User Confusion**: Scores don't reflect actual technical debt severity

Analysis shows that with a 0.1x multiplier, even a function with severe issues (score 8.0) becomes 0.8, falling into the LOW priority band. This defeats the purpose of technical debt detection.

## Objective

Adjust role multipliers to more moderate values that still reduce false positives while maintaining meaningful score differentiation. The new multipliers should:

- Reduce scores enough to deprioritize simple delegation patterns
- Maintain enough signal to identify problematic implementations
- Preserve score distribution across the full 0-10 range
- Enable proper prioritization within each function role category

## Requirements

### Functional Requirements

1. **Update Role Multipliers**
   - IOWrapper: 0.1x → 0.5x (50% reduction instead of 90%)
   - Orchestrator: 0.2x → 0.6x (40% reduction instead of 80%)
   - PatternMatch: 0.1x → 0.4x (60% reduction instead of 90%)
   - PureLogic: Keep at 1.5x (50% boost for business logic)
   - EntryPoint: Keep at 0.8x (20% reduction)
   - Unknown: Keep at 1.0x (no adjustment)

2. **Maintain Score Ranges**
   - Ensure adjusted scores still span 0-10 range
   - Preserve priority band thresholds (CRITICAL ≥8, HIGH ≥6, MEDIUM ≥4, LOW <4)
   - Avoid score clustering at extremes

3. **Configurable Multipliers**
   - Add configuration options for role multipliers
   - Allow users to tune multipliers based on codebase characteristics
   - Provide sensible defaults that work for most projects

### Non-Functional Requirements

1. **Backwards Compatibility**: Maintain same scoring structure, only adjust values
2. **Performance**: No performance impact (simple multiplication change)
3. **Explainability**: Document rationale for each multiplier value
4. **Testability**: Comprehensive tests for different role/score combinations

## Acceptance Criteria

- [ ] Role multipliers updated to new values in code
- [ ] Configuration options added for customizing multipliers
- [ ] Score distribution improved (no longer clustered at 5.0)
- [ ] IOWrapper functions with high complexity show meaningful scores (>2.0)
- [ ] Orchestrator functions maintain differentiation between simple and complex
- [ ] PatternMatch functions still deprioritized but not invisible
- [ ] Tests verify score ranges for each role type
- [ ] Documentation explains multiplier rationale
- [ ] No functions get scores below 0.5 unless truly trivial
- [ ] Priority bands still produce reasonable categorization

## Technical Details

### Implementation Approach

1. **Update Multiplier Constants**
```rust
// In src/priority/semantic_classifier.rs
pub fn get_role_multiplier(role: FunctionRole) -> f64 {
    match role {
        FunctionRole::PureLogic => 1.5,     // Keep: High priority for business logic
        FunctionRole::Orchestrator => 0.6,  // Was 0.2: Moderate reduction
        FunctionRole::IOWrapper => 0.5,     // Was 0.1: Half reduction
        FunctionRole::EntryPoint => 0.8,    // Keep: Slight reduction
        FunctionRole::PatternMatch => 0.4,  // Was 0.1: Significant but not extreme
        FunctionRole::Unknown => 1.0,       // Keep: No adjustment
    }
}
```

2. **Add Configuration Support**
```rust
// In src/config.rs
pub struct RoleMultipliers {
    pub pure_logic: f64,
    pub orchestrator: f64,
    pub io_wrapper: f64,
    pub entry_point: f64,
    pub pattern_match: f64,
    pub unknown: f64,
}

impl Default for RoleMultipliers {
    fn default() -> Self {
        Self {
            pure_logic: 1.5,
            orchestrator: 0.6,
            io_wrapper: 0.5,
            entry_point: 0.8,
            pattern_match: 0.4,
            unknown: 1.0,
        }
    }
}
```

### Architecture Changes

- Modify `semantic_classifier.rs` to use configurable multipliers
- Update `config.rs` to include role multiplier configuration
- Ensure multipliers are loaded from configuration at startup

### Data Structures

No changes to core data structures, only to multiplier values and configuration.

## Dependencies

- **Prerequisites**: 
  - Spec 19: Unified Debt Prioritization (established role system)
  - Spec 55: Remove ROI from Scoring (simplifies scoring system)
- **Affected Components**:
  - Semantic classifier module
  - Configuration system
  - Scoring calculation
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test each role multiplier produces expected score ranges
  - Verify no scores collapse to near-zero
  - Test configuration loading and defaults
- **Integration Tests**:
  - Analyze real codebase to verify score distribution
  - Ensure IO-heavy code still shows meaningful priorities
  - Verify orchestrators with issues are not hidden
- **Performance Tests**:
  - Confirm no performance impact (simple value change)
- **User Acceptance**:
  - Scores should better reflect intuitive priority
  - Technical debt in all function types should be visible

## Documentation Requirements

- **Code Documentation**:
  - Document rationale for each multiplier value
  - Explain how multipliers affect final scores
- **User Documentation**:
  - Update README with new multiplier values
  - Provide guidance on when to adjust multipliers
  - Include examples of score impacts
- **Architecture Updates**:
  - Document configuration options for multipliers

## Implementation Notes

1. **Multiplier Rationale**:
   - IOWrapper (0.5x): Thin wrappers are lower priority, but not invisible
   - Orchestrator (0.6x): Delegation patterns need less testing, but complexity matters
   - PatternMatch (0.4x): Repetitive patterns are simpler, but can still have issues
   - PureLogic (1.5x): Business logic is highest priority for testing

2. **Score Impact Examples**:
   - Complex IOWrapper (base 8.0): Old = 0.8 (LOW), New = 4.0 (MEDIUM)
   - Simple Orchestrator (base 3.0): Old = 0.6 (LOW), New = 1.8 (LOW)
   - Complex PatternMatch (base 7.0): Old = 0.7 (LOW), New = 2.8 (LOW)

3. **Configuration Guidelines**:
   - Increase IOWrapper multiplier for database-heavy applications
   - Decrease Orchestrator multiplier for microservice architectures
   - Adjust based on team's testing philosophy

## Migration and Compatibility

### Breaking Changes
- Score values will change for affected function types
- Functions previously scored near zero will have higher scores
- Priority categorization may shift for some functions

### Migration Path
1. Users with default configuration: Automatic improvement
2. Users with custom multipliers: Can keep existing or adopt new defaults
3. Score-based automation: May need threshold adjustments

### Compatibility Notes
- JSON output structure unchanged
- Priority bands remain the same
- Only numerical values change

## Expected Outcomes

1. **Better Score Distribution**: Scores will use full 0-10 range instead of clustering
2. **Visible Technical Debt**: Issues in IO wrappers and orchestrators become visible
3. **Improved Prioritization**: Can distinguish between simple and complex implementations
4. **Reduced False Negatives**: Won't miss real issues due to over-aggressive reduction
5. **Maintained False Positive Reduction**: Still deprioritizes simple patterns appropriately

## Risks and Mitigation

1. **Risk**: Some users may prefer aggressive multipliers
   - **Mitigation**: Make fully configurable with easy reversion

2. **Risk**: Scores may increase for previously low-priority items
   - **Mitigation**: Document changes clearly, provide migration guide

3. **Risk**: May surface more items than users want to address
   - **Mitigation**: Filtering and threshold options remain available
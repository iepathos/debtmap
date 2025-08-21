---
number: 55
title: Remove ROI from Scoring System
category: optimization
priority: high
status: draft
dependencies: [19, 44]
created: 2025-01-21
---

# Specification 55: Remove ROI from Scoring System

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [19, 44]

## Context

The current debtmap scoring system includes Return on Investment (ROI) as a weighted factor (25% by default) in the unified debt prioritization algorithm. Analysis of the scoring system reveals several critical issues:

1. **All functions receive identical ROI scores**: Every analyzed function gets an ROI value of 0.1 (the minimum clamp value), which normalizes to 0.3 in the scoring system
2. **No differentiation**: With 25% of the scoring weight providing zero differentiation, the tool cannot properly prioritize technical debt
3. **Complex but ineffective calculation**: The ROI calculation involves effort estimation, impact assessment, cascade effects, and module type bonuses, but still produces uniform results
4. **Score convergence**: Combined with coverage factor (30% weight) being 0.0 for covered functions, 55% of the scoring weight provides no differentiation, causing all scores to converge to ~5.0

The ROI calculation was intended to estimate the benefit-to-effort ratio of fixing technical debt items, but the current implementation:
- Relies on difficult-to-calibrate effort estimates
- Uses cascade calculations that don't reflect real-world impact
- Applies arbitrary multipliers and clamping that mask actual differences
- Produces meaningless uniform results that harm prioritization

## Objective

Remove the ROI factor from the unified scoring system and redistribute its weight to factors that provide meaningful differentiation. This will:
- Improve score differentiation between technical debt items
- Simplify the scoring algorithm by removing ineffective complexity
- Make scores more predictable and explainable
- Enable better prioritization of technical debt

## Requirements

### Functional Requirements

1. **Remove ROI Calculation**
   - Remove all ROI-related code from the unified scoring system
   - Remove ROICalculator and related modules
   - Remove roi_score parameter from calculate_unified_priority functions
   - Clean up unused ROI-related data structures

2. **Redistribute Scoring Weights**
   - Current weights total 100%: Coverage (30%), ROI (25%), Complexity (20%), Dependency (10%), Semantic (5%), Security (5%), Organization (5%)
   - Redistribute ROI's 25% weight proportionally:
     - Coverage: 30% → 40% (+10%)
     - Complexity: 20% → 30% (+10%)
     - Dependency: 10% → 15% (+5%)
     - Keep Semantic, Security, Organization at 5% each

3. **Update Score Normalization**
   - Ensure scores maintain proper 0-10 range after weight redistribution
   - Verify score distribution provides meaningful differentiation
   - Maintain backwards compatibility with priority thresholds (CRITICAL ≥8, HIGH ≥6, MEDIUM ≥4, LOW <4)

4. **Clean Up Related Code**
   - Remove effort estimation models
   - Remove cascade impact calculations (unless used elsewhere)
   - Remove module type bonuses related to ROI
   - Remove ROI-related configuration options

### Non-Functional Requirements

1. **Performance**: Removing ROI calculation should improve analysis speed by 10-20%
2. **Maintainability**: Simpler scoring system with fewer moving parts
3. **Explainability**: Scores should be easier to understand without ROI complexity
4. **Compatibility**: Maintain CLI interface compatibility (deprecate but don't break ROI-related flags)

## Acceptance Criteria

- [ ] ROI calculation completely removed from scoring system
- [ ] Scoring weights redistributed according to specification
- [ ] All technical debt items show differentiated scores (not all 5.0)
- [ ] Score distribution spans meaningful range (typically 2-8)
- [ ] Priority assignments (CRITICAL/HIGH/MEDIUM/LOW) remain consistent
- [ ] Tests updated to reflect new scoring weights
- [ ] Documentation updated to explain new scoring system
- [ ] Performance improvement of at least 10% on large codebases
- [ ] No breaking changes to CLI interface
- [ ] Deprecation warnings added for ROI-related configuration options

## Technical Details

### Implementation Approach

1. **Phase 1: Remove ROI Components**
   - Delete `src/risk/roi/` directory
   - Remove ROI-related imports and dependencies
   - Remove roi_score parameters from function signatures

2. **Phase 2: Update Scoring Weights**
   - Modify `src/config.rs` default weights
   - Update weight normalization to ensure sum equals 1.0
   - Adjust weight documentation

3. **Phase 3: Update Scoring Functions**
   - Modify `calculate_unified_priority` to remove ROI factor
   - Update `UnifiedScore` struct to remove roi_factor field
   - Adjust score calculation to use new weights

4. **Phase 4: Update Tests and Documentation**
   - Update unit tests with new expected scores
   - Update integration tests to verify score differentiation
   - Update README and other documentation

### Architecture Changes

Remove these components:
- `src/risk/roi/mod.rs` - ROI calculator module
- `src/risk/roi/cascade.rs` - Cascade impact calculations
- `src/risk/roi/effort.rs` - Effort estimation models
- `src/risk/roi/risk_model.rs` - Risk reduction models

Modify these components:
- `src/priority/unified_scorer.rs` - Remove ROI integration
- `src/config.rs` - Update default weights
- `src/main.rs` - Remove ROI calculation calls

### Data Structures

Update `UnifiedScore` struct:
```rust
pub struct UnifiedScore {
    pub complexity_factor: f64,    // 0-10, 30% weight (was 20%)
    pub coverage_factor: f64,      // 0-10, 40% weight (was 30%)
    // Remove: pub roi_factor: f64,
    pub semantic_factor: f64,      // 0-10, 5% weight
    pub dependency_factor: f64,    // 0-10, 15% weight (was 10%)
    pub security_factor: f64,      // 0-10, 5% weight
    pub organization_factor: f64,  // 0-10, 5% weight
    pub role_multiplier: f64,
    pub final_score: f64,
}
```

Update default weights in `ScoringWeights`:
```rust
fn default_coverage_weight() -> f64 { 0.40 }    // was 0.30
fn default_complexity_weight() -> f64 { 0.30 }  // was 0.20
fn default_dependency_weight() -> f64 { 0.15 }  // was 0.10
// Remove: fn default_roi_weight() -> f64 { 0.25 }
```

## Dependencies

- **Prerequisites**: 
  - Spec 19: Unified Debt Prioritization (established current scoring system)
  - Spec 44: Enhanced Scoring Differentiation (identified scoring issues)
- **Affected Components**:
  - Priority scoring system
  - Configuration system
  - CLI interface
  - Test suite
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test new weight calculations
  - Verify score ranges and distributions
  - Test backwards compatibility of priority thresholds
- **Integration Tests**:
  - Verify score differentiation on real codebases
  - Test that no two functions get identical scores (unless truly identical)
  - Verify performance improvements
- **Performance Tests**:
  - Benchmark before and after ROI removal
  - Verify 10%+ speed improvement
- **User Acceptance**:
  - Verify scores are more meaningful and actionable
  - Ensure priority recommendations make sense

## Documentation Requirements

- **Code Documentation**:
  - Document new weight rationale in code comments
  - Add deprecation notices for ROI-related code
- **User Documentation**:
  - Update README to explain new scoring system
  - Document weight redistribution rationale
  - Add migration guide for users with custom ROI configurations
- **Architecture Updates**:
  - Update ARCHITECTURE.md to remove ROI components
  - Document simplified scoring flow

## Implementation Notes

1. **Deprecation Strategy**: Keep ROI-related CLI flags but make them no-ops with deprecation warnings
2. **Configuration Migration**: If users have custom ROI weights, redistribute proportionally to other factors
3. **Score Stability**: Ensure that priority thresholds still produce reasonable categorization
4. **Backwards Compatibility**: Maintain JSON output structure (roi_factor can be 0 or omitted)

## Migration and Compatibility

### Breaking Changes
- UnifiedScore struct will no longer have roi_factor field
- ROI-related configuration options will be deprecated
- Score values will change (generally increase due to weight redistribution)

### Migration Path
1. Users with default configuration: No action needed
2. Users with custom ROI weights: Weights will be automatically redistributed
3. Users parsing JSON output: roi_factor field will be 0 or omitted
4. Users with score-based automation: May need to adjust thresholds

### Deprecation Timeline
- Version X: Add deprecation warnings for ROI options
- Version X+1: Remove ROI code completely
- Version X+2: Remove deprecated CLI flags

## Expected Outcomes

After implementing this specification:
1. **Better Differentiation**: Scores will range from ~2 to ~8 instead of all being ~5
2. **Clearer Priorities**: Technical debt items will have meaningful priority differences
3. **Simpler System**: ~1000 lines of complex ROI code removed
4. **Faster Analysis**: 10-20% performance improvement from removing ROI calculations
5. **More Maintainable**: Fewer edge cases and complex calculations to maintain

## Risks and Mitigation

1. **Risk**: Users may rely on ROI scores
   - **Mitigation**: Provide deprecation period and migration guide

2. **Risk**: Score distribution may become skewed
   - **Mitigation**: Carefully test weight redistribution with real codebases

3. **Risk**: Priority thresholds may need adjustment
   - **Mitigation**: Analyze score distributions and adjust thresholds if needed

4. **Risk**: Some valid use cases for ROI may exist
   - **Mitigation**: Document alternative approaches for benefit/effort analysis
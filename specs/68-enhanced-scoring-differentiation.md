---
number: 68
title: Enhanced Scoring Differentiation for Effective Debt Reduction
category: optimization
priority: critical
status: draft
dependencies: [44, 52, 60]
created: 2025-08-31
---

# Specification 68: Enhanced Scoring Differentiation for Effective Debt Reduction

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [44, 52, 60]

## Context

The current debtmap scoring system suffers from severe score compression, with all top-ranked items receiving identical scores (5.05) despite vastly different characteristics. Analysis of recent debt reduction attempts shows the workflow is addressing low-impact issues while missing critical coverage gaps and high-complexity untested code.

Key problems identified:
1. **Score Compression**: All top 10 items have identical 5.05 score despite complexity ranging from 1-7 and coverage from 0-92%
2. **Entropy Over-Dampening**: 100% dampening for low-entropy code makes simple untested functions appear unimportant
3. **Misaligned Recommendations**: Focus on minor refactorings instead of critical coverage gaps
4. **Ineffective Prioritization**: Recent commits show work on already-tested code while 22.45% of codebase remains untested

The scoring formula appears to use additive weighted scoring with aggressive normalization, causing all scores to converge around the same value.

## Objective

Redesign the scoring algorithm to provide meaningful differentiation between debt items, prioritizing high-impact improvements that will significantly reduce technical debt. The new system should:

1. Produce scores with at least 2x spread between top items
2. Heavily prioritize untested code, especially complex or critical functions
3. Reduce entropy dampening to avoid penalizing simple but important code
4. Generate actionable recommendations aligned with actual impact
5. Use multiplicative or exponential scoring for better separation

## Requirements

### Functional Requirements

1. **Multiplicative Scoring Model**
   - Replace additive weighted sum with multiplicative formula
   - Base formula: `Score = (Coverage_Gap ^ α) × (Complexity ^ β) × (Dependency ^ γ) × Role_Modifier`
   - Where α ≈ 1.5, β ≈ 0.8, γ ≈ 0.5 (configurable)
   - Ensures zero coverage creates high scores regardless of other factors

2. **Reduced Entropy Dampening**
   - Maximum dampening of 50% (not 100%)
   - Apply dampening only when entropy < 0.2 (very repetitive)
   - Use formula: `dampening = max(0.5, 1.0 - (0.5 × (0.2 - entropy) / 0.2))`
   - Preserve importance of simple validation/configuration code

3. **Coverage Gap Emphasis**
   - Increase coverage weight to 50-60% (from 40%)
   - Use coverage gap (1 - coverage) not coverage itself
   - Apply exponential scaling: `coverage_factor = (1 - coverage) ^ 1.5`
   - Zero coverage should dominate scoring

4. **Complexity-Coverage Interaction**
   - High complexity + low coverage = multiplicative penalty
   - Formula: `interaction_bonus = 1 + (complexity_factor × coverage_gap)`
   - Prioritizes complex untested code over simple untested code

5. **Smart Recommendation Generation**
   - For 0% coverage + complexity > 5: "Add comprehensive test suite covering N paths"
   - For 0% coverage + complexity ≤ 5: "Add focused tests for business logic"
   - For low coverage + high complexity: "Refactor to pure functions, then add tests"
   - Include effort estimates based on complexity

### Non-Functional Requirements

1. **Score Distribution**: Top 10 items should span at least 2x range (e.g., 10.0 to 5.0)
2. **Stability**: Small changes shouldn't drastically alter rankings
3. **Performance**: Scoring overhead < 10% of analysis time
4. **Explainability**: Show score calculation breakdown in verbose mode
5. **Configurability**: All parameters adjustable via configuration

## Acceptance Criteria

- [ ] Top 10 debt items have scores spanning at least 2x range
- [ ] Functions with 0% coverage rank higher than those with 50%+ coverage
- [ ] Complex untested functions rank highest overall
- [ ] Entropy dampening never exceeds 50%
- [ ] Simple untested validation code still gets reasonable scores
- [ ] Recommendations focus on test coverage before refactoring
- [ ] Score calculation breakdown available in verbose output
- [ ] Configuration allows tuning all scoring parameters
- [ ] Integration tests verify score differentiation
- [ ] Performance impact < 10% on large codebases

## Technical Details

### Implementation Approach

1. **New Scoring Algorithm**
```rust
pub fn calculate_enhanced_score(
    complexity: f64,
    coverage: f64,
    dependencies: usize,
    entropy: f64,
    role: FunctionRole,
    config: &ScoringConfig,
) -> f64 {
    // Coverage gap with exponential scaling
    let coverage_gap = 1.0 - coverage;
    let coverage_factor = coverage_gap.powf(config.coverage_exponent); // Default 1.5
    
    // Complexity with sublinear scaling
    let complexity_factor = complexity.powf(config.complexity_exponent); // Default 0.8
    
    // Dependency impact with sqrt scaling
    let dependency_factor = (dependencies as f64 + 1.0).powf(config.dependency_exponent); // Default 0.5
    
    // Reduced entropy dampening
    let entropy_dampening = if entropy < 0.2 {
        0.5 + (0.5 * entropy / 0.2) // 50-100% of score preserved
    } else {
        1.0 // No dampening for normal entropy
    };
    
    // Role modifier (less aggressive)
    let role_modifier = match role {
        FunctionRole::EntryPoint => 1.5,
        FunctionRole::CoreLogic => 1.3,
        FunctionRole::API => 1.2,
        FunctionRole::Utility => 1.0,
        FunctionRole::Test => 0.3,
        _ => 1.0,
    };
    
    // Multiplicative formula
    let base_score = coverage_factor * complexity_factor * dependency_factor;
    
    // Apply modifiers
    let adjusted_score = base_score * entropy_dampening * role_modifier;
    
    // Complexity-coverage interaction bonus
    if coverage < 0.5 && complexity > 5.0 {
        adjusted_score * 1.5 // 50% bonus for complex untested code
    } else {
        adjusted_score
    }
}
```

2. **Score Normalization**
```rust
pub fn normalize_scores(scores: &mut Vec<f64>) {
    // Use percentile-based normalization instead of linear
    scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
    
    let p99 = scores[scores.len() / 100];
    let p50 = scores[scores.len() / 2];
    let p10 = scores[scores.len() * 9 / 10];
    
    for score in scores.iter_mut() {
        if *score >= p99 {
            *score = 10.0; // Top 1% gets max score
        } else if *score >= p50 {
            *score = 5.0 + 5.0 * (*score - p50) / (p99 - p50);
        } else if *score >= p10 {
            *score = 2.0 + 3.0 * (*score - p10) / (p50 - p10);
        } else {
            *score = 2.0 * (*score / p10);
        }
    }
}
```

3. **Enhanced Recommendations**
```rust
pub fn generate_recommendation(
    complexity: f64,
    coverage: f64,
    debt_types: &[DebtType],
) -> Recommendation {
    match (coverage, complexity) {
        (0.0, c) if c > 5.0 => Recommendation {
            action: "Add comprehensive test suite",
            detail: format!("Function has {} paths to test. Start with happy path, then edge cases", c as u32),
            effort: EstimatedEffort::High,
            impact: ImpactLevel::Critical,
        },
        (0.0, _) => Recommendation {
            action: "Add focused unit tests",
            detail: "Simple function needs basic test coverage",
            effort: EstimatedEffort::Low,
            impact: ImpactLevel::High,
        },
        (cov, c) if cov < 0.5 && c > 7.0 => Recommendation {
            action: "Refactor to pure functions, then test",
            detail: format!("Extract {} pure functions to simplify testing", (c / 3.0) as u32),
            effort: EstimatedEffort::Medium,
            impact: ImpactLevel::High,
        },
        _ => generate_standard_recommendation(debt_types),
    }
}
```

### Architecture Changes

- Replace additive scoring with multiplicative model
- Implement percentile-based normalization
- Add configurable exponents for each factor
- Enhance recommendation engine with complexity-aware suggestions

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    pub coverage_exponent: f64,     // Default: 1.5
    pub complexity_exponent: f64,   // Default: 0.8
    pub dependency_exponent: f64,   // Default: 0.5
    pub max_entropy_dampening: f64, // Default: 0.5
    pub entropy_threshold: f64,     // Default: 0.2
    pub interaction_bonus: f64,     // Default: 1.5
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub action: &'static str,
    pub detail: String,
    pub effort: EstimatedEffort,
    pub impact: ImpactLevel,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 44: Enhanced Scoring Differentiation (predecessor)
  - Spec 52: Entropy-Based Complexity Scoring (entropy calculation)
  - Spec 60: Configurable Scoring Weights (configuration system)
- **Affected Components**:
  - UnifiedScorer module
  - Priority calculation system
  - Recommendation generator
  - Output formatters
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Verify multiplicative scoring produces spread
  - Test entropy dampening limits
  - Validate coverage gap emphasis
  - Test complexity-coverage interaction
- **Integration Tests**:
  - Compare scores for diverse code samples
  - Verify untested complex code ranks highest
  - Test recommendation quality
  - Validate configuration overrides
- **Performance Tests**:
  - Measure scoring overhead on large codebases
  - Profile multiplicative vs additive performance
- **Regression Tests**:
  - Ensure no score compression occurs
  - Verify differentiation maintained

## Documentation Requirements

- **Code Documentation**:
  - Document scoring formula with examples
  - Explain each parameter's impact
  - Provide tuning guidelines
- **User Documentation**:
  - Scoring interpretation guide
  - Configuration tuning guide
  - FAQ on score meanings
- **Architecture Updates**:
  - Update ARCHITECTURE.md with new scoring model
  - Document percentile normalization

## Implementation Notes

1. **Score Interpretation**:
   - 8.0-10.0: Critical - immediate attention needed
   - 5.0-8.0: High - address in current sprint
   - 3.0-5.0: Medium - plan for next sprint
   - 1.0-3.0: Low - track but defer

2. **Tuning Guidelines**:
   - Increase coverage_exponent for coverage-critical projects
   - Increase complexity_exponent for maintainability focus
   - Adjust entropy_threshold based on codebase patterns

3. **Migration Considerations**:
   - Scores will change significantly
   - Document score interpretation changes
   - Provide migration guide for thresholds

## Migration and Compatibility

### Breaking Changes
- Score values will change dramatically (intended)
- Threshold-based filtering needs adjustment
- Score interpretation differs from previous version

### Migration Path
1. Run with --show-score-calculation to understand new scores
2. Adjust any threshold-based automation
3. Update dashboards/reports expecting old score ranges

### Compatibility
- Old configuration files work but produce different scores
- New scoring can be disabled via --legacy-scoring flag (temporary)

## Expected Outcomes

1. **Better Prioritization**: Focus on high-impact debt items
2. **Reduced False Positives**: Less noise from low-impact issues
3. **Actionable Recommendations**: Clear next steps for each item
4. **Measurable Progress**: Debt score reduction correlates with actual improvement
5. **Efficient Workflows**: Automation targets right issues

## Risks and Mitigation

1. **Risk**: Dramatic score changes confuse users
   - **Mitigation**: Clear documentation and migration guide

2. **Risk**: Over-emphasis on coverage
   - **Mitigation**: Configurable exponents for adjustment

3. **Risk**: Complex calculation hurts performance
   - **Mitigation**: Cache intermediate values, optimize hot paths
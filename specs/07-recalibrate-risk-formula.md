---
number: 07
title: Recalibrate Risk Formula for Coverage-Weighted Analysis
category: optimization
priority: critical
status: draft
dependencies: [05]
created: 2025-01-10
---

# Specification 07: Recalibrate Risk Formula for Coverage-Weighted Analysis

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [05 - Complexity-Coverage Risk Analysis]

## Context

The current risk analysis implementation shows a significant disconnect between actual codebase risk indicators and the calculated risk scores. Analysis of a codebase with 37% coverage and technical debt 12.9x over threshold still reports "LOW" risk (3.6), suggesting the risk formula severely underweights coverage gaps and technical debt accumulation.

Key issues identified:
- Low coverage (37%) not properly reflected in risk scores
- Technical debt score of 1290 (threshold: 100) not impacting overall risk
- All functions showing <1% risk reduction potential
- Zero functions classified as "Well Tested"

## Objective

Recalibrate the risk scoring formula to accurately reflect the multiplicative effects of low coverage combined with high technical debt, ensuring that risk scores provide actionable insights for testing investment decisions.

## Requirements

### Functional Requirements

1. **Coverage Weight Adjustment**
   - Increase coverage weight from current 0.3 to minimum 0.5
   - Implement exponential penalty for coverage below 40%
   - Add bonus multiplier for coverage above 80%

2. **Technical Debt Integration**
   - Incorporate debt score ratio (actual/threshold) into risk calculation
   - Apply multiplicative factor when debt exceeds threshold
   - Scale risk based on debt type severity

3. **Untested Module Penalties**
   - Apply 2x risk multiplier for completely untested files
   - Add cascading risk for untested dependencies
   - Prioritize entry points and core modules

4. **Risk Score Normalization**
   - Ensure risk scores use full 0-10 range effectively
   - Define clear thresholds: Critical (8+), High (6-8), Medium (4-6), Low (2-4), Minimal (<2)
   - Prevent clustering of scores in narrow ranges

### Non-Functional Requirements

1. **Performance**: Risk calculation must complete within 100ms for 1000 functions
2. **Backwards Compatibility**: Support legacy risk score format with --legacy-risk flag
3. **Explainability**: Provide breakdown of risk components on demand
4. **Configurability**: Allow custom weight adjustments via configuration

## Acceptance Criteria

- [ ] Risk scores properly reflect coverage gaps (low coverage = higher risk)
- [ ] Technical debt ratio influences overall risk calculation
- [ ] Untested modules show significantly higher risk than tested ones
- [ ] Risk distribution uses full 0-10 scale effectively
- [ ] At least 10% of functions in typical codebases show as "Well Tested"
- [ ] Risk calculation provides detailed breakdown when requested
- [ ] Performance meets <100ms requirement for 1000 functions
- [ ] Configuration file supports custom weight adjustments
- [ ] Legacy risk calculation available via flag
- [ ] Unit tests validate all risk scenarios
- [ ] Integration tests confirm real-world accuracy

## Technical Details

### Implementation Approach

1. **New Risk Formula Structure**
```rust
pub struct RiskWeights {
    coverage: f64,        // Default: 0.5 (increased from 0.3)
    complexity: f64,      // Default: 0.3
    debt: f64,           // Default: 0.2 (new component)
    untested_penalty: f64, // Default: 2.0
    debt_threshold_multiplier: f64, // Default: 1.5
}

pub fn calculate_risk_v2(
    function: &Function,
    coverage: Option<f64>,
    debt_score: f64,
    debt_threshold: f64,
    weights: &RiskWeights,
) -> RiskScore {
    let base_risk = calculate_base_risk(function, coverage, weights);
    let debt_factor = calculate_debt_factor(debt_score, debt_threshold);
    let coverage_penalty = calculate_coverage_penalty(coverage);
    
    let final_risk = base_risk * debt_factor * coverage_penalty;
    
    RiskScore {
        value: final_risk.min(10.0),
        components: RiskComponents {
            base: base_risk,
            debt_factor,
            coverage_penalty,
            breakdown: generate_breakdown(function, coverage, debt_score),
        },
    }
}
```

2. **Coverage Penalty Function**
```rust
fn calculate_coverage_penalty(coverage: Option<f64>) -> f64 {
    match coverage {
        None => 2.0,  // No coverage data = high penalty
        Some(c) if c < 0.2 => 3.0,  // Critical gap
        Some(c) if c < 0.4 => 2.0,  // Severe gap
        Some(c) if c < 0.6 => 1.5,  // Moderate gap
        Some(c) if c < 0.8 => 1.2,  // Minor gap
        Some(c) => 0.8,  // Good coverage = risk reduction
    }
}
```

3. **Debt Factor Calculation**
```rust
fn calculate_debt_factor(score: f64, threshold: f64) -> f64 {
    let ratio = score / threshold;
    match ratio {
        r if r <= 1.0 => 1.0,  // Within threshold
        r if r <= 2.0 => 1.2,  // Slightly over
        r if r <= 5.0 => 1.5,  // Significantly over
        r if r <= 10.0 => 2.0, // Severely over
        _ => 2.5,  // Critical debt level
    }
}
```

### Architecture Changes

1. **Risk Module Restructuring**
   - Move from `src/risk/mod.rs` single formula to strategy pattern
   - Create `RiskStrategy` trait for different calculation methods
   - Implement `LegacyRiskStrategy` and `EnhancedRiskStrategy`

2. **Configuration Enhancement**
   - Add risk weights to `.debtmap.toml` configuration
   - Support environment variable overrides
   - Provide preset profiles (strict, balanced, lenient)

### Data Structures

```rust
pub struct RiskComponents {
    pub base: f64,
    pub debt_factor: f64,
    pub coverage_penalty: f64,
    pub breakdown: Vec<RiskFactor>,
}

pub struct RiskFactor {
    pub name: String,
    pub weight: f64,
    pub raw_value: f64,
    pub contribution: f64,
}

pub struct RiskProfile {
    pub name: String,
    pub weights: RiskWeights,
    pub thresholds: RiskThresholds,
}
```

### APIs and Interfaces

```rust
// New public API
pub trait RiskCalculator {
    fn calculate(&self, context: &RiskContext) -> RiskScore;
    fn explain(&self, context: &RiskContext) -> RiskExplanation;
}

// Configuration API
pub struct RiskConfig {
    pub strategy: RiskStrategy,
    pub weights: Option<RiskWeights>,
    pub profile: Option<String>,
}
```

## Dependencies

- **Prerequisites**: Spec 05 (Complexity-Coverage Risk Analysis) must be fully implemented
- **Affected Components**: 
  - `src/risk/mod.rs` - Core risk calculation
  - `src/risk/insights.rs` - Risk-based recommendations
  - `src/risk/priority.rs` - Testing prioritization
  - `src/io/writers/*` - Output formatting for risk scores
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test each penalty function independently
  - Validate risk score ranges and distributions
  - Test configuration loading and overrides
  - Verify backwards compatibility mode

- **Integration Tests**: 
  - Test with real codebases of varying coverage levels
  - Validate risk scores against known risk patterns
  - Test performance with large function sets
  - Verify output format compatibility

- **Performance Tests**: 
  - Benchmark risk calculation for 10,000 functions
  - Memory usage profiling for large codebases
  - Cache effectiveness measurements

- **User Acceptance**: 
  - Risk scores align with developer intuition
  - High-risk areas match historical bug locations
  - Recommendations are actionable and valuable

## Documentation Requirements

- **Code Documentation**: 
  - Document all weight constants and their rationale
  - Explain each penalty function with examples
  - Provide risk calculation walkthrough

- **User Documentation**: 
  - Add risk calibration guide to README
  - Document configuration options
  - Provide risk interpretation guidelines
  - Include examples of risk profiles

- **Architecture Updates**: 
  - Update ARCHITECTURE.md with new risk calculation strategy
  - Document the transition from v1 to v2 risk scoring
  - Add decision record for weight choices

## Implementation Notes

1. **Phased Rollout**
   - Phase 1: Implement new formula alongside existing
   - Phase 2: A/B test with select projects
   - Phase 3: Make new formula default with legacy flag
   - Phase 4: Deprecate legacy formula

2. **Validation Approach**
   - Collect baseline metrics from 10+ projects
   - Compare v1 vs v2 risk distributions
   - Validate against historical bug data if available
   - Gather user feedback on risk accuracy

3. **Edge Cases**
   - Handle division by zero in debt ratio
   - Manage missing coverage data gracefully
   - Prevent negative risk scores
   - Cap maximum risk at 10.0

## Migration and Compatibility

- **Breaking Changes**: 
  - Risk score values will change significantly
  - Risk thresholds need recalibration
  - CI/CD pipelines may need adjustment

- **Migration Path**:
  1. Add --legacy-risk flag for existing users
  2. Provide risk score comparison tool
  3. Document threshold adjustments needed
  4. Offer 3-month transition period

- **Configuration Migration**:
  - Auto-generate new config from old thresholds
  - Provide migration wizard for complex setups
  - Log warnings for deprecated settings
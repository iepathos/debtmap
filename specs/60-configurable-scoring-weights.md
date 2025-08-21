---
number: 60
title: Make All Scoring Weights Configurable
category: optimization
priority: medium
status: draft
dependencies: [55, 58]
created: 2025-01-21
---

# Specification 60: Make All Scoring Weights Configurable

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [55, 58]

## Context

The current debtmap scoring system has a mix of configurable and hardcoded weights:

**Configurable weights** (via ScoringWeights):
- Coverage: 30% (40% after spec 55)
- Complexity: 20% (35% after specs 55 & 58)
- ROI: 25% (removed in spec 55)
- Semantic: 5% (removed in spec 58)
- Dependency: 10% (20% after specs 55 & 58)
- Security: 5%
- Organization: 5% (removed in spec 58)

**Hardcoded weights**:
- Testing factor: 0.05 (5%)
- Resource factor: 0.05 (5%)
- Duplication factor: 0.05 (5%)

This inconsistency creates several problems:
1. Users cannot tune all aspects of scoring
2. Hardcoded values don't respect configuration
3. Weights don't sum to 1.0 when additional factors are included
4. Different codebases need different weight distributions

## Objective

Make all scoring weights fully configurable through a consistent configuration system that:
- Allows tuning of every scoring factor
- Validates weights sum to 1.0 (or auto-normalizes)
- Provides sensible defaults for common scenarios
- Supports weight presets for different project types
- Enables complete control over scoring priorities

## Requirements

### Functional Requirements

1. **Complete Weight Configuration**
   - Add testing_weight to ScoringWeights
   - Add resource_weight to ScoringWeights
   - Add duplication_weight to ScoringWeights
   - Add performance_weight for future use
   - Ensure all weights are loaded from configuration

2. **Weight Validation**
   - Validate weights sum to 1.0 (±0.01 tolerance)
   - Auto-normalize if sum differs from 1.0
   - Warn user about normalization
   - Prevent negative weights

3. **Weight Presets**
   - Default: Balanced weights for general projects
   - Security-focused: Higher security and validation weights
   - Performance-focused: Higher complexity and resource weights
   - Test-focused: Higher coverage and testing weights
   - Legacy: Higher duplication and organization weights

4. **Configuration Interface**
   - YAML/TOML configuration file support
   - Environment variable overrides
   - Command-line flag overrides
   - Interactive weight tuning mode

### Non-Functional Requirements

1. **Flexibility**: Support any weight distribution
2. **Validation**: Prevent invalid configurations
3. **Documentation**: Clear explanation of each weight
4. **Performance**: No impact on scoring speed
5. **Backwards Compatibility**: Existing configs continue working

## Acceptance Criteria

- [ ] All scoring weights configurable via ScoringWeights
- [ ] No hardcoded weight values in scoring calculation
- [ ] Weight validation ensures sum equals 1.0
- [ ] Auto-normalization with user notification
- [ ] At least 3 weight presets implemented
- [ ] Configuration file loading works
- [ ] Environment variable overrides work
- [ ] Command-line overrides work
- [ ] Tests verify weight flexibility
- [ ] Documentation explains all weights

## Technical Details

### Implementation Approach

1. **Expand ScoringWeights Structure**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    // Core factors (must sum to 1.0)
    #[serde(default = "default_coverage_weight")]
    pub coverage: f64,          // Default: 0.40
    
    #[serde(default = "default_complexity_weight")]
    pub complexity: f64,        // Default: 0.35
    
    #[serde(default = "default_dependency_weight")]
    pub dependency: f64,        // Default: 0.20
    
    #[serde(default = "default_security_weight")]
    pub security: f64,          // Default: 0.05
    
    // Additional factors (previously hardcoded)
    #[serde(default = "default_testing_weight")]
    pub testing: f64,           // Default: 0.00
    
    #[serde(default = "default_resource_weight")]
    pub resource: f64,          // Default: 0.00
    
    #[serde(default = "default_duplication_weight")]
    pub duplication: f64,       // Default: 0.00
    
    #[serde(default = "default_performance_weight")]
    pub performance: f64,       // Default: 0.00
}

impl ScoringWeights {
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.coverage + self.complexity + self.dependency + 
                 self.security + self.testing + self.resource + 
                 self.duplication + self.performance;
        
        if (sum - 1.0).abs() > 0.01 {
            return Err(format!("Weights sum to {:.2}, not 1.0", sum));
        }
        
        if self.any_negative() {
            return Err("Negative weights not allowed".to_string());
        }
        
        Ok(())
    }
    
    pub fn normalize(&mut self) {
        let sum = self.sum();
        if sum > 0.0 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.dependency /= sum;
            self.security /= sum;
            self.testing /= sum;
            self.resource /= sum;
            self.duplication /= sum;
            self.performance /= sum;
        }
    }
}
```

2. **Weight Presets**
```rust
pub enum WeightPreset {
    Default,
    SecurityFocused,
    PerformanceFocused,
    TestFocused,
    Legacy,
    Custom(ScoringWeights),
}

impl WeightPreset {
    pub fn to_weights(&self) -> ScoringWeights {
        match self {
            WeightPreset::Default => ScoringWeights {
                coverage: 0.40,
                complexity: 0.35,
                dependency: 0.20,
                security: 0.05,
                testing: 0.00,
                resource: 0.00,
                duplication: 0.00,
                performance: 0.00,
            },
            WeightPreset::SecurityFocused => ScoringWeights {
                coverage: 0.30,
                complexity: 0.25,
                dependency: 0.15,
                security: 0.25,
                testing: 0.05,
                resource: 0.00,
                duplication: 0.00,
                performance: 0.00,
            },
            WeightPreset::TestFocused => ScoringWeights {
                coverage: 0.50,
                complexity: 0.20,
                dependency: 0.10,
                security: 0.05,
                testing: 0.15,
                resource: 0.00,
                duplication: 0.00,
                performance: 0.00,
            },
            // ... other presets
            WeightPreset::Custom(weights) => weights.clone(),
        }
    }
}
```

3. **Update Score Calculation**
```rust
// In calculate_unified_priority_with_debt
let weights = config::get_scoring_weights();

// No more hardcoded weights
let weighted_complexity = complexity_factor * weights.complexity;
let weighted_coverage = coverage_factor * weights.coverage;
let weighted_dependency = dependency_factor * weights.dependency;
let weighted_security = security_factor * weights.security;
let weighted_testing = testing_factor * weights.testing;      // Was hardcoded 0.05
let weighted_resource = resource_factor * weights.resource;   // Was hardcoded 0.05
let weighted_duplication = duplication_factor * weights.duplication; // Was hardcoded 0.05
let weighted_performance = performance_factor * weights.performance; // New

let base_score = weighted_complexity + weighted_coverage + 
                weighted_dependency + weighted_security +
                weighted_testing + weighted_resource + 
                weighted_duplication + weighted_performance;
```

### Architecture Changes

- Expand configuration system to handle all weights
- Add weight validation and normalization
- Implement preset system
- Update CLI to accept weight overrides

### Data Structures

- Expanded ScoringWeights with all factors
- WeightPreset enum for common configurations
- Validation results for weight configuration

## Dependencies

- **Prerequisites**: 
  - Spec 55: Remove ROI from Scoring (simplifies weights)
  - Spec 58: Remove Double Penalties (reduces factors)
- **Affected Components**:
  - Configuration system
  - Unified scorer
  - CLI argument parser
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test weight validation (sum to 1.0)
  - Test normalization algorithm
  - Test preset configurations
  - Test negative weight rejection
- **Integration Tests**:
  - Test configuration file loading
  - Test environment variable overrides
  - Test CLI flag overrides
  - Verify scoring with different weights
- **Configuration Tests**:
  - Test invalid weight combinations
  - Test extreme weight distributions
  - Test zero weights for factors

## Documentation Requirements

- **Code Documentation**:
  - Document each weight's purpose
  - Explain normalization behavior
  - Describe preset rationales
- **User Documentation**:
  - Weight configuration guide
  - Preset selection guide
  - Examples for different project types
  - Migration from hardcoded weights
- **Configuration Documentation**:
  - YAML/TOML examples
  - Environment variable names
  - CLI flag documentation

## Implementation Notes

1. **Default Weights Rationale**:
   - Coverage (40%): Most important for risk
   - Complexity (35%): Core maintainability metric
   - Dependency (20%): Critical path importance
   - Security (5%): Specialized but important

2. **Preset Use Cases**:
   - SecurityFocused: Financial, healthcare apps
   - TestFocused: High-reliability systems
   - PerformanceFocused: Real-time systems
   - Legacy: Refactoring old codebases

3. **Configuration Priority**:
   1. CLI flags (highest priority)
   2. Environment variables
   3. Configuration file
   4. Preset selection
   5. Defaults (lowest priority)

## Migration and Compatibility

### Breaking Changes
- Configuration files need new weight fields
- Scores will change based on weight redistribution

### Migration Path
1. Existing configs use defaults for new weights
2. Validation warns about missing weights
3. Auto-normalization maintains validity

### Compatibility
- Old configuration files still work
- Missing weights default to 0.0
- Automatic normalization ensures validity

## Expected Outcomes

1. **Full Customization**: Every aspect of scoring tunable
2. **Project-Specific Tuning**: Optimize for specific needs
3. **Consistent Configuration**: All weights in one place
4. **Better Documentation**: Clear understanding of weights
5. **Preset Convenience**: Quick setup for common scenarios

## Risks and Mitigation

1. **Risk**: Users create invalid weight distributions
   - **Mitigation**: Validation and auto-normalization

2. **Risk**: Too many options confuse users
   - **Mitigation**: Good defaults and presets

3. **Risk**: Weights don't sum to 1.0
   - **Mitigation**: Automatic normalization with warning
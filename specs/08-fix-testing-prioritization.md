---
number: 08
title: Fix Testing Prioritization Algorithm
category: optimization
priority: critical
status: draft
dependencies: [05, 07]
created: 2025-01-10
---

# Specification 08: Fix Testing Prioritization Algorithm

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [05 - Complexity-Coverage Risk Analysis, 07 - Recalibrate Risk Formula]

## Context

The current testing prioritization algorithm produces suboptimal recommendations that fail to identify the most impactful testing opportunities. Analysis reveals several critical issues:

1. **Uniform ROI Problem**: All recommendations show identical ROI (1.1), indicating a flawed calculation
2. **Wrong Focus**: Low-complexity functions prioritized over completely untested critical modules
3. **Ignored Zero-Coverage**: Modules with 0% coverage (main.rs, terminal.rs, insights.rs) not prioritized
4. **Unrealistic Impact**: All recommendations show <1% risk reduction
5. **Missing Context**: Entry points and core modules not given appropriate weight

The system currently recommends testing functions with complexity 4-6 while ignoring entire untested subsystems with hundreds of lines of code.

## Objective

Redesign the testing prioritization algorithm to identify and rank testing opportunities that provide maximum risk reduction and business value, with special emphasis on zero-coverage modules, critical paths, and high-impact components.

## Requirements

### Functional Requirements

1. **Zero-Coverage Priority**
   - Always prioritize completely untested modules first
   - Rank zero-coverage modules by criticality and size
   - Identify and highlight untested entry points

2. **Module Criticality Assessment**
   - Assign criticality scores based on module type:
     - Entry points (main.rs, lib.rs): Critical
     - Core business logic: High
     - IO/Output modules: Medium
     - Utilities: Low
   - Factor dependency count into criticality

3. **Dynamic ROI Calculation**
   - Calculate actual risk reduction potential
   - Factor in test writing effort based on complexity
   - Include coverage improvement impact
   - Consider cascade effects on dependent modules

4. **Smart Grouping**
   - Group related functions for batch testing
   - Identify test suite opportunities
   - Suggest integration test targets
   - Recommend module-level test strategies

5. **Effort Estimation**
   - Estimate test cases needed based on complexity
   - Provide time estimates for test development
   - Account for setup/teardown complexity
   - Consider mocking requirements

### Non-Functional Requirements

1. **Accuracy**: ROI calculations must reflect realistic outcomes
2. **Performance**: Prioritization completes in <500ms for 1000 functions
3. **Explainability**: Each recommendation includes clear rationale
4. **Configurability**: Support custom priority weights
5. **Adaptability**: Learn from historical testing patterns

## Acceptance Criteria

- [ ] Zero-coverage modules appear first in recommendations
- [ ] ROI values show meaningful variation (not all 1.1)
- [ ] Critical paths properly identified and prioritized
- [ ] Risk reduction estimates are realistic (1-20% range)
- [ ] Effort estimates align with complexity metrics
- [ ] Module grouping produces logical test suites
- [ ] Entry points receive highest priority
- [ ] Recommendations include clear rationale
- [ ] Performance meets <500ms requirement
- [ ] Configuration supports custom weights
- [ ] Unit tests cover all prioritization scenarios
- [ ] Integration tests validate real-world effectiveness

## Technical Details

### Implementation Approach

1. **Multi-Stage Prioritization Pipeline**
```rust
pub struct PrioritizationPipeline {
    stages: Vec<Box<dyn PrioritizationStage>>,
}

impl PrioritizationPipeline {
    pub fn new() -> Self {
        Self {
            stages: vec![
                Box::new(ZeroCoverageStage::new()),
                Box::new(CriticalPathStage::new()),
                Box::new(ComplexityRiskStage::new()),
                Box::new(DependencyImpactStage::new()),
                Box::new(EffortOptimizationStage::new()),
            ],
        }
    }
}
```

2. **Module Criticality Scoring**
```rust
pub struct CriticalityScorer {
    patterns: HashMap<String, f64>,
}

impl CriticalityScorer {
    pub fn score(&self, module: &Module) -> f64 {
        let base_score = self.pattern_match_score(&module.path);
        let dependency_factor = self.dependency_score(module);
        let size_factor = (module.lines as f64).ln() / 10.0;
        
        (base_score * dependency_factor * size_factor).min(10.0)
    }
    
    fn pattern_match_score(&self, path: &Path) -> f64 {
        match path.file_name().and_then(|n| n.to_str()) {
            Some("main.rs") | Some("lib.rs") => 10.0,
            Some(name) if name.contains("core") => 8.0,
            Some(name) if name.contains("api") => 7.0,
            Some(name) if name.contains("service") => 6.0,
            Some(name) if name.contains("model") => 5.0,
            Some(name) if name.contains("util") => 3.0,
            _ => 4.0,
        }
    }
}
```

3. **Dynamic ROI Calculation**
```rust
pub struct ROICalculator {
    risk_model: Box<dyn RiskCalculator>,
    effort_model: Box<dyn EffortEstimator>,
}

impl ROICalculator {
    pub fn calculate(&self, target: &TestTarget) -> ROI {
        let risk_reduction = self.calculate_risk_reduction(target);
        let effort = self.effort_model.estimate(target);
        let cascade_impact = self.calculate_cascade_effect(target);
        
        ROI {
            value: (risk_reduction + cascade_impact) / effort,
            risk_reduction,
            effort,
            cascade_impact,
            explanation: self.generate_explanation(target),
        }
    }
    
    fn calculate_risk_reduction(&self, target: &TestTarget) -> f64 {
        let current_risk = target.current_risk;
        let projected_risk = self.risk_model.project_with_coverage(
            target,
            target.projected_coverage,
        );
        
        ((current_risk - projected_risk) / current_risk) * 100.0
    }
}
```

4. **Test Effort Estimation**
```rust
pub struct EffortEstimator {
    complexity_weights: ComplexityWeights,
}

impl EffortEstimator {
    pub fn estimate(&self, target: &TestTarget) -> f64 {
        let base_effort = self.complexity_to_test_cases(target.complexity);
        let setup_effort = self.estimate_setup_complexity(target);
        let mock_effort = self.estimate_mocking_needs(target);
        
        base_effort + setup_effort + mock_effort
    }
    
    fn complexity_to_test_cases(&self, complexity: &Complexity) -> f64 {
        // McCabe's formula: minimum test cases = cyclomatic complexity + 1
        let min_cases = complexity.cyclomatic as f64 + 1.0;
        // Adjust for cognitive complexity
        let cognitive_factor = (complexity.cognitive as f64 / 10.0).max(1.0);
        
        min_cases * cognitive_factor
    }
}
```

### Architecture Changes

1. **Prioritization Module Restructure**
   - Replace monolithic prioritization function with pipeline
   - Introduce stage-based processing for flexibility
   - Add caching layer for criticality scores

2. **Integration Points**
   - Connect with dependency graph for impact analysis
   - Integrate with git history for change frequency
   - Link with issue tracker for bug density (future)

### Data Structures

```rust
pub struct TestTarget {
    pub id: String,
    pub path: PathBuf,
    pub function: Option<String>,
    pub module_type: ModuleType,
    pub current_coverage: f64,
    pub current_risk: f64,
    pub complexity: Complexity,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub lines: usize,
}

pub struct TestRecommendation {
    pub target: TestTarget,
    pub priority: f64,
    pub roi: ROI,
    pub effort: TestEffort,
    pub impact: ImpactAnalysis,
    pub rationale: String,
    pub suggested_approach: TestApproach,
}

pub struct TestEffort {
    pub estimated_cases: usize,
    pub estimated_hours: f64,
    pub complexity_level: ComplexityLevel,
    pub setup_requirements: Vec<String>,
}

pub enum TestApproach {
    UnitTest,
    IntegrationTest,
    ModuleTest,
    EndToEndTest,
}
```

### APIs and Interfaces

```rust
pub trait PrioritizationStage {
    fn process(&self, targets: Vec<TestTarget>) -> Vec<TestTarget>;
    fn name(&self) -> &str;
}

pub trait EffortEstimator {
    fn estimate(&self, target: &TestTarget) -> f64;
    fn explain(&self, target: &TestTarget) -> String;
}

pub trait ImpactAnalyzer {
    fn analyze(&self, target: &TestTarget) -> ImpactAnalysis;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 05 (Complexity-Coverage Risk Analysis) for risk calculations
  - Spec 07 (Recalibrate Risk Formula) for accurate risk scoring
- **Affected Components**:
  - `src/risk/priority.rs` - Complete rewrite
  - `src/risk/insights.rs` - Update recommendation generation
  - `src/risk/mod.rs` - Add criticality scoring
  - `src/core/metrics.rs` - Enhance with module metadata
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test each prioritization stage independently
  - Validate ROI calculations with known inputs
  - Test effort estimation accuracy
  - Verify criticality scoring patterns

- **Integration Tests**:
  - Test full pipeline with sample codebases
  - Validate zero-coverage prioritization
  - Test module grouping logic
  - Verify recommendation quality

- **Performance Tests**:
  - Benchmark with 10,000 functions
  - Measure memory usage for large graphs
  - Test caching effectiveness

- **User Acceptance**:
  - Recommendations match developer intuition
  - Effort estimates align with actual time
  - Priority order makes practical sense
  - Rationales are clear and actionable

## Documentation Requirements

- **Code Documentation**:
  - Document prioritization pipeline stages
  - Explain ROI calculation methodology
  - Describe effort estimation formulas
  - Provide criticality scoring rationale

- **User Documentation**:
  - Add prioritization guide to README
  - Document configuration options
  - Provide interpretation guidelines
  - Include practical examples

- **Architecture Updates**:
  - Update ARCHITECTURE.md with new pipeline design
  - Document stage processing order
  - Add decision records for algorithm choices

## Implementation Notes

1. **Incremental Development**
   - Phase 1: Implement zero-coverage prioritization
   - Phase 2: Add criticality scoring
   - Phase 3: Integrate dynamic ROI
   - Phase 4: Add effort estimation
   - Phase 5: Implement smart grouping

2. **Validation Strategy**
   - Compare with manual expert prioritization
   - Track actual vs estimated effort
   - Measure risk reduction outcomes
   - Gather user feedback iteratively

3. **Special Considerations**
   - Handle monorepos with multiple entry points
   - Support different testing frameworks
   - Account for test infrastructure complexity
   - Consider existing test patterns

## Migration and Compatibility

- **Breaking Changes**:
  - Recommendation format changes
  - Priority scores use different scale
  - ROI calculation completely different

- **Migration Path**:
  1. Parallel run old and new algorithms
  2. Provide comparison tool
  3. Gradual rollout with feature flag
  4. Full deprecation after validation

- **Backwards Compatibility**:
  - Support legacy output format with flag
  - Provide mapping between old/new scores
  - Document all changes clearly
---
number: 24
title: Refined Risk Scoring Methodology
category: foundation
priority: high
status: draft
dependencies: [13]
created: 2025-01-13
---

# Specification 24: Refined Risk Scoring Methodology

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [13 - Add Comprehensive Risk Categories]

## Context

The current risk scoring system has several critical flaws that undermine the accuracy and usefulness of debtmap's recommendations:

1. **Generic "Technical Debt" Classification**: Functions receive arbitrary risk scores (5.0) without meaningful context about what type of risk they represent
2. **Meaningless Risk Values**: Risk scores lack clear thresholds, baselines, or actionable meaning
3. **False Risk Attribution**: Well-designed functions like `create_json_output()` and `write_results()` incorrectly flagged as risky
4. **No Risk Context**: Users don't understand why something is risky or what actions to take
5. **Poor Risk Prioritization**: Low-complexity functions incorrectly assumed to be problematic

This results in noise that masks genuine risks and erodes user confidence in debtmap's analysis.

## Objective

Replace the current generic risk scoring with a precise, context-aware methodology that provides meaningful risk classifications, clear thresholds, and actionable recommendations based on evidence-driven risk assessment rather than arbitrary numerical assignments.

## Requirements

### Functional Requirements

1. **Evidence-Based Risk Assessment**
   - Risk scores derived from measurable code characteristics
   - Clear correlation between risk factors and scores
   - Transparent calculation methodology
   - Confidence intervals for risk predictions

2. **Contextual Risk Classification**
   - Replace generic "technical debt" with specific risk types
   - Function role-aware risk assessment (orchestrator vs. logic vs. I/O)
   - Module criticality influence on risk scores
   - Historical change patterns impact on risk

3. **Actionable Risk Categories**
   - Each risk type maps to specific remediation actions
   - Clear priority ordering based on impact and effort
   - ROI calculations for risk reduction activities
   - Success metrics for risk mitigation

4. **Meaningful Risk Thresholds**
   - Statistical baselines derived from large codebases
   - Percentile-based risk classifications (P50, P90, P95, P99)
   - Industry-standard risk levels with clear meanings
   - Configurable thresholds for different project contexts

5. **Risk Trend Analysis**
   - Track risk changes over time
   - Identify risk pattern trends
   - Predict future risk trajectory
   - Alert on significant risk increases

### Non-Functional Requirements

1. **Precision**: Risk classifications accurate for 90% of functions
2. **Transparency**: All risk factors clearly documented and explainable
3. **Performance**: Risk scoring adds <10% to analysis time
4. **Configurability**: Teams can adjust thresholds and weights

## Acceptance Criteria

- [ ] Generic "technical debt" classification eliminated
- [ ] Risk scores based on measurable evidence (complexity, coverage, coupling, etc.)
- [ ] Clear risk thresholds with statistical basis (P50, P90, P95, P99)
- [ ] Each risk type has specific, actionable remediation recommendations
- [ ] Function role consideration in risk assessment (orchestrator vs. pure logic)
- [ ] False positives reduced by 80% compared to current system
- [ ] Risk explanations clearly communicate why function is risky
- [ ] Integration with existing priority scoring system
- [ ] Performance impact under 10% of total analysis time
- [ ] Comprehensive test suite with known risk patterns
- [ ] Documentation explaining all risk factors and thresholds

## Technical Details

### Implementation Approach

1. **Evidence-Based Risk Calculator**
```rust
pub struct EvidenceBasedRiskCalculator {
    complexity_analyzer: ComplexityRiskAnalyzer,
    coverage_analyzer: CoverageRiskAnalyzer,
    coupling_analyzer: CouplingRiskAnalyzer,
    change_analyzer: ChangeFrequencyAnalyzer,
    role_classifier: FunctionRoleClassifier,
    threshold_provider: StatisticalThresholdProvider,
}

impl EvidenceBasedRiskCalculator {
    pub fn calculate_risk(&self, function: &FunctionAnalysis) -> RiskAssessment {
        let role = self.role_classifier.classify_function(function);
        let context = self.build_risk_context(function);
        
        let risk_factors = vec![
            self.complexity_analyzer.analyze(function, &context),
            self.coverage_analyzer.analyze(function, &context),
            self.coupling_analyzer.analyze(function, &context),
            self.change_analyzer.analyze(function, &context),
        ];
        
        let risk_score = self.aggregate_risk_factors(&risk_factors, &role);
        let risk_classification = self.classify_risk_level(risk_score, &role);
        let recommendations = self.generate_recommendations(&risk_factors, &role);
        
        RiskAssessment {
            score: risk_score,
            classification: risk_classification,
            factors: risk_factors,
            role_context: role,
            recommendations,
            confidence: self.calculate_confidence(&risk_factors),
        }
    }
}
```

2. **Specific Risk Analyzers**
```rust
pub struct ComplexityRiskAnalyzer {
    thresholds: ComplexityThresholds,
}

impl ComplexityRiskAnalyzer {
    pub fn analyze(&self, function: &FunctionAnalysis, context: &RiskContext) -> RiskFactor {
        let cyclomatic = function.metrics.cyclomatic_complexity;
        let cognitive = function.metrics.cognitive_complexity;
        let lines = function.metrics.lines_of_code;
        
        // Role-adjusted complexity thresholds
        let adjusted_thresholds = self.adjust_for_role(&context.role);
        
        let complexity_score = self.calculate_complexity_risk(
            cyclomatic,
            cognitive,
            lines,
            &adjusted_thresholds
        );
        
        let evidence = ComplexityEvidence {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            lines_of_code: lines,
            threshold_exceeded: complexity_score > adjusted_thresholds.moderate,
            role_adjusted: context.role != FunctionRole::PureLogic,
        };
        
        RiskFactor {
            risk_type: RiskType::Complexity,
            score: complexity_score,
            severity: self.classify_complexity_severity(complexity_score, &adjusted_thresholds),
            evidence: RiskEvidence::Complexity(evidence),
            remediation_actions: self.get_complexity_actions(complexity_score),
            weight: self.get_weight_for_role(&context.role),
        }
    }
    
    fn adjust_for_role(&self, role: &FunctionRole) -> ComplexityThresholds {
        match role {
            FunctionRole::PureLogic => self.thresholds.clone(), // Strict thresholds
            FunctionRole::Orchestrator => ComplexityThresholds {
                low: self.thresholds.low * 1.5,
                moderate: self.thresholds.moderate * 1.5,
                high: self.thresholds.high * 1.5,
                critical: self.thresholds.critical * 1.5,
            },
            FunctionRole::IOWrapper => ComplexityThresholds {
                low: self.thresholds.low * 2.0, // Very lenient for I/O
                moderate: self.thresholds.moderate * 2.0,
                high: self.thresholds.high * 2.0,
                critical: self.thresholds.critical * 2.0,
            },
            FunctionRole::EntryPoint => ComplexityThresholds {
                low: self.thresholds.low * 1.2,
                moderate: self.thresholds.moderate * 1.2,
                high: self.thresholds.high * 1.2,
                critical: self.thresholds.critical * 1.2,
            },
        }
    }
}
```

3. **Statistical Threshold Provider**
```rust
pub struct StatisticalThresholdProvider {
    baseline_data: BaselineDatabase,
    project_context: ProjectContext,
}

impl StatisticalThresholdProvider {
    pub fn get_complexity_thresholds(&self, role: &FunctionRole) -> ComplexityThresholds {
        let baseline = self.baseline_data.get_complexity_distribution(role);
        
        ComplexityThresholds {
            low: baseline.percentile(50),      // P50 - median
            moderate: baseline.percentile(75), // P75 - above average
            high: baseline.percentile(90),     // P90 - high
            critical: baseline.percentile(95), // P95 - very high
        }
    }
    
    pub fn get_coverage_thresholds(&self, role: &FunctionRole) -> CoverageThresholds {
        let baseline = self.baseline_data.get_coverage_distribution(role);
        
        CoverageThresholds {
            excellent: baseline.percentile(90), // P90 - well tested
            good: baseline.percentile(75),      // P75 - adequately tested
            moderate: baseline.percentile(50),  // P50 - some testing
            poor: baseline.percentile(25),      // P25 - minimal testing
            critical: baseline.percentile(10),  // P10 - essentially untested
        }
    }
}
```

4. **Risk Classification System**
```rust
pub enum RiskType {
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
        lines: u32,
        threshold_type: ComplexityThreshold,
    },
    Coverage {
        coverage_percentage: f64,
        critical_paths_uncovered: u32,
        test_quality: TestQuality,
    },
    Coupling {
        afferent_coupling: u32,
        efferent_coupling: u32,
        instability: f64,
        circular_dependencies: u32,
    },
    ChangeFrequency {
        commits_last_month: u32,
        bug_fix_ratio: f64,
        hotspot_intensity: f64,
    },
    Architecture {
        layer_violations: u32,
        god_class_indicators: Vec<String>,
        single_responsibility_score: f64,
    },
}

pub enum RiskSeverity {
    None,        // No significant risk
    Low,         // Monitor but no immediate action needed
    Moderate,    // Should be addressed in next sprint
    High,        // Should be addressed this sprint
    Critical,    // Immediate attention required
}

pub struct RiskAssessment {
    pub score: f64,
    pub classification: RiskClassification,
    pub factors: Vec<RiskFactor>,
    pub role_context: FunctionRole,
    pub recommendations: Vec<RemediationAction>,
    pub confidence: f64,
    pub explanation: String,
}

pub enum RiskClassification {
    WellDesigned,     // Score 0.0-2.0 - Good example
    Acceptable,       // Score 2.0-4.0 - Minor improvements possible
    NeedsImprovement, // Score 4.0-7.0 - Should be refactored
    Risky,           // Score 7.0-9.0 - High priority for improvement
    Critical,        // Score 9.0-10.0 - Immediate attention required
}
```

5. **Remediation Action System**
```rust
pub enum RemediationAction {
    RefactorComplexity {
        current_complexity: u32,
        target_complexity: u32,
        suggested_techniques: Vec<RefactoringTechnique>,
        estimated_effort_hours: u32,
        expected_risk_reduction: f64,
    },
    AddTestCoverage {
        current_coverage: f64,
        target_coverage: f64,
        critical_paths: Vec<String>,
        test_types_needed: Vec<TestType>,
        estimated_effort_hours: u32,
    },
    ReduceCoupling {
        current_coupling: CouplingMetrics,
        coupling_issues: Vec<CouplingIssue>,
        suggested_patterns: Vec<DesignPattern>,
        estimated_effort_hours: u32,
    },
    ExtractLogic {
        extraction_candidates: Vec<ExtractionCandidate>,
        pure_function_opportunities: u32,
        testability_improvement: f64,
    },
}

pub enum RefactoringTechnique {
    ExtractMethod,
    ReduceNesting,
    EliminateElseAfterReturn,
    ReplaceConditionalWithPolymorphism,
    IntroduceParameterObject,
    ExtractClass,
}
```

### Architecture Changes

1. **Risk Module Restructuring**
   - Create `src/risk/evidence/` for evidence-based analyzers
   - Add `src/risk/thresholds/` for statistical threshold management
   - Implement `src/risk/classification/` for risk categorization
   - Create `src/risk/remediation/` for action recommendations

2. **Integration Points**
   - Replace generic risk scoring throughout codebase
   - Update output formats to show risk explanations
   - Integrate with priority scoring system
   - Connect to existing coverage and complexity analysis

### Data Structures

```rust
pub struct RiskFactor {
    pub risk_type: RiskType,
    pub score: f64,
    pub severity: RiskSeverity,
    pub evidence: RiskEvidence,
    pub remediation_actions: Vec<RemediationAction>,
    pub weight: f64,
    pub confidence: f64,
}

pub enum RiskEvidence {
    Complexity(ComplexityEvidence),
    Coverage(CoverageEvidence),
    Coupling(CouplingEvidence),
    ChangeFrequency(ChangeEvidence),
    Architecture(ArchitectureEvidence),
}

pub struct ComplexityEvidence {
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub lines_of_code: u32,
    pub nesting_depth: u32,
    pub threshold_exceeded: bool,
    pub role_adjusted: bool,
    pub comparison_to_baseline: ComparisonResult,
}

pub enum ComparisonResult {
    BelowMedian,    // Better than 50% of similar functions
    AboveMedian,    // Worse than 50% of similar functions
    AboveP75,       // Worse than 75% of similar functions
    AboveP90,       // Worse than 90% of similar functions
    AboveP95,       // Worse than 95% of similar functions
}

pub struct BaselineDatabase {
    complexity_distributions: HashMap<FunctionRole, StatisticalDistribution>,
    coverage_distributions: HashMap<FunctionRole, StatisticalDistribution>,
    coupling_distributions: HashMap<ModuleType, StatisticalDistribution>,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 13 (Add Comprehensive Risk Categories) for category framework
- **Affected Components**:
  - `src/risk/` - Complete refactoring of risk scoring
  - `src/debt/prioritization.rs` - Update to use new risk scores
  - `src/io/writers/` - Update output to show risk explanations
  - All existing risk-related tests
- **External Dependencies**:
  - Statistical computation library (e.g., `statrs` crate)

## Testing Strategy

- **Unit Tests**:
  - Test each risk analyzer with known patterns
  - Validate statistical threshold calculations
  - Test role-based threshold adjustments
  - Verify risk classification accuracy

- **Integration Tests**:
  - Test with codebases having known risk patterns
  - Validate against manually classified functions
  - Compare against existing static analysis tools
  - Performance testing with large codebases

- **Validation Tests**:
  - Expert review of risk classifications
  - A/B testing with development teams
  - Comparison with historical bug data
  - False positive/negative analysis

## Documentation Requirements

- **Code Documentation**:
  - Document all risk calculation algorithms
  - Explain statistical baseline methodology
  - Document threshold derivation process
  - Provide risk factor interpretation guide

- **User Documentation**:
  - Risk assessment methodology guide
  - Interpretation handbook for risk scores
  - Action planning guide for remediation
  - Configuration guide for custom thresholds

## Implementation Notes

1. **Phased Implementation**
   - Phase 1: Evidence-based complexity and coverage risk
   - Phase 2: Coupling and architecture risk factors
   - Phase 3: Change frequency and historical analysis
   - Phase 4: Custom risk factor plugins

2. **Statistical Baseline Collection**
   - Analyze large open-source Rust codebases
   - Build representative baseline distributions
   - Regular updates to baseline data
   - Project-specific baseline learning

3. **Risk Score Calibration**
   - Start with conservative thresholds
   - Gradual refinement based on user feedback
   - A/B testing for threshold optimization
   - Machine learning for pattern recognition

## Migration and Compatibility

- **Breaking Changes**:
  - Risk score values will change significantly
  - Risk output format includes more detail
  - Some functions previously flagged may not be flagged

- **Migration Strategy**:
  - Parallel scoring period to compare results
  - Migration guide for interpreting new scores
  - Preserve historical data for trend analysis
  - Gradual rollout with feedback collection

- **Backward Compatibility**:
  - Option to use legacy risk scoring temporarily
  - Export compatibility for existing integrations
  - Maintain same CLI interface
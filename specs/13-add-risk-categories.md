---
number: 13
title: Add Comprehensive Risk Categories
category: foundation
priority: high
status: draft
dependencies: [07, 10]
created: 2025-01-10
---

# Specification 13: Add Comprehensive Risk Categories

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [07 - Recalibrate Risk Formula, 10 - Add Context-Aware Risk]

## Context

The current risk analysis provides a single aggregate risk score that conflates multiple distinct risk types, making it difficult to understand the nature of risks and take appropriate mitigation actions. Different types of risk require different remediation strategies:

- **Coverage Risk**: Requires writing tests
- **Integration Risk**: Needs integration testing
- **Architectural Risk**: Demands refactoring
- **Maintenance Risk**: Benefits from documentation and simplification

Without categorized risk assessment, teams cannot prioritize the right type of improvement work or understand the risk profile of their codebase.

## Objective

Implement a comprehensive risk categorization system that identifies, measures, and reports distinct risk types, enabling teams to understand their risk profile and choose appropriate mitigation strategies for each category of risk.

## Requirements

### Functional Requirements

1. **Coverage Risk Assessment**
   - Direct test coverage gaps
   - Branch coverage analysis
   - Path coverage estimation
   - Mutation testing readiness
   - Test quality indicators

2. **Integration Risk Analysis**
   - Untested interaction paths
   - API boundary coverage
   - Cross-module dependencies
   - External service interfaces
   - Event-driven interactions

3. **Architectural Risk Evaluation**
   - High coupling metrics
   - Circular dependencies
   - Layering violations
   - Single points of failure
   - Monolithic components

4. **Maintenance Risk Measurement**
   - Code complexity metrics
   - Documentation coverage
   - Code duplication levels
   - Technical debt accumulation
   - Knowledge concentration

5. **Security Risk Indicators**
   - Untested security paths
   - Input validation gaps
   - Authentication/authorization coverage
   - Sensitive data handling
   - Known vulnerability patterns

6. **Performance Risk Detection**
   - Untested performance paths
   - Algorithm complexity
   - Resource usage patterns
   - Scalability bottlenecks
   - Cache effectiveness

### Non-Functional Requirements

1. **Granularity**: Support file, module, and system-level categories
2. **Composability**: Allow risk categories to be combined
3. **Extensibility**: Easy to add new risk categories
4. **Visualization**: Support risk profile visualization
5. **Actionability**: Each category maps to clear actions

## Acceptance Criteria

- [ ] All six risk categories properly calculated
- [ ] Risk profiles show category breakdown
- [ ] Each category has clear thresholds
- [ ] Categories map to specific actions
- [ ] Visualization clearly shows risk distribution
- [ ] Aggregation works at multiple levels
- [ ] Categories can be weighted differently
- [ ] Historical tracking of category changes
- [ ] Export supports category breakdown
- [ ] Documentation explains each category
- [ ] Unit tests cover all categories
- [ ] Integration tests validate accuracy

## Technical Details

### Implementation Approach

1. **Risk Category Framework**
```rust
pub trait RiskCategory {
    fn name(&self) -> &str;
    fn calculate(&self, target: &AnalysisTarget) -> CategoryRisk;
    fn aggregate(&self, risks: Vec<CategoryRisk>) -> CategoryRisk;
    fn threshold(&self) -> RiskThreshold;
    fn actions(&self) -> Vec<MitigationAction>;
}

pub struct RiskCategorySystem {
    categories: Vec<Box<dyn RiskCategory>>,
    weights: HashMap<String, f64>,
}

impl RiskCategorySystem {
    pub fn analyze(&self, codebase: &Codebase) -> RiskProfile {
        let mut profile = RiskProfile::new();
        
        for category in &self.categories {
            let risk = category.calculate(&codebase);
            profile.add_category(category.name(), risk);
        }
        
        profile.calculate_aggregate(&self.weights);
        profile
    }
}
```

2. **Coverage Risk Implementation**
```rust
pub struct CoverageRiskCategory {
    thresholds: CoverageThresholds,
}

impl RiskCategory for CoverageRiskCategory {
    fn calculate(&self, target: &AnalysisTarget) -> CategoryRisk {
        let coverage_gaps = self.identify_gaps(target);
        let branch_coverage = self.calculate_branch_coverage(target);
        let test_quality = self.assess_test_quality(target);
        
        CategoryRisk {
            score: self.calculate_score(coverage_gaps, branch_coverage, test_quality),
            severity: self.determine_severity(coverage_gaps),
            components: vec![
                RiskComponent::CoverageGap(coverage_gaps),
                RiskComponent::BranchCoverage(branch_coverage),
                RiskComponent::TestQuality(test_quality),
            ],
            hotspots: self.identify_hotspots(target),
            trend: self.calculate_trend(target),
        }
    }
    
    fn actions(&self) -> Vec<MitigationAction> {
        vec![
            MitigationAction::WriteUnitTests,
            MitigationAction::AddIntegrationTests,
            MitigationAction::ImproveTestQuality,
            MitigationAction::EnableMutationTesting,
        ]
    }
}
```

3. **Integration Risk Calculator**
```rust
pub struct IntegrationRiskCategory {
    interaction_analyzer: InteractionAnalyzer,
}

impl IntegrationRiskCategory {
    fn calculate(&self, target: &AnalysisTarget) -> CategoryRisk {
        let untested_paths = self.find_untested_interaction_paths(target);
        let api_coverage = self.calculate_api_boundary_coverage(target);
        let external_deps = self.analyze_external_dependencies(target);
        
        CategoryRisk {
            score: self.calculate_integration_score(
                untested_paths,
                api_coverage,
                external_deps,
            ),
            components: vec![
                RiskComponent::UntestedPaths(untested_paths),
                RiskComponent::ApiCoverage(api_coverage),
                RiskComponent::ExternalDeps(external_deps),
            ],
            hotspots: self.identify_integration_hotspots(target),
            recommendations: self.generate_integration_test_plan(target),
        }
    }
}
```

4. **Architectural Risk Analyzer**
```rust
pub struct ArchitecturalRiskCategory {
    coupling_analyzer: CouplingAnalyzer,
    dependency_analyzer: DependencyAnalyzer,
}

impl ArchitecturalRiskCategory {
    fn calculate(&self, target: &AnalysisTarget) -> CategoryRisk {
        let coupling_metrics = self.coupling_analyzer.analyze(target);
        let circular_deps = self.dependency_analyzer.find_cycles(target);
        let layering_violations = self.detect_layering_violations(target);
        let spof = self.identify_single_points_of_failure(target);
        
        CategoryRisk {
            score: self.calculate_architectural_score(
                coupling_metrics,
                circular_deps,
                layering_violations,
                spof,
            ),
            severity: self.determine_architectural_severity(target),
            components: vec![
                RiskComponent::HighCoupling(coupling_metrics),
                RiskComponent::CircularDeps(circular_deps),
                RiskComponent::LayeringViolations(layering_violations),
                RiskComponent::SinglePointsOfFailure(spof),
            ],
            refactoring_suggestions: self.generate_refactoring_plan(target),
        }
    }
}
```

5. **Risk Profile Aggregation**
```rust
pub struct RiskProfile {
    categories: HashMap<String, CategoryRisk>,
    aggregate_score: f64,
    risk_distribution: RiskDistribution,
    trend: RiskTrend,
}

impl RiskProfile {
    pub fn calculate_aggregate(&mut self, weights: &HashMap<String, f64>) {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        
        for (category, risk) in &self.categories {
            let weight = weights.get(category).unwrap_or(&1.0);
            weighted_sum += risk.score * weight;
            total_weight += weight;
        }
        
        self.aggregate_score = weighted_sum / total_weight;
        self.risk_distribution = self.calculate_distribution();
    }
    
    pub fn get_top_risks(&self, n: usize) -> Vec<PrioritizedRisk> {
        let mut risks: Vec<_> = self.categories.iter()
            .flat_map(|(cat, risk)| {
                risk.hotspots.iter().map(move |h| {
                    PrioritizedRisk {
                        category: cat.clone(),
                        hotspot: h.clone(),
                        score: h.risk_score,
                        actions: self.get_actions_for_category(cat),
                    }
                })
            })
            .collect();
        
        risks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        risks.truncate(n);
        risks
    }
}
```

### Architecture Changes

1. **Risk Module Restructuring**
   - Create `src/risk/categories/` module
   - Implement category trait system
   - Add profile aggregation
   - Create visualization support

2. **Data Flow Enhancement**
   - Parallel category calculation
   - Incremental risk updates
   - Category-specific caching
   - Historical tracking

### Data Structures

```rust
pub struct CategoryRisk {
    pub score: f64,
    pub severity: Severity,
    pub components: Vec<RiskComponent>,
    pub hotspots: Vec<RiskHotspot>,
    pub trend: RiskTrend,
    pub confidence: f64,
}

pub enum RiskComponent {
    CoverageGap(CoverageGap),
    BranchCoverage(f64),
    TestQuality(QualityScore),
    UntestedPaths(Vec<InteractionPath>),
    ApiCoverage(f64),
    ExternalDeps(Vec<Dependency>),
    HighCoupling(CouplingMetrics),
    CircularDeps(Vec<Cycle>),
    ComplexCode(ComplexityMetrics),
    Duplication(DuplicationMetrics),
}

pub struct RiskHotspot {
    pub location: Location,
    pub risk_score: f64,
    pub category: String,
    pub description: String,
    pub suggested_action: MitigationAction,
}

pub enum MitigationAction {
    WriteUnitTests,
    AddIntegrationTests,
    RefactorComplexity,
    BreakCircularDeps,
    ReduceCoupling,
    AddDocumentation,
    ExtractDuplication,
    ImproveTestQuality,
    AddSecurityTests,
    OptimizePerformance,
}

pub struct RiskDistribution {
    pub by_category: HashMap<String, f64>,
    pub by_severity: HashMap<Severity, usize>,
    pub by_module: HashMap<String, RiskProfile>,
}

pub struct RiskTrend {
    pub direction: TrendDirection,
    pub rate: f64,
    pub history: Vec<HistoricalRisk>,
}
```

### APIs and Interfaces

```rust
pub trait RiskCategoryProvider {
    fn categories(&self) -> Vec<Box<dyn RiskCategory>>;
    fn weights(&self) -> HashMap<String, f64>;
}

pub struct RiskCategoryConfig {
    pub enabled_categories: Vec<String>,
    pub custom_weights: HashMap<String, f64>,
    pub thresholds: HashMap<String, RiskThreshold>,
    pub aggregation_method: AggregationMethod,
}

pub enum AggregationMethod {
    WeightedAverage,
    Maximum,
    Multiplicative,
    Custom(Box<dyn Fn(&[f64]) -> f64>),
}
```

## Dependencies

- **Prerequisites**:
  - Spec 07 (Recalibrate Risk Formula) for base calculations
  - Spec 10 (Context-Aware Risk) for context integration
- **Affected Components**:
  - `src/risk/categories/` - New module
  - `src/risk/mod.rs` - Integration point
  - `src/io/writers/*` - Update for category output
  - `src/risk/insights.rs` - Category-specific insights
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test each category calculator
  - Validate aggregation methods
  - Test threshold detection
  - Verify action mapping

- **Integration Tests**:
  - Test with diverse codebases
  - Validate category accuracy
  - Test visualization output
  - Verify historical tracking

- **Validation Tests**:
  - Expert review of categorization
  - Comparison with security tools
  - Performance profiling
  - User acceptance testing

## Documentation Requirements

- **Code Documentation**:
  - Document each risk category
  - Explain scoring algorithms
  - Describe aggregation methods
  - Provide threshold rationale

- **User Documentation**:
  - Risk category guide
  - Interpretation handbook
  - Action mapping reference
  - Configuration examples

## Implementation Notes

1. **Phased Rollout**
   - Phase 1: Coverage and Integration risks
   - Phase 2: Architectural and Maintenance risks
   - Phase 3: Security and Performance risks
   - Phase 4: Custom categories support

2. **Visualization Support**
   - Risk radar charts
   - Category heat maps
   - Trend visualizations
   - Module risk profiles

3. **Extensibility Design**
   - Plugin architecture for categories
   - Custom category definition
   - External tool integration
   - API for risk consumers

## Migration and Compatibility

- **Breaking Changes**:
  - Risk output format changes
  - New configuration schema
  - API modifications

- **Migration Path**:
  1. Add categories alongside total
  2. Deprecation warnings
  3. Dual output period
  4. Full migration
  5. Legacy removal

- **Integration Points**:
  - CI/CD risk gates by category
  - IDE plugin support
  - Dashboard integration
  - Reporting tools
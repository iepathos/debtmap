---
number: 13
title: Complete Risk Categories Implementation
category: foundation
priority: high
status: in-progress
dependencies: [07, 10]
created: 2025-01-10
updated: 2025-01-16
---

# Specification 13: Complete Risk Categories Implementation

**Category**: foundation
**Priority**: high
**Status**: in-progress (60% complete)
**Dependencies**: [07 - Recalibrate Risk Formula, 10 - Add Context-Aware Risk]

## Context

Debtmap has a strong foundation for risk categorization with an evidence-based system already in place. The current implementation includes:

### Already Implemented
- **Evidence-based risk system** (`src/risk/evidence/`) with modular analyzers
- **Risk types**: Complexity, Coverage, Coupling, ChangeFrequency, Architecture
- **Statistical thresholds** with role-based and module-type baselines
- **Remediation actions** mapped to specific risk factors
- **Context-aware calculations** using function roles and visibility

### Current Limitations
- Risk categories exist but lack unified profile aggregation
- No configurable category weights
- Missing Security and Performance risk categories
- No historical tracking of risk trends
- Limited visualization beyond terminal output
- Integration risk analysis limited to coupling metrics

## Objective

Complete the risk categorization system by building on the existing evidence-based foundation to provide comprehensive risk profiles with configurable weights, new risk categories, and enhanced visualization.

## Requirements

### Functional Requirements

#### Enhance Existing Categories

1. **Coverage Risk Assessment** ✅ (Mostly Complete)
   - ✅ Direct test coverage gaps (`CoverageRiskAnalyzer`)
   - ✅ Test quality indicators (`TestQuality` enum)
   - ⚠️ Branch coverage analysis (partial)
   - ❌ Path coverage estimation
   - ❌ Mutation testing readiness

2. **Architectural Risk Evaluation** ✅ (Complete)
   - ✅ High coupling metrics (`CouplingRiskAnalyzer`)
   - ✅ Circular dependencies (in `CouplingEvidence`)
   - ✅ Layering violations (in `ArchitectureEvidence`)
   - ✅ God class indicators
   - ⚠️ Single points of failure (partial)

3. **Maintenance Risk Measurement** ⚠️ (Partial)
   - ✅ Code complexity metrics (`ComplexityRiskAnalyzer`)
   - ✅ Change frequency (`ChangeRiskAnalyzer`)
   - ❌ Documentation coverage
   - ❌ Code duplication levels
   - ❌ Knowledge concentration

#### Add New Categories

4. **Integration Risk Analysis** ⚠️ (Limited)
   - ✅ Cross-module dependencies (via coupling)
   - ❌ Untested interaction paths
   - ❌ API boundary coverage
   - ❌ External service interfaces
   - ❌ Event-driven interactions

5. **Security Risk Indicators** ❌ (Not Implemented)
   - Untested security paths
   - Input validation gaps
   - Authentication/authorization coverage
   - Sensitive data handling
   - Known vulnerability patterns

6. **Performance Risk Detection** ❌ (Not Implemented)
   - Untested performance paths
   - Algorithm complexity (O(n²) detection)
   - Resource usage patterns
   - Scalability bottlenecks
   - Cache effectiveness

### Non-Functional Requirements

1. **Granularity**: Support file, module, and system-level categories
2. **Composability**: Allow risk categories to be combined
3. **Extensibility**: Easy to add new risk categories  
4. **Visualization**: Support risk profile visualization
5. **Actionability**: Each category maps to clear actions
6. **Configuration**: User-configurable weights and thresholds

## Acceptance Criteria

### Already Met ✅
- [x] Coverage, Complexity, Coupling, Change risk categories calculated
- [x] Each category has clear thresholds (statistical baselines)
- [x] Categories map to specific remediation actions
- [x] Role-based and module-type aware calculations

### To Be Implemented
- [ ] Add Security and Performance risk categories
- [ ] Create unified RiskProfile aggregation structure
- [ ] Implement configurable category weights
- [ ] Add historical tracking of category changes
- [ ] Enhanced visualization (beyond terminal)
- [ ] Integration risk path analysis
- [ ] Export supports full category breakdown
- [ ] Documentation for new categories
- [ ] Unit tests for new categories
- [ ] Integration tests validate accuracy

## Technical Details

### Current Architecture

The existing evidence-based system provides a solid foundation:

```rust
// Current structure in src/risk/evidence/mod.rs
pub enum RiskType {
    Complexity { ... },
    Coverage { ... },
    Coupling { ... },
    ChangeFrequency { ... },
    Architecture { ... },
}

// Current calculator in src/risk/evidence_calculator.rs
pub struct EvidenceBasedRiskCalculator {
    complexity_analyzer: ComplexityRiskAnalyzer,
    coverage_analyzer: CoverageRiskAnalyzer,
    coupling_analyzer: CouplingRiskAnalyzer,
    change_analyzer: ChangeRiskAnalyzer,
}
```

### Proposed Enhancements

1. **Unified Risk Profile Structure**
```rust
// New: src/risk/profile.rs
pub struct RiskProfile {
    categories: HashMap<String, RiskFactor>,
    aggregate_score: f64,
    weights: HashMap<String, f64>,
    distribution: RiskDistribution,
    trend: Option<RiskTrend>,
}

impl RiskProfile {
    pub fn from_evidence(assessment: RiskAssessment, weights: &HashMap<String, f64>) -> Self {
        // Convert existing RiskAssessment to RiskProfile
        // Aggregate with configurable weights
    }
    
    pub fn calculate_aggregate(&mut self) {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        
        for (category, factor) in &self.categories {
            let weight = self.weights.get(category).unwrap_or(&1.0);
            weighted_sum += factor.score * weight;
            total_weight += weight;
        }
        
        self.aggregate_score = weighted_sum / total_weight;
    }
}

```

2. **New Security Risk Analyzer**
```rust
// New: src/risk/evidence/security_analyzer.rs
pub struct SecurityRiskAnalyzer {
    threshold_provider: StatisticalThresholdProvider,
}

impl SecurityRiskAnalyzer {
    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
        call_graph: &CallGraph,
    ) -> RiskFactor {
        let input_validation = self.check_input_validation(function);
        let auth_coverage = self.check_auth_coverage(function, call_graph);
        let sensitive_data = self.detect_sensitive_data_handling(function);
        
        RiskFactor {
            risk_type: RiskType::Security {
                input_validation_gaps: input_validation,
                auth_coverage,
                sensitive_data_exposed: sensitive_data,
                vulnerability_patterns: self.detect_patterns(function),
            },
            score: self.calculate_security_score(...),
            severity: self.determine_severity(...),
            evidence: RiskEvidence::Security(...),
            remediation_actions: vec![
                RemediationAction::AddInputValidation,
                RemediationAction::AddSecurityTests,
            ],
            weight: 1.2, // Higher weight for security
            confidence: 0.8,
        }
    }
}
```

3. **New Performance Risk Analyzer**
```rust  
// New: src/risk/evidence/performance_analyzer.rs
pub struct PerformanceRiskAnalyzer {
    complexity_detector: AlgorithmicComplexityDetector,
}

impl PerformanceRiskAnalyzer {
    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
    ) -> RiskFactor {
        let algo_complexity = self.complexity_detector.analyze(function);
        let resource_usage = self.analyze_resource_patterns(function);
        
        RiskFactor {
            risk_type: RiskType::Performance {
                algorithmic_complexity: algo_complexity,
                resource_patterns: resource_usage,
                scalability_score: self.calculate_scalability(...),
            },
            score: self.calculate_performance_score(...),
            severity: self.determine_severity(...),
            evidence: RiskEvidence::Performance(...),
            remediation_actions: vec![
                RemediationAction::OptimizeAlgorithm,
                RemediationAction::AddPerformanceTests,
            ],
            weight: 1.0,
            confidence: 0.7,
        }
    }
}
```

4. **Enhanced Integration Risk**
```rust
// Enhance: src/risk/evidence/integration_analyzer.rs
pub struct IntegrationRiskAnalyzer {
    call_graph: CallGraph,
    coverage_data: Option<LcovData>,
}

impl IntegrationRiskAnalyzer {
    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
    ) -> RiskFactor {
        // Analyze interaction paths using call graph
        let untested_paths = self.find_untested_interaction_paths(function);
        let api_boundaries = self.identify_api_boundaries(function);
        let external_deps = self.analyze_external_dependencies(function);
        
        RiskFactor {
            risk_type: RiskType::Integration {
                untested_interaction_paths: untested_paths.len(),
                api_boundary_coverage: self.calculate_api_coverage(&api_boundaries),
                external_dependencies: external_deps,
                event_handlers: self.find_event_handlers(function),
            },
            score: self.calculate_integration_score(...),
            severity: self.determine_severity(...),
            evidence: RiskEvidence::Integration(...),
            remediation_actions: vec![
                RemediationAction::AddIntegrationTests {
                    paths: untested_paths,
                    estimated_effort_hours: untested_paths.len() as u32 * 2,
                },
            ],
            weight: 1.1,
            confidence: 0.85,
        }
    }
}
```

### Architecture Changes

1. **Risk Module Restructuring**
   - ✅ Already has `src/risk/evidence/` with modular analyzers
   - ⚠️ Add `src/risk/profile.rs` for aggregation
   - ❌ Add `src/risk/evidence/security_analyzer.rs`
   - ❌ Add `src/risk/evidence/performance_analyzer.rs`
   - ❌ Add `src/risk/evidence/integration_analyzer.rs`
   - ❌ Add `src/risk/history/` for tracking

2. **Integration with Existing System**
   - Extend `EvidenceBasedRiskCalculator` with new analyzers
   - Create adapter from `RiskAssessment` to `RiskProfile`
   - Maintain backward compatibility with existing `RiskInsight`
   - Add configuration layer for weights

### Enhanced Data Structures

```rust
// Extend existing RiskType enum in src/risk/evidence/mod.rs
pub enum RiskType {
    // Existing types...
    Complexity { ... },
    Coverage { ... },
    Coupling { ... },
    ChangeFrequency { ... },
    Architecture { ... },
    
    // New types to add:
    Security {
        input_validation_gaps: u32,
        auth_coverage: f64,
        sensitive_data_exposed: bool,
        vulnerability_patterns: Vec<String>,
    },
    Performance {
        algorithmic_complexity: AlgorithmicComplexity,
        resource_patterns: Vec<ResourcePattern>,
        scalability_score: f64,
    },
    Integration {
        untested_interaction_paths: usize,
        api_boundary_coverage: f64,
        external_dependencies: Vec<String>,
        event_handlers: Vec<String>,
    },
}

// New structures for risk profiles
pub struct RiskProfile {
    pub categories: HashMap<String, RiskFactor>,
    pub aggregate_score: f64,
    pub weights: HashMap<String, f64>,
    pub distribution: RiskDistribution,
    pub trend: Option<RiskTrend>,
    pub timestamp: DateTime<Utc>,
}

pub struct RiskDistribution {
    pub by_category: HashMap<String, f64>,
    pub by_severity: HashMap<RiskSeverity, usize>,
    pub by_module: HashMap<PathBuf, ModuleRisk>,
}

pub struct RiskTrend {
    pub direction: TrendDirection,
    pub rate: f64,
    pub history: Vec<HistoricalRisk>,
}

pub struct HistoricalRisk {
    pub timestamp: DateTime<Utc>,
    pub profile: RiskProfile,
    pub commit_sha: Option<String>,
}
```

### APIs and Interfaces

```rust
// Configuration for risk categories
pub struct RiskCategoryConfig {
    pub enabled_categories: Vec<String>,
    pub custom_weights: HashMap<String, f64>,
    pub thresholds: HashMap<String, RiskThreshold>,
    pub aggregation_method: AggregationMethod,
}

pub enum AggregationMethod {
    WeightedAverage,    // Default
    Maximum,           // Most conservative
    Multiplicative,    // Compound risk
    Custom(Box<dyn Fn(&[f64]) -> f64>),
}

// Extend EvidenceBasedRiskCalculator
impl EvidenceBasedRiskCalculator {
    pub fn with_security_analyzer(mut self, analyzer: SecurityRiskAnalyzer) -> Self {
        self.security_analyzer = Some(analyzer);
        self
    }
    
    pub fn with_performance_analyzer(mut self, analyzer: PerformanceRiskAnalyzer) -> Self {
        self.performance_analyzer = Some(analyzer);
        self
    }
    
    pub fn calculate_risk_profile(
        &self,
        function: &FunctionAnalysis,
        call_graph: &CallGraph,
        coverage_data: Option<&LcovData>,
        config: &RiskCategoryConfig,
    ) -> RiskProfile {
        let assessment = self.calculate_risk(function, call_graph, coverage_data);
        RiskProfile::from_evidence(assessment, &config.custom_weights)
    }
}
```

## Dependencies

- **Prerequisites**: ✅ Already implemented
  - Spec 07 (Recalibrate Risk Formula) - Using statistical thresholds
  - Spec 10 (Context-Aware Risk) - Role and module-type awareness
  
- **Affected Components**:
  - `src/risk/evidence/` - Add new analyzers
  - `src/risk/profile.rs` - New aggregation module
  - `src/risk/evidence_calculator.rs` - Extend with new analyzers
  - `src/io/writers/*` - Update for profile output
  - `src/risk/insights.rs` - Profile-based insights
  
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - ✅ Existing analyzers already tested
  - Test new Security and Performance analyzers
  - Test RiskProfile aggregation
  - Validate weight configuration
  - Test historical tracking

- **Integration Tests**:
  - Test full risk profile generation
  - Validate with diverse codebases
  - Test backward compatibility
  - Verify configuration loading

## Documentation Requirements

- **Code Documentation**:
  - Document new risk categories (Security, Performance)
  - Explain profile aggregation
  - Document configuration options
  - Update existing analyzer docs

- **User Documentation**:
  - Risk profile interpretation guide
  - Configuration examples
  - Migration guide from RiskInsight to RiskProfile

## Implementation Plan

### Phase 1: Foundation (Week 1)
- [ ] Create `src/risk/profile.rs` with RiskProfile structure
- [ ] Add configuration support for category weights
- [ ] Create adapter from RiskAssessment to RiskProfile
- [ ] Maintain backward compatibility

### Phase 2: New Categories (Week 2)
- [ ] Implement SecurityRiskAnalyzer
- [ ] Implement PerformanceRiskAnalyzer
- [ ] Enhance IntegrationRiskAnalyzer with path analysis
- [ ] Add tests for new analyzers

### Phase 3: Integration (Week 3)
- [ ] Integrate new analyzers into EvidenceBasedRiskCalculator
- [ ] Update output writers for risk profiles
- [ ] Add configuration file support
- [ ] Update CLI to support category filtering

### Phase 4: Polish (Week 4)
- [ ] Add historical tracking support
- [ ] Enhance visualization output
- [ ] Documentation and examples
- [ ] Performance optimization

## Migration and Compatibility

- **Non-Breaking Approach**:
  - Keep existing RiskInsight and RiskAssessment
  - Add new RiskProfile alongside existing structures
  - Provide adapters between old and new formats
  - Gradual deprecation of old structures

- **Migration Path**:
  1. Add RiskProfile as opt-in feature (--risk-profile flag)
  2. Run both systems in parallel for validation
  3. Make RiskProfile default with fallback option
  4. Deprecate old system after stability period
  5. Remove legacy code in major version

- **Integration Points**:
  - Extend existing terminal writer for profiles
  - Add JSON export for risk profiles
  - Support category-based thresholds in CI/CD
  - Enable profile comparison across commits
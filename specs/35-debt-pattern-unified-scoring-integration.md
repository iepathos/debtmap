---
number: 35
title: Debt Pattern Unified Scoring Integration
category: foundation
priority: critical
status: draft
dependencies: [19, 21, 28, 29, 30, 31, 32, 33, 34]
created: 2025-08-17
---

# Specification 35: Debt Pattern Unified Scoring Integration

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [19 (Unified Prioritization), 21 (Dead Code), 28-34 (Pattern Detectors)]

## Context

Debtmap currently has comprehensive detection capabilities for 25+ technical debt patterns across security, performance, organization, testing, and resource management domains. However, these detected patterns are not integrated into the unified scoring system that determines fix priority. 

Currently, detected issues are reported as separate `DebtItem` entities while function prioritization uses only basic metrics (complexity, coverage, ROI) and crude name-based heuristics for security/organization scoring. This creates a disconnect where critical security vulnerabilities or performance issues may not influence the priority ranking of functions that need attention.

The system needs an aggregation layer that collects all detected issues per function and incorporates them into the unified scoring calculation, ensuring that actual detected problems (not just patterns) drive prioritization decisions.

## Objective

Create a comprehensive integration between the pattern detection system and unified scoring system, ensuring that all detected technical debt issues directly influence function priority scores through a configurable, weighted aggregation mechanism.

## Requirements

### Functional Requirements

1. **Debt Aggregation System**
   - Create a `FunctionDebtProfile` structure that aggregates all detected issues per function
   - Map each `DebtItem` to its corresponding function location
   - Categorize issues by debt domain (security, performance, organization, testing, resource)
   - Support efficient lookup of debt profile by function identifier

2. **Enhanced Score Calculation**
   - Update `calculate_security_factor` to use actual detected security issues
   - Update `calculate_organization_factor` to use actual detected organization issues
   - Add `calculate_performance_factor` based on detected performance issues
   - Add `calculate_testing_factor` based on detected testing issues
   - Add `calculate_resource_factor` based on detected resource issues
   - Combine pattern-based heuristics with actual detection results

3. **Debt Type Mapping**
   - Create comprehensive mapping from `core::DebtType` enum values to score categories
   - Define severity weights for each debt type
   - Support configurable severity multipliers in `.debtmap.toml`

4. **Score Integration Pipeline**
   - Modify `create_unified_debt_item` functions to accept debt profiles
   - Update `calculate_unified_priority` to include all debt factors
   - Ensure backward compatibility with existing scoring weights

5. **Aggregation Performance**
   - Implement efficient indexing for debt items by file and line range
   - Support incremental updates when new detectors are added
   - Cache aggregated profiles during analysis

### Non-Functional Requirements

1. **Performance**: Aggregation should add <10% overhead to analysis time
2. **Memory**: Debt profiles should use <100MB additional memory for 10K functions
3. **Configurability**: All debt type weights must be configurable
4. **Extensibility**: New debt detectors should automatically integrate
5. **Transparency**: Score breakdown must show contribution from detected issues

## Acceptance Criteria

- [ ] All detected `DebtItem` issues are aggregated by function location
- [ ] Security score reflects actual detected security vulnerabilities (not just name patterns)
- [ ] Organization score reflects actual detected organization issues
- [ ] Performance score is calculated from detected performance anti-patterns
- [ ] Testing score is calculated from detected testing quality issues
- [ ] Resource score is calculated from detected resource management issues
- [ ] Score breakdown displays detected issue counts and their contribution
- [ ] Configuration file supports weights for each debt category and type
- [ ] Existing tests pass with enhanced scoring
- [ ] New integration tests verify debt aggregation and scoring
- [ ] Documentation updated with debt integration architecture
- [ ] Performance impact is less than 10% on large codebases

## Technical Details

### Implementation Approach

1. **Phase 1: Debt Aggregation Infrastructure**
   ```rust
   pub struct FunctionDebtProfile {
       pub function_id: FunctionId,
       pub security_issues: Vec<DebtItem>,
       pub performance_issues: Vec<DebtItem>,
       pub organization_issues: Vec<DebtItem>,
       pub testing_issues: Vec<DebtItem>,
       pub resource_issues: Vec<DebtItem>,
       pub duplication_issues: Vec<DebtItem>,
   }
   
   pub struct DebtAggregator {
       profiles: HashMap<FunctionId, FunctionDebtProfile>,
       debt_index: HashMap<PathBuf, Vec<DebtItem>>,
   }
   ```

2. **Phase 2: Enhanced Score Calculation**
   ```rust
   fn calculate_security_factor(
       func: &FunctionMetrics,
       debt_profile: Option<&FunctionDebtProfile>
   ) -> f64 {
       let pattern_score = calculate_pattern_based_score(func);
       let detected_score = debt_profile
           .map(|p| calculate_detected_security_score(&p.security_issues))
           .unwrap_or(0.0);
       (pattern_score + detected_score).min(10.0)
   }
   ```

3. **Phase 3: Debt Type Mapping**
   ```rust
   impl DebtCategory {
       pub fn from_debt_type(debt_type: &core::DebtType) -> Self {
           match debt_type {
               DebtType::Security => DebtCategory::Security,
               DebtType::ErrorSwallowing => DebtCategory::Resource,
               DebtType::CodeOrganization => DebtCategory::Organization,
               DebtType::Performance => DebtCategory::Performance,
               // ... comprehensive mapping
           }
       }
       
       pub fn severity_weight(&self) -> f64 {
           match self {
               DebtCategory::Security => 3.0,  // High weight
               DebtCategory::Performance => 2.0,
               DebtCategory::Organization => 1.5,
               DebtCategory::Testing => 1.0,
               DebtCategory::Resource => 2.5,
           }
       }
   }
   ```

### Architecture Changes

1. **New Module**: `src/priority/debt_aggregator.rs`
   - Responsible for collecting and indexing debt items
   - Provides efficient lookup by function location
   - Handles debt categorization and severity calculation

2. **Modified Modules**:
   - `src/priority/unified_scorer.rs`: Enhanced score calculation functions
   - `src/main.rs`: Integration of aggregator in analysis pipeline
   - `src/config.rs`: New configuration for debt type weights

3. **Data Flow**:
   ```
   Detectors → DebtItems → DebtAggregator → FunctionDebtProfiles
                                          ↓
   FunctionMetrics → UnifiedScorer ← ← ← ←
                            ↓
                      UnifiedDebtItem (with integrated scores)
   ```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub enum DebtCategory {
    Security,
    Performance,
    Organization,
    Testing,
    Resource,
}

#[derive(Debug, Clone)]
pub struct DebtSeverity {
    pub category: DebtCategory,
    pub weight: f64,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtWeights {
    pub security_issues: HashMap<String, f64>,
    pub performance_issues: HashMap<String, f64>,
    pub organization_issues: HashMap<String, f64>,
    pub testing_issues: HashMap<String, f64>,
    pub resource_issues: HashMap<String, f64>,
}
```

### APIs and Interfaces

```rust
pub trait DebtAggregation {
    fn aggregate_debt(&mut self, items: Vec<DebtItem>);
    fn get_profile(&self, func_id: &FunctionId) -> Option<&FunctionDebtProfile>;
    fn calculate_debt_scores(&self, func_id: &FunctionId) -> DebtScores;
}

pub struct DebtScores {
    pub security: f64,
    pub performance: f64,
    pub organization: f64,
    pub testing: f64,
    pub resource: f64,
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 19: Unified Debt Prioritization (base scoring system)
  - Specs 28-34: All pattern detectors must be implemented
  
- **Affected Components**:
  - `priority::unified_scorer`: Major modifications for debt integration
  - `main::create_unified_analysis`: Integration point for aggregator
  - All detector modules: Ensure proper DebtItem creation
  
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test debt aggregation by function location
  - Test score calculation with various debt profiles
  - Test debt type to category mapping
  - Test configuration parsing and validation

- **Integration Tests**:
  - Create sample code with known security issues, verify scoring
  - Create sample code with performance issues, verify scoring
  - Test that detected issues influence priority ranking
  - Test score breakdown shows detected issue contributions

- **Performance Tests**:
  - Benchmark aggregation overhead on 10K+ functions
  - Memory usage profiling with large debt profiles
  - Cache effectiveness measurements

- **User Acceptance**:
  - Run on real codebases, verify priority changes are sensible
  - Ensure security issues get appropriately high priority
  - Validate that score breakdowns are informative

## Documentation Requirements

- **Code Documentation**:
  - Document aggregation algorithm and indexing strategy
  - Document score calculation formulas with examples
  - Document configuration options for debt weights

- **User Documentation**:
  - Update README with debt integration explanation
  - Add configuration examples for debt type weights
  - Document how detected issues influence scoring

- **Architecture Updates**:
  - Update ARCHITECTURE.md with debt aggregation layer
  - Document data flow from detectors to scores
  - Add sequence diagrams for aggregation pipeline

## Implementation Notes

1. **Backward Compatibility**: Ensure existing scoring continues to work if no debt items are detected
2. **Incremental Rollout**: Can enable debt integration via feature flag initially
3. **Performance Optimization**: Use lazy evaluation for debt aggregation
4. **Debugging Support**: Add --debug-debt flag to show aggregation details
5. **Validation**: Ensure debt items have valid location information

## Migration and Compatibility

- **Configuration Migration**: 
  - Existing `.debtmap.toml` files remain valid
  - New debt weight sections are optional with defaults
  
- **Score Changes**:
  - Functions with detected issues will see score increases
  - Priority rankings may change significantly
  - Provide migration guide explaining score changes
  
- **API Compatibility**:
  - All existing CLI commands continue to work
  - Output format changes are additive only
  - JSON output includes new debt profile fields
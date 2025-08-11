---
number: 16
title: Refactoring Detection for Orchestration and I/O Functions
category: optimization
priority: high
status: draft
dependencies: [14, 15]
created: 2025-08-11
---

# Specification 16: Refactoring Detection for Orchestration and I/O Functions

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [14, 15]

## Context

Currently, debtmap identifies untested functions and recommends adding tests based on coverage data and complexity metrics. However, it fails to recognize a common architectural pattern: orchestration and I/O functions that mix concerns. These functions often appear in `main.rs` or CLI entry points and contain both I/O operations (printing, file reading) and business logic (formatting, parsing, decision-making).

The current approach leads to misleading recommendations. For example, debtmap might flag a `print_risk_function()` as needing tests with high ROI, when the real issue is that the formatting logic should be extracted into a pure, testable function. This creates noise in the analysis and directs developers toward unproductive work (trying to test I/O directly) rather than productive refactoring.

This specification addresses the need to detect these mixed-concern patterns and recommend refactoring to extract pure logic, following functional programming principles of maintaining a "functional core with an imperative shell."

## Objective

Enhance debtmap to detect orchestration and I/O functions that mix concerns, and recommend extracting pure business logic into testable functions rather than attempting to test I/O operations directly. This will provide more actionable recommendations that improve both testability and architecture.

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect functions containing I/O operations (println!, file operations, stdin/stdout)
   - Identify embedded business logic within I/O functions (formatting, parsing, calculations)
   - Recognize thin delegation patterns (functions that only call other functions)
   - Detect orchestration patterns (functions that coordinate multiple operations)

2. **Refactoring Recommendations**
   - Generate specific refactoring suggestions for mixed-concern functions
   - Recommend extraction of pure functions from I/O wrappers
   - Suggest appropriate module organization for extracted logic
   - Provide clear naming suggestions for extracted functions

3. **Risk Scoring Adjustments**
   - Adjust ROI calculations to deprioritize thin I/O wrappers
   - Increase priority for functions with extractable business logic
   - Properly attribute coverage through delegation chains
   - Distinguish between "needs tests" and "needs refactoring"

4. **Output Enhancements**
   - Add new "Refactoring Opportunities" section to output
   - Clearly indicate when refactoring is preferred over testing
   - Show estimated effort for refactoring vs testing
   - Provide actionable steps for each refactoring opportunity

### Non-Functional Requirements

1. **Performance**
   - Pattern detection should not significantly impact analysis time
   - AST traversal should remain efficient with caching where appropriate

2. **Accuracy**
   - Minimize false positives in pattern detection
   - Confidence scoring for refactoring recommendations

3. **Compatibility**
   - Maintain backward compatibility with existing output formats
   - Allow opting out of refactoring detection via configuration

## Acceptance Criteria

- [ ] Functions with I/O operations and embedded logic are correctly identified
- [ ] Thin delegation functions are recognized and deprioritized for testing
- [ ] Refactoring recommendations include specific extraction suggestions
- [ ] ROI calculations properly account for refactoring vs testing effort
- [ ] Cross-module coverage attribution works for delegation patterns
- [ ] Output clearly distinguishes between testing and refactoring needs
- [ ] Functions like `print_risk_function()` recommend extracting formatting logic
- [ ] Functions like `parse_languages()` suggest moving to dedicated modules
- [ ] Pure delegation functions are marked as "coverage inherited" when applicable
- [ ] Integration tests demonstrate correct pattern detection across languages

## Technical Details

### Implementation Approach

1. **Create new module `src/risk/refactoring.rs`**
   - Define refactoring opportunity types and patterns
   - Implement pattern detection algorithms
   - Create confidence scoring system

2. **Enhance AST analysis**
   - Add I/O operation detection to AST visitors
   - Track function call chains for delegation detection
   - Identify pure vs impure code sections

3. **Implement delegation graph**
   - Build graph of function delegations
   - Track cross-module dependencies
   - Enable coverage attribution through graph

### Architecture Changes

1. **New Components**
   ```rust
   // src/risk/refactoring.rs
   pub struct RefactoringOpportunity {
       pub function: FunctionMetrics,
       pub pattern: MixedConcernPattern,
       pub suggested_refactoring: String,
       pub extracted_function_name: Option<String>,
       pub estimated_roi: f64,
       pub confidence: f64,
   }

   pub enum MixedConcernPattern {
       IOWithLogic,           // I/O operations with embedded business logic
       ParsingInOrchestration, // Complex parsing in main/CLI modules
       DecisionWithExecution,  // Decision logic mixed with execution
       DataTransformInIO,      // Data transformation within I/O operations
       ThinDelegation,        // Simple forwarding to another function
   }

   pub struct DelegationGraph {
       edges: Vec<DelegationEdge>,
       coverage_attribution: HashMap<FunctionId, f64>,
   }
   ```

2. **Modified Risk Analysis**
   - Integrate refactoring detection into risk calculation pipeline
   - Adjust ROI formulas based on detected patterns
   - Add delegation-aware coverage attribution

### Data Structures

```rust
// Additional fields for FunctionRisk
pub struct FunctionRisk {
    // ... existing fields ...
    pub refactoring_opportunity: Option<RefactoringOpportunity>,
    pub delegated_coverage: Option<f64>,
    pub is_orchestration_function: bool,
}

// New debt category
pub enum DebtCategory {
    TestingDebt,
    RefactoringDebt,  // New category
    ComplexityDebt,
    DuplicationDebt,
}
```

### APIs and Interfaces

1. **CLI Additions**
   ```bash
   --detect-refactoring    Enable refactoring opportunity detection
   --no-delegation-credit  Disable coverage attribution through delegation
   ```

2. **Configuration**
   ```toml
   [refactoring]
   enabled = true
   min_confidence = 0.7
   ignore_patterns = ["tests/*", "benches/*"]
   ```

## Dependencies

- **Prerequisites**: 
  - Spec 14 (Dependency-aware ROI calculation) - for understanding module relationships
  - Spec 15 (Automated tech debt prioritization) - for integration with prioritization system

- **Affected Components**:
  - `src/risk/mod.rs` - Integration of refactoring detection
  - `src/analyzers/` - Enhanced AST analysis for I/O detection
  - `src/core/mod.rs` - Additional fields for refactoring opportunities
  - `src/cli.rs` - New command-line options

- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Pattern detection for each `MixedConcernPattern` type
  - Delegation graph construction and traversal
  - Coverage attribution calculations
  - Confidence scoring algorithms

- **Integration Tests**:
  - End-to-end testing with sample codebases containing mixed concerns
  - Verify correct identification of main.rs orchestration functions
  - Test cross-module delegation detection
  - Validate ROI adjustments for refactoring opportunities

- **Performance Tests**:
  - Benchmark pattern detection on large codebases
  - Ensure minimal performance impact (<5% increase in analysis time)

- **User Acceptance**:
  - Output should clearly guide users toward refactoring over testing where appropriate
  - Recommendations should be actionable and specific

## Documentation Requirements

- **Code Documentation**:
  - Document pattern detection algorithms
  - Explain confidence scoring methodology
  - Provide examples of each pattern type

- **User Documentation**:
  - Add section on refactoring detection to README
  - Explain difference between testing and refactoring debt
  - Provide examples of common patterns and recommended refactorings

- **Architecture Updates**:
  - Update ARCHITECTURE.md with refactoring module description
  - Document delegation graph data structure
  - Explain coverage attribution mechanism

## Implementation Notes

1. **Pattern Detection Priority**:
   - Start with simple I/O + logic detection
   - Add delegation detection as second phase
   - Implement confidence scoring iteratively

2. **False Positive Mitigation**:
   - Use confidence thresholds to filter recommendations
   - Allow pattern-specific suppressions
   - Provide user feedback mechanism for tuning

3. **Integration Considerations**:
   - Ensure refactoring detection integrates smoothly with existing risk analysis
   - Maintain performance with incremental analysis where possible
   - Consider caching delegation graphs for large projects

4. **Example Transformations**:
   ```rust
   // Before: Mixed concern
   fn print_risk_function(func: &RiskFunction) {
       let coverage = func.coverage.map(|c| format!("{:.0}%", c * 100.0))
           .unwrap_or("0%".to_string());
       println!("Risk: {}, Coverage: {}", func.risk, coverage);
   }

   // After: Separated concerns
   fn format_risk_summary(func: &RiskFunction) -> String {
       let coverage = func.coverage.map(|c| format!("{:.0}%", c * 100.0))
           .unwrap_or("0%".to_string());
       format!("Risk: {}, Coverage: {}", func.risk, coverage)
   }

   fn print_risk_function(func: &RiskFunction) {
       println!("{}", format_risk_summary(func));
   }
   ```

## Migration and Compatibility

- **Backward Compatibility**:
  - Refactoring detection is opt-in by default
  - Existing output formats remain unchanged when feature is disabled
  - ROI calculations maintain same scale and meaning

- **Migration Path**:
  - Users can enable refactoring detection gradually
  - Existing suppression comments continue to work
  - No breaking changes to CLI interface

- **Configuration Migration**:
  - New configuration options have sensible defaults
  - Existing configs continue to work without modification
  - Documentation provided for new options
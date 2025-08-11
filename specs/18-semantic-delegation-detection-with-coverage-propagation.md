---
number: 18
title: Semantic Delegation Detection with Coverage Propagation
category: optimization
priority: high
status: draft
dependencies: [05, 08]
created: 2025-01-11
---

# Specification 18: Semantic Delegation Detection with Coverage Propagation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [05 - Risk Analysis, 08 - Testing Prioritization]

## Context

Debtmap currently prioritizes functions for testing based on complexity metrics and coverage data. However, it treats all uncovered functions equally, leading to false positives where orchestration functions that merely delegate to well-tested code are marked as high priority. This violates functional programming principles where the "functional core, imperative shell" pattern suggests that thin orchestration layers don't always need direct unit tests.

The current issue manifests in functions like `generate_report_if_requested` which:
- Has cyclomatic complexity of 1 (no branching)
- Has cognitive complexity of 2 (simple chaining)
- Is marked as a critical entry point
- Gets ROI score of 8.8 despite being pure orchestration
- Simply delegates to `determine_output_format` (tested) and `output_results_with_risk`

This leads to misleading recommendations where trivial orchestration code appears more important to test than actual business logic.

## Objective

Implement semantic analysis to distinguish between different types of functions (pure logic, orchestration, I/O wrappers) and adjust testing prioritization accordingly. Functions that merely orchestrate well-tested code should receive lower priority than untested business logic, even if they are entry points.

## Requirements

### Functional Requirements

1. **Call Graph Analysis**
   - Track which functions call which other functions
   - Build a directed graph of function dependencies
   - Identify function call patterns (direct call, method chaining, error propagation)
   - Extract call information from AST during parsing

2. **Function Role Classification**
   - Classify functions into roles based on their characteristics:
     - `PureLogic`: Business logic without side effects (high test priority)
     - `Orchestrator`: Coordinates other functions with minimal logic (low priority if delegates are tested)
     - `IOWrapper`: Thin wrapper around I/O operations (minimal priority)
     - `EntryPoint`: Main entry points requiring integration tests (medium priority)
     - `Unknown`: Cannot determine role (use current algorithm)

3. **Coverage Propagation**
   - Calculate "transitive coverage" for orchestration functions
   - If a function only calls tested functions (>80% coverage), consider it transitively covered
   - Track coverage inheritance through the call graph
   - Distinguish between direct, transitive, partial, and no coverage

4. **Pattern Detection**
   - Detect common delegation patterns:
     - Simple delegation: `return other_function(args)`
     - Pipeline composition: `a().map(b).and_then(c)`
     - Error propagation: `do_something()?`
     - Method chaining: `.map().unwrap_or()`
   - Identify functions that add no business logic beyond coordination

5. **Enhanced ROI Calculation**
   - Adjust ROI scores based on function role:
     - Pure logic with no coverage: HIGH multiplier (1.5x)
     - Orchestration calling tested functions: LOW multiplier (0.2x)
     - I/O wrappers: MINIMAL multiplier (0.1x)
     - Entry points: MEDIUM multiplier (0.5x) for integration test priority
   - Don't apply minimum weight enforcement to orchestration functions

6. **Improved Reporting**
   - Show delegation relationships in output:
     - "Delegates to N tested functions" for orchestrators
     - "Transitive coverage: X%" for functions calling tested code
     - "Role: Orchestrator/PureLogic/IOWrapper" classification
   - Separate recommendations for unit vs integration testing

### Non-Functional Requirements

1. **Performance**
   - Call graph construction should add <10% to analysis time
   - Use lazy evaluation for transitive coverage calculation
   - Cache call graph between runs when files haven't changed

2. **Accuracy**
   - Correctly identify at least 90% of obvious orchestration patterns
   - Avoid false positives where complex functions are misclassified
   - Handle edge cases like recursive calls and cycles

3. **Compatibility**
   - Maintain backward compatibility with existing CLI interface
   - Work with or without LCOV coverage data
   - Support all currently supported languages (Rust, Python, JavaScript, TypeScript)

## Acceptance Criteria

- [ ] Call graph is built from AST during analysis
- [ ] Functions are classified into roles with >90% accuracy on test cases
- [ ] Orchestration functions calling tested code get reduced ROI scores
- [ ] `generate_report_if_requested` gets ROI < 2.0 (down from 8.8)
- [ ] Pure business logic functions maintain high priority
- [ ] Transitive coverage is calculated and displayed in reports
- [ ] Pattern detection identifies common delegation patterns
- [ ] Performance impact is less than 10% on large codebases
- [ ] All existing tests pass with the new implementation
- [ ] New tests validate delegation detection and coverage propagation

## Technical Details

### Implementation Approach

1. **Phase 1: Call Graph Infrastructure**
   - Add `CallGraph` struct to track function relationships
   - Extend AST parsing to extract function calls
   - Build graph during analysis phase
   - Store in analysis context for use by risk calculator

2. **Phase 2: Pattern Detection**
   - Implement pattern matchers for common delegation patterns
   - Add heuristics for identifying pure functions vs side effects
   - Use AST node types to classify function behavior
   - Consider both syntax and semantics

3. **Phase 3: Role Classification**
   - Implement classification algorithm using:
     - Complexity metrics (cyclomatic, cognitive)
     - Call graph information
     - Pattern detection results
     - Function name/path heuristics
   - Use decision tree or rule-based approach

4. **Phase 4: Coverage Propagation**
   - Implement transitive coverage calculation
   - Use graph traversal to propagate coverage
   - Handle cycles and recursive calls
   - Cache results for performance

5. **Phase 5: ROI Adjustment**
   - Modify ROI calculator to consider function roles
   - Implement role-based multipliers
   - Remove minimum weight enforcement for orchestrators
   - Adjust effort estimates based on role

### Architecture Changes

```rust
// New modules
mod call_graph {
    pub struct CallGraph {
        edges: HashMap<FunctionId, Vec<FunctionCall>>,
        nodes: HashMap<FunctionId, FunctionInfo>,
    }
    
    pub struct FunctionCall {
        target: FunctionId,
        call_type: CallType,
        line: usize,
    }
    
    pub enum CallType {
        Direct,
        MethodChain,
        ErrorPropagation,
        Async,
    }
}

mod function_classifier {
    pub enum FunctionRole {
        PureLogic,
        Orchestrator,
        IOWrapper,
        EntryPoint,
        Unknown,
    }
    
    pub struct Classifier {
        call_graph: CallGraph,
        coverage_data: Option<CoverageData>,
    }
}

mod coverage_propagation {
    pub struct TransitiveCoverage {
        direct: f64,
        transitive: f64,
        covered_callees: Vec<FunctionId>,
    }
}
```

### Data Structures

```rust
// Extend existing structures
pub struct FunctionMetrics {
    // ... existing fields ...
    pub role: FunctionRole,
    pub calls: Vec<FunctionCall>,
    pub called_by: Vec<FunctionId>,
    pub transitive_coverage: Option<TransitiveCoverage>,
}

pub struct TestTarget {
    // ... existing fields ...
    pub function_role: FunctionRole,
    pub delegates_to_tested: bool,
    pub transitive_coverage: f64,
}
```

### APIs and Interfaces

```rust
// New public APIs
pub trait CallGraphBuilder {
    fn build_from_ast(&mut self, ast: &AST) -> CallGraph;
    fn add_function_call(&mut self, from: FunctionId, to: FunctionId, call_type: CallType);
}

pub trait FunctionClassifier {
    fn classify(&self, func: &FunctionMetrics, graph: &CallGraph) -> FunctionRole;
    fn is_orchestration_pattern(&self, func: &FunctionMetrics) -> bool;
}

pub trait CoveragePropagator {
    fn calculate_transitive_coverage(&self, func: &FunctionMetrics, graph: &CallGraph) -> TransitiveCoverage;
    fn is_transitively_covered(&self, func: &FunctionMetrics, threshold: f64) -> bool;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 05: Risk Analysis (provides coverage integration)
  - Spec 08: Testing Prioritization (provides ROI calculation)
- **Affected Components**:
  - `risk::roi::mod.rs` - ROI calculation adjustments
  - `analyzers/` - AST parsing extensions for call extraction
  - `risk::priority` - Classification integration
  - `io::output` - Report formatting changes
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test pattern detection for various delegation patterns
  - Validate function role classification accuracy
  - Test coverage propagation with mock call graphs
  - Verify ROI adjustments for different roles

- **Integration Tests**:
  - Test with real codebases containing orchestration patterns
  - Verify `generate_report_if_requested` gets correct classification
  - Test with and without LCOV coverage data
  - Validate performance impact on large projects

- **Performance Tests**:
  - Benchmark call graph construction time
  - Measure memory usage with large call graphs
  - Test cache effectiveness

- **User Acceptance**:
  - Verify reduced false positives in recommendations
  - Confirm orchestration functions show as "transitively covered"
  - Validate improved recommendation quality

## Documentation Requirements

- **Code Documentation**:
  - Document classification heuristics and rationale
  - Explain coverage propagation algorithm
  - Add examples of detected patterns

- **User Documentation**:
  - Update README with explanation of function roles
  - Document transitive coverage concept
  - Add examples of improved recommendations

- **Architecture Updates**:
  - Update ARCHITECTURE.md with call graph component
  - Document data flow for coverage propagation
  - Add decision tree for role classification

## Implementation Notes

1. Start with Rust language support, then extend to others
2. Use visitor pattern for AST traversal to extract calls
3. Consider using petgraph crate for call graph representation
4. Implement incremental call graph updates for performance
5. Add feature flag to disable semantic analysis if needed
6. Consider machine learning for role classification in future

## Migration and Compatibility

- No breaking changes to CLI interface
- Existing reports will show additional information
- ROI scores will change (generally lower for orchestration)
- Can be disabled with `--no-semantic-analysis` flag if issues arise
- Cache format will change, requiring cache invalidation
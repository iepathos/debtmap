---
number: 71
title: Python Function Purity Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-01
---

# Specification 71: Python Function Purity Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Rust analyzer includes purity detection to identify pure functions (functions without side effects) with confidence scores. This is valuable for understanding code maintainability, testability, and potential for optimization. The Python analyzer completely lacks this capability, marked as TODO in the codebase.

Pure functions are easier to:
- Test in isolation
- Reason about and maintain
- Parallelize and optimize
- Refactor and compose

Python's dynamic nature makes purity detection challenging but valuable for code quality assessment.

## Objective

Implement comprehensive purity detection for Python functions that identifies side effects and provides confidence scores for function purity, matching the capabilities available in the Rust analyzer.

## Requirements

### Functional Requirements
- Detect side effects in Python functions (I/O, mutations, global access)
- Calculate purity confidence scores (0.0 to 1.0)
- Handle Python-specific constructs (generators, comprehensions, decorators)
- Track method calls and attribute access for side effect detection
- Support async functions and coroutines

### Non-Functional Requirements
- Accurate detection with < 10% false positive rate
- Efficient analysis without deep call graph traversal
- Configurable side effect patterns
- Extensible for custom purity rules

## Acceptance Criteria

- [ ] PythonPurityDetector class implemented
- [ ] Side effect detection for common Python patterns
- [ ] Confidence scoring algorithm implemented
- [ ] Integration with Python FunctionMetrics
- [ ] Support for class methods and static methods
- [ ] Handling of decorators and wrapped functions
- [ ] Unit tests covering all detection patterns
- [ ] Documentation of purity detection rules

## Technical Details

### Implementation Approach
1. Create `analyzers::python_purity` module
2. Implement AST visitor for side effect detection
3. Build confidence scoring based on detected patterns
4. Integrate with Python function analysis pipeline

### Architecture Changes
- New module: `src/analyzers/python_purity.rs`
- Extend FunctionMetrics with purity fields
- Add purity analysis to extract_function_metrics

### Data Structures
```rust
pub struct PythonPurityDetector {
    known_pure_functions: HashSet<String>,
    known_impure_functions: HashSet<String>,
    side_effect_patterns: Vec<SideEffectPattern>,
}

pub struct PurityAnalysis {
    pub is_pure: bool,
    pub confidence: f32,
    pub side_effects: Vec<SideEffect>,
}

pub enum SideEffect {
    GlobalWrite(String),
    AttributeMutation,
    IOOperation,
    ExternalCall(String),
    ExceptionRaising,
}
```

### APIs and Interfaces
- `PythonPurityDetector::analyze_function(func_def: &ast::StmtFunctionDef) -> PurityAnalysis`
- Integration with existing Python analyzer

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/analyzers/python.rs`
  - `src/core/metrics.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

- **Unit Tests**: Individual side effect patterns
- **Integration Tests**: Complete function analysis
- **Accuracy Tests**: Known pure/impure function sets
- **Edge Cases**: Decorators, nested functions, lambdas

## Documentation Requirements

- **Code Documentation**: Purity rules and detection patterns
- **User Documentation**: Interpretation of purity scores
- **Examples**: Common pure/impure patterns in Python

## Implementation Notes

- Consider Python built-ins (print, open, etc.) as impure
- Handle method calls based on receiver type
- Account for list/dict comprehension side effects
- Special handling for property decorators
- Consider generator functions separately

## Migration and Compatibility

During prototype phase: New feature addition with no breaking changes. Purity information will be optional in output and backward compatible.
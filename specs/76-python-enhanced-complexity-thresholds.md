---
number: 76
title: Python Enhanced Complexity Thresholds
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-09-01
---

# Specification 76: Python Enhanced Complexity Thresholds

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The Rust analyzer uses sophisticated ComplexityThresholds with role-based differentiation (Test, EntryPoint, CoreLogic), preset configurations (Strict, Balanced, Relaxed), and enhanced detection including RecursiveMatchDetector and IfElseChainAnalyzer. The Python analyzer only uses a simple flat threshold (default 10) without any role differentiation or advanced pattern detection. This results in less nuanced complexity assessment for Python code.

Role-based thresholds recognize that:
- Test functions may have different acceptable complexity
- Entry points often have orchestration complexity
- Core logic should maintain lower complexity
- Different code patterns warrant different thresholds

## Objective

Implement enhanced complexity threshold management for Python with role-based differentiation, configurable presets, and advanced pattern detection to provide more accurate and actionable complexity assessment.

## Requirements

### Functional Requirements
- Implement role-based complexity thresholds
- Support threshold presets (Strict, Balanced, Relaxed)
- Detect function roles (Test, EntryPoint, CoreLogic, Utility)
- Implement recursive pattern detection for Python
- Add if/elif chain analysis
- Support comprehension complexity analysis
- Provide enhanced complexity messages
- Enable runtime threshold configuration

### Non-Functional Requirements
- Compatible with existing complexity metrics
- Minimal performance overhead
- Configurable through existing config system
- Clear and actionable messages

## Acceptance Criteria

- [ ] ComplexityThresholds implementation for Python
- [ ] Role detection algorithm for Python functions
- [ ] Preset configurations available
- [ ] Recursive pattern detector for Python
- [ ] If/elif chain analyzer implementation
- [ ] Comprehension complexity analysis
- [ ] Enhanced message generation
- [ ] Runtime configuration support
- [ ] Integration with Python analyzer
- [ ] Unit tests for role detection
- [ ] Threshold validation tests

## Technical Details

### Implementation Approach
1. Port ComplexityThresholds to Python context
2. Implement Python-specific role detection
3. Create pattern detectors for Python AST
4. Build enhanced message generator
5. Integrate with existing complexity calculation

### Architecture Changes
- New module: `src/complexity/python_thresholds.rs`
- Enhanced complexity analysis for Python
- Role-based threshold application

### Data Structures
```rust
pub struct PythonComplexityThresholds {
    pub base: ComplexityThresholds,
    pub comprehension_factor: f32,
    pub decorator_impact: f32,
}

pub enum PythonFunctionRole {
    Test,
    EntryPoint,
    CoreLogic,
    Utility,
    DataClass,
    Handler,
}

pub struct PythonPatternDetector {
    pub recursive_patterns: Vec<RecursivePattern>,
    pub chain_analyzer: ChainAnalyzer,
    pub comprehension_analyzer: ComprehensionAnalyzer,
}
```

### APIs and Interfaces
- `detect_function_role(func_def: &ast::StmtFunctionDef) -> PythonFunctionRole`
- `should_flag_function(metrics: &FunctionMetrics, role: PythonFunctionRole) -> bool`
- Enhanced message generation API

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/analyzers/python.rs`
  - `src/complexity/mod.rs`
  - `src/config.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

- **Unit Tests**: Role detection accuracy
- **Threshold Tests**: Preset validation
- **Pattern Tests**: Detector accuracy
- **Integration Tests**: Full complexity analysis

## Documentation Requirements

- **Code Documentation**: Role detection logic
- **User Documentation**: Threshold configuration
- **Examples**: Different complexity patterns

## Implementation Notes

- Detect main functions and entry points
- Identify test functions (test_, pytest, unittest)
- Handle decorators for role hints
- Consider async functions separately
- Account for generator functions
- Special handling for __init__ and __main__

## Migration and Compatibility

During prototype phase: Enhancement to existing complexity analysis. Will provide additional context without breaking existing threshold behavior. Default behavior remains unchanged unless configured.
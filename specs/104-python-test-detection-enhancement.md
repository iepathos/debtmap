---
number: 104
title: Python Test Detection Enhancement
category: testing
priority: high
status: draft
dependencies: []
created: 2025-09-29
---

# Specification 104: Python Test Detection Enhancement

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current Python analyzer has extremely limited test detection capabilities, only recognizing functions that start with `test_`. This misses the majority of test patterns used in modern Python projects:

Current limitations:
- Only detects `test_` prefix
- Misses pytest decorators (`@pytest.mark`, `@pytest.fixture`)
- Ignores unittest class methods
- No doctest detection
- No recognition of `setUp`, `tearDown`, fixtures
- No support for parameterized tests
- Coverage integration doesn't correlate with test detection

This results in:
- Test functions incorrectly marked as "untested"
- Inaccurate test coverage metrics
- False positive debt items for test code
- Poor quality analysis for test-heavy codebases

## Objective

Implement comprehensive test detection for Python that recognizes all common testing patterns including pytest, unittest, doctest, and nose frameworks to provide accurate test identification and coverage correlation.

## Requirements

### Functional Requirements

- Detect pytest patterns:
  - `@pytest.mark.*` decorators
  - `@pytest.fixture` decorators
  - `@pytest.parametrize` decorators
  - Fixture functions and usage
  - `conftest.py` special handling
- Detect unittest patterns:
  - Test classes (`TestCase` subclasses)
  - `setUp`, `tearDown`, `setUpClass`, `tearDownClass`
  - Test methods in test classes
  - `@unittest.skip` decorators
- Detect doctest patterns:
  - Functions with `>>>` in docstrings
  - Doctest modules
- Detect nose patterns:
  - `@with_setup` decorators
  - Setup/teardown functions
- Support test file detection:
  - `test_*.py` and `*_test.py` files
  - Files in `tests/` directories
  - Files in `test/` directories
- Integrate with coverage data to validate test execution

### Non-Functional Requirements

- Minimal performance impact on analysis
- Accurate detection without false positives
- Extensible for new test frameworks
- Clear reporting of detected test types

## Acceptance Criteria

- [ ] Pytest decorators correctly identified as test markers
- [ ] Unittest TestCase methods detected as tests
- [ ] Fixtures recognized and not marked as dead code
- [ ] Doctest patterns detected in docstrings
- [ ] Test files automatically marked as test modules
- [ ] Coverage data correctly correlated with detected tests
- [ ] 95%+ accuracy on common test patterns
- [ ] Unit tests cover all detection patterns
- [ ] Documentation includes supported test patterns

## Technical Details

### Implementation Approach

1. Enhance `is_test_function` in `src/analyzers/python.rs`
2. Create `TestPatternDetector` for comprehensive detection
3. Add decorator analysis for test markers
4. Implement class hierarchy analysis for unittest
5. Add docstring parsing for doctest detection
6. Integrate with coverage data for validation

### Architecture Changes

```rust
// src/analysis/python_test_detector.rs
pub struct PythonTestDetector {
    test_frameworks: HashSet<TestFramework>,
    custom_patterns: Vec<String>,
}

pub enum TestFramework {
    Pytest,
    Unittest,
    Doctest,
    Nose,
}

pub struct TestDetectionResult {
    is_test: bool,
    test_type: Option<TestType>,
    framework: Option<TestFramework>,
    confidence: f32,
}

pub enum TestType {
    TestFunction,
    TestMethod,
    Fixture,
    Helper,
    Setup,
    Teardown,
    Parameterized,
}
```

### Data Structures

- `TestPattern`: Defines patterns for test detection
- `TestContext`: Tracks test-related context (class, module)
- `DecoratorInfo`: Parsed decorator information

### APIs and Interfaces

```rust
impl PythonTestDetector {
    pub fn detect_test(&self, func: &ast::StmtFunctionDef, context: &TestContext) -> TestDetectionResult;
    pub fn is_test_file(&self, path: &Path) -> bool;
    pub fn extract_decorators(&self, decorators: &[ast::Expr]) -> Vec<DecoratorInfo>;
    pub fn has_doctest(&self, docstring: &str) -> bool;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/python.rs`
  - `src/core/FunctionMetrics`
  - Coverage integration modules
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each test pattern detection
- **Integration Tests**: Complete test file analysis
- **Framework Tests**: Real pytest/unittest code samples
- **Regression Tests**: Ensure no false positives
- **Coverage Tests**: Validate coverage correlation

## Documentation Requirements

- **Code Documentation**: Document each test pattern
- **User Documentation**: Supported test frameworks and patterns
- **Migration Guide**: Impact on existing analysis
- **Examples**: Sample test detection for each framework

## Implementation Notes

- Start with pytest as most common framework
- Use AST visitor pattern for efficiency
- Cache decorator analysis results
- Consider test discovery order (decorators > name > context)
- Log detected test patterns for debugging
- Handle edge cases like nested test classes

## Migration and Compatibility

- Backward compatible with existing `test_` detection
- Existing metrics remain valid but improve
- No configuration changes required
- Automatic enhancement on next analysis
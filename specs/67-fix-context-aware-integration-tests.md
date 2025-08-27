---
number: 67
title: Fix Context-Aware Integration Tests
category: testing
priority: high
status: draft
dependencies: [43]
created: 2025-08-27
---

# Specification 67: Fix Context-Aware Integration Tests

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: [43 (Context-Aware False Positive Reduction)]

## Context

Four integration tests have been marked as ignored because they were converted from subprocess-based testing to direct library API calls, but are now failing. These tests are critical for validating the context-aware false positive reduction functionality (spec 43). The tests are currently ignored with the reason "Converted to library API - needs further debugging", indicating they need to be fixed to properly validate the context-aware analyzer functionality.

The affected tests are:
- `test_context_aware_filters_parameter_analyzer` - Tests that context-aware filtering reduces false positives in parameter analysis
- `test_context_aware_filters_rust_call_graph` - Tests that context-aware filtering reduces complexity issues in call graph analysis
- `test_context_aware_on_entire_codebase` - Tests context-aware behavior on the full codebase (also marked as too slow)
- `test_context_aware_on_specific_dirs` - Tests context-aware behavior on specific directories

## Objective

Fix the four ignored context-aware integration tests to properly validate that the context-aware analyzer reduces false positives, particularly for test functions and other appropriate contexts. Ensure tests run reliably using the library API directly instead of spawning subprocesses, which was causing deadlocks.

## Requirements

### Functional Requirements
- Fix `test_context_aware_filters_parameter_analyzer` to properly test complexity filtering in test functions
- Fix `test_context_aware_filters_rust_call_graph` to validate context-aware analysis of call graph functions
- Fix `test_context_aware_on_entire_codebase` to be performant and reliable (or split into smaller tests)
- Fix `test_context_aware_on_specific_dirs` to properly analyze specific directories with context awareness
- Ensure all tests properly use the library API instead of subprocess spawning
- Tests must validate that context-aware mode reduces false positives compared to non-context-aware mode

### Non-Functional Requirements
- Tests must run without deadlocks or hangs
- Tests must complete within reasonable time limits (under 30 seconds each except for full codebase test)
- Tests must be deterministic and not depend on environment state
- Tests must properly clean up any environment variables they set

## Acceptance Criteria

- [ ] All four context-aware integration tests pass when `#[ignore]` attribute is removed
- [ ] Tests demonstrate measurable reduction in false positives when context-aware mode is enabled
- [ ] Test functions are properly filtered out from complexity/debt reports in test files
- [ ] Tests use the library API directly without spawning cargo subprocesses
- [ ] Tests properly validate that context-aware analyzer is working as designed
- [ ] No test deadlocks or hangs occur
- [ ] Tests complete in reasonable time (full codebase test can remain ignored if necessary)
- [ ] Tests are independent and can run in any order

## Technical Details

### Implementation Approach

1. **Fix Library API Usage**
   - Review the `analyze_file_directly` function in `tests/common/mod.rs`
   - Ensure proper context-aware analyzer initialization
   - Verify environment variable handling for `DEBTMAP_CONTEXT_AWARE`

2. **Fix Test Logic**
   - Ensure tests properly compare results with and without context-aware mode
   - Validate that the right debt types are being filtered
   - Check that test function detection is working correctly

3. **Address Specific Issues**
   - Parameter analyzer test: Ensure test functions are properly identified and filtered
   - Call graph test: Verify that utility functions are not flagged incorrectly
   - Codebase test: Consider breaking into smaller, focused tests or keeping as ignored
   - Directory test: Ensure proper directory traversal and file filtering

### Architecture Changes

No major architectural changes required. Focus on fixing test implementation and ensuring proper use of existing context-aware analyzer functionality.

### Data Structures

Use existing data structures:
- `AnalysisResults` from library API
- `DebtItem` for tracking detected issues
- `FunctionContext` for context detection

### APIs and Interfaces

Ensure proper use of:
- `get_analyzer_with_context()` for creating context-aware analyzers
- `analyze_file()` for direct file analysis
- Environment variable handling for configuration

## Dependencies

- **Prerequisites**: Spec 43 (Context-Aware False Positive Reduction) must be fully implemented
- **Affected Components**: 
  - `tests/integration_false_positive_test.rs` - Main test file to fix
  - `tests/common/mod.rs` - Common test utilities
  - `src/analyzers/context_aware.rs` - Context-aware analyzer implementation
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Ensure individual context detection functions work correctly
- **Integration Tests**: These ARE the integration tests - ensure they properly validate the feature
- **Performance Tests**: Measure test execution time to ensure reasonable performance
- **User Acceptance**: Run tests in CI/CD pipeline to ensure reliability

## Documentation Requirements

- **Code Documentation**: Add clear comments explaining what each test validates
- **User Documentation**: Update testing documentation to explain context-aware testing approach
- **Architecture Updates**: None required

## Implementation Notes

1. Consider whether the full codebase test should remain permanently ignored due to performance
2. Ensure tests are idempotent and don't depend on previous test state
3. Use proper test assertions with helpful error messages
4. Consider adding debug output to help diagnose failures
5. Ensure environment variables are properly set and cleaned up

## Migration and Compatibility

During prototype phase: No backward compatibility concerns. Focus on getting tests working correctly with the current library API implementation.
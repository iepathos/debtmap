---
number: 63
title: Replace Subprocess Tests with Library APIs
category: testing
priority: high
status: draft
dependencies: []
created: 2025-08-23
---

# Specification 63: Replace Subprocess Tests with Library APIs

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current test suite contains integration tests that spawn subprocess commands using `Command::new("cargo")` to test the debtmap binary. These tests are causing significant issues:

1. **Test Hangs**: When running the full test suite with `cargo nextest run` or `cargo test`, several tests hang indefinitely
2. **Resource Contention**: Subprocess spawning creates resource contention and potential deadlocks
3. **Slow Execution**: Building and running the binary via cargo adds significant overhead
4. **Flaky Tests**: Subprocess-based tests are inherently less reliable due to external dependencies
5. **Difficult Debugging**: When tests hang, it's hard to diagnose the root cause

The affected tests include:
- `integration_false_positive_test.rs` - spawns `cargo run` to test the analyzer
- Tests that may indirectly trigger subprocess operations through the codebase

## Objective

Replace all subprocess-spawning tests with direct library API calls, mock-based testing, or pre-built binary execution with proper timeouts. This will eliminate test hangs, improve test reliability, and significantly reduce test execution time.

## Requirements

### Functional Requirements

1. **Library API Testing**
   - Convert subprocess tests to use the library API directly
   - Import and call analyzer functions instead of spawning cargo processes
   - Maintain the same test coverage and validation logic

2. **Mock-Based Testing**
   - Create mock implementations for command execution where appropriate
   - Allow tests to verify command invocation without actual execution
   - Support injection of mock results for testing error conditions

3. **Pre-Built Binary Testing**
   - Build the binary once before running tests that need it
   - Execute the pre-built binary with timeout constraints
   - Cache the binary between test runs to avoid rebuilds

4. **Test Infrastructure**
   - Create test utilities for common testing patterns
   - Provide helpers for setting up test fixtures and analyzing results
   - Implement timeout wrappers for any remaining external processes

### Non-Functional Requirements

1. **Performance**
   - Tests must run at least 50% faster than current subprocess tests
   - No test should take longer than 5 seconds to execute
   - Parallel test execution must work without resource contention

2. **Reliability**
   - Tests must not hang under any circumstances
   - All tests must be deterministic and reproducible
   - Resource cleanup must be guaranteed even on test failure

3. **Maintainability**
   - Test code should be as readable as production code
   - Common patterns should be extracted into reusable utilities
   - Tests should be easy to debug when they fail

## Acceptance Criteria

- [ ] All subprocess-spawning tests are identified and documented
- [ ] Each subprocess test is converted to use library API or mocks
- [ ] No test uses `Command::new("cargo")` or similar subprocess spawning
- [ ] All tests pass with both `cargo test` and `cargo nextest run`
- [ ] Test suite runs without hanging when executed without filters
- [ ] Test execution time is reduced by at least 50%
- [ ] Test utilities are documented with usage examples
- [ ] Pre-built binary caching mechanism is implemented if needed
- [ ] All tests have explicit timeouts where external resources are involved
- [ ] Test coverage remains the same or improves

## Technical Details

### Implementation Approach

1. **Phase 1: Audit and Inventory**
   - Identify all tests that spawn subprocesses
   - Document what each test is trying to validate
   - Determine the best replacement strategy for each test

2. **Phase 2: Create Test Infrastructure**
   - Build test utility module with common helpers
   - Implement mock command executor trait
   - Create fixture management utilities

3. **Phase 3: Convert Tests**
   - Start with `integration_false_positive_test.rs`
   - Convert to use library API directly
   - Validate that test coverage is maintained

4. **Phase 4: Binary Caching (if needed)**
   - Implement build-once strategy for binary tests
   - Add timeout wrappers for binary execution
   - Cache binary in target directory

### Architecture Changes

1. **New Test Utilities Module**
   ```rust
   // tests/common/mod.rs
   pub mod fixtures;
   pub mod mock_executor;
   pub mod analysis_helpers;
   ```

2. **Mock Command Executor**
   ```rust
   pub trait CommandExecutor {
       fn execute(&self, args: &[&str]) -> Result<Output, Error>;
   }
   
   pub struct MockExecutor {
       expected_calls: Vec<ExpectedCall>,
       responses: HashMap<String, Output>,
   }
   ```

3. **Analysis Helper Functions**
   ```rust
   pub fn analyze_code_snippet(code: &str, lang: Language) -> AnalysisResult {
       // Direct library API call
   }
   
   pub fn run_with_timeout<F, T>(f: F, timeout: Duration) -> Result<T>
   where F: FnOnce() -> T
   ```

### Data Structures

```rust
// Test fixture management
pub struct TestFixture {
    pub code: String,
    pub language: Language,
    pub expected_issues: Vec<DebtItem>,
    pub config: Option<Config>,
}

// Binary execution result
pub struct BinaryResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

// Mock expectation
pub struct ExpectedCall {
    pub args: Vec<String>,
    pub response: Output,
    pub times: usize,
}
```

### APIs and Interfaces

1. **Direct Library API Usage**
   ```rust
   use debtmap::{analyze_file, get_analyzer, Config};
   
   #[test]
   fn test_analysis() {
       let analyzer = get_analyzer(Language::Rust);
       let result = analyze_file(code, path, &*analyzer);
       assert!(result.is_ok());
   }
   ```

2. **Mock-Based Testing**
   ```rust
   #[test]
   fn test_with_mock() {
       let mut mock = MockExecutor::new();
       mock.expect_call("analyze", vec!["file.rs"])
           .returns(successful_output());
       
       let result = run_analysis_with_executor(&mock);
       assert!(result.is_ok());
       mock.verify();
   }
   ```

3. **Pre-Built Binary Testing**
   ```rust
   #[test]
   fn test_with_binary() {
       let binary = get_or_build_binary();
       let result = run_with_timeout(|| {
           binary.execute(&["analyze", "test.rs"])
       }, Duration::from_secs(5));
       assert!(result.is_ok());
   }
   ```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - All integration tests in `tests/` directory
  - Test execution in CI/CD pipeline
  - Justfile test commands
- **External Dependencies**: None new

## Testing Strategy

- **Unit Tests**: Test the new mock executor and helper functions
- **Integration Tests**: Verify converted tests maintain same validation
- **Performance Tests**: Measure test execution time improvements
- **Reliability Tests**: Run test suite multiple times to ensure no hangs

## Documentation Requirements

- **Code Documentation**: Document all test utilities and their usage
- **Migration Guide**: Document how to convert subprocess tests
- **Best Practices**: Guidelines for writing new integration tests
- **README Updates**: Update testing section with new approach

## Implementation Notes

1. **Gradual Migration**: Convert tests one at a time to ensure stability
2. **Backward Compatibility**: Keep old tests temporarily with skip attributes
3. **CI/CD Updates**: May need to update CI configuration for binary caching
4. **Error Messages**: Ensure test failures provide clear diagnostic information
5. **Resource Cleanup**: Use RAII patterns to ensure cleanup even on panics

## Migration and Compatibility

During the prototype phase, breaking changes to test infrastructure are acceptable. Focus on:

1. **Correctness**: Tests must accurately validate functionality
2. **Performance**: Significant speed improvements are required
3. **Reliability**: No test hangs or flaky failures
4. **Maintainability**: Clean, understandable test code

The migration will involve:
1. Adding skip attributes to problematic tests
2. Converting tests incrementally
3. Removing skip attributes once converted
4. Deleting old subprocess-based test code

## Example Conversion

### Before (Subprocess)
```rust
#[test]
fn test_analyzer() {
    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "src/main.rs"])
        .output()
        .expect("Failed to run");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DEBT:"));
}
```

### After (Library API)
```rust
#[test]
fn test_analyzer() {
    let code = std::fs::read_to_string("src/main.rs").unwrap();
    let analyzer = get_analyzer(Language::Rust);
    let result = analyze_file(code, PathBuf::from("src/main.rs"), &*analyzer).unwrap();
    
    assert!(!result.debt_items.is_empty());
}
```

## Success Metrics

1. **Test Execution Time**: Reduced by >50%
2. **Test Reliability**: 0 hanging tests, 100% deterministic
3. **Code Coverage**: Maintained or improved
4. **Developer Experience**: Easier to write and debug tests
5. **CI/CD Performance**: Faster pipeline execution
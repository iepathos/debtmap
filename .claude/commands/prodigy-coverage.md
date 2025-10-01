# /prodigy-coverage

Analyze test coverage gaps using MMM context data and generate a targeted specification for implementing comprehensive test coverage improvements with proper unit tests and mocking.

## Variables

SCOPE: $ARGUMENTS (optional - specify scope like "src/core", "src/context", or "all")

## Execute

### Phase 1: Test Coverage Analysis

1. **Run Coverage Analysis**
   - Execute `cargo tarpaulin --out Json` to generate coverage data
   - Identify files with low coverage percentages
   - Find untested functions and critical code paths
   - Analyze test quality and effectiveness

2. **Identify Coverage Gaps**
   - List functions without any test coverage
   - Find critical paths that lack tests
   - Identify edge cases not covered by existing tests
   - Check for missing error handling tests

3. **Coverage Gap Prioritization**
   - **Critical**: `untested_functions` with `criticality: "High"` that truly lack tests
   - **Public APIs**: Functions in `architecture.json` components without proper unit tests
   - **Low Coverage Files**: Files from `file_coverage` with < 50% coverage
   - **Hotspots**: Functions in `technical_debt.json` hotspots with poor test quality

### Phase 2: Fallback Analysis (Only if MMM context unavailable)

**If `MMM_CONTEXT_AVAILABLE` ≠ true**:

1. **Quick Coverage Check**
   ```bash
   cargo tarpaulin --skip-clean --engine llvm --out Json --output-dir target/coverage --timeout 120
   ```

2. **Basic Gap Identification**
   - Files with <70% line coverage
   - Public functions without any test coverage
   - Error Result types without error path tests

**Important**: MMM context provides much better analysis. Encourage context generation.

### Phase 3: Pre-Generation Validation

**CRITICAL**: Validate coverage gaps before generating spec.

1. **Test Existence Check**
   ```bash
   # For each untested_function, check if tests already exist
   for function in untested_functions:
       grep -n "test_${function.name}" "${function.file}" || true
       grep -n "#\[test\].*${function.name}" "${function.file}" || true
   ```

2. **Test Quality Analysis**
   - Identify tests that only expect failures (anti-pattern)
   - Find tests that don't mock external dependencies
   - Detect integration tests masquerading as unit tests
   - Check for tests not included in coverage runs

3. **True Coverage Gaps**
   - Functions without ANY test coverage
   - Functions with tests that don't execute the implementation
   - Code paths within functions that aren't covered
   - Error handling branches without test coverage

### Phase 4: Generate Coverage Improvement Specification

**CRITICAL**: Create actionable spec file for proper unit test implementation.

1. **Spec File Location**
   - Directory: `specs/`
   - Filename: `iteration-{unix_timestamp}-coverage-improvements.md`
   - Must match pattern: `*-coverage-improvements.md`

2. **Smart Function Selection**
   - Only include functions that TRULY lack proper tests
   - Exclude functions that have tests but poor coverage measurement
   - Focus on functions where unit tests with mocking will help
   - Prioritize by: actual untested status → criticality → complexity

### Phase 5: Spec Content Generation

**Create comprehensive UNIT test implementation instructions**:

1. **Function-Level Unit Test Plans**
   - For each truly untested function: exact file path, function signature
   - **CRITICAL**: Include mock setup for ALL external dependencies
   - Show how to mock: Claude API, subprocess, file system, network
   - Include both success and error test cases with proper assertions
   - Focus on testing business logic, not integration points

2. **Mock-First Test Design**
   - Create reusable mock implementations
   - Show mock configuration for different scenarios
   - Test error paths by configuring mocks to fail
   - Ensure tests run without ANY external dependencies

3. **Test Organization**
   - **Unit tests**: In same file as implementation using `#[cfg(test)]`
   - **Mocks**: In `src/testing/mocks/` for reusability
   - **Integration tests**: Separate, only after unit tests are complete
   - Include commands: `cargo test --lib` for unit tests only

### Phase 6: Modern Rust Unit Testing Patterns

**Include these UNIT TEST patterns with mocking**:

1. **Async Function Unit Testing with Mocks**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::testing::mocks::{MockClaudeClient, MockSubprocess};
       
       #[tokio::test]
       async fn test_async_function_success() {
           // Setup mocks
           let mut mock_claude = MockClaudeClient::new();
           mock_claude.expect_send_message()
               .returning(|_| Ok("Success response".to_string()));
           
           // Test the function with mock
           let result = async_function_under_test(&mock_claude).await;
           assert!(result.is_ok());
           assert_eq!(result.unwrap(), expected_value);
       }
   }
   ```

2. **Error Path Testing with Mock Failures**
   ```rust
   #[test]
   fn test_function_error_cases() {
       // Configure mock to fail
       let mut mock_fs = MockFileSystem::new();
       mock_fs.expect_read_file()
           .returning(|_| Err(io::Error::new(io::ErrorKind::NotFound, "File not found")));
       
       // Test error handling
       let result = function_with_file_ops(&mock_fs);
       assert!(result.is_err());
       assert!(result.unwrap_err().to_string().contains("File not found"));
   }
   ```

3. **Subprocess Mocking Example**
   ```rust
   #[test]
   fn test_command_execution() {
       let mut mock_subprocess = MockSubprocess::new();
       mock_subprocess.expect_run_command()
           .with(eq("cargo"), eq(&["test"]))
           .returning(|_, _| Ok("All tests passed".to_string()));
       
       let result = run_tests_with_subprocess(&mock_subprocess);
       assert!(result.is_ok());
   }
   ```

### Phase 6: Generate and Commit Specification

**REQUIRED OUTPUT**: Create spec file at `specs/temp/iteration-{timestamp}-coverage-improvements.md`

1. **Spec Template Structure**
   ```markdown
   # Coverage Improvements - Iteration {timestamp}
   
   ## Overview
   Test coverage improvements based on MMM context analysis.
   Current coverage: {overall_coverage}% → Target: {target_coverage}%
   {If MMM_FOCUS: Focus area: {focus}}
   
   ## Critical Functions Needing Unit Tests
   
   ### Function: `{function_name}` in {file_path}:{line_number}
   **Criticality**: {High|Medium|Low}
   **Current Status**: {No test coverage | Has integration test expecting failure | Test exists but doesn't execute}
   **Dependencies**: {List external dependencies that need mocking}
   
   #### Add these UNIT tests to {file_path}:
   ```rust
   #[cfg(test)] 
   mod tests {
       use super::*;
       use crate::testing::mocks::{MockClaudeClient, MockSubprocess, MockFileSystem};
       use std::sync::Arc;
       
       #[test] // or #[tokio::test] for async
       fn test_{function_name}_success() {
           // Setup mocks for dependencies
           let mut mock_claude = MockClaudeClient::new();
           mock_claude.expect_send_message()
               .with(eq("/command"))
               .returning(|_| Ok("expected response".to_string()));
           
           let mock_subprocess = Arc::new(MockSubprocess::new());
           
           // Call function with mocks
           let result = {function_name}(&mock_claude, &mock_subprocess, test_input);
           
           // Assert business logic outcomes
           assert!(result.is_ok());
           assert_eq!(result.unwrap(), expected_output);
       }
       
       #[test]
       fn test_{function_name}_handles_claude_error() {
           // Mock Claude API failure
           let mut mock_claude = MockClaudeClient::new();
           mock_claude.expect_send_message()
               .returning(|_| Err(Error::new("API unavailable")));
           
           // Test error handling
           let result = {function_name}(&mock_claude, valid_input);
           assert!(result.is_err());
           assert!(result.unwrap_err().to_string().contains("API unavailable"));
       }
       
       #[test]
       fn test_{function_name}_handles_subprocess_failure() {
           // Mock subprocess failure
           let mut mock_subprocess = MockSubprocess::new();
           mock_subprocess.expect_run_command()
               .returning(|_, _| Err(Error::new("Command failed")));
           
           // Test error propagation
           let result = {function_name}(&mock_subprocess, test_params);
           assert!(result.is_err());
       }
   }
   ```
   
   ## Mock Infrastructure Required
   
   ### Create Mock Implementations in `src/testing/mocks/`
   ```rust
   // src/testing/mocks/claude.rs
   use mockall::mock;
   
   mock! {
       pub ClaudeClient {
           pub async fn send_message(&self, message: &str) -> Result<String, Error>;
           pub async fn execute_command(&self, cmd: &str, args: &[String]) -> Result<String, Error>;
       }
   }
   
   // src/testing/mocks/subprocess.rs
   mock! {
       pub SubprocessManager {
           pub fn run_command(&self, cmd: &str, args: &[&str]) -> Result<String, Error>;
           pub async fn run_async(&self, cmd: &str, args: &[&str]) -> Result<String, Error>;
       }
   }
   ```
   
   ## Implementation Checklist
   - [ ] Create mock infrastructure in src/testing/mocks/
   - [ ] Add unit tests for {count} critical functions with proper mocking
   - [ ] Remove/refactor {count} ineffective integration tests
   - [ ] Verify tests pass without external dependencies: `cargo test --lib`
   - [ ] Check coverage improves: `cargo tarpaulin --lib`
   - [ ] Ensure all high-criticality functions have unit tests
   - [ ] Follow project conventions from .prodigy/context/conventions.json
   ```

2. **Context Data Integration**
   - Extract exact function names, file paths, line numbers from `untested_functions`
   - **VERIFY** each function doesn't already have tests before including
   - Use `file_coverage` data to identify low-coverage files
   - Check `hybrid_coverage.json` for quality metrics and priority gaps
   - Reference `conventions.json` for project testing patterns

3. **Validation Requirements**
   - Each function must have unit tests with mocked dependencies
   - Tests must run without Claude API or external services
   - Include multiple error scenarios (API failure, subprocess failure, etc.)
   - Show exact mock setup for each dependency
   - Verify with `cargo test --lib` (unit tests only)

4. **Git Commit (Automation Mode)**
   ```bash
   mkdir -p specs/temp
   # Create spec file
   git add specs/temp/iteration-{timestamp}-coverage-improvements.md
   git commit -m "test: generate coverage improvement spec for iteration-{timestamp}"
   ```
   
   **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.
   
   **Skip commit if**: No critical coverage gaps found (overall coverage >85%)

## Success Criteria & Output

**Create spec only if**: 
- Critical functions truly lack unit tests (not just poor coverage measurement)
- >5 functions need proper unit tests with mocking
- Overall coverage <75% AND can be improved with unit tests

**Console Output**:
```
✓ MMM context loaded - {total} functions analyzed
✓ Found {existing_count} functions with existing tests needing improvement
✓ Found {new_count} functions truly lacking unit tests
✓ Generated spec: iteration-{timestamp}-coverage-improvements.md  
✓ Focus: Unit tests with proper mocking for {count} functions
```

**Or if no gaps**:
```
✓ Coverage analysis complete - {coverage}%
✓ All critical functions have tests (may need coverage tool configuration)
✓ Consider checking why existing tests don't generate coverage
```

**Spec File Output**: `specs/temp/iteration-{timestamp}-coverage-improvements.md`

## Coverage Targets

**Priority Levels**:
- **Critical**: Functions with `criticality: "High"` in untested_functions
- **Public APIs**: Architecture components without adequate test coverage  
- **Focus Areas**: Functions matching MMM_FOCUS directive
- **Error Paths**: Result-returning functions without error case tests

**Target Thresholds**:
- Overall project coverage: Current + 10% (minimum 75%)
- Critical functions: 100% coverage
- Public APIs: >90% coverage
- New code: 100% coverage requirement

## Command Integration

**Workflow Chain**: `prodigy-coverage` → generates spec → `prodigy-implement-spec` → `prodigy-lint`

**Context Dependencies**: 
- Requires `.prodigy/context/test_coverage.json` with `untested_functions` array
- Uses `.prodigy/context/architecture.json` for component interfaces
- Follows patterns from `.prodigy/context/conventions.json`
- References current metrics from `.prodigy/metrics/current.json`

**Output Contract**: Spec file matching `*-coverage-improvements.md` pattern for workflow consumption

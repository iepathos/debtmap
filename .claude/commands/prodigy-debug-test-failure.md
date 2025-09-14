# /prodigy-debug-test-failure

Fix failing tests by analyzing test output and applying targeted fixes.

**Important**: This command MUST create a commit after successfully fixing tests, as it's configured with `commit_required: true` in workflow on_failure handlers.

## Variables

--spec: Optional path to spec file that generated code that introduced regression.
--output: Test failure output from cargo nextest run

## Execute

1. **Parse current test output** to identify what's failing NOW:
   - Failed test names and file locations (may differ from previous attempts)
   - Error types (assertion, panic, compile, async)
   - Specific error messages and stack traces

2. **Read the spec file** to understand test intent and implementation

3. **Apply fixes using functional programming principles**:
   ```rust
   // Assertion failure → Update expected values
   assert_eq!(result, 42); // Change to actual value

   // Missing imports → Add use statements
   use tempfile::TempDir;
   use mockall::predicate::*;

   // Async test → Convert to tokio::test
   #[tokio::test]
   async fn test_async() { ... }

   // Missing setup → Add fixtures with immutable patterns
   let temp_dir = TempDir::new()?;
   std::env::set_current_dir(&temp_dir)?;

   // Use functional patterns in tests
   let results: Vec<_> = items
       .iter()
       .filter(|item| item.is_valid())
       .map(|item| item.transform())
       .collect();
   ```

4. **Fix strategy (apply all relevant fixes)**:
   - Import errors → Add missing use statements
   - Assertion failures → Adjust expected values to match actual
   - Async issues → Convert to #[tokio::test]
   - Missing setup → Add fixtures, mocks, or test data
   - Each run may reveal new failures after fixing others

5. **Verify all tests pass**:
   ```bash
   cargo nextest run  # Run full suite, not just specific tests
   ```

6. **Create a commit** after fixing the tests:
   ```bash
   git add -A
   git commit -m "fix: resolve test failures with functional patterns

   - Fixed N failing tests
   - Applied functional programming principles:
     * Used iterator chains instead of loops
     * Extracted pure functions for test logic
     * Preferred immutability in test fixtures
   - [List specific fixes applied]"
   ```

7. **Output**:
   - Success: "✓ All tests passing after fixing N tests"
   - Failed: "✗ Fixed N tests but M still failing"

## Common Patterns - Idiomatic Rust with Functional Programming

**Import fixes**:
```rust
use std::path::PathBuf;
use anyhow::Result;
```

**Async runtime**:
```rust
#[tokio::test]  // Not #[test]
async fn test_name() { }
```

**Test doubles with functional patterns**:
```rust
// Prefer immutable mocks where possible
let mock = MockService::new()
    .with_expectation(|call| call.returning(|| Ok(42)));
```

**File system with functional composition**:
```rust
let temp = TempDir::new()?;
let path = temp.path().join("test.txt");

// Use functional patterns for file operations
let content = std::fs::read_to_string(&path)
    .map(|s| s.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>())?;
```

**Test pure functions**:
```rust
// Extract pure logic from tests
fn validate_result(input: &str) -> bool {
    input.starts_with("test_") && input.len() > 5
}

#[test]
fn test_validation() {
    assert!(validate_result("test_example"));
    assert!(!validate_result("test"));
}
```

**Use iterator chains in assertions**:
```rust
let valid_items: Vec<_> = results
    .iter()
    .filter(|r| r.is_valid())
    .collect();
assert_eq!(valid_items.len(), expected_count);
```

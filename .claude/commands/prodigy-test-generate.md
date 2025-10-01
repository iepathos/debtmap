# Test Generation Command

Generates comprehensive test suites for untested or poorly tested code.

## Usage

```
/prodigy-test-generate [target]
```

Examples:
- `/prodigy-test-generate` - Generate tests for all untested code
- `/prodigy-test-generate src/analyzer/` - Focus on analyzer module
- `/prodigy-test-generate --integration` - Generate integration tests
- `/prodigy-test-generate --edge-cases` - Focus on edge case testing

## What This Command Does

1. **Analyzes Test Coverage**
   - Identifies untested functions and modules
   - Finds code paths without test coverage
   - Detects missing edge case tests
   - Evaluates test quality

2. **Generates Test Spec**
   - Creates a spec for comprehensive test generation
   - Includes unit tests for all public functions
   - Adds integration tests for workflows
   - Covers edge cases and error conditions

3. **Commits Test Plan**
   - Commits the test generation spec
   - Ready for implementation
   - Maintains test organization standards

## Test Types

- **Unit Tests**: Test individual functions in isolation
- **Integration Tests**: Test module interactions
- **Edge Cases**: Test boundary conditions and error paths
- **Property Tests**: Generate random test cases
- **Regression Tests**: Prevent bug recurrence

## Coverage Goals

- Achieve >80% code coverage
- Test all public APIs
- Cover error conditions
- Validate edge cases
- Test concurrent scenarios

## Output Format

Generates and commits a test spec:

```
test: generate test spec for {target} tests-{timestamp}
```

## Best Practices

- Tests should be independent
- Use descriptive test names
- Test one thing per test
- Include both positive and negative cases
- Mock external dependencies
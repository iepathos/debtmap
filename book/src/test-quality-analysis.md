# Test Quality Analysis

Debtmap can analyze the quality and patterns in your test code to help identify testing gaps and improvement opportunities.

## Overview

Test quality analysis examines your test suite to identify:
- Assertion patterns and their quality
- Overly complex tests that are hard to maintain
- Potentially flaky tests
- Testing framework usage patterns

## Assertion Patterns

Debtmap identifies and categorizes assertion types used in tests:

```rust
// Strong assertions
assert_eq!(actual, expected);
assert_ne!(forbidden, actual);

// Weaker assertions
assert!(condition);
```

## Test Complexity

Tests should be simple and focused. Debtmap flags tests that exhibit:
- High cyclomatic complexity
- Deep nesting
- Multiple responsibilities

## Flaky Test Detection

Pattern analysis helps identify potentially flaky tests through:
- Time-dependent logic
- Non-deterministic operations
- External dependency patterns

## Framework Detection

Debtmap automatically detects testing frameworks:
- Rust: `#[test]`, `#[tokio::test]`, proptest
- Python: pytest, unittest
- JavaScript: jest, mocha

## Test Classification

Tests are classified by type:
- **Unit tests**: Isolated function testing
- **Integration tests**: Cross-module testing
- **Property tests**: Generative testing with proptest

## Configuration

```toml
[test_quality]
enabled = true
complexity_threshold = 10
```

## See Also

- [Coverage Integration](coverage-integration.md)
- [Complexity Metrics](analysis-guide/complexity-metrics.md)

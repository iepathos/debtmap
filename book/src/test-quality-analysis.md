# Test Quality Analysis

Debtmap provides comprehensive analysis of test code quality, detecting anti-patterns, identifying potentially flaky tests, and providing actionable recommendations for improvement.

## Overview

Test quality analysis examines your test suite to identify:

- **Assertion patterns** - Missing or weak assertions that reduce test effectiveness
- **Test complexity** - Overly complex tests that are hard to maintain
- **Flaky test patterns** - Tests that may fail intermittently
- **Framework detection** - Automatic detection of testing frameworks
- **Test type classification** - Categorization of tests by type (unit, integration, property, benchmark)

The analysis pipeline uses multiple detectors defined in `src/testing/mod.rs:111-115`:

```rust
let detectors: Vec<Box<dyn TestingDetector>> = vec![
    Box::new(assertion_detector::AssertionDetector::new()),
    Box::new(complexity_detector::TestComplexityDetector::new()),
    Box::new(flaky_detector::FlakyTestDetector::new()),
];
```

## Test Type Classification

Tests are classified into distinct types based on attributes, file paths, and naming patterns. The classification is handled by `TestClassifier` in `src/testing/rust/test_classifier.rs`.

### Test Types

From `src/testing/rust/mod.rs:69-76`:

| Test Type | Description | Detection Method |
|-----------|-------------|------------------|
| `UnitTest` | Isolated function testing | Default for `src/` tests |
| `IntegrationTest` | Cross-module testing | Tests in `tests/` directory |
| `BenchmarkTest` | Performance benchmarks | `#[bench]` attribute |
| `PropertyTest` | Generative testing | `proptest!` or `quickcheck!` macros |
| `DocTest` | Documentation tests | Extracted from doc comments |

### Classification Logic

The classifier checks in order (`src/testing/rust/test_classifier.rs:22-44`):

1. **Benchmark detection**: Functions with `#[bench]` attribute
2. **Property test detection**: Functions using proptest or quickcheck
3. **Integration test path**: Files in `tests/` directory
4. **Default**: Unit test

```rust
// Example: Integration test detection
fn is_integration_test_path(&self, path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/tests/") || path_str.starts_with("tests/")
}
```

## Assertion Pattern Detection

The `AssertionDetector` (`src/testing/assertion_detector.rs`) identifies tests with missing or inadequate assertions.

### Detected Assertion Types

From `src/testing/rust/mod.rs:79-95`:

| Assertion Type | Description | Quality Rating |
|----------------|-------------|----------------|
| `Assert` | `assert!(condition)` | Weak - no context on failure |
| `AssertEq` | `assert_eq!(left, right)` | Strong - shows expected vs actual |
| `AssertNe` | `assert_ne!(left, right)` | Strong - shows values |
| `Matches` | `matches!(value, pattern)` | Medium - pattern-based |
| `ShouldPanic` | `#[should_panic]` attribute | Valid for panic tests |
| `ResultOk` | `Ok(())` return | Valid for Result-based tests |
| `Custom(String)` | Custom assertion macros | Depends on implementation |

### Assertion Macro Recognition

The detector recognizes these macros (`src/testing/assertion_detector.rs:227-237`):

```rust
fn is_assertion_macro(name: &str) -> bool {
    matches!(
        name,
        "assert" | "assert_eq" | "assert_ne" | "assert_matches"
            | "debug_assert" | "debug_assert_eq" | "debug_assert_ne"
    )
}
```

### Tests Without Assertions

Tests flagged as having no assertions receive suggested fixes:

```rust
// From src/testing/assertion_detector.rs:257-278
fn suggest_assertions(analysis: &TestStructureAnalysis) -> Vec<String> {
    if analysis.has_action && !analysis.has_assertions {
        vec![
            "Add assertions to verify the behavior".to_string(),
            "Consider using assert!, assert_eq!, or assert_ne!".to_string(),
        ]
    }
    // ...
}
```

## Test Complexity Analysis

The `TestComplexityDetector` (`src/testing/complexity_detector.rs`) measures test complexity and suggests simplifications.

### Complexity Sources

From `src/testing/mod.rs:39-46`:

| Source | Description | Threshold |
|--------|-------------|-----------|
| `ExcessiveMocking` | Too many mock setups | > 3 mocks |
| `NestedConditionals` | Deeply nested if/match | Nesting > 1 level |
| `MultipleAssertions` | Too many assertions | > 5 assertions |
| `LoopInTest` | Loops in test code | Any loop detected |
| `ExcessiveSetup` | Long test functions | > 30 lines |

### Complexity Scoring

The complexity score is calculated in `src/testing/complexity_detector.rs:304-309`:

```rust
pub(crate) fn calculate_total_complexity(analysis: &TestComplexityAnalysis) -> u32 {
    analysis.cyclomatic_complexity
        + (analysis.mock_setup_count as u32 * 2)
        + analysis.assertion_complexity
        + (analysis.line_count as u32 / 10)  // Penalty for long tests
}
```

The Rust-specific complexity scoring (`src/testing/rust/mod.rs:24-29` in doc comments):

- **Conditional statements**: +2 per `if`/`match`
- **Loops**: +3 per loop
- **Assertions beyond 5**: +1 per additional assertion
- **Nesting depth > 2**: +2 per level
- **Tests > 30 lines**: +(lines-30)/10

### Simplification Recommendations

From `src/testing/mod.rs:48-55`:

| Recommendation | When Applied | Action |
|----------------|--------------|--------|
| `ExtractHelper` | Long tests with shared setup | Extract common code to helper function |
| `SplitTest` | Many assertions + many mocks | Split into focused tests |
| `ParameterizeTest` | High cyclomatic complexity (> 5) | Use parameterized testing |
| `SimplifySetup` | Default recommendation | Reduce test setup complexity |
| `ReduceMocking` | Excessive mocks (> max_mock_setups) | Use real implementations or simpler mocks |

The suggestion logic (`src/testing/complexity_detector.rs:320-334`):

```rust
pub(crate) fn suggest_simplification(
    analysis: &TestComplexityAnalysis,
    detector: &TestComplexityDetector,
) -> TestSimplification {
    match () {
        _ if analysis.mock_setup_count > detector.max_mock_setups => {
            TestSimplification::ReduceMocking
        }
        _ if analysis.line_count > detector.max_test_length => {
            classify_length_based_simplification(analysis)
        }
        _ if analysis.cyclomatic_complexity > 5 => TestSimplification::ParameterizeTest,
        _ => TestSimplification::SimplifySetup,
    }
}
```

## Flaky Test Detection

The `FlakyTestDetector` (`src/testing/flaky_detector.rs`) identifies patterns that can cause intermittent test failures.

### Flakiness Types

From `src/testing/mod.rs:57-65`:

| Type | Description | Impact |
|------|-------------|--------|
| `TimingDependency` | Uses sleep, timeouts, or time measurements | High |
| `RandomValues` | Uses random number generation | Medium |
| `ExternalDependency` | Calls external services or APIs | Critical |
| `FilesystemDependency` | Reads/writes files | Medium |
| `NetworkDependency` | Network operations | Critical |
| `ThreadingIssue` | Thread spawning or synchronization | High |

### Rust-Specific Flakiness Types

From `src/testing/rust/mod.rs:98-107`:

| Type | Description |
|------|-------------|
| `HashOrdering` | HashMap iteration (non-deterministic order) |
| `ThreadingIssue` | Unsynchronized concurrent access |

### Reliability Impact Levels

From `src/testing/mod.rs:67-73`:

- **Critical**: External dependencies, network calls - high failure probability
- **High**: Timing dependencies, threading issues - moderate failure probability
- **Medium**: Random values, filesystem operations - occasional failures
- **Low**: Minor ordering issues - rare failures

### Detection Patterns

The detector uses pattern categories defined in `src/testing/flaky_detector.rs:190-284`:

**Timing Patterns** (`TimingDependency`):
```
sleep, Instant::now, SystemTime::now, Duration::from, delay,
timeout, wait_for, park_timeout, recv_timeout
```

**Random Patterns** (`RandomValues`):
```
rand, random, thread_rng, StdRng, SmallRng, gen_range,
sample, shuffle, choose
```

**External Service Patterns** (`ExternalDependency`):
```
reqwest, hyper, http, Client::new, HttpClient, ApiClient,
database, db, postgres, mysql, redis, mongodb, sqlx, diesel
```

**Filesystem Patterns** (`FilesystemDependency`):
```
fs::, File::, std::fs, tokio::fs, async_std::fs,
read_to_string, write, create, remove_file, remove_dir
```

**Network Patterns** (`NetworkDependency`):
```
TcpStream, TcpListener, UdpSocket, connect, bind,
listen, accept, send_to, recv_from
```

### Stabilization Suggestions

Each flaky pattern includes a stabilization suggestion:

```rust
// From src/testing/flaky_detector.rs:156-187
_ if is_timing_function(path_str) => Some(FlakinessIndicator {
    flakiness_type: FlakinessType::TimingDependency,
    impact: ReliabilityImpact::High,
    suggestion: "Replace sleep/timing dependencies with deterministic waits or mocks"
        .to_string(),
}),
_ if is_external_service_call(path_str) => Some(FlakinessIndicator {
    flakiness_type: FlakinessType::ExternalDependency,
    impact: ReliabilityImpact::Critical,
    suggestion: "Mock external service calls for unit tests".to_string(),
}),
```

## Timing Classification

The timing classifier (`src/testing/timing_classifier.rs`) categorizes timing-related operations to assess flakiness risk.

### Timing Categories

From `src/testing/timing_classifier.rs:31-44`:

| Category | Description | Flaky Risk |
|----------|-------------|------------|
| `CurrentTime` | `Instant::now()` | Yes |
| `SystemTime` | `SystemTime::now()` | Yes |
| `DurationCreation` | `Duration::from_*()` | No |
| `ElapsedTime` | `elapsed()`, `duration_since()` | Yes |
| `Sleep` | Thread sleep operations | Yes |
| `Timeout` | Operations with timeout | Yes |
| `Wait` | Waiting operations (not await) | Yes |
| `ThreadTimeout` | `park_timeout`, `recv_timeout` | Yes |
| `Delay` | Delay operations | Yes |
| `Timer` | Timer-based operations | Yes |
| `Unknown` | Unrecognized patterns | No |

Only `DurationCreation` and `Unknown` are considered non-flaky.

## Test Quality Issue Types

The Rust-specific module tracks comprehensive issue types (`src/testing/rust/mod.rs:131-140`):

| Issue Type | Severity | Description |
|------------|----------|-------------|
| `NoAssertions` | High | Test has no assertions |
| `TooComplex(u32)` | Medium | Complexity score exceeds threshold |
| `FlakyPattern(type)` | High | Contains flaky pattern |
| `ExcessiveMocking(usize)` | Medium | Too many mock setups |
| `IsolationIssue` | High | Test affects shared state |
| `TestsTooMuch` | Medium | Tests too many concerns |
| `SlowTest` | Low | Test may be slow |

### Severity Levels

From `src/testing/rust/mod.rs:110-116`:

- **Critical**: Fundamental test quality issues
- **High**: Significant problems affecting reliability
- **Medium**: Quality concerns worth addressing
- **Low**: Minor improvements possible

## Framework Detection

Debtmap automatically detects testing frameworks to provide context-aware analysis.

### Supported Frameworks

From `src/testing/rust/mod.rs:54-66`:

| Framework | Detection | Description |
|-----------|-----------|-------------|
| `Std` | `#[test]` attribute | Standard library test |
| `Criterion` | `criterion` crate usage | Benchmarking framework |
| `Proptest` | `proptest!` macro | Property-based testing |
| `Quickcheck` | `quickcheck!` macro | Property-based testing |
| `Rstest` | `#[rstest]` attribute | Parameterized testing |

### Multi-Language Support

From `src/testing/mod.rs:89-106`:

```rust
// Test attribute detection
path_str == "test"
    || path_str == "tokio::test"
    || path_str == "async_std::test"
    || path_str == "bench"
    || path_str.ends_with("::test")
```

The documentation also covers:
- **Rust**: `#[test]`, `#[tokio::test]`, proptest, rstest, criterion
- **Python**: pytest, unittest
- **JavaScript**: jest, mocha

## Fast vs Slow Test Detection

Slow tests are identified as a quality issue (`src/testing/rust/mod.rs:139`):

```rust
RustTestIssueType::SlowTest
```

Detection criteria include:
- Long test functions (> 50 lines by default)
- Timing operations that suggest waiting
- External service calls that may have latency

## Configuration

### Basic Configuration

```toml
[test_quality]
enabled = true
complexity_threshold = 10
```

### Advanced Configuration

From `src/testing/complexity_detector.rs:9-13`:

```toml
[test_quality]
enabled = true

# Maximum allowed test complexity score
complexity_threshold = 10

# Maximum number of mock setups per test
max_mock_setups = 5

# Maximum test function length (lines)
max_test_length = 50
```

Default values from `TestComplexityDetector::new()`:
- `max_test_complexity`: 10
- `max_mock_setups`: 5
- `max_test_length`: 50

## Common Issues and Solutions

### Issue: Test Without Assertions

**Detection**: `TestWithoutAssertions` anti-pattern

**Example of problematic code**:
```rust
#[test]
fn test_without_assertion() {
    let result = calculate(10);
    // No assertion!
}
```

**Fix**:
```rust
#[test]
fn test_with_assertion() {
    let result = calculate(10);
    assert_eq!(result, 20);
}
```

### Issue: Timing-Dependent Test

**Detection**: `FlakinessType::TimingDependency`

**Example of problematic code**:
```rust
#[test]
fn test_timing_dependent() {
    let start = Instant::now();
    do_work();
    assert!(start.elapsed() < Duration::from_millis(100));
}
```

**Fix**:
```rust
#[test]
fn test_deterministic() {
    let result = do_work();
    assert!(result.is_success());
}
```

### Issue: Excessive Mocking

**Detection**: `ComplexitySource::ExcessiveMocking`

**Solution**: Consider using real implementations, test doubles, or restructuring to reduce mock count below 5.

### Issue: Tests with Loops

**Detection**: `ComplexitySource::LoopInTest`

**Solution**: Use parameterized tests with `#[rstest]` or property-based testing with `proptest` instead of loops.

## See Also

- [Coverage Integration](coverage-integration.md) - Combining coverage with test quality analysis
- [Complexity Metrics](analysis-guide/complexity-metrics.md) - Understanding complexity scoring
- [Configuration](configuration.md) - Full configuration reference

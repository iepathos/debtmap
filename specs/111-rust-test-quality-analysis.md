---
number: 111
title: Rust Test Quality Analysis
category: testing
priority: high
status: draft
dependencies: []
created: 2025-10-15
---

# Specification 111: Rust Test Quality Analysis

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Python analyzer has comprehensive test quality analysis including assertion detection, flaky pattern identification, excessive mocking detection, and test complexity scoring. The Rust analyzer has minimal test analysis, only identifying tests through `#[test]` attributes and the `test_` prefix without any quality assessment.

Current Rust test analysis limitations:
- No assertion counting or validation
- No test complexity metrics
- No flaky test pattern detection
- No test isolation analysis
- No detection of excessive setup/teardown
- No framework-specific analysis (criterion, proptest)
- No integration test vs unit test distinction
- No test maintainability scoring

This creates a significant capability gap where Python projects get detailed test quality feedback while Rust projects receive minimal test insights, despite Rust's testing being equally important.

## Objective

Implement comprehensive test quality analysis for Rust that matches Python's capabilities, including assertion validation, test complexity metrics, flaky pattern detection, framework-specific analysis, and test maintainability scoring to provide actionable feedback on Rust test quality.

## Requirements

### Functional Requirements

1. **Test Detection and Classification**
   - Detect unit tests (`#[test]` attribute)
   - Identify integration tests (`tests/` directory)
   - Recognize benchmark tests (`#[bench]`, criterion)
   - Detect property tests (proptest, quickcheck)
   - Identify doc tests (code blocks in doc comments)
   - Track test modules (`#[cfg(test)]`)
   - Support test organization patterns

2. **Assertion Analysis**
   - Count assertions per test
   - Identify tests with no assertions
   - Track assertion types (`assert!`, `assert_eq!`, `assert_ne!`)
   - Detect custom assertion macros
   - Identify Result-based tests (`#[should_panic]`, `Result<()>`)
   - Track assertion complexity
   - Detect assertion anti-patterns

3. **Test Complexity Scoring**
   - Calculate test complexity metrics:
     - Conditional statements: +2 per if/match
     - Loops: +3 per loop
     - Assertions: +1 per assertion beyond 5
     - Setup complexity: +2 per complex setup
     - Nesting depth: +2 per level > 2
     - Line count: +(lines-30)/10 for tests > 30 lines
   - Track test maintainability index
   - Identify overly complex tests

4. **Flaky Test Pattern Detection**
   - Timing dependencies:
     - `std::thread::sleep` usage
     - `std::time::Instant::now()` comparisons
     - Timeout-based assertions
   - Non-deterministic values:
     - Random number generation (`rand` crate)
     - UUID generation
     - Hash ordering (HashMap iteration)
   - External dependencies:
     - Network calls (HTTP requests)
     - File system dependencies (hardcoded paths)
     - Database connections
   - Concurrency issues:
     - Unsynchronized thread access
     - Race conditions in tests
     - Channel timing dependencies

5. **Test Isolation Analysis**
   - Detect shared mutable state between tests
   - Identify global state usage
   - Track static variable modifications
   - Detect filesystem artifacts not cleaned up
   - Identify database state not reset
   - Find tests that depend on execution order

6. **Test Organization Analysis**
   - Identify overly large test modules
   - Detect missing test organization (mod structure)
   - Track test naming conventions
   - Identify helper functions that should be extracted
   - Detect test duplication patterns

7. **Framework-Specific Analysis**
   - **Standard library tests**: Basic assertion patterns
   - **Criterion benchmarks**: Setup/teardown validation
   - **Proptest**: Property test quality metrics
   - **Quickcheck**: Generator complexity
   - **Mockall/Mockito**: Mock usage patterns and counts
   - **rstest**: Parameterized test validation

8. **Test Anti-Patterns**
   - Tests that test too much (multiple concerns)
   - Tests with excessive setup (fixture complexity)
   - Tests that don't actually test (only setup, no assertions)
   - Tests with unclear failure messages
   - Brittle tests (over-specified assertions)
   - Slow tests (> 1 second for unit tests)

### Non-Functional Requirements

- **Accuracy**: < 10% false positive rate
- **Performance**: < 5% overhead on Rust analysis time
- **Coverage**: Analyze 95%+ of test code
- **Framework Support**: Support 5+ major test frameworks
- **Maintainability**: Clear, extensible pattern definitions

## Acceptance Criteria

- [ ] Tests without assertions detected and flagged
- [ ] Test complexity scored with multiple factors
- [ ] Flaky test patterns identified (timing, randomness, external deps)
- [ ] Test isolation issues detected (shared state, filesystem)
- [ ] Framework-specific patterns supported (criterion, proptest)
- [ ] Mock usage tracked and excessive mocking flagged
- [ ] Test organization issues identified
- [ ] Integration with existing Rust analyzer
- [ ] Confidence scoring for each detection
- [ ] 95%+ of tests analyzed correctly
- [ ] Unit tests for all pattern types
- [ ] Documentation includes test quality guide

## Technical Details

### Implementation Approach

1. Create `RustTestQualityAnalyzer` in `src/analyzers/rust/test_quality.rs`
2. Implement test detection and classification
3. Add assertion counting and validation
4. Implement complexity scoring system
5. Add flaky pattern detection
6. Integrate with main Rust analyzer pipeline

### Architecture Changes

```rust
// src/analyzers/rust/test_quality.rs
pub struct RustTestQualityAnalyzer {
    test_registry: HashMap<FunctionId, TestInfo>,
    framework_detector: TestFrameworkDetector,
    flaky_patterns: FlakyPatternRegistry,
    complexity_scorer: TestComplexityScorer,
}

pub struct TestInfo {
    test_type: TestType,
    location: Location,
    assertions: Vec<Assertion>,
    complexity_score: f32,
    flaky_indicators: Vec<FlakyIndicator>,
    isolation_issues: Vec<IsolationIssue>,
    framework: Option<TestFramework>,
}

pub enum TestType {
    UnitTest,
    IntegrationTest,
    BenchmarkTest,
    PropertyTest,
    DocTest,
}

pub enum TestFramework {
    Std,           // Standard #[test]
    Criterion,     // criterion benchmarks
    Proptest,      // proptest property tests
    Quickcheck,    // quickcheck property tests
    Rstest,        // rstest parameterized tests
}

pub struct Assertion {
    assertion_type: AssertionType,
    location: Location,
    complexity: u32,
}

pub enum AssertionType {
    Assert,           // assert!(condition)
    AssertEq,         // assert_eq!(left, right)
    AssertNe,         // assert_ne!(left, right)
    Matches,          // matches!(value, pattern)
    ShouldPanic,      // #[should_panic]
    ResultOk,         // Ok(()) return
    Custom(String),   // Custom assertion macro
}

pub struct FlakyIndicator {
    indicator_type: FlakyType,
    location: Location,
    severity: Severity,
    explanation: String,
}

pub enum FlakyType {
    TimingDependency,
    RandomValue,
    ExternalDependency,
    FileSystemDependency,
    NetworkDependency,
    ThreadingIssue,
    HashOrdering,
}

pub struct IsolationIssue {
    issue_type: IsolationIssueType,
    location: Location,
    explanation: String,
}

pub enum IsolationIssueType {
    SharedMutableState,
    GlobalStateModification,
    StaticVariable,
    FilesystemArtifact,
    DatabaseState,
    ExecutionOrderDependency,
}

pub struct TestComplexityScore {
    total_score: f32,
    factors: ComplexityFactors,
    maintainability_index: f32,
}

pub struct ComplexityFactors {
    conditionals: u32,
    loops: u32,
    assertions: u32,
    setup_complexity: u32,
    nesting_depth: u32,
    line_count: u32,
}

pub struct TestQualityIssue {
    issue_type: TestIssueType,
    severity: Severity,
    confidence: f32,
    location: Location,
    test_name: String,
    explanation: String,
    suggestion: String,
}

pub enum TestIssueType {
    NoAssertions,
    TooComplex,
    FlakyPattern,
    IsolationIssue,
    ExcessiveMocking,
    PoorOrganization,
    TestsTooMuch,
    NoActualTest,
    UnclearFailure,
    BrittleTest,
    SlowTest,
}
```

### Data Structures

```rust
// Flaky pattern definitions
pub const FLAKY_PATTERNS: &[FlakyPattern] = &[
    FlakyPattern {
        name: "thread_sleep",
        pattern: "std::thread::sleep",
        description: "Tests using sleep are timing-dependent and flaky",
        severity: Severity::High,
    },
    FlakyPattern {
        name: "instant_now",
        pattern: "std::time::Instant::now",
        description: "Tests comparing timestamps can be flaky",
        severity: Severity::Medium,
    },
    FlakyPattern {
        name: "rand_usage",
        pattern: "rand::",
        description: "Tests using random values are non-deterministic",
        severity: Severity::High,
    },
    FlakyPattern {
        name: "hashmap_iteration",
        pattern: "HashMap::iter",
        description: "HashMap iteration order is non-deterministic",
        severity: Severity::Medium,
    },
];

// Test complexity thresholds
pub const COMPLEXITY_THRESHOLDS: ComplexityThresholds = ComplexityThresholds {
    simple: 5.0,
    moderate: 10.0,
    complex: 15.0,
    very_complex: 20.0,
};

// Assertion patterns
pub const ASSERTION_PATTERNS: &[&str] = &[
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "assert_matches!",
    "debug_assert!",
    "debug_assert_eq!",
    "debug_assert_ne!",
];
```

### APIs and Interfaces

```rust
impl RustTestQualityAnalyzer {
    pub fn new() -> Self;

    /// Analyze test quality for a function
    pub fn analyze_test(&mut self, func: &syn::ItemFn) -> Option<TestInfo>;

    /// Detect test type from attributes and location
    pub fn detect_test_type(&self, func: &syn::ItemFn, file_path: &Path) -> Option<TestType>;

    /// Count and analyze assertions
    pub fn analyze_assertions(&self, func: &syn::ItemFn) -> Vec<Assertion>;

    /// Calculate test complexity score
    pub fn calculate_complexity(&self, func: &syn::ItemFn) -> TestComplexityScore;

    /// Detect flaky patterns
    pub fn detect_flaky_patterns(&self, func: &syn::ItemFn) -> Vec<FlakyIndicator>;

    /// Detect isolation issues
    pub fn detect_isolation_issues(&self, func: &syn::ItemFn) -> Vec<IsolationIssue>;

    /// Detect framework being used
    pub fn detect_framework(&self, func: &syn::ItemFn) -> Option<TestFramework>;

    /// Generate quality issues
    pub fn generate_issues(&self, test_info: &TestInfo) -> Vec<TestQualityIssue>;
}

// Integration with Rust analyzer
impl RustAnalyzer {
    fn analyze_test_quality(&mut self, tree: &syn::File) -> Vec<TestQualityIssue> {
        let mut analyzer = RustTestQualityAnalyzer::new();
        // ... analysis logic
    }
}
```

### Test Pattern Detection Examples

```rust
// Pattern 1: Test with no assertions
#[test]
fn test_no_assertions() {
    // DETECTED: NoAssertions
    let value = create_value();
    process_value(value);
    // Missing: assert! or assertion
}

// Pattern 2: Overly complex test
#[test]
fn test_too_complex() {
    // DETECTED: TooComplex (complexity score > 15)
    let mut data = setup_complex_data();

    for item in data.iter_mut() {
        if item.is_valid() {
            match item.process() {
                Ok(result) => {
                    if result.needs_update() {
                        for field in result.fields() {
                            assert_eq!(field.value(), expected_value(field.id()));
                        }
                    }
                }
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    }
}

// Pattern 3: Flaky test with timing dependency
#[test]
fn test_with_sleep() {
    // DETECTED: FlakyPattern (TimingDependency)
    start_async_operation();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let result = check_completion();
    assert!(result.is_complete());
}

// Pattern 4: Flaky test with random values
#[test]
fn test_with_random() {
    use rand::Rng;
    // DETECTED: FlakyPattern (RandomValue)
    let value = rand::thread_rng().gen_range(0..100);
    assert!(process(value).is_ok());
}

// Pattern 5: Test with shared mutable state
static mut COUNTER: i32 = 0;

#[test]
fn test_with_global_state() {
    // DETECTED: IsolationIssue (GlobalStateModification)
    unsafe {
        COUNTER += 1;
        assert_eq!(COUNTER, 1); // Fails if tests run in parallel
    }
}

// Pattern 6: Proper unit test (no issues)
#[test]
fn test_proper_unit() {
    let value = 42;
    let result = double(value);
    assert_eq!(result, 84);
}

// Pattern 7: Property test (different analysis)
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property(x in 0..1000) {
        assert!(double(x) == x * 2);
    }
}

// Pattern 8: Criterion benchmark
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| my_function(black_box(42)));
    });
}

// Pattern 9: Test with excessive mocking
#[test]
fn test_excessive_mocking() {
    use mockall::predicate::*;

    // DETECTED: ExcessiveMocking (> 5 mocks)
    let mut mock1 = MockService1::new();
    let mut mock2 = MockService2::new();
    let mut mock3 = MockService3::new();
    let mut mock4 = MockService4::new();
    let mut mock5 = MockService5::new();
    let mut mock6 = MockService6::new();

    mock1.expect_call().returning(|| Ok(()));
    // ... etc
}

// Pattern 10: Test with filesystem dependency
#[test]
fn test_filesystem_dependency() {
    // DETECTED: FlakyPattern (FileSystemDependency)
    let path = "/tmp/test_file.txt"; // Hardcoded path
    std::fs::write(path, "test data").unwrap();
    let result = process_file(path);
    assert!(result.is_ok());
    // Missing: cleanup
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/rust.rs` - Main Rust analyzer integration
  - `src/core/debt_item.rs` - Add test quality debt types
  - `src/priority/scoring.rs` - Add test quality scoring
- **External Dependencies**: None (uses existing `syn` crate)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_detect_no_assertions() {
        let code = r#"
#[test]
fn test_something() {
    let value = create_value();
    process_value(value);
}
"#;
        let issues = analyze_test_quality(code);
        assert!(issues.iter().any(|i| matches!(i.issue_type, TestIssueType::NoAssertions)));
    }

    #[test]
    fn test_detect_flaky_sleep() {
        let code = r#"
#[test]
fn test_timing() {
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(true);
}
"#;
        let issues = analyze_test_quality(code);
        assert!(issues.iter().any(|i| matches!(i.issue_type, TestIssueType::FlakyPattern)));
    }

    #[test]
    fn test_complexity_scoring() {
        let code = r#"
#[test]
fn complex_test() {
    for i in 0..10 {
        if i % 2 == 0 {
            for j in 0..5 {
                assert_eq!(i * j, expected(i, j));
            }
        }
    }
}
"#;
        let test_info = analyze_test(code);
        assert!(test_info.complexity_score.total_score > 15.0);
    }

    #[test]
    fn test_proper_test_no_issues() {
        let code = r#"
#[test]
fn proper_test() {
    let value = 42;
    let result = double(value);
    assert_eq!(result, 84);
}
"#;
        let issues = analyze_test_quality(code);
        assert!(issues.is_empty());
    }
}
```

### Integration Tests

1. **Real Rust codebase analysis**:
   - Analyze Rust projects with comprehensive test suites
   - Verify detection of known test quality issues
   - Measure false positive rate

2. **Framework-specific tests**:
   - Test criterion benchmark analysis
   - Test proptest property test validation
   - Test rstest parameterized test analysis

3. **Performance tests**:
   - Analyze large Rust codebase with thousands of tests
   - Measure analysis overhead (target: < 5%)
   - Profile test analysis performance

4. **Accuracy tests**:
   - Test suite of 100 known test quality issues
   - Measure detection rate (target: 95%+)
   - Measure false positive rate (target: < 10%)

## Documentation Requirements

### Code Documentation

- Document each test quality pattern with examples
- Explain complexity scoring algorithm
- Provide guidelines for adding new patterns
- Document framework-specific analysis

### User Documentation

Add to debtmap user guide:

```markdown
## Rust Test Quality Analysis

Debtmap analyzes Rust test quality to identify maintainability issues:

### Test Assertions

Ensure tests have proper assertions:

```rust
// Bad: No assertions
#[test]
fn test_processing() {
    let value = create_value();
    process_value(value);
}

// Good: Clear assertion
#[test]
fn test_processing() {
    let value = create_value();
    let result = process_value(value);
    assert_eq!(result, expected_result());
}
```

### Test Complexity

Keep tests simple and focused:

```rust
// Bad: Too complex
#[test]
fn complex_test() {
    for i in 0..10 {
        if i % 2 == 0 {
            match process(i) {
                Ok(result) => {
                    for item in result {
                        assert!(validate(item));
                    }
                }
                Err(e) => panic!("Error: {}", e),
            }
        }
    }
}

// Good: Simple and focused
#[test]
fn test_even_numbers() {
    let input = vec![0, 2, 4];
    for value in input {
        let result = process(value).unwrap();
        assert!(validate_all(&result));
    }
}
```

### Flaky Test Prevention

Avoid timing and random dependencies:

```rust
// Bad: Timing-dependent
#[test]
fn flaky_test() {
    start_async_operation();
    std::thread::sleep(Duration::from_millis(100));
    assert!(is_complete());
}

// Good: Polling with timeout
#[test]
fn robust_test() {
    start_async_operation();
    wait_until(Duration::from_secs(5), || is_complete());
}

// Bad: Random values
#[test]
fn random_test() {
    let value = rand::thread_rng().gen_range(0..100);
    assert!(process(value).is_ok());
}

// Good: Fixed values or property tests
#[test]
fn deterministic_test() {
    let test_values = vec![0, 50, 99];
    for value in test_values {
        assert!(process(value).is_ok());
    }
}
```

### Configuration

Control test quality analysis:

```toml
[analysis.rust.test_quality]
detect_no_assertions = true
detect_complex_tests = true
detect_flaky_patterns = true
detect_isolation_issues = true
complexity_threshold = 15.0
min_confidence = 0.7

# Flaky pattern detection
detect_timing_dependencies = true
detect_random_values = true
detect_filesystem_dependencies = true
detect_network_dependencies = true

# Framework-specific
analyze_criterion_benchmarks = true
analyze_property_tests = true
```
```

### Architecture Updates

Update ARCHITECTURE.md:
- Add test quality analysis to Rust analyzer section
- Document complexity scoring algorithm
- Explain flaky pattern detection
- Add diagram showing test analysis flow

## Implementation Notes

### Phase 1: Basic Detection (Week 1)
- Implement test type detection
- Add assertion counting
- Create basic complexity scoring
- Unit tests for core functionality

### Phase 2: Flaky Patterns (Week 2)
- Implement flaky pattern detection
- Add timing dependency detection
- Track random value usage
- Integration tests for flaky detection

### Phase 3: Advanced Analysis (Week 3)
- Add isolation issue detection
- Implement framework-specific analysis
- Add test organization analysis
- Performance optimization

### Phase 4: Integration (Week 4)
- Integrate with main Rust analyzer
- Add to debt item generation
- Update scoring system
- Documentation and examples

### Complexity Calculation

```rust
fn calculate_test_complexity(func: &syn::ItemFn) -> TestComplexityScore {
    let mut score = 0.0;
    let mut factors = ComplexityFactors::default();

    // Count conditionals
    factors.conditionals = count_conditionals(func);
    score += factors.conditionals as f32 * 2.0;

    // Count loops
    factors.loops = count_loops(func);
    score += factors.loops as f32 * 3.0;

    // Count assertions (penalty for > 5)
    factors.assertions = count_assertions(func);
    if factors.assertions > 5 {
        score += (factors.assertions - 5) as f32;
    }

    // Nesting depth (penalty for > 2)
    factors.nesting_depth = calculate_nesting_depth(func);
    if factors.nesting_depth > 2 {
        score += (factors.nesting_depth - 2) as f32 * 2.0;
    }

    // Line count (penalty for > 30)
    factors.line_count = count_lines(func);
    if factors.line_count > 30 {
        score += ((factors.line_count - 30) as f32) / 10.0;
    }

    // Maintainability index (inverse of complexity)
    let maintainability_index = 100.0 - (score * 2.0).min(100.0);

    TestComplexityScore {
        total_score: score,
        factors,
        maintainability_index,
    }
}
```

### Framework Detection

```rust
fn detect_test_framework(func: &syn::ItemFn) -> Option<TestFramework> {
    // Check attributes
    for attr in &func.attrs {
        if attr.path().is_ident("test") {
            return Some(TestFramework::Std);
        }
        if attr.path().is_ident("bench") {
            return Some(TestFramework::Criterion);
        }
    }

    // Check function signature for framework-specific patterns
    if has_criterion_parameter(func) {
        return Some(TestFramework::Criterion);
    }

    // Check for proptest macro
    if has_proptest_macro(func) {
        return Some(TestFramework::Proptest);
    }

    None
}
```

## Migration and Compatibility

### Backward Compatibility

- No breaking changes to existing Rust analysis
- New debt items are additive
- Existing JSON output remains compatible
- Can be disabled via configuration

### Configuration Options

```toml
[analysis.rust]
enable_test_quality_analysis = true

[analysis.rust.test_quality]
detect_no_assertions = true
detect_complex_tests = true
detect_flaky_patterns = true
detect_isolation_issues = true
detect_excessive_mocking = true
complexity_threshold = 15.0
min_confidence = 0.7

# Framework analysis
analyze_criterion = true
analyze_proptest = true
analyze_rstest = true

# Ignore rules
ignore_integration_tests = false
ignore_benchmark_tests = false
min_assertions = 1
```

### Migration Path

1. **Default disabled**: Initial release with feature flag
2. **Opt-in period**: Users enable via configuration
3. **Feedback period**: Adjust thresholds based on feedback
4. **Gradual rollout**: Enable by default after validation

## Success Metrics

- **Detection rate**: 95%+ of test quality issues detected
- **False positive rate**: < 10%
- **Performance overhead**: < 5% on Rust analysis time
- **User adoption**: 50%+ of Rust projects enable analysis
- **Test quality improvement**: 30% reduction in test-related issues after 3 months

## Future Enhancements

1. **Test coverage integration**: Correlate quality with coverage
2. **Mutation testing**: Identify tests that don't catch mutations
3. **Test generation**: Suggest additional test cases
4. **Performance regression detection**: Track test execution time
5. **Test dependency analysis**: Identify test coupling
6. **Custom quality rules**: Allow project-specific quality criteria
7. **IDE integration**: Real-time test quality feedback

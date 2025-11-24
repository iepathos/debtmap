---
number: 200
title: Testing Infrastructure with MockEnv
category: testing
priority: high
status: draft
dependencies: [195, 196, 197, 198, 199]
created: 2025-11-24
---

# Specification 200: Testing Infrastructure with MockEnv

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Specs 195-199 (Complete Stillwater Integration)

## Context

With Specs 195-199 complete, debtmap now has:
- Pure functions for core calculations (Spec 196)
- Error accumulation with Validation (Spec 197)
- Effect composition for I/O (Spec 198)
- Reader pattern for configuration (Spec 199)

However, testing infrastructure hasn't caught up. Current tests:
- Use TempDir extensively (slow, brittle)
- Require real file I/O (50-100ms per test)
- Need complex mocking setups
- Can't easily test error scenarios
- Hard to test parallel operations

Stillwater provides `MockEnv` for testing effects without real I/O, and assertion macros for clearer test code. This specification completes the integration by modernizing debtmap's test infrastructure to leverage all stillwater capabilities.

## Objective

Implement comprehensive testing infrastructure using stillwater's MockEnv and assertion macros, achieving 10-100x faster tests while improving test clarity and coverage.

## Requirements

### Functional Requirements

#### MockEnv Builder
- Fluent API for setting up test environments
- Mock file system with in-memory files
- Mock coverage data without real files
- Mock cache without real storage
- Mock config without files

#### Assertion Macros
- `assert_success!(result)` for Effect success
- `assert_failure!(result)` for Effect failure
- `assert_validation_errors!(validation, count)` for validation errors
- `assert_contains_error!(result, pattern)` for error content

#### Test Helpers
- `parse_test_code!(code)` for inline Rust code
- `create_test_ast(complexity)` for synthetic ASTs
- `create_test_config(overrides)` for test configs
- `create_test_coverage(percentage)` for mock coverage

#### Property-Based Testing
- Generators for ASTs, configs, and inputs
- Properties for pure functions
- Shrinking for minimal failing cases

### Non-Functional Requirements
- Tests 10-100x faster than current
- Test coverage 90%+ on pure functions
- No brittleness from temp files
- Clear, readable test code

## Acceptance Criteria

- [ ] `MockEnv` builder implemented with fluent API
- [ ] Assertion macros available in tests
- [ ] Test helpers for common setups
- [ ] Property-based tests for pure functions
- [ ] Test suite runs in < 10s (currently ~60s)
- [ ] 90%+ coverage on pure functions
- [ ] No TempDir usage in unit tests
- [ ] Documentation with testing examples
- [ ] Migration guide for existing tests

## Technical Details

### Implementation Approach

#### 1. MockEnv Builder

```rust
// src/testing/mock_env.rs

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Builder for mock test environments
#[derive(Clone)]
pub struct MockEnv {
    files: Arc<Mutex<HashMap<PathBuf, String>>>,
    coverage: Arc<Mutex<HashMap<PathBuf, Coverage>>>,
    cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    config: Config,
}

impl MockEnv {
    /// Create new mock environment
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            coverage: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(Mutex::new(HashMap::new())),
            config: Config::default(),
        }
    }

    /// Add file to mock file system
    pub fn with_file(self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.lock().unwrap().insert(path.into(), content.into());
        self
    }

    /// Add multiple files at once
    pub fn with_files(mut self, files: Vec<(&str, &str)>) -> Self {
        for (path, content) in files {
            self = self.with_file(path, content);
        }
        self
    }

    /// Add coverage data
    pub fn with_coverage(self, path: impl Into<PathBuf>, coverage: Coverage) -> Self {
        self.coverage.lock().unwrap().insert(path.into(), coverage);
        self
    }

    /// Set mock configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Add cache entry
    pub fn with_cache_entry<T: Serialize>(
        self,
        key: impl Into<String>,
        value: &T,
    ) -> Self {
        let serialized = bincode::serialize(value).unwrap();
        self.cache.lock().unwrap().insert(key.into(), serialized);
        self
    }
}

impl AnalysisEnv for MockEnv {
    fn file_system(&self) -> &dyn FileSystem {
        self
    }

    fn coverage_loader(&self) -> &dyn CoverageLoader {
        self
    }

    fn cache(&self) -> &dyn Cache {
        self
    }

    fn config(&self) -> &Config {
        &self.config
    }
}

impl FileSystem for MockEnv {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found: {}", path.display()))
    }

    fn write(&self, path: &Path, content: &str) -> Result<()> {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.lock().unwrap().contains_key(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }
}

impl CoverageLoader for MockEnv {
    fn load_lcov(&self, path: &Path) -> Result<Coverage> {
        self.coverage
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No coverage for: {}", path.display()))
    }

    fn load_cobertura(&self, path: &Path) -> Result<Coverage> {
        self.load_lcov(path)  // Use same mock data
    }
}

impl Cache for MockEnv {
    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.cache
            .lock()
            .unwrap()
            .get(key)
            .and_then(|bytes| bincode::deserialize(bytes).ok())
    }

    fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = bincode::serialize(value)?;
        self.cache.lock().unwrap().insert(key.to_string(), serialized);
        Ok(())
    }

    fn invalidate(&self, key: &str) -> Result<()> {
        self.cache.lock().unwrap().remove(key);
        Ok(())
    }
}
```

#### 2. Assertion Macros

```rust
// src/testing/assertions.rs

/// Assert that an Effect succeeds
#[macro_export]
macro_rules! assert_success {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => panic!("Expected success, got error: {}", e),
        }
    };
}

/// Assert that an Effect fails
#[macro_export]
macro_rules! assert_failure {
    ($result:expr) => {
        match $result {
            Ok(value) => panic!("Expected failure, got success: {:?}", value),
            Err(e) => e,
        }
    };
}

/// Assert validation has specific number of errors
#[macro_export]
macro_rules! assert_validation_errors {
    ($validation:expr, $count:expr) => {
        match $validation {
            Validation::Success(_) => {
                panic!("Expected {} validation errors, got success", $count)
            }
            Validation::Failure(errors) => {
                assert_eq!(
                    errors.len(),
                    $count,
                    "Expected {} errors, got {}: {:?}",
                    $count,
                    errors.len(),
                    errors
                );
                errors
            }
        }
    };
}

/// Assert error contains specific message
#[macro_export]
macro_rules! assert_contains_error {
    ($result:expr, $pattern:expr) => {
        let err = assert_failure!($result);
        assert!(
            err.to_string().contains($pattern),
            "Error '{}' does not contain '{}'",
            err,
            $pattern
        );
    };
}
```

#### 3. Test Helpers

```rust
// src/testing/helpers.rs

/// Parse inline Rust code for testing
pub fn parse_test_code(code: &str) -> syn::File {
    syn::parse_str(code).expect("Failed to parse test code")
}

/// Create test AST with specific complexity
pub fn create_test_ast(if_count: u32) -> syn::File {
    let mut code = "fn test_function() {\n".to_string();

    for i in 0..if_count {
        code.push_str(&format!("    if x{} {{ }} \n", i));
    }

    code.push_str("}\n");

    parse_test_code(&code)
}

/// Create test config with overrides
pub fn create_test_config() -> ConfigBuilder {
    ConfigBuilder::default()
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn default() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn complexity_threshold(mut self, threshold: f64) -> Self {
        self.config.thresholds.complexity = threshold;
        self
    }

    pub fn coverage_threshold(mut self, threshold: f64) -> Self {
        self.config.thresholds.coverage = threshold;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

/// Create mock coverage data
pub fn create_test_coverage(lines: usize, hits: usize) -> Coverage {
    Coverage {
        lines,
        hits,
        percentage: (hits as f64 / lines as f64) * 100.0,
    }
}

/// Create realistic project structure for testing
pub fn create_test_project() -> MockEnv {
    MockEnv::new()
        .with_files(vec![
            ("src/main.rs", "fn main() { println!(\"Hello\"); }"),
            ("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
            ("src/utils.rs", "pub fn helper() { /* ... */ }"),
            ("tests/integration_test.rs", "#[test] fn test_main() { }"),
        ])
        .with_coverage("src/main.rs", create_test_coverage(10, 8))
        .with_coverage("src/lib.rs", create_test_coverage(20, 20))
        .with_config(Config::default())
}
```

#### 4. Property-Based Testing

```rust
// src/testing/proptest.rs

use proptest::prelude::*;

/// Generate random ASTs for testing
pub fn any_ast() -> impl Strategy<Value = syn::File> {
    any::<u32>()
        .prop_map(|complexity| {
            create_test_ast(complexity % 20)  // Limit complexity
        })
}

/// Generate random configs
pub fn any_config() -> impl Strategy<Value = Config> {
    (1.0f64..100.0, 0.0f64..100.0, 1u32..10)
        .prop_map(|(complexity, coverage, depth)| Config {
            thresholds: Thresholds {
                complexity,
                coverage,
                depth,
            },
            ..Config::default()
        })
}

/// Generate random coverage data
pub fn any_coverage() -> impl Strategy<Value = Coverage> {
    (1usize..1000, 0usize..1000)
        .prop_filter("hits <= lines", |(lines, hits)| hits <= lines)
        .prop_map(|(lines, hits)| create_test_coverage(lines, hits))
}

// Example property-based tests
proptest! {
    #[test]
    fn complexity_monotonic(ast in any_ast()) {
        let complexity = calculate_cyclomatic_pure(&ast);
        // Property: complexity is always positive
        prop_assert!(complexity > 0);
    }

    #[test]
    fn coverage_percentage_valid(coverage in any_coverage()) {
        // Property: percentage is always 0-100
        prop_assert!(coverage.percentage >= 0.0);
        prop_assert!(coverage.percentage <= 100.0);
    }

    #[test]
    fn scoring_deterministic(
        complexity in 1u32..100,
        coverage in any_coverage(),
        config in any_config(),
    ) {
        let ast = create_test_ast(complexity);
        let score1 = calculate_score_pure(&ast, &coverage, &config);
        let score2 = calculate_score_pure(&ast, &coverage, &config);

        // Property: same inputs = same output
        prop_assert_eq!(score1, score2);
    }
}
```

### Example Test Migrations

#### Before: Slow Integration Test

```rust
#[test]
fn test_analyze_file() {
    // Setup: Create temp files (slow)
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    fs::write(&test_file, r#"
        fn example() {
            if true {
                while false {
                    println!("test");
                }
            }
        }
    "#).unwrap();

    // Create config file
    let config_file = temp_dir.path().join("config.toml");
    fs::write(&config_file, "complexity = 10.0").unwrap();

    // Run analysis
    let config = load_config(&config_file).unwrap();
    let result = analyze_file(&test_file, &config).unwrap();

    // Verify
    assert_eq!(result.complexity, 3);
}
// Runtime: ~50ms (file I/O overhead)
```

#### After: Fast Unit Test with MockEnv

```rust
#[test]
fn test_analyze_file() {
    // Setup: Pure in-memory environment (instant)
    let env = MockEnv::new()
        .with_file("test.rs", r#"
            fn example() {
                if true {
                    while false {
                        println!("test");
                    }
                }
            }
        "#)
        .with_config(
            create_test_config()
                .complexity_threshold(10.0)
                .build()
        );

    // Run analysis
    let result = analyze_file_effect("test.rs".into())
        .run(&env);

    // Verify with assertion macro
    let analysis = assert_success!(result);
    assert_eq!(analysis.complexity, 3);
}
// Runtime: ~0.5ms (100x faster, no I/O)
```

#### Property-Based Test

```rust
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    fn complexity_increases_with_branches(branches in 0u32..50) {
        // Property: More branches = higher or equal complexity
        let ast = create_test_ast(branches);
        let complexity = calculate_cyclomatic_pure(&ast);

        prop_assert!(complexity >= branches);
    }

    #[test]
    fn score_bounded(
        ast in any_ast(),
        coverage in any_coverage(),
        config in any_config(),
    ) {
        // Property: Score is always in valid range
        let score = calculate_score_pure(&ast, &coverage, &config);

        prop_assert!(score >= 0.0);
        prop_assert!(score <= 100.0);
    }
}
```

## Dependencies

- **Prerequisites**: Specs 195-199 (Complete stillwater integration)
- **Completes**: Stillwater integration series
- **Benefits**: All future tests

## Testing Strategy

- **Test the tests**: Meta-tests for MockEnv behavior
- **Migration tests**: Verify migrated tests produce same results
- **Performance tests**: Measure speedup (target: 10-100x)
- **Coverage tests**: Achieve 90%+ on pure functions

## Documentation Requirements

- **Testing Guide**: Comprehensive testing documentation
- **Examples**: 10+ example tests showing patterns
- **Migration Guide**: How to migrate existing tests
- **Cheat Sheet**: Quick reference for common patterns

## Implementation Notes

### Files to Create
- `src/testing/mock_env.rs` - MockEnv builder
- `src/testing/assertions.rs` - Assertion macros
- `src/testing/helpers.rs` - Test helpers
- `src/testing/proptest.rs` - Property-based testing

### Files to Modify
- Migrate 100+ existing tests to use MockEnv
- Remove TempDir usage from unit tests
- Add property-based tests for pure functions

### Estimated Effort
- MockEnv implementation: 6-8 hours
- Assertion macros: 2-3 hours
- Test helpers: 4-6 hours
- Property-based setup: 3-4 hours
- Test migration: 12-16 hours
- Documentation: 4-6 hours
- **Total: 31-43 hours**

## Success Metrics

- **Speed**: Test suite < 10s (currently ~60s) = 6x faster
- **Coverage**: 90%+ on pure functions (currently ~75%)
- **Reliability**: 0 flaky tests (currently 2-3)
- **Readability**: Clear, concise test code

## Migration and Compatibility

### Gradual Migration
- New tests use MockEnv immediately
- Migrate existing tests incrementally
- Keep integration tests for end-to-end validation
- No breaking changes

### Priority for Migration
1. Complexity calculation tests (highest value)
2. Validation tests
3. Scoring tests
4. End-to-end tests (keep some as integration tests)

## Completion

This spec completes the stillwater integration series (Specs 195-200), bringing modern functional programming patterns, comprehensive error handling, and world-class testing infrastructure to debtmap.

**Total estimated effort for complete integration**: ~150-180 hours
**Expected benefits**:
- 10-100x faster tests
- Better error messages (show all errors at once)
- Clearer architecture (pure core, imperative shell)
- Easier maintenance (pure functions, clear dependencies)
- Higher code quality (easier to test = more tests)

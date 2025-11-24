---
number: 198
title: Effect Composition for I/O Operations
category: foundation
priority: high
status: draft
dependencies: [195, 196, 197]
created: 2025-11-24
---

# Specification 198: Effect Composition for I/O Operations

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 195 (Foundation), Spec 196 (Pure Functions), Spec 197 (Validation)

## Context

Debtmap's I/O operations (file reading, coverage loading, cache access) are currently interleaved with business logic throughout the codebase, making testing difficult and requiring extensive mocking infrastructure. Functions that should be pure calculations depend on file system access, making them slow to test and hard to reason about.

With Specs 195-197 complete, we now have:
- Environment trait for dependency injection
- Pure functions for core calculations
- Validation for error accumulation

This specification wraps all I/O operations in `Effect` types, enabling:
- **Testability**: Mock entire environment instead of individual operations
- **Composition**: Chain I/O operations with clear data flow
- **Context**: Preserve error context through call chains
- **Fallbacks**: Implement retry and fallback strategies easily

## Objective

Wrap all I/O operations (file system, coverage loading, cache) in Effect types, enabling pure functional composition while maintaining testability and backwards compatibility.

## Requirements

### Functional Requirements

#### File Operations
- Wrap `fs::read_to_string` in `IO::read_file()` effect
- Wrap `fs::write` in `IO::write_file()` effect
- Wrap directory walking in `IO::walk_dir()` effect
- Support file existence checks as effects

#### Coverage Operations
- Wrap LCOV parsing in `IO::load_lcov()` effect
- Wrap Cobertura parsing in `IO::load_cobertura()` effect
- Support multiple coverage format fallbacks with `Effect::race`
- Preserve error context for coverage failures

#### Cache Operations
- Wrap cache reads in `IO::cache_get()` effect
- Wrap cache writes in `IO::cache_set()` effect
- Support cache invalidation as effect
- Enable cache-optional mode easily

#### Effect Composition
- Chain operations with `.and_then()`
- Add context with `.context()`
- Handle errors with `.map_err()`
- Compose parallel operations with `Effect::par_all()`

### Non-Functional Requirements
- Zero runtime overhead vs direct I/O
- Backwards-compatible wrappers
- All tests pass with MockEnv
- Clear error messages with full context

## Acceptance Criteria

- [ ] `src/io/effects.rs` created with all I/O effects
- [ ] File operations wrapped (read, write, walk)
- [ ] Coverage operations wrapped (LCOV, Cobertura)
- [ ] Cache operations wrapped (get, set, invalidate)
- [ ] `Effect::race` used for coverage fallbacks
- [ ] `Effect::par_all` used for parallel file processing
- [ ] Context added at each pipeline stage
- [ ] MockEnv supports all I/O operations
- [ ] Backwards-compatible wrappers maintained
- [ ] Integration tests use MockEnv
- [ ] Performance benchmarks show no regression

## Technical Details

### Implementation Approach

#### 1. File I/O Effects

```rust
// src/io/effects.rs
use stillwater::{Effect, IO};

/// Read file contents as Effect
pub fn read_file_effect(path: PathBuf) -> AnalysisEffect<String> {
    Effect::new(move |env| {
        env.file_system()
            .read_to_string(&path)
            .map_err(|e| AnalysisError::IoError(
                format!("Failed to read {}: {}", path.display(), e)
            ))
    })
    .context(format!("Reading file: {}", path.display()))
}

/// Write file contents as Effect
pub fn write_file_effect(path: PathBuf, content: String) -> AnalysisEffect<()> {
    Effect::new(move |env| {
        env.file_system()
            .write(&path, &content)
            .map_err(|e| AnalysisError::IoError(
                format!("Failed to write {}: {}", path.display(), e)
            ))
    })
    .context(format!("Writing file: {}", path.display()))
}

/// Walk directory as Effect
pub fn walk_dir_effect(path: PathBuf) -> AnalysisEffect<Vec<PathBuf>> {
    Effect::new(move |env| {
        env.file_system()
            .walk_dir(&path)
            .map_err(|e| AnalysisError::IoError(
                format!("Failed to walk {}: {}", path.display(), e)
            ))
    })
    .context(format!("Walking directory: {}", path.display()))
}

/// Check file existence as Effect
pub fn file_exists_effect(path: PathBuf) -> AnalysisEffect<bool> {
    Effect::new(move |env| {
        Ok(env.file_system().exists(&path))
    })
}
```

#### 2. Coverage Loading Effects

```rust
// src/risk/effects.rs

/// Load coverage with fallback strategies
pub fn load_coverage_effect(
    primary_path: PathBuf,
    project_root: PathBuf,
) -> AnalysisEffect<Coverage> {
    // Try multiple strategies in parallel, use first success
    Effect::race(vec![
        // Strategy 1: User-specified path
        load_lcov_effect(primary_path.clone()),

        // Strategy 2: Default cargo-llvm-cov location
        load_lcov_effect(
            project_root.join("target/llvm-cov-target/debug/coverage/lcov.info")
        ),

        // Strategy 3: Cargo-tarpaulin location
        load_cobertura_effect(
            project_root.join("target/coverage/cobertura.xml")
        ),
    ])
    .context("Loading coverage data from multiple locations")
}

fn load_lcov_effect(path: PathBuf) -> AnalysisEffect<Coverage> {
    read_file_effect(path.clone())
        .and_then(|content| {
            Effect::from_result(
                parse_lcov(&content)
                    .map_err(|e| AnalysisError::CoverageError(
                        format!("Failed to parse LCOV: {}", e)
                    ))
            )
        })
        .context(format!("Loading LCOV from {}", path.display()))
}

fn load_cobertura_effect(path: PathBuf) -> AnalysisEffect<Coverage> {
    read_file_effect(path.clone())
        .and_then(|content| {
            Effect::from_result(
                parse_cobertura(&content)
                    .map_err(|e| AnalysisError::CoverageError(
                        format!("Failed to parse Cobertura: {}", e)
                    ))
            )
        })
        .context(format!("Loading Cobertura from {}", path.display()))
}
```

#### 3. Cache Effects

```rust
// src/cache/effects.rs

/// Get value from cache as Effect
pub fn cache_get_effect<T>(key: String) -> AnalysisEffect<Option<T>>
where
    T: DeserializeOwned + Send + 'static,
{
    Effect::new(move |env| {
        Ok(env.cache().get(&key))
    })
    .context(format!("Reading from cache: {}", key))
}

/// Set value in cache as Effect
pub fn cache_set_effect<T>(key: String, value: T) -> AnalysisEffect<()>
where
    T: Serialize + Send + 'static,
{
    Effect::new(move |env| {
        env.cache()
            .set(&key, &value)
            .map_err(|e| AnalysisError::Other(
                format!("Cache write failed: {}", e)
            ))
    })
    .context(format!("Writing to cache: {}", key))
}

/// Invalidate cache entry as Effect
pub fn cache_invalidate_effect(key: String) -> AnalysisEffect<()> {
    Effect::new(move |env| {
        env.cache()
            .invalidate(&key)
            .map_err(|e| AnalysisError::Other(
                format!("Cache invalidation failed: {}", e)
            ))
    })
    .context(format!("Invalidating cache: {}", key))
}
```

#### 4. Composed Analysis Pipeline

```rust
// src/builders/effect_pipeline.rs

/// Full analysis pipeline using Effect composition
pub fn analyze_file_pipeline(
    path: PathBuf,
) -> AnalysisEffect<FileAnalysis> {
    // Read file
    read_file_effect(path.clone())
        .context("Reading source file")

        // Parse AST
        .and_then(|content| {
            Effect::from_result(
                syn::parse_file(&content)
                    .map_err(|e| AnalysisError::ParseError(e.to_string()))
            )
        })
        .context("Parsing Rust syntax")

        // Calculate complexity (pure function)
        .map(|ast| {
            let complexity = calculate_cyclomatic_pure(&ast);
            let cognitive = calculate_cognitive_pure(&ast);
            let patterns = detect_patterns_pure(&ast);
            (ast, complexity, cognitive, patterns)
        })
        .context("Analyzing complexity")

        // Load coverage
        .and_then(|(ast, complexity, cognitive, patterns)| {
            load_coverage_effect(path.clone(), PathBuf::from("."))
                .map(move |coverage| (ast, complexity, cognitive, patterns, coverage))
        })
        .context("Loading coverage data")

        // Calculate scores (pure function)
        .map(|(ast, complexity, cognitive, patterns, coverage)| {
            let score = calculate_score_pure(complexity, cognitive, &coverage);
            FileAnalysis {
                path: path.clone(),
                complexity,
                cognitive,
                patterns,
                coverage,
                score,
            }
        })
        .context("Calculating final score")

        // Cache result
        .and_then(|analysis| {
            let cache_key = format!("analysis:{}", path.display());
            cache_set_effect(cache_key, analysis.clone())
                .map(|_| analysis)
        })
        .context("Caching analysis results")
}

/// Analyze multiple files in parallel
pub fn analyze_files_parallel(
    files: Vec<PathBuf>,
) -> AnalysisEffect<Vec<FileAnalysis>> {
    let effects: Vec<AnalysisEffect<FileAnalysis>> = files
        .into_iter()
        .map(analyze_file_pipeline)
        .collect();

    Effect::par_all(effects)
        .context("Analyzing files in parallel")
}
```

#### 5. Backwards-Compatible Wrappers

```rust
// src/builders/mod.rs

/// Backwards-compatible analysis function
pub fn analyze_file(
    path: &Path,
    config: &Config,
) -> anyhow::Result<FileAnalysis> {
    let env = RealEnv::new(config.clone());

    analyze_file_pipeline(path.to_path_buf())
        .run(&env)
        .map_err(Into::into)
}

/// Backwards-compatible multi-file analysis
pub fn analyze_files(
    files: &[PathBuf],
    config: &Config,
) -> anyhow::Result<Vec<FileAnalysis>> {
    let env = RealEnv::new(config.clone());

    analyze_files_parallel(files.to_vec())
        .run(&env)
        .map_err(Into::into)
}
```

### Testing with MockEnv

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stillwater::MockEnv;

    #[test]
    fn test_analysis_pipeline_with_mock() {
        // Setup mock environment
        let env = MockEnv::new()
            .with_file("src/test.rs", r#"
                fn example() {
                    if true {
                        while false {
                            println!("test");
                        }
                    }
                }
            "#)
            .with_coverage("src/test.rs", Coverage::new(10, 8))
            .with_cache(HashMap::new());

        // Run pipeline
        let result = analyze_file_pipeline("src/test.rs".into())
            .run(&env)
            .unwrap();

        // Verify results
        assert_eq!(result.complexity, 3);
        assert_eq!(result.coverage.percentage, 80.0);
        assert!(result.score > 5.0);
    }

    #[test]
    fn test_coverage_fallback() {
        // Primary path doesn't exist, fallback should work
        let env = MockEnv::new()
            .with_coverage(
                "target/llvm-cov-target/debug/coverage/lcov.info",
                Coverage::new(100, 80)
            );

        let result = load_coverage_effect(
            "nonexistent.info".into(),
            ".".into()
        )
        .run(&env)
        .unwrap();

        assert_eq!(result.percentage, 80.0);
    }

    #[test]
    fn test_parallel_analysis() {
        let env = MockEnv::new()
            .with_file("src/file1.rs", "fn foo() {}")
            .with_file("src/file2.rs", "fn bar() {}")
            .with_file("src/file3.rs", "fn baz() {}");

        let files = vec![
            "src/file1.rs".into(),
            "src/file2.rs".into(),
            "src/file3.rs".into(),
        ];

        let result = analyze_files_parallel(files)
            .run(&env)
            .unwrap();

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_error_context_preserved() {
        let env = MockEnv::new();  // No files

        let result = analyze_file_pipeline("nonexistent.rs".into())
            .run(&env);

        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("Reading source file"));
                assert!(msg.contains("nonexistent.rs"));
            }
            Ok(_) => panic!("Expected error"),
        }
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (Foundation) - Environment and Effect types
  - Spec 196 (Pure Functions) - Pure calculation functions
  - Spec 197 (Validation) - Error accumulation patterns
- **Blocks**:
  - Spec 199 (Reader Pattern) - Uses Effect composition
  - Spec 200 (Testing) - Enables MockEnv testing
- **Affected Components**:
  - `src/io/` - File operations
  - `src/risk/` - Coverage loading
  - `src/cache/` - Cache operations
  - `src/builders/` - Analysis pipelines
  - `tests/` - All tests can use MockEnv

## Testing Strategy

- **Unit Tests**: Test each Effect in isolation with MockEnv
- **Integration Tests**: Test full pipeline with MockEnv
- **Error Handling**: Verify context preservation
- **Performance**: Ensure no regression vs direct I/O

## Documentation Requirements

- **Code Docs**: Explain Effect composition patterns
- **Examples**: Show common Effect usage patterns
- **Architecture**: Document I/O separation

## Implementation Notes

### Files to Create
- `src/io/effects.rs` - File I/O effects
- `src/risk/effects.rs` - Coverage effects
- `src/cache/effects.rs` - Cache effects
- `src/builders/effect_pipeline.rs` - Composed pipelines

### Files to Modify
- `src/builders/mod.rs` - Backwards-compatible wrappers
- `src/lib.rs` - Export new modules

### Estimated Effort
- File effects: 4-6 hours
- Coverage effects: 4-6 hours
- Cache effects: 3-4 hours
- Pipeline composition: 6-8 hours
- Testing: 6-8 hours
- **Total: 23-32 hours**

## Success Metrics

- **Testability**: All I/O mockable via MockEnv
- **Context**: Error messages include full chain
- **Performance**: No regression vs direct I/O
- **Compatibility**: All existing tests pass

## Migration and Compatibility

### Non-Breaking
- New Effect APIs added
- Old APIs maintained
- Gradual migration possible

### Migration Path
1. Add Effect wrappers (this spec)
2. Update builders to use Effects
3. Migrate tests to MockEnv
4. Eventually remove direct I/O calls

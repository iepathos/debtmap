---
number: 203
title: Traverse Pattern for Batch File Analysis
category: optimization
priority: high
status: draft
dependencies: [195, 196, 197, 198]
created: 2025-11-27
---

# Specification 203: Traverse Pattern for Batch File Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Specs 195-198 (stillwater foundation)

## Context

Debtmap currently processes files individually using traditional loop-based patterns with `anyhow::Result`. This approach has several limitations:

1. **Sequential processing** - Files are analyzed one at a time
2. **Fail-fast semantics** - Analysis stops at the first error
3. **Lost parallelism** - No automatic parallel execution
4. **Inconsistent error handling** - Mix of Result and manual error collection

Stillwater provides `traverse` and `traverse_effect` functions that enable:
- **Parallel execution** of independent analyses
- **Error accumulation** with `Validation` for comprehensive error reporting
- **Cleaner composition** of analysis pipelines
- **Consistent patterns** across the codebase

## Objective

Migrate batch file analysis operations to use stillwater's traverse pattern, enabling parallel processing and comprehensive error accumulation while maintaining backwards compatibility with existing APIs.

## Requirements

### Functional Requirements

1. **Parallel File Analysis**
   - Use `traverse_effect` to analyze multiple files concurrently
   - Leverage Rayon integration for CPU-bound analysis tasks
   - Maintain deterministic output ordering regardless of execution order

2. **Error Accumulation**
   - Use `Validation` with `traverse` to collect ALL analysis errors
   - Provide comprehensive error reports showing all issues
   - Support both fail-fast and accumulating modes

3. **Backwards Compatibility**
   - Provide wrapper functions returning `anyhow::Result` for existing callers
   - Support gradual migration without breaking changes
   - Maintain existing API signatures where needed

### Non-Functional Requirements

1. **Performance**
   - Batch analysis should be at least 2x faster on multi-core systems
   - Memory usage should not increase significantly
   - Support configurable parallelism limits

2. **Testability**
   - All new functions should be testable with `DebtmapTestEnv`
   - Unit tests should not require file system access
   - Property tests for error accumulation guarantees

## Acceptance Criteria

- [ ] Create `analyze_files_effect` function using `traverse_effect` for parallel analysis
- [ ] Create `validate_files` function using `traverse` for error accumulation
- [ ] Add parallelism configuration to `DebtmapConfig`
- [ ] Migrate `FileWalker` integration to use effect-based directory walking
- [ ] Add benchmarks comparing sequential vs parallel analysis
- [ ] Existing tests continue to pass
- [ ] New integration tests for parallel analysis
- [ ] Documentation with examples of traverse pattern usage

## Technical Details

### Implementation Approach

#### 1. Parallel Analysis Effect

```rust
use stillwater::traverse::traverse_effect;
use stillwater::effect::prelude::*;

/// Analyze multiple files in parallel using stillwater's traverse_effect.
pub fn analyze_files_effect(
    paths: Vec<PathBuf>,
) -> AnalysisEffect<Vec<FileAnalysisResult>> {
    traverse_effect(paths, |path| analyze_single_file_effect(path))
}

/// Analyze a single file as an Effect.
fn analyze_single_file_effect(path: PathBuf) -> AnalysisEffect<FileAnalysisResult> {
    from_fn(move |env: &RealEnv| {
        let content = env.file_system().read_to_string(&path)?;
        let analyzer = get_analyzer_for_path(&path);
        analyzer.analyze(&path, &content).map_err(Into::into)
    }).boxed()
}
```

#### 2. Validation for Error Accumulation

```rust
use stillwater::traverse::traverse;

/// Validate multiple files, accumulating ALL errors.
pub fn validate_files(
    paths: &[PathBuf],
) -> AnalysisValidation<Vec<ValidatedFile>> {
    traverse(paths.to_vec(), |path| validate_single_file(&path))
}

/// Validate a single file's syntax and structure.
fn validate_single_file(path: &Path) -> AnalysisValidation<ValidatedFile> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return validation_failure(AnalysisError::io_with_path(
            format!("Failed to read: {}", e),
            path,
        )),
    };

    let mut errors = Vec::new();

    // Check syntax
    if let Err(e) = check_syntax(&content, path) {
        errors.push(e);
    }

    // Check structure
    if let Err(e) = check_structure(&content, path) {
        errors.push(e);
    }

    if errors.is_empty() {
        validation_success(ValidatedFile { path: path.to_path_buf(), content })
    } else {
        validation_failures(errors)
    }
}
```

#### 3. Configuration for Parallelism

```rust
// In config/mod.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParallelConfig {
    /// Enable parallel processing (default: true)
    pub enabled: bool,

    /// Maximum concurrent operations (default: num_cpus)
    pub max_concurrency: Option<usize>,

    /// Batch size for chunked processing
    pub batch_size: Option<usize>,
}
```

#### 4. Integration with Existing Analysis Pipeline

```rust
// In analyzers/mod.rs
impl ProjectAnalyzer {
    /// Analyze project using parallel traversal.
    pub fn analyze_project_effect(
        &self,
        project_root: PathBuf,
    ) -> AnalysisEffect<ProjectAnalysis> {
        walk_dir_with_config_effect(project_root.clone(), self.languages.clone())
            .and_then(move |files| {
                analyze_files_effect(files)
                    .map(|results| ProjectAnalysis::from_file_results(results))
            })
            .boxed()
    }
}
```

### Architecture Changes

1. **New Module**: `src/analyzers/batch.rs`
   - Houses traverse-based batch analysis functions
   - Provides both Effect and Validation variants

2. **Modified Module**: `src/analyzers/mod.rs`
   - Add parallel analysis dispatcher
   - Integrate with existing FileAnalyzer trait

3. **Modified Module**: `src/io/effects.rs`
   - Add `walk_and_analyze_effect` composed operation
   - Support chunked processing for large codebases

### Data Structures

```rust
/// Result of analyzing a single file with full context.
#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    pub path: PathBuf,
    pub metrics: FileMetrics,
    pub debt_items: Vec<DebtItem>,
    pub analysis_time: Duration,
}

/// Validated file ready for analysis.
#[derive(Debug, Clone)]
pub struct ValidatedFile {
    pub path: PathBuf,
    pub content: String,
}

/// Configuration for batch analysis operations.
#[derive(Debug, Clone)]
pub struct BatchAnalysisConfig {
    pub parallelism: ParallelConfig,
    pub fail_fast: bool,
    pub collect_timing: bool,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 195 (stillwater foundation)
  - Spec 196 (pure function extraction)
  - Spec 197 (validation error accumulation)
  - Spec 198 (effect composition)

- **Affected Components**:
  - `src/analyzers/mod.rs`
  - `src/analyzers/file_analyzer.rs`
  - `src/io/effects.rs`
  - `src/config/mod.rs`

- **External Dependencies**:
  - stillwater 0.11.0+ (already integrated)

## Testing Strategy

### Unit Tests
- Test `traverse_effect` with mock file system
- Test error accumulation with intentionally failing files
- Test parallelism configuration options

### Integration Tests
- Test full project analysis with real files
- Test error reporting with mixed valid/invalid files
- Test performance with various project sizes

### Performance Tests
- Benchmark sequential vs parallel analysis
- Measure memory usage under parallel load
- Test scalability with large codebases

### User Acceptance
- Verify error messages are comprehensive
- Ensure performance improvement is perceptible
- Validate existing CLI behavior unchanged

## Documentation Requirements

- **Code Documentation**: Comprehensive rustdoc for all new public functions
- **User Documentation**: Update README with parallel analysis examples
- **Architecture Updates**: Update DESIGN.md with traverse pattern documentation

## Implementation Notes

1. **Parallelism Limits**: Use `par_all_limit` from stillwater when analyzing very large codebases to prevent resource exhaustion.

2. **Error Context**: Ensure each file's errors include the file path for easy identification.

3. **Progress Reporting**: Consider adding progress callbacks for long-running batch operations.

4. **Graceful Degradation**: If parallel execution fails, fall back to sequential processing.

## Migration and Compatibility

- **No Breaking Changes**: All changes are additive
- **Gradual Migration**: Existing code continues to work
- **Opt-in Parallelism**: Parallel analysis is opt-in via configuration
- **Result Wrappers**: Provide `_result` variants for backwards compatibility

```rust
// Backwards-compatible wrapper
pub fn analyze_files(paths: Vec<PathBuf>, config: DebtmapConfig) -> anyhow::Result<Vec<FileAnalysisResult>> {
    run_effect(analyze_files_effect(paths), config)
}
```

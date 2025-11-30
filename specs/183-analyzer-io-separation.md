---
number: 183
title: Analyzer I/O Separation (Pure Core Implementation)
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 183: Analyzer I/O Separation (Pure Core Implementation)

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `collect_file_metrics` function in `src/analysis_utils.rs:14-66` violates the Stillwater principle of "Pure Core, Imperative Shell" by mixing multiple concerns in a single function:

1. **I/O**: Reading environment variables (`DEBTMAP_MAX_FILES`)
2. **Side effects**: Printing warnings with `eprintln!`
3. **I/O**: Creating and managing progress bars
4. **Pure computation**: Parallel file analysis
5. **Side effects**: Updating progress bar state

This makes the function difficult to:
- Test (requires mocking env vars, stdout)
- Reason about (what parts are deterministic?)
- Reuse (can't analyze files without progress tracking)
- Compose (tightly coupled to specific I/O operations)

According to the Stillwater philosophy, we should separate:
- **Still Water (Pure Core)**: File analysis logic
- **Streams (Effects)**: Configuration, progress, I/O

## Objective

Refactor `collect_file_metrics` and related analysis utilities to follow the "Pure Core, Imperative Shell" pattern:

```rust
// Pure core - deterministic, testable, reusable
fn determine_files_to_process(files: &[PathBuf], max: Option<usize>) -> &[PathBuf]
fn analyze_files_parallel(files: &[PathBuf]) -> Vec<FileMetrics>

// Effect shell - I/O operations
fn with_progress_tracking<T>(effect: AnalysisEffect<T>) -> AnalysisEffect<T>
fn read_max_files_config() -> Option<usize>
```

Each layer has clear responsibilities:
- **Pure functions**: Take data, return data, no side effects
- **Effect functions**: Manage I/O, use Stillwater Effect system
- **Composition**: Pipeline pure and effectful operations

## Requirements

### Functional Requirements

1. **Pure Analysis Functions**
   - `determine_files_to_process` - Pure slice operation (no I/O)
   - `analyze_files_parallel` - Pure parallel map (operates on data)
   - No environment variable reading
   - No printing or logging
   - No progress bar management
   - Deterministic results

2. **Effect Wrappers**
   - `read_max_files_config` - Reads env var, returns Option<usize>
   - `with_progress_tracking` - Adds progress bar to any effect
   - `warn_file_limit` - Prints warning (explicit side effect)
   - Use Stillwater Effect system for composition

3. **Composable Pipeline**
   - Configuration reading separate from analysis
   - Progress tracking orthogonal to analysis
   - Each component independently testable
   - Clear data flow

4. **Backward Compatibility**
   - Preserve existing behavior
   - Same progress bar appearance
   - Same warning messages
   - Same performance characteristics

### Non-Functional Requirements

1. **Testability**
   - Pure functions unit tested without mocks
   - Effects integration tested
   - Fast tests (no I/O in unit tests)

2. **Performance**
   - No performance regression
   - Maintain parallel processing
   - Same or better throughput

3. **Composability**
   - Can analyze files without progress tracking
   - Can use progress tracking with other operations
   - Can override max files without env vars

## Acceptance Criteria

- [ ] `determine_files_to_process` function created (pure, 5-10 lines)
- [ ] `analyze_files_parallel` function created (pure, 10-15 lines)
- [ ] `read_max_files_config` effect function created (5-10 lines)
- [ ] `with_progress_tracking` effect wrapper created (15-25 lines)
- [ ] `warn_file_limit` effect function created (5-10 lines)
- [ ] Original `collect_file_metrics` refactored to compose these functions
- [ ] Pure functions have no I/O (verified by inspection)
- [ ] All unit tests test pure functions without mocking
- [ ] Integration tests verify full pipeline
- [ ] No performance regression (benchmark within 5%)
- [ ] All existing tests pass
- [ ] No clippy warnings

## Technical Details

### Implementation Approach

**Current Implementation (Mixed):**

```rust
// src/analysis_utils.rs:14-66
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    // I/O: Read environment variable
    let (total_files, files_to_process) = match std::env::var("DEBTMAP_MAX_FILES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(0) => (files.len(), files),
        Some(max_files) => {
            let limited = max_files.min(files.len());
            // Side effect: Print warning
            if files.len() > max_files {
                eprintln!("[WARN] Processing limited to {} files...", max_files);
            }
            (limited, &files[..limited])
        }
        None => (files.len(), files),
    };

    // I/O: Create progress bar
    let progress = ProgressManager::global().map(|pm| {
        let pb = pm.create_bar(total_files as u64, TEMPLATE_FILE_ANALYSIS);
        pb.set_message("Analyzing files");
        pb
    });

    // Pure computation: Parallel analysis
    let results: Vec<FileMetrics> = files_to_process
        .par_iter()
        .progress_with(progress.clone().unwrap_or_else(indicatif::ProgressBar::hidden))
        .filter_map(|path| analyze_single_file(path.as_path()))
        .collect();

    // Side effect: Update progress
    if let Some(pb) = progress {
        pb.finish_with_message(format!("Analyzed {} files", results.len()));
    }

    results
}
```

**Target Implementation (Separated):**

**Pure Core:**

```rust
/// Determines which files to process based on optional limit.
///
/// This is a pure function - given the same inputs, always returns
/// the same output. No I/O, no side effects.
///
/// # Arguments
///
/// * `files` - All available files
/// * `max` - Optional maximum number of files to process
///
/// # Returns
///
/// Slice of files to process (either all files or first `max` files)
pub fn determine_files_to_process(
    files: &[PathBuf],
    max: Option<usize>
) -> &[PathBuf] {
    match max {
        Some(0) | None => files,
        Some(max_files) => {
            let limit = max_files.min(files.len());
            &files[..limit]
        }
    }
}

/// Analyzes files in parallel, returning metrics for each.
///
/// Pure function - operates on in-memory data, no I/O.
/// Uses rayon for parallel processing.
///
/// # Arguments
///
/// * `files` - Files to analyze
///
/// # Returns
///
/// Vector of metrics for successfully analyzed files
pub fn analyze_files_parallel(files: &[PathBuf]) -> Vec<FileMetrics> {
    files
        .par_iter()
        .filter_map(|path| analyze_single_file(path.as_path()))
        .collect()
}

/// Checks if file limit will be applied.
///
/// Pure predicate function.
pub fn should_warn_file_limit(total: usize, max: Option<usize>) -> bool {
    match max {
        Some(limit) if limit > 0 => total > limit,
        _ => false,
    }
}
```

**Effect Shell:**

```rust
/// Reads maximum files configuration from environment.
///
/// Effect function - performs I/O by reading environment variable.
pub fn read_max_files_config() -> Option<usize> {
    std::env::var("DEBTMAP_MAX_FILES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
}

/// Warns user about file limit being applied.
///
/// Effect function - performs side effect (printing to stderr).
pub fn warn_file_limit(total: usize, limit: usize) {
    eprintln!(
        "[WARN] Processing limited to {} of {} files (set DEBTMAP_MAX_FILES=0 to process all)",
        limit, total
    );
}

/// Wraps an effect with progress tracking.
///
/// This is a higher-order effect that adds progress bar management
/// to any analysis effect.
pub fn with_progress_tracking<T>(
    total: usize,
    message: &str,
    effect: impl FnOnce() -> T,
) -> T {
    let progress = ProgressManager::global().map(|pm| {
        let pb = pm.create_bar(total as u64, TEMPLATE_FILE_ANALYSIS);
        pb.set_message(message);
        pb
    });

    let result = effect();

    if let Some(pb) = progress {
        pb.finish_with_message("Complete");
    }

    result
}
```

**Composition (Public API):**

```rust
/// Collects file metrics with configuration and progress tracking.
///
/// This function composes pure and effectful operations:
/// 1. Read configuration (effect)
/// 2. Determine files to process (pure)
/// 3. Warn if limiting (effect, conditional)
/// 4. Analyze files with progress (pure analysis + effect wrapper)
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    // Effect: Read configuration
    let max_files = read_max_files_config();

    // Pure: Determine files to process
    let files_to_process = determine_files_to_process(files, max_files);

    // Effect: Warn if limiting
    if should_warn_file_limit(files.len(), max_files) {
        warn_file_limit(files.len(), files_to_process.len());
    }

    // Pure analysis + Effect wrapper: Progress tracking
    with_progress_tracking(
        files_to_process.len(),
        "Analyzing files",
        || analyze_files_parallel(files_to_process)
    )
}
```

### Alternative: Full Effect System

For maximum purity, use Stillwater Effect system throughout:

```rust
use stillwater::prelude::*;

/// Analyzes files with full effect system.
pub fn collect_file_metrics_effect(files: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>> {
    asks_config(|cfg| cfg.max_files)
        .map(|max| determine_files_to_process(&files, max))
        .and_then(|files_to_process| {
            // Effect: Warn if needed
            if should_warn_file_limit(files.len(), Some(files_to_process.len())) {
                from_fn(|_| {
                    warn_file_limit(files.len(), files_to_process.len());
                    Ok(())
                })
            } else {
                pure(())
            }
            .map(|_| files_to_process)
        })
        .map(|files_to_process| {
            // Pure: Parallel analysis
            analyze_files_parallel(files_to_process)
        })
}
```

### Architecture Changes

**Before:**
```
collect_file_metrics (52 lines, mixed I/O + logic)
  ├─ Read env var (I/O)
  ├─ Determine files (logic)
  ├─ Print warning (I/O)
  ├─ Create progress bar (I/O)
  ├─ Analyze files (pure computation)
  └─ Update progress (I/O)
```

**After (Option 1: Simple Separation):**
```
collect_file_metrics (15 lines, composition)
  ├─ read_max_files_config (5 lines) - Effect
  ├─ determine_files_to_process (8 lines) - Pure
  ├─ should_warn_file_limit (5 lines) - Pure
  ├─ warn_file_limit (5 lines) - Effect
  ├─ analyze_files_parallel (10 lines) - Pure
  └─ with_progress_tracking (20 lines) - Effect wrapper
```

**After (Option 2: Full Effect System):**
```
collect_file_metrics_effect (20 lines, effect pipeline)
  ├─ asks_config - Effect (reads config)
  ├─ determine_files_to_process - Pure (in map)
  ├─ warn_file_limit - Effect (conditional)
  └─ analyze_files_parallel - Pure (in map)
```

### Data Structures

No new data structures needed. Existing types used:

```rust
pub struct FileMetrics { ... }  // Already exists
pub type AnalysisEffect<T> = BoxedEffect<T, AnalysisError, RealEnv>;  // Already exists
```

### APIs and Interfaces

**Public API (backward compatible):**

```rust
// Existing function signature preserved
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics>;
```

**New Public Functions:**

```rust
// Pure functions (easily tested)
pub fn determine_files_to_process(files: &[PathBuf], max: Option<usize>) -> &[PathBuf];
pub fn analyze_files_parallel(files: &[PathBuf]) -> Vec<FileMetrics>;
pub fn should_warn_file_limit(total: usize, max: Option<usize>) -> bool;

// Effect functions (marked clearly)
pub fn read_max_files_config() -> Option<usize>;
pub fn warn_file_limit(total: usize, limit: usize);
pub fn with_progress_tracking<T>(total: usize, message: &str, effect: impl FnOnce() -> T) -> T;

// Optional: Full effect version
pub fn collect_file_metrics_effect(files: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>>;
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analysis_utils.rs` - Primary changes
  - `src/commands/analyze.rs` - May call collect_file_metrics
  - Any tests that verify progress bar behavior
- **External Dependencies**:
  - `stillwater` (already used)
  - `rayon` (already used)
  - `indicatif` (already used for progress bars)

## Testing Strategy

### Unit Tests (Pure Functions)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_files_no_limit() {
        let files = vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")];
        let result = determine_files_to_process(&files, None);

        assert_eq!(result.len(), 3);
        assert_eq!(result, &files[..]);
    }

    #[test]
    fn test_determine_files_with_limit() {
        let files = vec![
            PathBuf::from("a"),
            PathBuf::from("b"),
            PathBuf::from("c"),
            PathBuf::from("d"),
        ];

        let result = determine_files_to_process(&files, Some(2));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], PathBuf::from("a"));
        assert_eq!(result[1], PathBuf::from("b"));
    }

    #[test]
    fn test_determine_files_limit_exceeds_available() {
        let files = vec![PathBuf::from("a"), PathBuf::from("b")];
        let result = determine_files_to_process(&files, Some(10));

        assert_eq!(result.len(), 2);  // Returns all available
    }

    #[test]
    fn test_determine_files_zero_limit() {
        let files = vec![PathBuf::from("a"), PathBuf::from("b")];
        let result = determine_files_to_process(&files, Some(0));

        assert_eq!(result.len(), 2);  // Zero means no limit
    }

    #[test]
    fn test_should_warn_file_limit() {
        assert!(should_warn_file_limit(100, Some(50)));
        assert!(!should_warn_file_limit(50, Some(100)));
        assert!(!should_warn_file_limit(100, None));
        assert!(!should_warn_file_limit(100, Some(0)));
    }

    #[test]
    fn test_analyze_files_parallel_deterministic() {
        let files = vec![
            PathBuf::from("tests/fixtures/simple.rs"),
            PathBuf::from("tests/fixtures/complex.rs"),
        ];

        let result1 = analyze_files_parallel(&files);
        let result2 = analyze_files_parallel(&files);

        // Same files, same results (deterministic)
        assert_eq!(result1.len(), result2.len());
        for (m1, m2) in result1.iter().zip(result2.iter()) {
            assert_eq!(m1.path, m2.path);
            assert_eq!(m1.complexity.cyclomatic, m2.complexity.cyclomatic);
        }
    }
}
```

### Integration Tests (Effects)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_read_max_files_config() {
        std::env::set_var("DEBTMAP_MAX_FILES", "42");

        let result = read_max_files_config();

        assert_eq!(result, Some(42));

        std::env::remove_var("DEBTMAP_MAX_FILES");
    }

    #[test]
    fn test_read_max_files_config_not_set() {
        std::env::remove_var("DEBTMAP_MAX_FILES");

        let result = read_max_files_config();

        assert_eq!(result, None);
    }

    #[test]
    fn test_collect_file_metrics_full_pipeline() {
        let files = vec![
            PathBuf::from("tests/fixtures/simple.rs"),
            PathBuf::from("tests/fixtures/complex.rs"),
        ];

        let result = collect_file_metrics(&files);

        assert!(!result.is_empty());
        assert!(result.len() <= files.len());
    }
}
```

### Performance Tests

```rust
#[bench]
fn bench_analyze_files_parallel(b: &mut Bencher) {
    let files: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("tests/fixtures/file_{}.rs", i)))
        .collect();

    b.iter(|| {
        analyze_files_parallel(&files)
    });
}

#[test]
fn test_no_performance_regression() {
    let files: Vec<PathBuf> = load_test_dataset(1000);

    let start = Instant::now();
    let _result = collect_file_metrics(&files);
    let duration = start.elapsed();

    // Should complete in reasonable time
    assert!(duration < Duration::from_secs(5));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Determines which files to process based on optional limit.
///
/// This is a pure function with no side effects. Given the same inputs,
/// it will always return the same output. This makes it easy to test
/// and reason about.
///
/// # Pure Function Properties
///
/// - No I/O operations
/// - No environment variable access
/// - No printing or logging
/// - Deterministic results
/// - No mutation of inputs
///
/// # Arguments
///
/// * `files` - Slice of all available files
/// * `max` - Optional maximum number of files to process
///   - `None` - Process all files
///   - `Some(0)` - Process all files (zero means no limit)
///   - `Some(n)` - Process first n files
///
/// # Returns
///
/// Slice of files to process. This is a subslice of the input,
/// so no allocation occurs.
///
/// # Examples
///
/// ```
/// let files = vec![path1, path2, path3, path4];
///
/// // Process all files
/// let all = determine_files_to_process(&files, None);
/// assert_eq!(all.len(), 4);
///
/// // Process first 2 files
/// let limited = determine_files_to_process(&files, Some(2));
/// assert_eq!(limited.len(), 2);
/// ```
pub fn determine_files_to_process(
    files: &[PathBuf],
    max: Option<usize>
) -> &[PathBuf] {
    // ...
}
```

### User Documentation

No user-facing documentation changes (internal refactoring).

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Pure Core, Imperative Shell Pattern

Debtmap follows the "Pure Core, Imperative Shell" pattern from the
Stillwater philosophy:

### Analysis Utilities Example

The `collect_file_metrics` function demonstrates this pattern:

**Pure Core (Still Water):**
- `determine_files_to_process` - Pure slicing operation
- `analyze_files_parallel` - Pure parallel map
- `should_warn_file_limit` - Pure predicate

These functions:
- Take data as input
- Return data as output
- Have no side effects
- Are deterministic
- Are easily unit tested

**Imperative Shell (Streams):**
- `read_max_files_config` - Reads environment variable
- `warn_file_limit` - Prints warning message
- `with_progress_tracking` - Manages progress bar

These functions:
- Perform I/O operations
- Have side effects
- Are integration tested
- Wrap pure logic

**Composition:**
The public `collect_file_metrics` function composes these layers:
1. Read configuration (effect)
2. Determine files (pure)
3. Warn if needed (effect)
4. Analyze files (pure)
5. Track progress (effect wrapper)

This separation enables:
- Fast unit tests (pure functions)
- Reusable logic (pure functions work anywhere)
- Clear boundaries (know what has side effects)
- Easy composition (mix and match as needed)
```

## Implementation Notes

### Refactoring Steps

1. **Create pure functions first** (can test immediately)
   - `determine_files_to_process`
   - `analyze_files_parallel`
   - `should_warn_file_limit`

2. **Extract effect functions** (keep existing behavior)
   - `read_max_files_config`
   - `warn_file_limit`
   - `with_progress_tracking`

3. **Refactor main function** to compose
   - Use pure + effect functions
   - Maintain same behavior
   - Keep same API

4. **Add tests**
   - Unit tests for pure functions (no mocks)
   - Integration tests for effects
   - Performance benchmarks

5. **Verify**
   - All tests pass
   - No performance regression
   - Same output format

### Common Pitfalls

1. **Hidden I/O** - Ensure pure functions truly have no I/O
2. **Progress bar coupling** - Keep progress orthogonal to analysis
3. **Performance** - Verify parallel processing still works
4. **Testing** - Don't mock in pure function tests

### Verification Checklist

For each "pure" function:

- [ ] No `std::fs::*` calls
- [ ] No `std::env::*` calls
- [ ] No `println!` / `eprintln!` / `log::*`
- [ ] No `ProgressBar` creation/updates
- [ ] No network calls
- [ ] No database access
- [ ] Returns same output for same input
- [ ] Can unit test in microseconds

## Migration and Compatibility

### Breaking Changes

**None** - Public API unchanged.

### Internal Changes

Other parts of codebase using these functions may benefit from using the new pure functions directly:

```rust
// Before: Had to use full pipeline
let metrics = collect_file_metrics(&files);

// After: Can use pure analysis without I/O
let metrics = analyze_files_parallel(&files);  // No env vars, no progress bar
```

### Migration Steps

No user migration needed. Internal refactoring only.

## Success Metrics

- ✅ 3 pure functions created (no I/O)
- ✅ 3 effect functions created (clear I/O)
- ✅ Pure functions unit tested without mocks
- ✅ All existing tests pass
- ✅ No performance regression
- ✅ Function complexity < 5 for all functions
- ✅ Each function under 20 lines
- ✅ Clear separation of pure/impure code

## Follow-up Work

Apply same pattern to other mixed I/O/logic functions:

- `prepare_files_for_duplication_check` (utils/analysis_helpers.rs)
- Other analysis utility functions
- Command handlers with mixed concerns

## References

- **STILLWATER_EVALUATION.md** - "Pure Core, Imperative Shell" section
- **Stillwater PHILOSOPHY.md** - "The Pond Model" mental model
- **CLAUDE.md** - Functional programming principles
- **src/analysis_utils.rs:14-66** - Current implementation

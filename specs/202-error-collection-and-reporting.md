---
number: 202
title: Error Collection and Reporting
category: foundation
priority: medium
status: draft
dependencies: [201]
created: 2025-12-06
---

# Specification 202: Error Collection and Reporting

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 201 (Error Visibility and Logging)

## Context

After implementing Spec 201 (Error Visibility and Logging), debtmap will log errors as they occur. However, errors are still handled in a fail-fast or ignore pattern - once logged, they're discarded and analysis continues. This creates two problems:

1. **No aggregated error summary** - Users see individual warnings scroll by but get no summary of what failed
2. **No programmatic error access** - Cannot generate error reports or understand error patterns
3. **Inconsistent user experience** - Some files silently skipped, some logged, but final output doesn't reflect what was incomplete

This violates **Stillwater's "Fail Completely" principle** for independent operations:

> **Validation (fail completely)**:
> - Accumulates ALL errors (Applicative)
> - Independent checks run in parallel
> - Example: Form validation

File analysis operations are **independent** - a failure in one file shouldn't prevent analyzing others, but we should collect **all** errors and present them to the user at the end.

### Current State (After Spec 201)

```rust
// Errors are logged but not collected
for file in files {
    let result = analyze_file(&file)
        .map_err(|e| eprintln!("Warning: {}", e))  // ✓ Logged (Spec 201)
        .ok();  // ❌ Still discarded, not collected
}

// User sees:
// Warning: Failed to read file1.rs: Permission denied
// Warning: Failed to parse file2.rs: syntax error
// [... analysis continues ...]
// Analysis complete! (no mention of 2 failures)
```

### Desired State (After Spec 202)

```rust
// Errors are collected AND logged
let (successes, failures): (Vec<_>, Vec<_>) = files
    .into_par_iter()
    .map(|file| analyze_file(&file))
    .partition_result();

// Log failures as they occur (Spec 201)
for failure in &failures {
    eprintln!("Warning: {:#}", failure);
}

// Report summary at end
eprintln!("\nAnalysis Summary:");
eprintln!("  Successfully analyzed: {} files", successes.len());
eprintln!("  Failed to analyze: {} files", failures.len());

if !failures.is_empty() {
    eprintln!("\nFailure breakdown:");
    report_error_summary(&failures);
}

// User sees:
// Warning: Failed to read file1.rs: Permission denied
// Warning: Failed to parse file2.rs: syntax error
// [... analysis continues ...]
//
// Analysis Summary:
//   Successfully analyzed: 98 files
//   Failed to analyze: 2 files
//
// Failure breakdown:
//   Permission errors: 1 file
//   Parse errors: 1 file
```

### Stillwater Philosophy Alignment

From Stillwater PHILOSOPHY.md:

> **Fail Fast vs Fail Completely**
>
> **Use Validation (fail completely) when:**
> - Independent validations that should all be checked
> - Example: Form validation, config validation
>
> **The Validation Funnel:**
> ```
> Input 1 ──→ ✓ or ✗ ─┐
> Input 2 ──→ ✓ or ✗ ─┤
> Input 3 ──→ ✓ or ✗ ─┼─→ All ✓ → Success
> Input 4 ──→ ✓ or ✗ ─┤   Any ✗ → All errors
> Input 5 ──→ ✓ or ✗ ─┘
> ```
>
> Don't stop at first `✗` - collect them all!

File analysis fits this pattern perfectly - each file is an independent input that should be checked, with all errors collected.

## Objective

Implement systematic error collection and reporting for batch operations in debtmap:

1. **Collect errors** during batch operations (file analysis, directory traversal)
2. **Return both successes and failures** instead of just successes
3. **Generate error summaries** showing patterns and frequency
4. **Report completion statistics** showing what succeeded vs failed
5. **Make analysis completeness visible** to users

This is the **systematic error aggregation (Phase 2)** that builds on the visibility from Spec 201.

## Requirements

### Functional Requirements

1. **Error Collection Data Structure**
   - Create `AnalysisResult<T>` type for batch operations
   - Collect both successful and failed operations
   - Preserve error context (file path, operation type, error message)
   - Support parallel collection (thread-safe)
   - Examples:
     ```rust
     pub struct AnalysisResults<T> {
         pub successes: Vec<T>,
         pub failures: Vec<AnalysisFailure>,
     }

     pub struct AnalysisFailure {
         pub path: PathBuf,
         pub operation: OperationType,
         pub error: anyhow::Error,
     }

     #[derive(Debug, Clone, Copy)]
     pub enum OperationType {
         FileRead,
         FileParse,
         DirectoryAccess,
         Analysis,
     }
     ```

2. **Partition Result Pattern**
   - Replace `.filter_map(|r| r.ok())` with `.partition_result()`
   - Return `(Vec<Success>, Vec<Error>)` from batch operations
   - Works with parallel iterators (rayon)
   - Examples:
     ```rust
     // Before (Spec 201): Errors logged but discarded
     let results: Vec<_> = files.par_iter()
         .filter_map(|f| analyze_file(f)
             .map_err(|e| eprintln!("Warning: {}", e))
             .ok())
         .collect();

     // After (Spec 202): Errors collected
     let (successes, failures): (Vec<_>, Vec<_>) = files.par_iter()
         .map(|f| analyze_file(f).map_err(|e| {
             eprintln!("Warning: {}", e);
             AnalysisFailure::new(f.clone(), OperationType::Analysis, e)
         }))
         .partition_result();
     ```

3. **Error Summary Generation**
   - Categorize errors by type (file read, parse, permission, etc.)
   - Count frequency of each error type
   - Identify most common error patterns
   - Generate human-readable summary
   - Examples:
     ```rust
     pub fn summarize_errors(failures: &[AnalysisFailure]) -> ErrorSummary {
         ErrorSummary {
             total: failures.len(),
             by_operation: group_by_operation(failures),
             by_error_kind: group_by_error_kind(failures),
             sample_errors: take_samples(failures, 5),
         }
     }
     ```

4. **Completion Reporting**
   - Report total files processed
   - Report success vs failure counts
   - Report error breakdown by category
   - Include sample error messages
   - Examples:
     ```
     Analysis Summary:
       Total files found: 100
       Successfully analyzed: 97
       Failed to analyze: 3

     Failure breakdown:
       Permission denied: 2 files
         - src/secret/private.rs
         - tests/restricted/test.rs
       Parse errors: 1 file
         - src/invalid.rs: expected item, found `}`
     ```

5. **High-Impact Batch Operations**
   - File analysis pipelines (`collect_file_metrics`)
   - Directory traversal (`discover_files`)
   - Call graph construction
   - Parallel analysis builders
   - Any operation processing multiple files

### Non-Functional Requirements

1. **Performance**
   - Minimal overhead from error collection
   - Use efficient data structures (Vec, HashMap)
   - No blocking on error aggregation
   - Parallel collection support

2. **User Experience**
   - Clear, actionable error summaries
   - Don't overwhelm with individual errors
   - Highlight patterns and common issues
   - Show sample errors for each category

3. **Developer Experience**
   - Easy to integrate into existing code
   - Type-safe error handling
   - Clear API for batch operations
   - Composable with existing Result types

## Acceptance Criteria

- [ ] `AnalysisResults<T>` type created with successes/failures fields
- [ ] `AnalysisFailure` type captures path, operation, and error
- [ ] `OperationType` enum covers all major operation types
- [ ] `.partition_result()` helper implemented for Result iterators
- [ ] `summarize_errors()` function groups and categorizes errors
- [ ] `report_completion_summary()` function generates user-friendly output
- [ ] `collect_file_metrics()` returns both successes and failures
- [ ] `discover_files()` returns both valid paths and errors
- [ ] Parallel analysis builders collect all errors
- [ ] Call graph construction reports failed files
- [ ] Error summaries tested with sample error sets
- [ ] Integration tests verify error collection in batch operations
- [ ] No performance regression (< 5% overhead)
- [ ] All existing tests pass
- [ ] Manual testing confirms useful error reports

## Technical Details

### Implementation Approach

**Phase 2a: Core Data Structures**

Create error collection types:

```rust
// src/errors/collection.rs

use std::path::PathBuf;
use anyhow::Error;

/// Results from batch analysis operations.
#[derive(Debug, Clone)]
pub struct AnalysisResults<T> {
    pub successes: Vec<T>,
    pub failures: Vec<AnalysisFailure>,
}

impl<T> AnalysisResults<T> {
    pub fn new(successes: Vec<T>, failures: Vec<AnalysisFailure>) -> Self {
        Self { successes, failures }
    }

    pub fn success_count(&self) -> usize {
        self.successes.len()
    }

    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub fn total_count(&self) -> usize {
        self.success_count() + self.failure_count()
    }

    pub fn is_complete_success(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_count() == 0 {
            return 1.0;
        }
        self.success_count() as f64 / self.total_count() as f64
    }
}

/// Information about a failed analysis operation.
#[derive(Debug, Clone)]
pub struct AnalysisFailure {
    pub path: PathBuf,
    pub operation: OperationType,
    pub error: String,  // String for Clone, original Error for details
}

impl AnalysisFailure {
    pub fn new(path: PathBuf, operation: OperationType, error: Error) -> Self {
        Self {
            path,
            operation,
            error: format!("{:#}", error),  // Pretty error format
        }
    }

    pub fn file_read(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::FileRead, error)
    }

    pub fn file_parse(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::FileParse, error)
    }

    pub fn directory_access(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::DirectoryAccess, error)
    }

    pub fn analysis(path: PathBuf, error: Error) -> Self {
        Self::new(path, OperationType::Analysis, error)
    }
}

/// Type of operation that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    FileRead,
    FileParse,
    DirectoryAccess,
    Analysis,
    Other,
}

impl OperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "File read",
            Self::FileParse => "File parse",
            Self::DirectoryAccess => "Directory access",
            Self::Analysis => "Analysis",
            Self::Other => "Other",
        }
    }
}
```

**Phase 2b: Error Summary Generation**

```rust
// src/errors/summary.rs

use std::collections::HashMap;
use super::collection::{AnalysisFailure, OperationType};

/// Summary of errors from batch operations.
#[derive(Debug)]
pub struct ErrorSummary {
    pub total: usize,
    pub by_operation: HashMap<OperationType, usize>,
    pub by_error_kind: HashMap<String, Vec<PathBuf>>,
    pub sample_errors: Vec<AnalysisFailure>,
}

impl ErrorSummary {
    pub fn from_failures(failures: &[AnalysisFailure]) -> Self {
        let total = failures.len();

        // Group by operation type
        let mut by_operation: HashMap<OperationType, usize> = HashMap::new();
        for failure in failures {
            *by_operation.entry(failure.operation).or_insert(0) += 1;
        }

        // Group by error kind (first line of error message)
        let mut by_error_kind: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for failure in failures {
            let error_kind = extract_error_kind(&failure.error);
            by_error_kind
                .entry(error_kind)
                .or_insert_with(Vec::new)
                .push(failure.path.clone());
        }

        // Take samples (up to 5 per category)
        let sample_errors = failures
            .iter()
            .take(10)
            .cloned()
            .collect();

        Self {
            total,
            by_operation,
            by_error_kind,
            sample_errors,
        }
    }

    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("\nFailure breakdown:\n"));

        // Report by operation type
        for (op_type, count) in &self.by_operation {
            report.push_str(&format!("  {}: {} file(s)\n", op_type.as_str(), count));
        }

        // Report by error kind
        report.push_str(&format!("\nError categories:\n"));
        for (error_kind, paths) in &self.by_error_kind {
            report.push_str(&format!("  {}: {} file(s)\n", error_kind, paths.len()));

            // Show first few examples
            for path in paths.iter().take(3) {
                report.push_str(&format!("    - {}\n", path.display()));
            }

            if paths.len() > 3 {
                report.push_str(&format!("    ... and {} more\n", paths.len() - 3));
            }
        }

        report
    }
}

/// Extracts error kind from error message (first line or error type).
fn extract_error_kind(error: &str) -> String {
    // Try to extract error category
    if error.contains("Permission denied") {
        "Permission denied".to_string()
    } else if error.contains("No such file") {
        "File not found".to_string()
    } else if error.contains("parse") || error.contains("expected") {
        "Parse error".to_string()
    } else if error.contains("timeout") {
        "Timeout".to_string()
    } else {
        // Use first line of error
        error.lines().next().unwrap_or("Unknown error").to_string()
    }
}
```

**Phase 2c: Completion Reporting**

```rust
// src/errors/reporting.rs

use super::collection::AnalysisResults;
use super::summary::ErrorSummary;

/// Reports completion summary for batch analysis.
pub fn report_completion_summary<T>(results: &AnalysisResults<T>) {
    eprintln!("\nAnalysis Summary:");
    eprintln!("  Total files processed: {}", results.total_count());
    eprintln!("  Successfully analyzed: {}", results.success_count());
    eprintln!("  Failed to analyze: {}", results.failure_count());
    eprintln!(
        "  Success rate: {:.1}%",
        results.success_rate() * 100.0
    );

    if !results.failures.is_empty() {
        let summary = ErrorSummary::from_failures(&results.failures);
        eprintln!("{}", summary.report());
    }
}

/// Reports brief summary (just counts).
pub fn report_brief_summary<T>(results: &AnalysisResults<T>) {
    if results.is_complete_success() {
        eprintln!(
            "✓ Successfully analyzed {} files",
            results.success_count()
        );
    } else {
        eprintln!(
            "⚠ Analyzed {} files ({} failed)",
            results.success_count(),
            results.failure_count()
        );
    }
}
```

**Phase 2d: Integration with Batch Operations**

```rust
// src/analysis_utils.rs (refactored)

use crate::errors::collection::{AnalysisResults, AnalysisFailure, OperationType};
use crate::errors::reporting::report_completion_summary;

/// Collects file metrics with error collection.
///
/// Returns both successful metrics and failures.
pub fn collect_file_metrics_with_errors(
    files: &[PathBuf]
) -> AnalysisResults<FileMetrics> {
    // Determine files to process (existing logic from Spec 183)
    let max_files = read_max_files_config();
    let files_to_process = determine_files_to_process(files, max_files);

    // Warn if limiting (existing logic from Spec 183)
    if should_warn_file_limit(files.len(), max_files) {
        warn_file_limit(files.len(), files_to_process.len());
    }

    // Analyze files with error collection
    let (successes, failures): (Vec<_>, Vec<_>) = files_to_process
        .par_iter()
        .map(|path| {
            analyze_single_file(path)
                .map_err(|e| {
                    // Log immediately (Spec 201)
                    eprintln!("Warning: Failed to analyze {}: {:#}", path.display(), e);

                    // Collect for summary (Spec 202)
                    AnalysisFailure::analysis(path.clone(), e)
                })
        })
        .partition_result();

    AnalysisResults::new(successes, failures)
}

/// Original function preserved for backward compatibility.
///
/// Returns only successes (for existing code).
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    collect_file_metrics_with_errors(files).successes
}
```

**Phase 2e: Helper Trait for Result Partition**

```rust
// src/errors/partition.rs

use rayon::prelude::*;

/// Extension trait for partitioning Result iterators.
pub trait PartitionResult<T, E>: Iterator<Item = Result<T, E>> + Sized {
    /// Partitions iterator into successes and failures.
    ///
    /// Similar to `Iterator::partition`, but for Results.
    fn partition_result(self) -> (Vec<T>, Vec<E>) {
        self.fold(
            (Vec::new(), Vec::new()),
            |(mut oks, mut errs), result| {
                match result {
                    Ok(val) => oks.push(val),
                    Err(err) => errs.push(err),
                }
                (oks, errs)
            }
        )
    }
}

/// Implement for all Result iterators.
impl<T, E, I> PartitionResult<T, E> for I
where
    I: Iterator<Item = Result<T, E>>,
{
}

/// Extension trait for parallel Result iterators.
pub trait ParPartitionResult<T, E>: ParallelIterator<Item = Result<T, E>> {
    /// Partitions parallel iterator into successes and failures.
    fn partition_result(self) -> (Vec<T>, Vec<E>)
    where
        T: Send,
        E: Send,
    {
        self.fold(
            || (Vec::new(), Vec::new()),
            |(mut oks, mut errs), result| {
                match result {
                    Ok(val) => oks.push(val),
                    Err(err) => errs.push(err),
                }
                (oks, errs)
            }
        )
        .reduce(
            || (Vec::new(), Vec::new()),
            |(mut oks1, mut errs1), (oks2, errs2)| {
                oks1.extend(oks2);
                errs1.extend(errs2);
                (oks1, errs1)
            }
        )
    }
}

impl<T, E, I> ParPartitionResult<T, E> for I
where
    I: ParallelIterator<Item = Result<T, E>>,
    T: Send,
    E: Send,
{
}
```

### Architecture Changes

**Before (Spec 201):**
```
collect_file_metrics(files) -> Vec<FileMetrics>
  ├─ files.par_iter()
  ├─ .map(analyze_file)
  ├─ .map_err(|e| eprintln!("Warning: {}", e))  // ✓ Logged
  ├─ .filter_map(|r| r.ok())                    // ❌ Errors discarded
  └─ .collect()

User sees: Individual warnings, but no summary
```

**After (Spec 202):**
```
collect_file_metrics_with_errors(files) -> AnalysisResults<FileMetrics>
  ├─ files.par_iter()
  ├─ .map(analyze_file)
  ├─ .map_err(|e| {
  │     eprintln!("Warning: {}", e));           // ✓ Logged (Spec 201)
  │     AnalysisFailure::new(...)               // ✓ Collected (Spec 202)
  │  })
  ├─ .partition_result()                        // ✓ Split successes/failures
  └─ AnalysisResults { successes, failures }

report_completion_summary(&results);            // ✓ Summary reported

User sees: Individual warnings + comprehensive summary
```

### Data Structures

```rust
pub mod errors {
    pub mod collection {
        pub struct AnalysisResults<T> { ... }
        pub struct AnalysisFailure { ... }
        pub enum OperationType { ... }
    }

    pub mod summary {
        pub struct ErrorSummary { ... }
    }

    pub mod reporting {
        pub fn report_completion_summary<T>(...);
        pub fn report_brief_summary<T>(...);
    }

    pub mod partition {
        pub trait PartitionResult<T, E> { ... }
        pub trait ParPartitionResult<T, E> { ... }
    }
}
```

### APIs and Interfaces

**New Public APIs:**

```rust
// Error collection types
pub use errors::collection::{AnalysisResults, AnalysisFailure, OperationType};

// Summary and reporting
pub use errors::summary::ErrorSummary;
pub use errors::reporting::{report_completion_summary, report_brief_summary};

// Helper traits
pub use errors::partition::{PartitionResult, ParPartitionResult};

// Batch operations returning AnalysisResults
pub fn collect_file_metrics_with_errors(files: &[PathBuf]) -> AnalysisResults<FileMetrics>;
pub fn discover_files_with_errors(root: &Path) -> AnalysisResults<PathBuf>;
```

**Backward Compatibility:**

```rust
// Existing functions preserved
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    collect_file_metrics_with_errors(files).successes
}
```

## Dependencies

- **Prerequisites**:
  - **Spec 201** (Error Visibility and Logging) - Must be completed first
- **Affected Components**:
  - `src/analysis_utils.rs` - Add error collection
  - `src/builders/parallel_unified_analysis.rs` - Collect parallel errors
  - `src/builders/parallel_call_graph.rs` - Collect call graph errors
  - `src/organization/codebase_type_analyzer.rs` - Collect discovery errors
  - `src/pipeline/stages/standard.rs` - Collect stage errors
  - `src/commands/analyze.rs` - Report summaries
- **External Dependencies**:
  - `anyhow` (already used)
  - `rayon` (already used)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_results_success_count() {
        let results = AnalysisResults {
            successes: vec![1, 2, 3],
            failures: vec![],
        };

        assert_eq!(results.success_count(), 3);
        assert_eq!(results.failure_count(), 0);
        assert_eq!(results.total_count(), 3);
        assert!(results.is_complete_success());
        assert_eq!(results.success_rate(), 1.0);
    }

    #[test]
    fn test_analysis_results_with_failures() {
        let results = AnalysisResults {
            successes: vec![1, 2, 3],
            failures: vec![
                AnalysisFailure::file_read(
                    PathBuf::from("a.rs"),
                    anyhow!("Permission denied")
                ),
                AnalysisFailure::file_parse(
                    PathBuf::from("b.rs"),
                    anyhow!("Parse error")
                ),
            ],
        };

        assert_eq!(results.success_count(), 3);
        assert_eq!(results.failure_count(), 2);
        assert_eq!(results.total_count(), 5);
        assert!(!results.is_complete_success());
        assert_eq!(results.success_rate(), 0.6);
    }

    #[test]
    fn test_error_summary_groups_by_operation() {
        let failures = vec![
            AnalysisFailure::file_read(PathBuf::from("a.rs"), anyhow!("Error 1")),
            AnalysisFailure::file_read(PathBuf::from("b.rs"), anyhow!("Error 2")),
            AnalysisFailure::file_parse(PathBuf::from("c.rs"), anyhow!("Error 3")),
        ];

        let summary = ErrorSummary::from_failures(&failures);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.by_operation[&OperationType::FileRead], 2);
        assert_eq!(summary.by_operation[&OperationType::FileParse], 1);
    }

    #[test]
    fn test_partition_result_sequential() {
        let results = vec![Ok(1), Err("e1"), Ok(2), Ok(3), Err("e2")];

        let (successes, failures) = results.into_iter().partition_result();

        assert_eq!(successes, vec![1, 2, 3]);
        assert_eq!(failures, vec!["e1", "e2"]);
    }

    #[test]
    fn test_partition_result_parallel() {
        let results: Vec<Result<i32, &str>> =
            vec![Ok(1), Err("e1"), Ok(2), Ok(3), Err("e2")];

        let (successes, failures) = results
            .into_par_iter()
            .partition_result();

        assert_eq!(successes.len(), 3);
        assert_eq!(failures.len(), 2);
        assert!(successes.contains(&1));
        assert!(successes.contains(&2));
        assert!(successes.contains(&3));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_collect_file_metrics_with_errors() {
    let files = vec![
        PathBuf::from("tests/fixtures/valid.rs"),
        PathBuf::from("tests/fixtures/invalid.rs"),
        PathBuf::from("tests/fixtures/nonexistent.rs"),
    ];

    let results = collect_file_metrics_with_errors(&files);

    assert!(results.success_count() >= 1);  // At least valid.rs
    assert!(results.failure_count() >= 1);  // At least nonexistent.rs
    assert_eq!(results.total_count(), 3);
}

#[test]
fn test_error_summary_report_format() {
    // Create sample failures
    let failures = vec![
        AnalysisFailure::file_read(
            PathBuf::from("a.rs"),
            anyhow!("Permission denied (os error 13)")
        ),
        AnalysisFailure::file_read(
            PathBuf::from("b.rs"),
            anyhow!("Permission denied (os error 13)")
        ),
        AnalysisFailure::file_parse(
            PathBuf::from("c.rs"),
            anyhow!("expected item, found `}`")
        ),
    ];

    let summary = ErrorSummary::from_failures(&failures);
    let report = summary.report();

    // Verify report contains expected sections
    assert!(report.contains("Failure breakdown:"));
    assert!(report.contains("Error categories:"));
    assert!(report.contains("Permission denied"));
    assert!(report.contains("Parse error"));
}
```

### Manual Testing

```bash
# Test with permission errors
chmod 000 test.rs
cargo run -- analyze .
# Should see:
#   Warning during analysis + summary at end

# Test with parse errors
echo "invalid rust" > bad.rs
cargo run -- analyze .
# Should see:
#   Parse error warning + categorized in summary

# Test with large codebase
cargo run -- analyze /large/codebase
# Should see:
#   Progress + warnings + final summary with statistics
```

## Documentation Requirements

### Code Documentation

```rust
/// Analyzes files and returns both successes and failures.
///
/// This function follows the Stillwater "fail completely" pattern:
/// each file is analyzed independently, and ALL results (both
/// successes and failures) are returned to the caller.
///
/// # Returns
///
/// `AnalysisResults` containing:
/// - `successes`: Successfully analyzed files
/// - `failures`: Files that failed analysis with error details
///
/// # Example
///
/// ```
/// let results = collect_file_metrics_with_errors(&files);
///
/// // Process successes
/// for metrics in results.successes {
///     process_metrics(metrics);
/// }
///
/// // Report failures
/// if !results.failures.is_empty() {
///     report_completion_summary(&results);
/// }
/// ```
pub fn collect_file_metrics_with_errors(
    files: &[PathBuf]
) -> AnalysisResults<FileMetrics> {
    // ...
}
```

### User Documentation

Update README with error reporting information:

```markdown
## Error Reporting

Debtmap reports both successful and failed analysis operations.

When analyzing a codebase, you'll see:
1. Individual warnings as errors occur
2. A summary at the end showing:
   - Total files processed
   - Success/failure counts
   - Error breakdown by category

Example output:
```
Warning: Failed to read src/private.rs: Permission denied
Warning: Failed to parse src/bad.rs: expected item

Analysis Summary:
  Total files processed: 100
  Successfully analyzed: 98
  Failed to analyze: 2
  Success rate: 98.0%

Failure breakdown:
  File read: 1 file(s)
  File parse: 1 file(s)

Error categories:
  Permission denied: 1 file(s)
    - src/private.rs
  Parse error: 1 file(s)
    - src/bad.rs
```
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Error Collection and Reporting

Debtmap follows the Stillwater "fail completely" pattern for batch operations.

### Fail Completely Pattern

For independent operations (analyzing multiple files), debtmap:
1. Processes all items, even if some fail
2. Collects both successes and failures
3. Reports comprehensive results at the end

### Example

```rust
// Batch operation returns both outcomes
let results: AnalysisResults<FileMetrics> = collect_file_metrics_with_errors(&files);

// Process successes
for metrics in results.successes {
    // ...
}

// Report failures
if !results.failures.is_empty() {
    report_completion_summary(&results);
}
```

### Benefits

- Users see complete picture of what succeeded/failed
- Error patterns become visible (e.g., permission issues)
- Actionable information for fixing issues
- Progress isn't hidden behind silent failures
```

## Implementation Notes

### Implementation Order

1. **Core types** (`AnalysisResults`, `AnalysisFailure`, `OperationType`)
2. **Partition helpers** (`PartitionResult` traits)
3. **Summary generation** (`ErrorSummary`)
4. **Reporting functions** (`report_completion_summary`)
5. **Integrate with `collect_file_metrics`**
6. **Integrate with other batch operations**
7. **Add to command handlers**

### Common Pitfalls

1. **Forgetting to report summary** - Always call `report_completion_summary` at end
2. **Losing parallel performance** - Use `par_partition_result` for rayon
3. **Too much detail** - Summarize, don't dump all errors
4. **Breaking existing code** - Preserve old functions for compatibility

### Verification Checklist

For each batch operation:

- [ ] Returns `AnalysisResults<T>` with both successes/failures
- [ ] Uses `.partition_result()` for collection
- [ ] Logs individual errors (Spec 201)
- [ ] Collects errors for summary (Spec 202)
- [ ] Calls `report_completion_summary()` at end
- [ ] Backward compatible version exists (returns just successes)
- [ ] Tests verify error collection works
- [ ] Manual test shows useful summary

## Migration and Compatibility

### Breaking Changes

**None for existing code** - New functions added, old functions preserved.

### New Functions

Callers can opt-in to error collection by using new `_with_errors` variants:

```rust
// Old (still works)
let metrics = collect_file_metrics(&files);

// New (opt-in error collection)
let results = collect_file_metrics_with_errors(&files);
report_completion_summary(&results);
```

### Migration Path

Recommended migration for command handlers:

```rust
// Before
fn handle_analyze_command(config: Config) -> Result<()> {
    let files = discover_files(&config.path)?;
    let metrics = collect_file_metrics(&files);
    format_and_print_results(&metrics);
    Ok(())
}

// After
fn handle_analyze_command(config: Config) -> Result<()> {
    let files = discover_files(&config.path)?;
    let results = collect_file_metrics_with_errors(&files);

    format_and_print_results(&results.successes);

    if !results.failures.is_empty() {
        report_completion_summary(&results);
    }

    Ok(())
}
```

## Success Metrics

- ✅ `AnalysisResults<T>` type created and used
- ✅ `.partition_result()` helper implemented
- ✅ Error summaries group and categorize failures
- ✅ Completion reports show success/failure statistics
- ✅ All batch operations collect errors
- ✅ Command handlers report summaries
- ✅ Users see actionable error information
- ✅ Error patterns visible in output
- ✅ No performance regression (< 5%)
- ✅ All existing tests pass
- ✅ Integration tests verify error collection

## Follow-up Work

After this specification:
- Apply pattern to all batch operations in codebase
- Consider structured error output (JSON) for programmatic access
- Add metrics/telemetry for error patterns
- Create user guide for interpreting error summaries

This specification completes the error handling improvements:
- **Spec 201**: Make errors visible (logging)
- **Spec 202**: Make errors actionable (collection & reporting)
- **Spec 183/187**: Fix root causes (architectural improvements)

## References

- **Spec 201** - Error Visibility and Logging (prerequisite)
- **Spec 183** - Analyzer I/O Separation (architectural fix)
- **Spec 187** - Extract Pure Functions (reduces error-prone patterns)
- **Stillwater PHILOSOPHY.md** - "Fail Completely" pattern for independent operations
- **Stillwater PHILOSOPHY.md** - "Errors Should Tell Stories" principle
- **CLAUDE.md** - Error handling standards

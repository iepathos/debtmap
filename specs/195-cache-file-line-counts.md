---
number: 195
title: Cache File Line Counts Per-File Instead of Per-Function
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 195: Cache File Line Counts Per-File Instead of Per-Function

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

During debt scoring, `calculate_file_line_count()` is called for every function being processed. This function reads the entire file from disk to count lines:

```rust
// src/priority/scoring/construction.rs:33-40
fn calculate_file_line_count(file_path: &Path) -> Option<usize> {
    use crate::metrics::LocCounter;
    let loc_counter = LocCounter::default();
    loc_counter
        .count_file(file_path)  // Reads entire file from disk!
        .ok()
        .map(|count| count.physical_lines)
}
```

**The Problem:**

If a file `foo.rs` contains 10 functions, the same file is read **10 times** to get the identical line count. For a large codebase like Zed with thousands of functions across hundreds of files, this causes:

- Thousands of redundant file reads
- Massive I/O overhead during debt scoring
- Extremely slow "score functions" phase

**Example Impact:**

```
Zed codebase: ~5000 functions across ~800 files
Current: 5000 file reads (one per function)
Optimal: 800 file reads (one per file)
Waste factor: 6.25x redundant I/O
```

The `file_line_count` field was added in spec 204 to cache the value and "avoid re-reading files during `calculate_total_impact`", but the caching happens at the wrong level - it caches per-function instead of per-file.

## Objective

Calculate file line counts **once per unique file** before processing functions, then look up the cached value during debt item creation. This eliminates redundant file I/O and dramatically improves debt scoring performance.

**Performance Target**: Reduce file reads from O(functions) to O(files), typically 5-10x reduction.

## Requirements

### Functional Requirements

1. **Pre-calculate File Line Counts**
   - Calculate line counts once per unique file before debt item creation
   - Store in a HashMap<PathBuf, usize> for O(1) lookup
   - Pass the cache through to debt item construction

2. **Update Construction Functions**
   - Add `file_line_counts: &HashMap<PathBuf, usize>` parameter
   - Look up cached value instead of reading file
   - Fall back to reading file only if not in cache (defensive)

3. **Maintain Existing API**
   - `file_line_count` field on `UnifiedDebtItem` unchanged
   - Same value as before, just computed more efficiently
   - No changes to output or behavior

### Non-Functional Requirements

1. **Performance**
   - File reads reduced from O(functions) to O(files)
   - Memory overhead: ~100 bytes per file (PathBuf + usize)
   - No measurable impact on memory for typical codebases

2. **Pure Core Principle**
   - File reading (I/O) happens at the boundary, once per file
   - Debt item construction receives cached data (pure lookup)
   - Follows Stillwater "pure core, imperative shell" pattern

3. **Backward Compatibility**
   - No changes to public API
   - No changes to output format
   - No changes to `UnifiedDebtItem` structure

## Acceptance Criteria

- [ ] File line counts calculated once per unique file before scoring phase
- [ ] `HashMap<PathBuf, usize>` cache passed to construction functions
- [ ] `build_unified_debt_item` uses cache lookup instead of file read
- [ ] Fallback to file read if path not in cache (defensive coding)
- [ ] Performance test shows file reads reduced from O(functions) to O(files)
- [ ] All existing tests pass unchanged
- [ ] Debt scoring phase completes in reasonable time for large codebases
- [ ] Memory usage increase is negligible (<1MB for 10K files)

## Technical Details

### Implementation Approach

**Phase 1: Create Cache Builder**

```rust
// src/builders/unified_analysis_phases/phases/scoring.rs or new module

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::metrics::LocCounter;

/// Pre-calculate file line counts for all unique files.
///
/// This is an I/O operation that should happen at the boundary,
/// before the pure debt item construction phase.
pub fn build_file_line_count_cache(
    metrics: &[FunctionMetrics],
) -> HashMap<PathBuf, usize> {
    let loc_counter = LocCounter::default();

    // Collect unique file paths
    let unique_files: HashSet<&PathBuf> = metrics
        .iter()
        .map(|m| &m.file)
        .collect();

    // Read each file once
    unique_files
        .into_iter()
        .filter_map(|path| {
            loc_counter
                .count_file(path)
                .ok()
                .map(|count| (path.clone(), count.physical_lines))
        })
        .collect()
}
```

**Phase 2: Update Construction Functions**

```rust
// src/priority/scoring/construction.rs

/// Look up cached file line count (pure function).
fn get_file_line_count(
    file_path: &Path,
    cache: &HashMap<PathBuf, usize>,
) -> Option<usize> {
    cache.get(file_path).copied()
}

// Update build_unified_debt_item signature
fn build_unified_debt_item(
    func: &FunctionMetrics,
    context: DebtAnalysisContext,
    deps: DependencyMetrics,
    file_line_counts: &HashMap<PathBuf, usize>,  // NEW
) -> UnifiedDebtItem {
    // ... existing code ...

    // Replace:
    // let file_line_count = calculate_file_line_count(&func.file);

    // With:
    let file_line_count = get_file_line_count(&func.file, file_line_counts);

    // ... rest of construction ...
}
```

**Phase 3: Update Orchestration**

```rust
// src/builders/unified_analysis_phases/orchestration.rs

pub fn create_unified_analysis(
    ctx: &AnalysisContext,
    call_graph: &CallGraph,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    enriched_metrics: &[FunctionMetrics],
    timings: &mut AnalysisTimings,
) -> UnifiedAnalysis {
    let start = Instant::now();

    // NEW: Pre-calculate file line counts (I/O at boundary)
    let file_line_counts = build_file_line_count_cache(enriched_metrics);

    // ... existing setup ...

    // Score functions with cached line counts
    let debt_items = scoring::process_metrics_to_debt_items(
        enriched_metrics,
        call_graph,
        &test_only_functions,
        ctx.coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        &debt_aggregator,
        Some(&data_flow_graph),
        ctx.risk_analyzer,
        ctx.project_path,
        &file_line_counts,  // NEW: pass cache
    );

    // ... rest of orchestration ...
}
```

**Phase 4: Thread Cache Through Call Chain**

Update all intermediate functions to pass the cache through:

```rust
// scoring.rs
pub fn process_metrics_to_debt_items(
    metrics: &[FunctionMetrics],
    // ... existing params ...
    file_line_counts: &HashMap<PathBuf, usize>,  // NEW
) -> Vec<UnifiedDebtItem> {
    metrics
        .iter()
        .filter(|metric| should_process_metric(metric, call_graph, test_only_functions))
        .flat_map(|metric| {
            create_debt_items_from_metric(
                metric,
                // ... existing params ...
                file_line_counts,  // NEW
            )
        })
        .collect()
}

// construction.rs
pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    // ... existing params ...
    file_line_counts: &HashMap<PathBuf, usize>,  // NEW
) -> Vec<UnifiedDebtItem> {
    // ... pass to build_unified_debt_item ...
}
```

### Architecture Changes

**Before (Per-Function I/O):**
```
process_metrics_to_debt_items
  └─ for each function:
       └─ build_unified_debt_item
            └─ calculate_file_line_count  <- READS FILE FROM DISK
```

**After (Cached Lookup):**
```
create_unified_analysis
  └─ build_file_line_count_cache  <- READS EACH FILE ONCE (I/O boundary)
  └─ process_metrics_to_debt_items
       └─ for each function:
            └─ build_unified_debt_item
                 └─ get_file_line_count  <- O(1) HASH LOOKUP (pure)
```

### Data Structures

```rust
/// Cache of file line counts for efficient lookup.
/// Key: file path, Value: physical line count
pub type FileLineCountCache = HashMap<PathBuf, usize>;
```

### APIs and Interfaces

**New Functions:**

```rust
/// Build cache of file line counts for all unique files in metrics.
///
/// This is an I/O operation that reads each unique file once.
/// Should be called at the boundary before pure debt item construction.
pub fn build_file_line_count_cache(
    metrics: &[FunctionMetrics],
) -> HashMap<PathBuf, usize>;

/// Look up cached file line count (pure function).
fn get_file_line_count(
    file_path: &Path,
    cache: &HashMap<PathBuf, usize>,
) -> Option<usize>;
```

**Modified Functions (signature changes):**

```rust
// Add file_line_counts parameter to:
pub fn process_metrics_to_debt_items(..., file_line_counts: &HashMap<PathBuf, usize>);
pub fn create_debt_items_from_metric(..., file_line_counts: &HashMap<PathBuf, usize>);
pub fn create_unified_debt_item_with_aggregator_and_data_flow(..., file_line_counts: &HashMap<PathBuf, usize>);
fn build_unified_debt_item(..., file_line_counts: &HashMap<PathBuf, usize>);
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/builders/unified_analysis_phases/orchestration.rs` - Add cache building
  - `src/builders/unified_analysis_phases/phases/scoring.rs` - Add cache parameter
  - `src/priority/scoring/construction.rs` - Use cache lookup
- **External Dependencies**: None (uses standard HashMap)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_file_line_count_cache() {
        let metrics = vec![
            create_test_metric("func1", "file1.rs"),
            create_test_metric("func2", "file1.rs"),  // Same file
            create_test_metric("func3", "file2.rs"),
        ];

        let cache = build_file_line_count_cache(&metrics);

        // Should have 2 entries (unique files)
        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key(Path::new("file1.rs")));
        assert!(cache.contains_key(Path::new("file2.rs")));
    }

    #[test]
    fn test_get_file_line_count_cache_hit() {
        let mut cache = HashMap::new();
        cache.insert(PathBuf::from("test.rs"), 100);

        let result = get_file_line_count(Path::new("test.rs"), &cache);
        assert_eq!(result, Some(100));
    }

    #[test]
    fn test_get_file_line_count_cache_miss() {
        let cache = HashMap::new();

        let result = get_file_line_count(Path::new("nonexistent.rs"), &cache);
        assert_eq!(result, None);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_debt_scoring_uses_cached_line_counts() {
    // Create test files
    let temp_dir = tempdir().unwrap();
    let file1 = create_test_file(&temp_dir, "test1.rs", "fn foo() {}\nfn bar() {}");
    let file2 = create_test_file(&temp_dir, "test2.rs", "fn baz() {}");

    let metrics = parse_files(&[file1.clone(), file2.clone()]);

    // Build cache
    let cache = build_file_line_count_cache(&metrics);

    // Verify cache contains correct line counts
    assert_eq!(cache.get(&file1), Some(&2));
    assert_eq!(cache.get(&file2), Some(&1));

    // Verify debt items use cached values
    let debt_items = process_metrics_to_debt_items(&metrics, ..., &cache);

    for item in debt_items {
        if item.location.file == file1 {
            assert_eq!(item.file_line_count, Some(2));
        }
    }
}
```

### Performance Tests

```rust
#[test]
fn test_file_read_count_reduced() {
    // This test verifies the optimization works
    let metrics = generate_metrics_with_many_functions_per_file(
        num_files: 100,
        functions_per_file: 10,
    );

    // Count unique files
    let unique_files: HashSet<_> = metrics.iter().map(|m| &m.file).collect();

    // Build cache - should read each file exactly once
    let cache = build_file_line_count_cache(&metrics);

    // Cache should have one entry per unique file
    assert_eq!(cache.len(), unique_files.len());
    assert_eq!(cache.len(), 100);  // Not 1000!
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Build cache of file line counts for efficient debt item construction.
///
/// # Purpose
///
/// This function pre-calculates file line counts for all unique files
/// before the debt scoring phase. This eliminates redundant file reads
/// during debt item construction, where each function would otherwise
/// trigger a file read.
///
/// # Performance
///
/// - Before: O(functions) file reads (e.g., 5000 reads for 5000 functions)
/// - After: O(files) file reads (e.g., 800 reads for 800 unique files)
///
/// # Pure Core Pattern
///
/// This is an I/O operation at the boundary. The returned HashMap
/// enables pure lookups during debt item construction.
///
/// # Example
///
/// ```rust
/// // At I/O boundary (orchestration)
/// let file_line_counts = build_file_line_count_cache(&metrics);
///
/// // Pure construction (no I/O)
/// let debt_items = process_metrics_to_debt_items(
///     &metrics,
///     // ... other params ...
///     &file_line_counts,  // Pure lookup, no file reads
/// );
/// ```
pub fn build_file_line_count_cache(
    metrics: &[FunctionMetrics],
) -> HashMap<PathBuf, usize>
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Performance Optimizations

### File Line Count Caching (Spec 195)

File line counts are pre-calculated once per unique file before debt scoring:

```
I/O Boundary (Orchestration)
    │
    ▼
build_file_line_count_cache  ◄── Reads each file ONCE
    │
    ▼
HashMap<PathBuf, usize>      ◄── O(1) lookup cache
    │
    ▼
Pure Construction Phase      ◄── No file I/O
    │
    ▼
Debt Items with file_line_count populated
```

This follows the "pure core, imperative shell" pattern from Stillwater.
```

## Implementation Notes

### Implementation Order

1. **Add cache building function** in orchestration or scoring module
2. **Update `build_unified_debt_item`** to use cache lookup
3. **Thread cache through call chain** (process_metrics_to_debt_items, etc.)
4. **Update orchestration** to build cache before scoring phase
5. **Add tests** for cache building and lookup
6. **Verify performance improvement** on large codebase

### Edge Cases

1. **Empty metrics** - Return empty cache
2. **File not readable** - Skip file, return None for that path
3. **Symlinks** - Handle canonicalization if needed
4. **File changes during analysis** - Cache is snapshot at build time (acceptable)

### Potential Gotchas

1. **Function signature changes** - Many functions need the new parameter
2. **Test updates** - Tests calling these functions need to pass cache
3. **Parallel iteration** - Cache building should happen before parallel scoring

## Migration and Compatibility

### Breaking Changes

**Internal only** - No public API changes. All changes are to internal construction functions.

### Migration Steps

1. Add cache building at orchestration level
2. Update function signatures with cache parameter
3. Update call sites to pass cache
4. Verify all tests pass

## Success Metrics

- ✅ File reads reduced from O(functions) to O(files)
- ✅ Debt scoring phase completes in reasonable time for Zed codebase
- ✅ Memory overhead < 1MB for typical codebases
- ✅ All existing tests pass unchanged
- ✅ `file_line_count` field populated correctly on all debt items
- ✅ Follows pure core / imperative shell pattern

## Follow-up Work

After this implementation:

1. **Spec 196**: Parallel debt item scoring with rayon (uses this cache)
2. Consider caching other per-file computations
3. Profile to identify remaining bottlenecks

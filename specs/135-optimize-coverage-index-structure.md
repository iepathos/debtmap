---
number: 135
title: Optimize Coverage Index with Nested HashMap Structure
category: optimization
priority: high
status: draft
dependencies: [134]
created: 2025-10-26
---

# Specification 135: Optimize Coverage Index with Nested HashMap Structure

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 134 (Demangle llvm-cov Function Names)

## Context

### Current Performance Bottleneck

The `CoverageIndex` in `src/risk/coverage_index.rs` uses a flat HashMap structure that requires O(n) linear scans for path-based lookups when exact paths don't match:

```rust
pub struct CoverageIndex {
    // Current: Flat structure with (file, function_name) tuple keys
    by_function: HashMap<(PathBuf, String), FunctionCoverage>,
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,
}
```

**Path Matching Strategies** (lines 131-149) perform O(n) scans:

```rust
fn find_by_path_strategies(&self, query_path: &Path, function_name: &str) -> Option<&FunctionCoverage> {
    // Strategy 1: O(n) iteration through ALL functions
    for ((indexed_path, fname), coverage) in &self.by_function {
        if fname == function_name && query_path.ends_with(indexed_path) {
            return Some(coverage);
        }
    }

    // Strategy 2: Another O(n) scan
    for ((indexed_path, fname), coverage) in &self.by_function {
        if fname == function_name && indexed_path.ends_with(&normalized_query) {
            return Some(coverage);
        }
    }

    // Strategy 3: Yet another O(n) scan for normalized equality
    // ...
}
```

### Performance Impact

**With Spec 134 implemented** (after demangling reduces function count):
- Function count: ~1,500 (down from 18,631)
- Lookups per analysis: ~19,600
- Comparisons per lookup: ~4,500 (3 strategies × 1,500 entries)
- Total operations: ~88 million
- Estimated time: ~1 minute

**With this optimization** (nested HashMap O(1) lookups):
- Function count: ~1,500 (same)
- Lookups per analysis: ~19,600 (same)
- Comparisons per lookup: ~1-3 (O(1) HashMap lookups)
- Total operations: ~19,600-60,000
- Estimated time: ~3 seconds

**Combined with Spec 134**: ~50,000x total speedup (10+ minutes → 3 seconds)

### Why Nested HashMap is Better

**Current Structure** (flat):
```rust
HashMap<(PathBuf, String), FunctionCoverage>
// Key: ("/path/to/file.rs", "function_name")
// Problem: Can't efficiently lookup by file first
```

**Proposed Structure** (nested):
```rust
HashMap<PathBuf, HashMap<String, FunctionCoverage>>
// Outer key: "/path/to/file.rs"
// Inner key: "function_name"
// Benefit: O(1) file lookup, then O(1) function lookup
```

### Real-World Scenario

**Typical Coverage Lookup**:
1. Parser finds function `ChangeTracker::track_changes` in `src/analysis/change_tracker.rs`
2. Needs coverage for scoring
3. Calls `get_function_coverage(&path, "track_changes")`

**Current (O(n))**:
- Iterate through all 1,500 functions
- Compare path and name for each
- ~4,500 string comparisons

**Optimized (O(1))**:
- Hash `src/analysis/change_tracker.rs` → lookup functions in that file (~50 functions)
- Hash `track_changes` → get coverage directly
- ~2 hash operations

**Speedup**: 2,250x per lookup!

## Objective

Restructure `CoverageIndex` to use nested HashMap architecture, eliminating O(n) linear scans and achieving O(1) lookup complexity for coverage queries, reducing coverage index lookup overhead from ~1 minute to ~3 seconds when combined with Spec 134.

## Requirements

### Functional Requirements

1. **Nested HashMap Structure**
   - Replace flat `by_function` HashMap with nested structure
   - Outer map: file path → inner map
   - Inner map: function name → coverage data
   - Maintain O(1) access characteristics

2. **Path Normalization Integration**
   - Support multiple path matching strategies as fallback
   - Try exact path first (O(1))
   - Fall back to suffix matching (O(files) not O(functions))
   - Fall back to normalized equality (O(files) not O(functions))

3. **Backward Compatible API**
   - Maintain all existing public methods
   - No changes to method signatures
   - Same behavior, better performance
   - Transparent to callers

4. **Index Building Optimization**
   - Build nested structure during `from_coverage()`
   - Maintain similar build time (~30ms)
   - No increase in memory usage (same data, different organization)

### Non-Functional Requirements

1. **Performance**
   - Lookup complexity: O(1) for exact matches
   - Lookup complexity: O(files) for path strategies (not O(functions))
   - Build time: < 50ms for 1,500 functions
   - Memory overhead: < 5% increase (HashMap overhead)

2. **Correctness**
   - All existing tests pass without modification
   - Identical results to current implementation
   - No data loss during restructuring
   - Deterministic lookup behavior

3. **Code Quality**
   - Clear separation of lookup strategies
   - Functional programming principles maintained
   - Comprehensive documentation
   - No new clippy warnings

## Acceptance Criteria

- [ ] `CoverageIndex` uses nested `HashMap<PathBuf, HashMap<String, FunctionCoverage>>` structure
- [ ] `get_function_coverage()` performs O(1) exact match lookups
- [ ] Path matching strategies iterate over files (O(files)) not functions (O(functions))
- [ ] All existing coverage tests pass without modification
- [ ] Coverage lookup performance improved by 50-100x
- [ ] Index build time remains under 50ms
- [ ] Memory usage increase is less than 5%
- [ ] No changes to public API signatures
- [ ] Benchmark shows 50-100x speedup for coverage lookups
- [ ] Integration with Spec 134 provides ~50,000x total speedup

## Technical Details

### Implementation Approach

**File**: `src/risk/coverage_index.rs`

**1. Update CoverageIndex Structure**

Current:
```rust
pub struct CoverageIndex {
    by_function: HashMap<(PathBuf, String), FunctionCoverage>,
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,
    stats: CoverageIndexStats,
}
```

Optimized:
```rust
pub struct CoverageIndex {
    /// Nested structure: file -> (function_name -> coverage)
    /// Enables O(1) file lookup followed by O(1) function lookup
    by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>,

    /// Maintain for line-based lookups (unchanged)
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,

    /// Pre-computed set of all file paths for faster iteration in fallback strategies
    file_paths: Vec<PathBuf>,

    stats: CoverageIndexStats,
}
```

**2. Update from_coverage() to Build Nested Structure**

Current (lines 69-92):
```rust
pub fn from_coverage(coverage: &LcovData) -> Self {
    let mut by_function = HashMap::new();
    let mut by_line = HashMap::new();

    for (file_path, functions) in &coverage.functions {
        for func in functions {
            by_function.insert(
                (file_path.clone(), func.name.clone()),
                func.clone()
            );
            // ...
        }
    }
}
```

Optimized:
```rust
pub fn from_coverage(coverage: &LcovData) -> Self {
    let start = Instant::now();

    let mut by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>> = HashMap::new();
    let mut by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>> = HashMap::new();
    let mut total_records = 0;

    for (file_path, functions) in &coverage.functions {
        // Build inner HashMap for this file's functions
        let mut file_functions = HashMap::new();
        let mut line_map = BTreeMap::new();

        for func in functions {
            total_records += 1;

            // Insert into nested structure
            file_functions.insert(func.name.clone(), func.clone());

            // Maintain line-based index
            line_map.insert(func.start_line, func.clone());
        }

        if !file_functions.is_empty() {
            by_file.insert(file_path.clone(), file_functions);
        }

        if !line_map.is_empty() {
            by_line.insert(file_path.clone(), line_map);
        }
    }

    // Pre-compute file paths for faster iteration
    let file_paths: Vec<PathBuf> = by_file.keys().cloned().collect();

    let index_build_time = start.elapsed();
    let total_files = by_file.len();

    CoverageIndex {
        by_file,
        by_line,
        file_paths,
        stats: CoverageIndexStats {
            total_files,
            total_records,
            index_build_time,
            estimated_memory_bytes: total_records * 200 + file_paths.len() * 100,
        },
    }
}
```

**3. Optimize get_function_coverage() for O(1) Lookup**

Current (lines 116-128):
```rust
pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
    // O(1) exact match
    if let Some(f) = self.by_function.get(&(file.to_path_buf(), function_name.to_string())) {
        return Some(f.coverage_percentage / 100.0);
    }

    // O(n) fallback strategies
    self.find_by_path_strategies(file, function_name)
        .map(|f| f.coverage_percentage / 100.0)
}
```

Optimized:
```rust
pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
    // O(1) exact match: file lookup + function lookup
    if let Some(file_functions) = self.by_file.get(file) {
        if let Some(f) = file_functions.get(function_name) {
            return Some(f.coverage_percentage / 100.0);
        }
    }

    // O(files) fallback strategies - much faster than O(functions)
    self.find_by_path_strategies(file, function_name)
        .map(|f| f.coverage_percentage / 100.0)
}
```

**4. Optimize find_by_path_strategies() to Iterate Over Files**

Current (O(functions) - lines 131-149):
```rust
fn find_by_path_strategies(&self, query_path: &Path, function_name: &str) -> Option<&FunctionCoverage> {
    let normalized_query = normalize_path(query_path);

    // Strategy 1: Iterate over ALL functions
    for ((indexed_path, fname), coverage) in &self.by_function {
        if fname == function_name && query_path.ends_with(indexed_path) {
            return Some(coverage);
        }
    }
    // ... more O(n) iterations
}
```

Optimized (O(files)):
```rust
fn find_by_path_strategies(&self, query_path: &Path, function_name: &str) -> Option<&FunctionCoverage> {
    let normalized_query = normalize_path(query_path);

    // Strategy 1: Suffix matching - iterate over FILES not functions
    // For 375 files with ~4 functions each, this is 375 iterations vs 1,500
    for file_path in &self.file_paths {
        if query_path.ends_with(file_path) {
            // O(1) lookup once we find the file
            if let Some(file_functions) = self.by_file.get(file_path) {
                if let Some(coverage) = file_functions.get(function_name) {
                    return Some(coverage);
                }
            }
        }
    }

    // Strategy 2: Reverse suffix matching - iterate over FILES
    for file_path in &self.file_paths {
        if file_path.ends_with(&normalized_query) {
            if let Some(file_functions) = self.by_file.get(file_path) {
                if let Some(coverage) = file_functions.get(function_name) {
                    return Some(coverage);
                }
            }
        }
    }

    // Strategy 3: Normalized equality - iterate over FILES
    let normalized_query = normalize_path(query_path);
    for file_path in &self.file_paths {
        if normalize_path(file_path) == normalized_query {
            if let Some(file_functions) = self.by_file.get(file_path) {
                if let Some(coverage) = file_functions.get(function_name) {
                    return Some(coverage);
                }
            }
        }
    }

    None
}
```

**5. Update Other Methods for Consistency**

```rust
pub fn get_function_coverage_with_line(&self, file: &Path, function_name: &str, line: usize) -> Option<f64> {
    // Try exact name match first (O(1))
    if let Some(coverage) = self.get_function_coverage(file, function_name) {
        return Some(coverage);
    }

    // Fall back to line-based lookup (unchanged, uses by_line)
    if let Some(line_map) = self.by_line.get(file) {
        // Binary search in BTreeMap - O(log m) where m = functions in file
        if let Some(func) = line_map.range(..=line).next_back() {
            return Some(func.1.coverage_percentage / 100.0);
        }
    }

    None
}

pub fn get_function_uncovered_lines(&self, file: &Path, function_name: &str, line: usize) -> Option<Vec<usize>> {
    // O(1) file lookup + O(1) function lookup
    if let Some(file_functions) = self.by_file.get(file) {
        if let Some(func) = file_functions.get(function_name) {
            return Some(func.uncovered_lines.clone());
        }
    }

    // Fall back to line-based lookup
    if let Some(line_map) = self.by_line.get(file) {
        if let Some(func) = line_map.range(..=line).next_back() {
            return Some(func.1.uncovered_lines.clone());
        }
    }

    None
}
```

### Architecture Changes

**Modified Components**:
- `src/risk/coverage_index.rs`: Complete restructuring of internal data structures
- No changes to public API
- No changes to calling code

**Data Structure Transformation**:
```
Before (Flat):
  HashMap {
    ("/file1.rs", "func_a") → FunctionCoverage,
    ("/file1.rs", "func_b") → FunctionCoverage,
    ("/file2.rs", "func_c") → FunctionCoverage,
    ...
  }

After (Nested):
  HashMap {
    "/file1.rs" → HashMap {
      "func_a" → FunctionCoverage,
      "func_b" → FunctionCoverage,
    },
    "/file2.rs" → HashMap {
      "func_c" → FunctionCoverage,
    },
    ...
  }
```

### Performance Analysis

**Lookup Complexity Comparison**:

| Operation | Current | Optimized | Speedup |
|-----------|---------|-----------|---------|
| Exact match | O(1) | O(1) | Same |
| Path strategy 1 | O(n) functions | O(m) files | n/m (4x-50x) |
| Path strategy 2 | O(n) functions | O(m) files | n/m (4x-50x) |
| Path strategy 3 | O(n) functions | O(m) files | n/m (4x-50x) |

Where:
- n = total functions (~1,500 after Spec 134)
- m = total files (~375)
- Average functions per file = n/m ≈ 4

**Real-World Impact**:

With Spec 134 (demangling) already implemented:
- Current: 19,600 lookups × 4,500 comparisons = 88 million ops → ~1 minute
- Optimized: 19,600 lookups × 1-375 comparisons = ~7 million ops → ~3 seconds

**Speedup**: ~12x faster (1 minute → 5 seconds)

**Combined with Spec 134**:
- Original: 19,600 × 56,000 = 1.1 billion ops → 10+ minutes
- Optimized: 19,600 × 1-3 = ~60,000 ops → 3 seconds
- **Total Speedup**: ~50,000x

### Memory Analysis

**Current Memory Usage**:
```
Flat HashMap: 1,500 entries × ~200 bytes = 300KB
```

**Optimized Memory Usage**:
```
Outer HashMap: 375 entries × ~50 bytes = 18.75KB
Inner HashMaps: 375 × ~4 functions × ~200 bytes = 300KB
File paths vector: 375 × ~100 bytes = 37.5KB
Total: ~356KB
```

**Memory Increase**: ~56KB (18% increase, acceptable for 50x speedup)

## Dependencies

**Prerequisites**:
- **Spec 134**: Demangle llvm-cov Function Names
  - Reduces function count from 18k to 1.5k
  - Makes this optimization more impactful
  - Can be implemented independently, but benefits compound

**Affected Components**:
- `src/risk/coverage_index.rs`: Complete rewrite of internal structure
- All coverage-related code: No changes needed (API unchanged)

**External Dependencies**: None - uses standard library HashMap

## Testing Strategy

### Unit Tests

**Test nested structure building**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_structure_from_coverage() {
        let mut coverage = LcovData::new();

        let file1 = PathBuf::from("/path/to/file1.rs");
        let file2 = PathBuf::from("/path/to/file2.rs");

        coverage.functions.insert(file1.clone(), vec![
            FunctionCoverage {
                name: "func_a".to_string(),
                start_line: 10,
                execution_count: 5,
                coverage_percentage: 80.0,
                uncovered_lines: vec![],
            },
            FunctionCoverage {
                name: "func_b".to_string(),
                start_line: 20,
                execution_count: 3,
                coverage_percentage: 60.0,
                uncovered_lines: vec![],
            },
        ]);

        coverage.functions.insert(file2.clone(), vec![
            FunctionCoverage {
                name: "func_c".to_string(),
                start_line: 15,
                execution_count: 10,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            },
        ]);

        let index = CoverageIndex::from_coverage(&coverage);

        // Verify nested structure
        assert_eq!(index.by_file.len(), 2);
        assert_eq!(index.by_file.get(&file1).unwrap().len(), 2);
        assert_eq!(index.by_file.get(&file2).unwrap().len(), 1);

        // Verify O(1) lookups work
        assert_eq!(index.get_function_coverage(&file1, "func_a"), Some(0.8));
        assert_eq!(index.get_function_coverage(&file2, "func_c"), Some(1.0));
    }

    #[test]
    fn test_exact_match_o1_lookup() {
        let index = create_test_index();
        let file = PathBuf::from("/path/to/file.rs");

        // Should find via O(1) exact match
        let coverage = index.get_function_coverage(&file, "test_func");
        assert!(coverage.is_some());
    }

    #[test]
    fn test_path_strategies_iterate_files_not_functions() {
        let index = create_large_test_index(); // 375 files, 1500 functions

        let query = PathBuf::from("/absolute/path/to/file.rs");

        let start = Instant::now();
        let _coverage = index.get_function_coverage(&query, "some_func");
        let elapsed = start.elapsed();

        // Should be fast even with path matching
        assert!(elapsed < Duration::from_millis(10));
    }
}
```

**Test backward compatibility**:
```rust
#[test]
fn test_same_results_as_old_implementation() {
    let coverage = load_test_coverage();

    let new_index = CoverageIndex::from_coverage(&coverage);

    // Test all lookup methods return same results
    for (file, functions) in &coverage.functions {
        for func in functions {
            let new_result = new_index.get_function_coverage(file, &func.name);
            assert_eq!(new_result, Some(func.coverage_percentage / 100.0));
        }
    }
}
```

### Integration Tests

**Test with real coverage data**:
```rust
#[test]
fn test_with_real_llvm_cov_coverage() {
    // Use actual llvm-cov generated lcov (after Spec 134 demangling)
    let lcov_path = "tests/fixtures/llvm_cov_demangled.info";
    let coverage = parse_lcov_file(Path::new(lcov_path)).unwrap();

    let index = CoverageIndex::from_coverage(&coverage);

    // Verify index stats
    assert!(index.stats.total_files > 300);
    assert!(index.stats.total_records > 1000);
    assert!(index.stats.total_records < 3000); // After demangling

    // Verify lookups work
    for (file, functions) in &coverage.functions {
        for func in functions.iter().take(10) { // Sample
            let result = index.get_function_coverage(file, &func.name);
            assert!(result.is_some());
        }
    }
}
```

### Performance Tests

**Benchmark nested vs flat structure**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_coverage_lookups(c: &mut Criterion) {
    let coverage = load_large_coverage(); // 1,500 functions after demangling
    let index = CoverageIndex::from_coverage(&coverage);

    let queries: Vec<(PathBuf, String)> = coverage
        .functions
        .iter()
        .flat_map(|(file, funcs)| {
            funcs.iter().take(50).map(move |f| (file.clone(), f.name.clone()))
        })
        .collect();

    c.bench_function("nested_structure_lookups", |b| {
        b.iter(|| {
            for (file, func_name) in &queries {
                black_box(index.get_function_coverage(file, func_name));
            }
        })
    });
}

criterion_group!(benches, bench_coverage_lookups);
criterion_main!(benches);
```

**Expected Results**:
```
Before (O(n) scans):
  1,000 lookups: 450ms (4,500 comparisons each)

After (O(1) lookups + O(files) fallback):
  1,000 lookups: 8ms (1-3 hash operations each)

Speedup: ~56x
```

## Documentation Requirements

### Code Documentation

**Update CoverageIndex documentation**:
```rust
/// Pre-indexed coverage data for O(1) function lookups
///
/// # Data Structure
///
/// Uses nested HashMap for efficient lookups:
/// - Outer map: file path → functions in that file
/// - Inner map: function name → coverage data
///
/// # Performance Characteristics
///
/// - **Build Time**: O(n) where n = coverage records
/// - **Exact Match Lookup**: O(1) - file hash + function hash
/// - **Path Strategy Lookup**: O(m) where m = number of files
/// - **Memory**: ~200 bytes per function + ~100 bytes per file
///
/// # Lookup Strategies
///
/// 1. **Exact match**: O(1) hash lookups
/// 2. **Suffix matching**: O(files) iteration + O(1) lookup
/// 3. **Normalized equality**: O(files) iteration + O(1) lookup
///
/// The nested structure ensures we only iterate over files (typically ~375)
/// not functions (typically ~1,500), providing 4x-50x speedup for path matching.
#[derive(Debug, Clone)]
pub struct CoverageIndex {
    /// Nested HashMap: file → (function_name → coverage)
    by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>,

    /// Line-based index for range queries (unchanged)
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,

    /// Pre-computed file paths for efficient iteration
    file_paths: Vec<PathBuf>,

    /// Statistics for observability
    stats: CoverageIndexStats,
}
```

### Architecture Documentation

Update `ARCHITECTURE.md`:
```markdown
## Coverage Index Architecture

The coverage index uses a nested HashMap structure for optimal lookup performance:

```
CoverageIndex
├── by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>
│   └── Enables O(1) file lookup + O(1) function lookup
├── by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>
│   └── Enables O(log m) line-based lookup within a file
└── file_paths: Vec<PathBuf>
    └── Pre-computed for O(files) iteration in fallback strategies
```

### Lookup Complexity

- **Exact match**: O(1) - direct hash lookup
- **Path strategies**: O(files) - iterate files, then O(1) lookup
- **Line-based**: O(log functions_in_file) - binary search in BTreeMap

This architecture ensures that even with 1,500 functions across 375 files,
lookups complete in microseconds rather than milliseconds.
```

### Performance Documentation

Update `book/src/parallel-processing.md`:
```markdown
## Coverage Index Optimization

Debtmap uses an optimized nested HashMap structure for coverage lookups:

**Before**: Flat structure required O(n) scans through all functions
**After**: Nested structure provides O(1) exact matches and O(files) fallback

**Performance Impact**:
- Exact match lookups: ~100 nanoseconds
- Path matching fallback: ~10 microseconds (375 file checks vs 1,500 function checks)
- Overall analysis speedup: 50-100x faster coverage lookups

Combined with function name demangling (Spec 134), this provides ~50,000x
total speedup for coverage-enabled analysis.
```

## Implementation Notes

### Design Decisions

1. **Nested HashMap vs Other Structures**:
   - **Considered**: Trie, RadixTree for path matching
   - **Chosen**: Nested HashMap for simplicity and O(1) exact matches
   - **Rationale**: Most lookups are exact matches; path strategies are rare

2. **Pre-computed file_paths Vector**:
   - **Purpose**: Avoid allocating Vec on every fallback lookup
   - **Cost**: ~37KB memory for 375 files
   - **Benefit**: Avoids repeated `.keys().collect()` allocations

3. **Maintain by_line Index**:
   - **Rationale**: Line-based lookups are different use case
   - **Benefit**: BTreeMap provides O(log m) range queries
   - **Trade-off**: Slight memory duplication, but worth it for performance

### Potential Gotchas

1. **Path Canonicalization**:
   - Must ensure consistent path formats
   - Solution: All paths normalized during parsing (Spec 134)

2. **Empty Files**:
   - Files with no functions shouldn't create empty inner HashMaps
   - Solution: Check `!file_functions.is_empty()` before inserting

3. **Memory Fragmentation**:
   - Many small HashMaps could cause fragmentation
   - Mitigation: Pre-size inner HashMaps with estimated capacity

### Optimization Opportunities

**Future enhancements** (out of scope):

1. **Capacity Pre-sizing**:
   ```rust
   let mut file_functions = HashMap::with_capacity(functions.len());
   ```

2. **Path Interning**:
   - Use `Arc<PathBuf>` to avoid path cloning
   - ~10% memory reduction

3. **Lazy Path Matching**:
   - Only compute path strategies on cache miss
   - Most lookups hit exact match

## Migration and Compatibility

### Breaking Changes

**None** - Internal restructuring only, API unchanged.

### Backward Compatibility

- All public methods maintain same signatures
- All existing tests pass without modification
- Performance improvements are transparent
- No changes to calling code required

### Migration Path

**For Users**: No action required - automatic improvement

**For Developers**:
1. Update `CoverageIndex` struct definition
2. Rewrite `from_coverage()` to build nested structure
3. Update lookup methods to use nested structure
4. Run tests to verify correctness
5. Benchmark to confirm performance gains

### Rollback Plan

Simple revert if issues discovered:
1. Restore previous `by_function` flat HashMap
2. Restore O(n) iteration logic
3. No data migration needed
4. Tests validate both implementations

## Success Metrics

### Performance Metrics

- [ ] Index build time: < 50ms (vs ~30ms baseline)
- [ ] Exact match lookups: < 1 microsecond
- [ ] Path strategy lookups: < 10 microseconds
- [ ] Overall coverage lookup speedup: 50-100x
- [ ] Combined with Spec 134: ~50,000x total speedup

### Quality Metrics

- [ ] All existing tests pass
- [ ] No increase in error rates
- [ ] Memory increase < 20%
- [ ] No clippy warnings
- [ ] Code coverage maintained

### Validation

**Before Implementation** (with Spec 134 demangling):
```bash
$ time debtmap analyze . --lcov target/coverage/lcov.info
# ~1 minute total (coverage lookups are bottleneck)
# 1,500 functions after demangling
```

**After Implementation**:
```bash
$ time debtmap analyze . --lcov target/coverage/lcov.info
# ~3-5 seconds total
# Coverage lookups negligible overhead
# O(1) exact matches + O(files) fallback
```

## Interaction with Other Specs

### Spec 134: Demangle llvm-cov Function Names

**Relationship**: Highly complementary

- **Spec 134**: Reduces dataset size (18k → 1.5k functions)
- **Spec 135**: Reduces lookup complexity (O(n) → O(1))
- **Combined**: Multiplicative improvement

**Implementation Order**:
1. Implement Spec 134 first (bigger immediate impact)
2. Measure baseline with demangled names
3. Implement Spec 135 on top
4. Measure combined speedup

**Why Both Are Needed**:
- Spec 134 alone: Still O(n) scans, just over fewer items
- Spec 135 alone: O(1) lookups, but over 18k items
- Together: O(1) lookups over 1.5k items = optimal

**Expected Combined Results**:
```
Original: O(18,631) → 10+ minutes
Spec 134 only: O(1,500) → ~1 minute
Spec 135 only: O(1) on 18,631 items → ~30 seconds
Both: O(1) on 1,500 items → ~3 seconds

Total Speedup: ~50,000x
```

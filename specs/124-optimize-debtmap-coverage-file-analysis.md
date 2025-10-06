---
number: 124
title: Optimize Debtmap Coverage File Analysis Performance
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-05
---

# Specification 124: Optimize Debtmap Coverage File Analysis Performance

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's file analysis phase experiences a 64x performance degradation when coverage data is enabled:

- **Without coverage**: File analysis = 53.6ms
- **With coverage**: File analysis = 3,450ms (64x slower!)

This dramatic slowdown suggests O(n²) behavior or redundant processing in the coverage correlation logic. For medium-sized projects (298 files), this adds ~3.4 seconds of unnecessary analysis time. For large projects, this could become prohibitive.

### Current Performance Profile

```
Without coverage:
🚀 Starting parallel phase 3 (file analysis)...
✅ Phase 3 complete in 53.615792ms (7 file items)

With coverage:
🚀 Starting parallel phase 3 (file analysis)...
✅ Phase 3 complete in 3.450407167s (3 file items)
```

**Key Observations:**
- 64x slowdown despite processing *fewer* file items (7 → 3)
- Other phases show minimal impact from coverage (2-3x slowdown)
- File analysis is the only phase with extreme degradation

### Root Cause Hypotheses

1. **Nested Iteration**: Coverage correlation may be doing O(files × functions × coverage_records)
2. **Redundant Parsing**: Coverage data may be re-parsed for each file instead of once
3. **Lock Contention**: Parallel file analysis may be contending on shared coverage data
4. **Inefficient Lookups**: Linear searches through coverage records instead of hash-based lookups

## Objective

Reduce coverage-enabled file analysis time to be within 3x of no-coverage baseline:
1. Current: 3,450ms with coverage vs 54ms without (64x)
2. Target: ≤ 160ms with coverage (3x of baseline)
3. Maintain accuracy and correctness of coverage correlation
4. Scale to large projects (1000+ files) without degradation

## Requirements

### Functional Requirements

1. **Indexed Coverage Lookups**: Pre-index coverage records by file path for O(1) lookups
2. **Single-Pass Parsing**: Parse coverage data once and cache the results
3. **Parallel-Friendly Design**: Minimize shared state to reduce lock contention
4. **Incremental Processing**: Process file coverage independently without cross-file dependencies

### Non-Functional Requirements

1. **Performance**: File analysis with coverage ≤ 3x slower than without coverage
2. **Scalability**: Maintain performance with 1000+ files and 10,000+ functions
3. **Memory**: Coverage index should use ≤ 50MB for typical projects
4. **Correctness**: Coverage correlation must be 100% accurate (no lost/incorrect data)

## Acceptance Criteria

- [ ] File analysis with coverage completes in ≤ 160ms (3x baseline of 54ms)
- [ ] Performance scales linearly with file count (O(n) not O(n²))
- [ ] Coverage index built once and shared read-only across threads
- [ ] Benchmark shows <3x overhead for coverage correlation
- [ ] Memory usage remains under 100MB for 1000-file projects
- [ ] All existing coverage correlation tests pass
- [ ] New performance regression test prevents future slowdowns

## Technical Details

### Current Architecture (Slow)

```rust
// PROBLEM: Linear search for every function in every file
fn correlate_coverage_for_file(file: &FileMetrics, coverage: &CoverageData) -> FileCoverage {
    let mut file_coverage = FileCoverage::new();

    for function in &file.functions {
        // O(n) search through ALL coverage records!
        for record in &coverage.records {
            if record.file == file.path && record.contains(function.location) {
                file_coverage.add(function, record);
            }
        }
    }

    file_coverage
}

// Time complexity: O(files × functions × coverage_records)
// For 298 files × 11 avg functions × 3000 coverage records ≈ 10M iterations!
```

### Proposed Architecture (Fast)

```rust
/// Pre-indexed coverage data for O(1) lookups
pub struct CoverageIndex {
    by_file: HashMap<PathBuf, FileCoverageData>,
    by_line: HashMap<PathBuf, BTreeMap<usize, CoverageRecord>>,
}

impl CoverageIndex {
    /// Build index once from coverage data (O(n))
    pub fn from_coverage(coverage: &CoverageData) -> Self {
        let mut by_file = HashMap::new();
        let mut by_line = HashMap::new();

        for record in &coverage.records {
            by_file.entry(record.file.clone())
                .or_insert_with(|| FileCoverageData::new())
                .add_record(record);

            by_line.entry(record.file.clone())
                .or_insert_with(|| BTreeMap::new())
                .insert(record.line, record.clone());
        }

        CoverageIndex { by_file, by_line }
    }

    /// Lookup coverage for function (O(log n) via BTreeMap)
    pub fn get_function_coverage(&self, file: &Path, location: &SourceLocation) -> Option<&CoverageRecord> {
        self.by_line.get(file)?
            .range(location.start_line..=location.end_line)
            .next()
            .map(|(_, record)| record)
    }
}

// New correlation logic (O(functions × log(coverage_records_per_file)))
fn correlate_coverage_for_file(file: &FileMetrics, index: &CoverageIndex) -> FileCoverage {
    let mut file_coverage = FileCoverage::new();

    for function in &file.functions {
        // O(log n) lookup in BTreeMap
        if let Some(record) = index.get_function_coverage(&file.path, &function.location) {
            file_coverage.add(function, record);
        }
    }

    file_coverage
}
```

### Implementation Strategy

1. **Phase 1: Profile Current Bottleneck**
   - Add timing instrumentation to coverage correlation
   - Identify exact slow operations (parsing, lookup, correlation)
   - Measure memory allocations during file analysis
   - Document current algorithmic complexity

2. **Phase 2: Build Coverage Index**
   - Create `CoverageIndex` struct with HashMap/BTreeMap
   - Implement index building from LCOV/Cobertura data
   - Add tests for index correctness
   - Benchmark index build time (should be fast)

3. **Phase 3: Optimize File Correlation**
   - Replace linear searches with index lookups
   - Ensure index is built once and shared (Arc<CoverageIndex>)
   - Update parallel file analysis to use shared index
   - Remove redundant coverage data cloning

4. **Phase 4: Verify & Benchmark**
   - Run performance benchmarks on small/medium/large projects
   - Verify no correctness regressions
   - Add regression test to prevent future slowdowns
   - Document performance characteristics

### File Changes Required

**New Files:**
- `src/analysis/coverage/index.rs`: Coverage indexing logic
- `benches/coverage_performance.rs`: Performance benchmarks

**Modified Files:**
- `src/analysis/coverage/mod.rs`: Use CoverageIndex
- `src/analysis/file_analysis.rs`: Pass index to correlation
- `src/analysis/parallel.rs`: Share index across threads
- `tests/integration/coverage_correlation.rs`: Add performance tests

### Data Structures

```rust
pub struct CoverageIndex {
    /// Coverage records indexed by file path
    by_file: HashMap<PathBuf, FileCoverageData>,

    /// Coverage records indexed by file path + line number (for range queries)
    by_line: HashMap<PathBuf, BTreeMap<usize, CoverageRecord>>,

    /// Statistics for debugging
    stats: CoverageIndexStats,
}

pub struct FileCoverageData {
    /// Total lines covered in file
    total_covered: usize,

    /// Total executable lines in file
    total_executable: usize,

    /// Coverage percentage for file
    coverage_percent: f64,

    /// Line-level coverage details
    lines: BTreeMap<usize, LineCoverage>,
}

pub struct CoverageIndexStats {
    pub total_files: usize,
    pub total_records: usize,
    pub index_build_time: Duration,
    pub memory_usage: usize,
}

#[derive(Clone)]
pub struct CoverageRecord {
    pub file: PathBuf,
    pub line: usize,
    pub hits: usize,
    pub is_covered: bool,
}

pub struct LineCoverage {
    pub line_number: usize,
    pub hit_count: usize,
    pub is_branch: bool,
    pub branch_coverage: Option<BranchCoverage>,
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- Coverage parsing module
- File analysis pipeline
- Parallel processing infrastructure
- Benchmarking suite

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_coverage_index_lookup() {
    let coverage = load_test_coverage();
    let index = CoverageIndex::from_coverage(&coverage);

    let location = SourceLocation { file: "src/lib.rs", start_line: 10, end_line: 20 };
    let record = index.get_function_coverage(Path::new("src/lib.rs"), &location);

    assert!(record.is_some());
    assert_eq!(record.unwrap().line, 10);
}

#[test]
fn test_index_build_performance() {
    let coverage = generate_large_coverage(10_000); // 10k records

    let start = Instant::now();
    let index = CoverageIndex::from_coverage(&coverage);
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(100), "Index build should be fast");
    assert_eq!(index.stats.total_records, 10_000);
}

#[test]
fn test_parallel_index_sharing() {
    let coverage = load_test_coverage();
    let index = Arc::new(CoverageIndex::from_coverage(&coverage));

    let files = discover_files(".")?;
    files.par_iter().for_each(|file| {
        // Each thread uses shared index (no cloning)
        let cov = correlate_coverage_for_file(file, &index);
        assert!(cov.is_valid());
    });
}
```

### Performance Benchmarks

```rust
#[bench]
fn bench_coverage_correlation_no_index(b: &mut Bencher) {
    let project = setup_medium_project(); // 298 files
    let coverage = load_coverage_data();

    b.iter(|| {
        // Old linear search approach
        correlate_all_files_linear(&project.files, &coverage)
    });
}

#[bench]
fn bench_coverage_correlation_with_index(b: &mut Bencher) {
    let project = setup_medium_project(); // 298 files
    let coverage = load_coverage_data();
    let index = CoverageIndex::from_coverage(&coverage);

    b.iter(|| {
        // New indexed approach
        correlate_all_files_indexed(&project.files, &index)
    });
}

#[bench]
fn bench_index_build_time(b: &mut Bencher) {
    let coverage = load_coverage_data();

    b.iter(|| {
        CoverageIndex::from_coverage(&coverage)
    });
}
```

### Integration Tests

```rust
#[test]
fn test_file_analysis_performance_regression() {
    let project = TestProject::medium_sized(); // 298 files
    let coverage = project.generate_coverage();

    // Baseline: no coverage
    let start_no_cov = Instant::now();
    let analysis_no_cov = analyze_files(&project.files, None);
    let time_no_cov = start_no_cov.elapsed();

    // With coverage (should be ≤3x baseline)
    let start_with_cov = Instant::now();
    let analysis_with_cov = analyze_files(&project.files, Some(&coverage));
    let time_with_cov = start_with_cov.elapsed();

    let slowdown = time_with_cov.as_millis() / time_no_cov.as_millis();
    assert!(slowdown <= 3, "Coverage should not slow down file analysis >3x (got {}x)", slowdown);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Index for efficient coverage data lookups.
///
/// # Performance Characteristics
///
/// - **Build Time**: O(n) where n = coverage records
/// - **Lookup Time**: O(log m) where m = records per file
/// - **Memory**: ~100 bytes per coverage record
///
/// # Usage
///
/// Build index once and share across threads:
///
/// ```rust
/// let coverage = parse_lcov("coverage.info")?;
/// let index = Arc::new(CoverageIndex::from_coverage(&coverage));
///
/// files.par_iter().for_each(|file| {
///     let cov = correlate_coverage(file, &index);
///     // Process coverage...
/// });
/// ```
pub struct CoverageIndex { /* ... */ }
```

### User Documentation

Add performance section to debtmap guide:

```markdown
## Coverage Performance

Debtmap indexes coverage data for fast correlation:

- **Index build**: One-time O(n) operation
- **Per-file lookup**: O(log m) where m = records per file
- **Memory**: ~100MB for 1M coverage records

### Performance Tips

1. Use `--coverage-file` instead of `--lcov-glob` to avoid re-parsing
2. Pre-process large coverage files with `debtmap coverage-index`
3. Exclude irrelevant files with `.debtmapignore` to reduce dataset
```

### Architecture Updates

Document coverage indexing in `ARCHITECTURE.md`:

```markdown
## Coverage Integration: Performance

Coverage correlation uses a two-level index:

1. **File-level index**: HashMap<PathBuf, FileCoverageData>
   - O(1) lookup by file path
   - Stores file-level aggregate metrics

2. **Line-level index**: BTreeMap<usize, CoverageRecord>
   - O(log n) range queries for function coverage
   - Supports multi-line function coverage

Index is built once and shared read-only across threads via Arc.
```

## Implementation Notes

### Index Build Optimization

**Use BTreeMap for line ranges** because:
- Supports efficient range queries (`range(start..=end)`)
- Maintains sorted order for fast iteration
- Better cache locality than HashMap for sequential access

**Pre-aggregate file metrics** to avoid re-calculating:
- Total coverage percentage per file
- Line/branch coverage statistics
- Function coverage summaries

### Memory Management

**Index size estimation:**
- 3,000 coverage records
- ~200 bytes per record (including HashMap/BTreeMap overhead)
- Total: ~600KB (negligible)

**For very large projects:**
- Consider lazy loading coverage by file
- Add option to disable coverage correlation if memory-constrained
- Provide coverage sampling mode (every Nth record)

### Parallel Processing

**Share index safely:**
```rust
let index = Arc::new(CoverageIndex::from_coverage(&coverage));
files.par_iter().for_each(|file| {
    correlate_coverage_for_file(file, &index); // Read-only access
});
```

**Avoid locks** by making index immutable after build.

## Migration and Compatibility

### Breaking Changes

**None** - this is a performance optimization with no API changes.

### Configuration

Add optional tuning parameters:

```toml
[debtmap.coverage]
# Enable coverage indexing (default: true)
use_index = true

# Index all files upfront vs lazy (default: true for <1000 files)
eager_index = true

# Memory limit for coverage index in MB (default: 500)
max_index_memory_mb = 500
```

### Rollout Plan

1. **Version 0.2.6**: Ship optimization as default behavior
2. **Fallback**: Provide `--no-coverage-index` flag for debugging
3. **Monitoring**: Log index build time and memory usage at debug level
4. **Documentation**: Update performance characteristics in README
